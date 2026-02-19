package cluster

import (
	"context"
	"fmt"
	"io"
	"log"
	"sync"
	"time"

	corev1 "k8s.io/api/core/v1"
	"golang.org/x/sync/errgroup"
)

// Aggregator manages multiple NodeClients and provides a unified cluster view.
type Aggregator struct {
	mu      sync.RWMutex
	clients map[string]*NodeClient // name -> client
}

// NewAggregator creates an aggregator with the given node clients.
func NewAggregator(clients []*NodeClient) *Aggregator {
	m := make(map[string]*NodeClient, len(clients))
	for _, c := range clients {
		m[c.Name] = c
	}
	return &Aggregator{clients: m}
}

// ClusterSummary contains aggregated cluster stats.
type ClusterSummary struct {
	NodeCount    int
	HealthyNodes int
	PodCount     int
	RunningPods  int
	Nodes        []NodeSummary
}

// NodeSummary contains stats for one node.
type NodeSummary struct {
	Name      string
	Healthy   bool
	PodCount  int
	LastPing  time.Time
}

// ListAllPods returns pods from all nodes, annotated with mkube.io/node.
func (a *Aggregator) ListAllPods(ctx context.Context) ([]corev1.Pod, error) {
	a.mu.RLock()
	clients := make([]*NodeClient, 0, len(a.clients))
	for _, c := range a.clients {
		clients = append(clients, c)
	}
	a.mu.RUnlock()

	type result struct {
		pods []corev1.Pod
		node string
	}

	var mu sync.Mutex
	var allPods []corev1.Pod

	g, ctx := errgroup.WithContext(ctx)
	for _, client := range clients {
		c := client
		g.Go(func() error {
			list, err := c.ListPods(ctx)
			if err != nil {
				log.Printf("error listing pods from %s: %v", c.Name, err)
				return nil // don't fail the whole operation
			}
			mu.Lock()
			for i := range list.Items {
				pod := list.Items[i]
				if pod.Annotations == nil {
					pod.Annotations = make(map[string]string)
				}
				pod.Annotations["mkube.io/node"] = c.Name
				allPods = append(allPods, pod)
			}
			mu.Unlock()
			return nil
		})
	}
	_ = g.Wait()
	return allPods, nil
}

// ListAllNodes returns node objects from all clients.
func (a *Aggregator) ListAllNodes(ctx context.Context) ([]corev1.Node, error) {
	a.mu.RLock()
	clients := make([]*NodeClient, 0, len(a.clients))
	for _, c := range a.clients {
		clients = append(clients, c)
	}
	a.mu.RUnlock()

	var mu sync.Mutex
	var nodes []corev1.Node

	g, ctx := errgroup.WithContext(ctx)
	for _, client := range clients {
		c := client
		g.Go(func() error {
			node, err := c.GetNode(ctx)
			if err != nil {
				log.Printf("error getting node from %s: %v", c.Name, err)
				return nil
			}
			mu.Lock()
			nodes = append(nodes, *node)
			mu.Unlock()
			return nil
		})
	}
	_ = g.Wait()
	return nodes, nil
}

// GetPod searches all nodes for the given pod.
func (a *Aggregator) GetPod(ctx context.Context, ns, name string) (*corev1.Pod, string, error) {
	a.mu.RLock()
	clients := make([]*NodeClient, 0, len(a.clients))
	for _, c := range a.clients {
		clients = append(clients, c)
	}
	a.mu.RUnlock()

	for _, c := range clients {
		pod, err := c.GetPod(ctx, ns, name)
		if err == nil {
			if pod.Annotations == nil {
				pod.Annotations = make(map[string]string)
			}
			pod.Annotations["mkube.io/node"] = c.Name
			return pod, c.Name, nil
		}
	}
	return nil, "", fmt.Errorf("pod %s/%s not found on any node", ns, name)
}

// CreatePod routes the pod to the appropriate node.
// If spec.nodeName is set, use that node. Otherwise use the node with fewest pods.
func (a *Aggregator) CreatePod(ctx context.Context, pod *corev1.Pod) (*corev1.Pod, error) {
	a.mu.RLock()
	defer a.mu.RUnlock()

	var target *NodeClient

	// Route by nodeName if specified
	if pod.Spec.NodeName != "" {
		if c, ok := a.clients[pod.Spec.NodeName]; ok {
			target = c
		} else {
			return nil, fmt.Errorf("node %q not found", pod.Spec.NodeName)
		}
	}

	// Least-pods scheduling
	if target == nil {
		minPods := int(^uint(0) >> 1)
		for _, c := range a.clients {
			if !c.IsHealthy() {
				continue
			}
			list, err := c.ListPods(ctx)
			if err != nil {
				continue
			}
			if len(list.Items) < minPods {
				minPods = len(list.Items)
				target = c
			}
		}
	}

	if target == nil {
		return nil, fmt.Errorf("no healthy nodes available")
	}

	return target.CreatePod(ctx, pod)
}

// DeletePod finds the owning node and deletes the pod.
func (a *Aggregator) DeletePod(ctx context.Context, ns, name string) error {
	_, nodeName, err := a.GetPod(ctx, ns, name)
	if err != nil {
		return err
	}

	a.mu.RLock()
	c, ok := a.clients[nodeName]
	a.mu.RUnlock()
	if !ok {
		return fmt.Errorf("node %q not found", nodeName)
	}

	return c.DeletePod(ctx, ns, name)
}

// GetPodLog proxies to the owning node.
func (a *Aggregator) GetPodLog(ctx context.Context, ns, name string) (io.ReadCloser, error) {
	_, nodeName, err := a.GetPod(ctx, ns, name)
	if err != nil {
		return nil, err
	}

	a.mu.RLock()
	c, ok := a.clients[nodeName]
	a.mu.RUnlock()
	if !ok {
		return nil, fmt.Errorf("node %q not found", nodeName)
	}

	return c.GetPodLog(ctx, ns, name)
}

// GetClusterSummary builds an aggregate summary of the cluster.
func (a *Aggregator) GetClusterSummary(ctx context.Context) *ClusterSummary {
	a.mu.RLock()
	clients := make([]*NodeClient, 0, len(a.clients))
	for _, c := range a.clients {
		clients = append(clients, c)
	}
	a.mu.RUnlock()

	summary := &ClusterSummary{
		NodeCount: len(clients),
	}

	for _, c := range clients {
		ns := NodeSummary{
			Name:     c.Name,
			Healthy:  c.IsHealthy(),
			LastPing: c.LastPing(),
		}
		if c.IsHealthy() {
			summary.HealthyNodes++
		}
		if list, err := c.ListPods(ctx); err == nil {
			ns.PodCount = len(list.Items)
			summary.PodCount += len(list.Items)
			for _, pod := range list.Items {
				if pod.Status.Phase == corev1.PodRunning {
					summary.RunningPods++
				}
			}
		}
		summary.Nodes = append(summary.Nodes, ns)
	}

	return summary
}

// RunHealthChecker periodically pings all nodes.
func (a *Aggregator) RunHealthChecker(ctx context.Context) {
	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()

	// Initial check
	a.pingAll(ctx)

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			a.pingAll(ctx)
		}
	}
}

func (a *Aggregator) pingAll(ctx context.Context) {
	a.mu.RLock()
	clients := make([]*NodeClient, 0, len(a.clients))
	for _, c := range a.clients {
		clients = append(clients, c)
	}
	a.mu.RUnlock()

	for _, c := range clients {
		if err := c.Ping(ctx); err != nil {
			log.Printf("health check failed for %s: %v", c.Name, err)
		}
	}
}

// GetNode returns a node by name.
func (a *Aggregator) GetNode(ctx context.Context, name string) (*corev1.Node, error) {
	a.mu.RLock()
	c, ok := a.clients[name]
	a.mu.RUnlock()
	if !ok {
		return nil, fmt.Errorf("node %q not found", name)
	}
	return c.GetNode(ctx)
}

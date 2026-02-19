package cluster

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	corev1 "k8s.io/api/core/v1"
)

// NodeClient wraps HTTP communication with a single mkube node API.
type NodeClient struct {
	Name       string
	Address    string
	httpClient *http.Client
	healthy    bool
	lastPing   time.Time
}

// NewNodeClient creates a client for one mkube node.
func NewNodeClient(name, address string) *NodeClient {
	return &NodeClient{
		Name:    name,
		Address: address,
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
		healthy: true,
	}
}

// Ping checks if the node is reachable.
func (c *NodeClient) Ping(ctx context.Context) error {
	req, err := http.NewRequestWithContext(ctx, "GET", c.Address+"/healthz", nil)
	if err != nil {
		c.healthy = false
		return err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		c.healthy = false
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		c.healthy = false
		return fmt.Errorf("node %s health check returned %d", c.Name, resp.StatusCode)
	}
	c.healthy = true
	c.lastPing = time.Now()
	return nil
}

// IsHealthy returns the cached health status.
func (c *NodeClient) IsHealthy() bool { return c.healthy }

// LastPing returns the time of the last successful ping.
func (c *NodeClient) LastPing() time.Time { return c.lastPing }

// ListPods returns all pods from this node.
func (c *NodeClient) ListPods(ctx context.Context) (*corev1.PodList, error) {
	var list corev1.PodList
	if err := c.getJSON(ctx, "/api/v1/pods", &list); err != nil {
		return nil, err
	}
	return &list, nil
}

// GetPod returns a specific pod.
func (c *NodeClient) GetPod(ctx context.Context, ns, name string) (*corev1.Pod, error) {
	var pod corev1.Pod
	if err := c.getJSON(ctx, fmt.Sprintf("/api/v1/namespaces/%s/pods/%s", ns, name), &pod); err != nil {
		return nil, err
	}
	return &pod, nil
}

// CreatePod creates a pod on this node.
func (c *NodeClient) CreatePod(ctx context.Context, pod *corev1.Pod) (*corev1.Pod, error) {
	var result corev1.Pod
	if err := c.postJSON(ctx, fmt.Sprintf("/api/v1/namespaces/%s/pods", pod.Namespace), pod, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// DeletePod deletes a pod on this node.
func (c *NodeClient) DeletePod(ctx context.Context, ns, name string) error {
	req, err := http.NewRequestWithContext(ctx, "DELETE",
		c.Address+fmt.Sprintf("/api/v1/namespaces/%s/pods/%s", ns, name), nil)
	if err != nil {
		return err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("delete pod failed (%d): %s", resp.StatusCode, string(b))
	}
	return nil
}

// GetPodLog returns a reader for pod logs.
func (c *NodeClient) GetPodLog(ctx context.Context, ns, name string) (io.ReadCloser, error) {
	req, err := http.NewRequestWithContext(ctx, "GET",
		c.Address+fmt.Sprintf("/api/v1/namespaces/%s/pods/%s/log", ns, name), nil)
	if err != nil {
		return nil, err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	if resp.StatusCode >= 400 {
		b, _ := io.ReadAll(resp.Body)
		resp.Body.Close()
		return nil, fmt.Errorf("get pod log failed (%d): %s", resp.StatusCode, string(b))
	}
	return resp.Body, nil
}

// GetNode returns the node object.
func (c *NodeClient) GetNode(ctx context.Context) (*corev1.Node, error) {
	var node corev1.Node
	if err := c.getJSON(ctx, fmt.Sprintf("/api/v1/nodes/%s", c.Name), &node); err != nil {
		return nil, err
	}
	return &node, nil
}

func (c *NodeClient) getJSON(ctx context.Context, path string, result interface{}) error {
	req, err := http.NewRequestWithContext(ctx, "GET", c.Address+path, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "application/json")
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("GET %s returned %d: %s", path, resp.StatusCode, string(b))
	}
	return json.NewDecoder(resp.Body).Decode(result)
}

func (c *NodeClient) postJSON(ctx context.Context, path string, body, result interface{}) error {
	data, err := json.Marshal(body)
	if err != nil {
		return err
	}
	req, err := http.NewRequestWithContext(ctx, "POST", c.Address+path, bytes.NewReader(data))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("POST %s returned %d: %s", path, resp.StatusCode, string(b))
	}
	if result != nil {
		return json.NewDecoder(resp.Body).Decode(result)
	}
	return nil
}

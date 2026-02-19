package ui

import (
	"encoding/json"
	"fmt"
	"html/template"
	"io/fs"
	"log"
	"net/http"
	"strings"
	"time"

	corev1 "k8s.io/api/core/v1"

	"github.com/glennswest/mkube-console/internal/cluster"
)

// Handler serves the web dashboard UI.
type Handler struct {
	aggregator  *cluster.Aggregator
	registryURL string
	logsURL     string
	pages       map[string]*template.Template
	staticFS    http.Handler
}

// View types used by templates.

type PageData struct {
	Title       string
	CurrentNav  string
	Breadcrumbs []Breadcrumb
	Content     interface{}
}

type Breadcrumb struct {
	Label string
	URL   string
}

type DashboardData struct {
	NodeCount    int
	HealthyNodes int
	PodCount     int
	RunningPods  int
	Nodes        []cluster.NodeSummary
	RecentPods   []PodView
}

type PodView struct {
	Name        string
	Namespace   string
	Node        string
	Status      string
	StatusClass string
	IP          string
	Age         string
	Containers  int
	Ready       int
}

type PodDetailData struct {
	Pod         PodView
	Containers  []ContainerView
	Volumes     []VolumeView
	Annotations map[string]string
	Labels      map[string]string
	Node        string
}

type ContainerView struct {
	Name    string
	Image   string
	State   string
	Ready   bool
	Reason  string
}

type VolumeView struct {
	Name      string
	MountPath string
}

type NodeView struct {
	Name         string
	Status       string
	StatusClass  string
	CPU          string
	Memory       string
	Pods         string
	Uptime       string
	Architecture string
	Board        string
	CPULoad      string
}

type NodeDetailData struct {
	Node NodeView
	Pods []PodView
}

type RegistryData struct {
	Available bool
	Repos     []RepoView
}

type RepoView struct {
	Name string
	Tags []string
}

// page name → content template name mapping
var pageContentMap = map[string]string{
	"dashboard":   "dashboard-content",
	"pods":        "pods-content",
	"pod-detail":  "pod-detail-content",
	"nodes":       "nodes-content",
	"node-detail": "node-detail-content",
	"registry":    "registry-content",
	"logs":        "logs-content",
}

// NewHandler creates a new UI handler.
func NewHandler(agg *cluster.Aggregator, registryURL, logsURL string) *Handler {
	funcMap := template.FuncMap{
		"inc":           func(i int) int { return i + 1 },
		"humanTime":     humanTime,
		"humanBytes":    humanBytes,
		"humanDuration": humanDuration,
		"join":          strings.Join,
		"toJSON": func(v interface{}) template.JS {
			b, _ := json.Marshal(v)
			return template.JS(b)
		},
	}

	// Parse shared templates (layout + partials)
	shared := template.Must(
		template.New("").Funcs(funcMap).ParseFS(content, "templates/layout.html", "templates/partials.html"),
	)

	// Build per-page template sets
	pages := make(map[string]*template.Template)
	for pageName, contentName := range pageContentMap {
		clone := template.Must(shared.Clone())
		template.Must(clone.ParseFS(content, "templates/"+pageName+".html"))
		template.Must(clone.New("page-content").Parse(`{{template "` + contentName + `" .}}`))
		pages[pageName] = clone
	}

	staticSub, _ := fs.Sub(content, "static")
	staticHandler := http.StripPrefix("/ui/static/", http.FileServer(http.FS(staticSub)))

	return &Handler{
		aggregator:  agg,
		registryURL: registryURL,
		logsURL:     logsURL,
		pages:       pages,
		staticFS:    staticHandler,
	}
}

// ServeHTTP routes UI requests.
func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	path := strings.TrimPrefix(r.URL.Path, "/ui")
	if path == "" {
		path = "/"
	}

	switch {
	case strings.HasPrefix(path, "/static/"):
		h.staticFS.ServeHTTP(w, r)

	case path == "/" || path == "":
		h.handleDashboard(w, r)

	case path == "/pods":
		h.handlePods(w, r)

	case strings.HasPrefix(path, "/pods/"):
		h.handlePodDetail(w, r, path)

	case path == "/nodes":
		h.handleNodes(w, r)

	case strings.HasPrefix(path, "/nodes/"):
		h.handleNodeDetail(w, r, path)

	case path == "/registry":
		h.handleRegistry(w, r)

	case path == "/logs":
		h.handleLogs(w, r)

	default:
		http.NotFound(w, r)
	}
}

func (h *Handler) isHTMX(r *http.Request) bool {
	return r.Header.Get("HX-Request") == "true"
}

func (h *Handler) render(w http.ResponseWriter, r *http.Request, page string, data PageData) {
	tmpl, ok := h.pages[page]
	if !ok {
		log.Printf("unknown page: %s", page)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "text/html; charset=utf-8")

	var err error
	if h.isHTMX(r) {
		err = tmpl.ExecuteTemplate(w, "page-content", data)
	} else {
		err = tmpl.ExecuteTemplate(w, "layout", data)
	}
	if err != nil {
		log.Printf("template error (%s): %v", page, err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
	}
}

// --- Handlers ---

func (h *Handler) handleDashboard(w http.ResponseWriter, r *http.Request) {
	summary := h.aggregator.GetClusterSummary(r.Context())

	pods, _ := h.aggregator.ListAllPods(r.Context())
	var recentPods []PodView
	for _, pod := range pods {
		recentPods = append(recentPods, buildPodView(pod))
		if len(recentPods) >= 10 {
			break
		}
	}

	h.render(w, r, "dashboard", PageData{
		Title:      "Dashboard",
		CurrentNav: "dashboard",
		Breadcrumbs: []Breadcrumb{{Label: "Dashboard", URL: "/ui/"}},
		Content: DashboardData{
			NodeCount:    summary.NodeCount,
			HealthyNodes: summary.HealthyNodes,
			PodCount:     summary.PodCount,
			RunningPods:  summary.RunningPods,
			Nodes:        summary.Nodes,
			RecentPods:   recentPods,
		},
	})
}

func (h *Handler) handlePods(w http.ResponseWriter, r *http.Request) {
	nsFilter := r.URL.Query().Get("namespace")

	pods, _ := h.aggregator.ListAllPods(r.Context())
	var podViews []PodView
	namespaces := make(map[string]bool)
	for _, pod := range pods {
		namespaces[pod.Namespace] = true
		if nsFilter != "" && pod.Namespace != nsFilter {
			continue
		}
		podViews = append(podViews, buildPodView(pod))
	}

	var nsList []string
	for ns := range namespaces {
		nsList = append(nsList, ns)
	}

	h.render(w, r, "pods", PageData{
		Title:      "Pods",
		CurrentNav: "pods",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Pods", URL: "/ui/pods"},
		},
		Content: struct {
			Pods       []PodView
			Namespaces []string
			Filter     string
		}{Pods: podViews, Namespaces: nsList, Filter: nsFilter},
	})
}

func (h *Handler) handlePodDetail(w http.ResponseWriter, r *http.Request, path string) {
	parts := strings.SplitN(strings.TrimPrefix(path, "/pods/"), "/", 2)
	if len(parts) != 2 {
		http.NotFound(w, r)
		return
	}
	ns, name := parts[0], parts[1]

	pod, nodeName, err := h.aggregator.GetPod(r.Context(), ns, name)
	if err != nil {
		http.NotFound(w, r)
		return
	}

	pv := buildPodView(*pod)
	var containers []ContainerView
	for _, cs := range pod.Status.ContainerStatuses {
		cv := ContainerView{
			Name:  cs.Name,
			Image: cs.Image,
			Ready: cs.Ready,
		}
		switch {
		case cs.State.Running != nil:
			cv.State = "Running"
		case cs.State.Waiting != nil:
			cv.State = "Waiting"
			cv.Reason = cs.State.Waiting.Reason
		case cs.State.Terminated != nil:
			cv.State = "Terminated"
			cv.Reason = cs.State.Terminated.Reason
		}
		containers = append(containers, cv)
	}

	var volumes []VolumeView
	for _, c := range pod.Spec.Containers {
		for _, vm := range c.VolumeMounts {
			volumes = append(volumes, VolumeView{Name: vm.Name, MountPath: vm.MountPath})
		}
	}

	h.render(w, r, "pod-detail", PageData{
		Title:      fmt.Sprintf("Pod: %s", name),
		CurrentNav: "pods",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Pods", URL: "/ui/pods"},
			{Label: name, URL: ""},
		},
		Content: PodDetailData{
			Pod:         pv,
			Containers:  containers,
			Volumes:     volumes,
			Annotations: pod.Annotations,
			Labels:      pod.Labels,
			Node:        nodeName,
		},
	})
}

func (h *Handler) handleNodes(w http.ResponseWriter, r *http.Request) {
	nodes, _ := h.aggregator.ListAllNodes(r.Context())
	var nodeViews []NodeView
	for _, node := range nodes {
		nodeViews = append(nodeViews, buildNodeView(node))
	}

	h.render(w, r, "nodes", PageData{
		Title:      "Nodes",
		CurrentNav: "nodes",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Nodes", URL: "/ui/nodes"},
		},
		Content: struct{ Nodes []NodeView }{Nodes: nodeViews},
	})
}

func (h *Handler) handleNodeDetail(w http.ResponseWriter, r *http.Request, path string) {
	name := strings.TrimPrefix(path, "/nodes/")

	node, err := h.aggregator.GetNode(r.Context(), name)
	if err != nil {
		http.NotFound(w, r)
		return
	}

	nv := buildNodeView(*node)

	pods, _ := h.aggregator.ListAllPods(r.Context())
	var podViews []PodView
	for _, pod := range pods {
		if pod.Annotations["mkube.io/node"] == name {
			podViews = append(podViews, buildPodView(pod))
		}
	}

	h.render(w, r, "node-detail", PageData{
		Title:      fmt.Sprintf("Node: %s", name),
		CurrentNav: "nodes",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Nodes", URL: "/ui/nodes"},
			{Label: name, URL: ""},
		},
		Content: NodeDetailData{Node: nv, Pods: podViews},
	})
}

func (h *Handler) handleRegistry(w http.ResponseWriter, r *http.Request) {
	data := RegistryData{Available: h.registryURL != ""}

	if h.registryURL != "" {
		// Try to fetch catalog from fastregistry
		resp, err := http.Get(h.registryURL + "/v2/_catalog")
		if err == nil && resp.StatusCode == http.StatusOK {
			defer resp.Body.Close()
			var catalog struct {
				Repositories []string `json:"repositories"`
			}
			if json.NewDecoder(resp.Body).Decode(&catalog) == nil {
				for _, repo := range catalog.Repositories {
					rv := RepoView{Name: repo}
					// Fetch tags
					tresp, err := http.Get(h.registryURL + "/v2/" + repo + "/tags/list")
					if err == nil && tresp.StatusCode == http.StatusOK {
						var tagList struct {
							Tags []string `json:"tags"`
						}
						if json.NewDecoder(tresp.Body).Decode(&tagList) == nil {
							rv.Tags = tagList.Tags
						}
						tresp.Body.Close()
					}
					data.Repos = append(data.Repos, rv)
				}
			}
		} else if resp != nil {
			resp.Body.Close()
		}
	}

	h.render(w, r, "registry", PageData{
		Title:      "Registry",
		CurrentNav: "registry",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Registry", URL: "/ui/registry"},
		},
		Content: data,
	})
}

func (h *Handler) handleLogs(w http.ResponseWriter, r *http.Request) {
	pods, _ := h.aggregator.ListAllPods(r.Context())
	var podViews []PodView
	for _, pod := range pods {
		podViews = append(podViews, buildPodView(pod))
	}

	h.render(w, r, "logs", PageData{
		Title:      "Logs",
		CurrentNav: "logs",
		Breadcrumbs: []Breadcrumb{
			{Label: "Dashboard", URL: "/ui/"},
			{Label: "Logs", URL: "/ui/logs"},
		},
		Content: struct {
			Pods    []PodView
			LogsURL string
		}{Pods: podViews, LogsURL: h.logsURL},
	})
}

// --- View Builders ---

func buildPodView(pod corev1.Pod) PodView {
	pv := PodView{
		Name:       pod.Name,
		Namespace:  pod.Namespace,
		Node:       pod.Annotations["mkube.io/node"],
		Status:     string(pod.Status.Phase),
		Containers: len(pod.Spec.Containers),
		IP:         pod.Status.PodIP,
	}
	if pod.Status.StartTime != nil {
		pv.Age = humanDuration(time.Since(pod.Status.StartTime.Time))
	}

	for _, cs := range pod.Status.ContainerStatuses {
		if cs.Ready {
			pv.Ready++
		}
	}

	switch pod.Status.Phase {
	case corev1.PodRunning:
		pv.StatusClass = "badge-success"
	case corev1.PodPending:
		pv.StatusClass = "badge-warning"
	case corev1.PodFailed:
		pv.StatusClass = "badge-error"
	default:
		pv.StatusClass = "badge-info"
	}

	return pv
}

func buildNodeView(node corev1.Node) NodeView {
	nv := NodeView{
		Name:         node.Name,
		Architecture: node.Status.NodeInfo.Architecture,
	}

	// Status from conditions
	nv.Status = "Unknown"
	nv.StatusClass = "badge-warning"
	for _, cond := range node.Status.Conditions {
		if cond.Type == corev1.NodeReady {
			if cond.Status == corev1.ConditionTrue {
				nv.Status = "Ready"
				nv.StatusClass = "badge-success"
			} else {
				nv.Status = "NotReady"
				nv.StatusClass = "badge-error"
			}
		}
	}

	// Resources
	if cpu, ok := node.Status.Capacity[corev1.ResourceCPU]; ok {
		nv.CPU = cpu.String()
	}
	if mem, ok := node.Status.Capacity[corev1.ResourceMemory]; ok {
		nv.Memory = humanBytes(mem.Value())
	}
	if pods, ok := node.Status.Allocatable[corev1.ResourcePods]; ok {
		nv.Pods = pods.String()
	}

	// Annotations
	if node.Annotations != nil {
		nv.Uptime = node.Annotations["mkube.io/uptime"]
		nv.Board = node.Annotations["mkube.io/board"]
		nv.CPULoad = node.Annotations["mkube.io/cpu-load"]
	}

	return nv
}

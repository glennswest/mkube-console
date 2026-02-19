package api

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	"github.com/glennswest/mkube-console/internal/cluster"
	"github.com/glennswest/mkube-console/internal/ui"
)

// Router serves the aggregated K8s API and dashboard UI.
type Router struct {
	aggregator *cluster.Aggregator
	ui         *ui.Handler
	registryURL string
	logsURL     string
}

// NewRouter creates a new API router.
func NewRouter(agg *cluster.Aggregator, registryURL, logsURL string) *Router {
	return &Router{
		aggregator:  agg,
		ui:          ui.NewHandler(agg, registryURL, logsURL),
		registryURL: registryURL,
		logsURL:     logsURL,
	}
}

// RegisterRoutes registers all API and UI routes on the mux.
func (rt *Router) RegisterRoutes(mux *http.ServeMux) {
	// API discovery
	mux.HandleFunc("GET /api", rt.handleAPIVersions)
	mux.HandleFunc("GET /api/v1", rt.handleAPIResources)

	// Pods
	mux.HandleFunc("GET /api/v1/pods", rt.handleListAllPods)
	mux.HandleFunc("GET /api/v1/namespaces/{namespace}/pods", rt.handleListNamespacedPods)
	mux.HandleFunc("GET /api/v1/namespaces/{namespace}/pods/{name}", rt.handleGetPod)
	mux.HandleFunc("POST /api/v1/namespaces/{namespace}/pods", rt.handleCreatePod)
	mux.HandleFunc("DELETE /api/v1/namespaces/{namespace}/pods/{name}", rt.handleDeletePod)
	mux.HandleFunc("GET /api/v1/namespaces/{namespace}/pods/{name}/log", rt.handleGetPodLog)

	// Nodes
	mux.HandleFunc("GET /api/v1/nodes", rt.handleListNodes)
	mux.HandleFunc("GET /api/v1/nodes/{name}", rt.handleGetNode)

	// Health
	mux.HandleFunc("GET /healthz", rt.handleHealthz)

	// Dashboard UI
	mux.Handle("/ui/", rt.ui)
	mux.HandleFunc("GET /", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/" {
			http.Redirect(w, r, "/ui/", http.StatusFound)
			return
		}
		http.NotFound(w, r)
	})
}

func (rt *Router) handleListAllPods(w http.ResponseWriter, r *http.Request) {
	pods, err := rt.aggregator.ListAllPods(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, corev1.PodList{
		TypeMeta: metav1.TypeMeta{APIVersion: "v1", Kind: "PodList"},
		Items:    pods,
	})
}

func (rt *Router) handleListNamespacedPods(w http.ResponseWriter, r *http.Request) {
	ns := r.PathValue("namespace")
	allPods, err := rt.aggregator.ListAllPods(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	var items []corev1.Pod
	for _, pod := range allPods {
		if pod.Namespace == ns {
			items = append(items, pod)
		}
	}
	writeJSON(w, http.StatusOK, corev1.PodList{
		TypeMeta: metav1.TypeMeta{APIVersion: "v1", Kind: "PodList"},
		Items:    items,
	})
}

func (rt *Router) handleGetPod(w http.ResponseWriter, r *http.Request) {
	ns := r.PathValue("namespace")
	name := r.PathValue("name")
	pod, _, err := rt.aggregator.GetPod(r.Context(), ns, name)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	writeJSON(w, http.StatusOK, pod)
}

func (rt *Router) handleCreatePod(w http.ResponseWriter, r *http.Request) {
	ns := r.PathValue("namespace")
	var pod corev1.Pod
	if err := json.NewDecoder(r.Body).Decode(&pod); err != nil {
		http.Error(w, fmt.Sprintf("invalid pod JSON: %v", err), http.StatusBadRequest)
		return
	}
	pod.Namespace = ns
	result, err := rt.aggregator.CreatePod(r.Context(), &pod)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusCreated, result)
}

func (rt *Router) handleDeletePod(w http.ResponseWriter, r *http.Request) {
	ns := r.PathValue("namespace")
	name := r.PathValue("name")
	if err := rt.aggregator.DeletePod(r.Context(), ns, name); err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	writeJSON(w, http.StatusOK, metav1.Status{
		TypeMeta: metav1.TypeMeta{APIVersion: "v1", Kind: "Status"},
		Status:   "Success",
		Message:  fmt.Sprintf("pod %q deleted", name),
	})
}

func (rt *Router) handleGetPodLog(w http.ResponseWriter, r *http.Request) {
	ns := r.PathValue("namespace")
	name := r.PathValue("name")
	rc, err := rt.aggregator.GetPodLog(r.Context(), ns, name)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	defer rc.Close()
	w.Header().Set("Content-Type", "text/plain; charset=utf-8")
	_, _ = io.Copy(w, rc)
}

func (rt *Router) handleListNodes(w http.ResponseWriter, r *http.Request) {
	nodes, err := rt.aggregator.ListAllNodes(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, corev1.NodeList{
		TypeMeta: metav1.TypeMeta{APIVersion: "v1", Kind: "NodeList"},
		Items:    nodes,
	})
}

func (rt *Router) handleGetNode(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	node, err := rt.aggregator.GetNode(r.Context(), name)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	writeJSON(w, http.StatusOK, node)
}

func (rt *Router) handleAPIVersions(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, metav1.APIVersions{
		TypeMeta: metav1.TypeMeta{Kind: "APIVersions"},
		Versions: []string{"v1"},
		ServerAddressByClientCIDRs: []metav1.ServerAddressByClientCIDR{
			{ClientCIDR: "0.0.0.0/0", ServerAddress: r.Host},
		},
	})
}

func (rt *Router) handleAPIResources(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, metav1.APIResourceList{
		TypeMeta:     metav1.TypeMeta{Kind: "APIResourceList"},
		GroupVersion: "v1",
		APIResources: []metav1.APIResource{
			{Name: "pods", Namespaced: true, Kind: "Pod", Verbs: metav1.Verbs{"get", "list", "create", "delete"}},
			{Name: "pods/log", Namespaced: true, Kind: "Pod", Verbs: metav1.Verbs{"get"}},
			{Name: "pods/status", Namespaced: true, Kind: "Pod", Verbs: metav1.Verbs{"get"}},
			{Name: "namespaces", Namespaced: false, Kind: "Namespace", Verbs: metav1.Verbs{"get", "list"}},
			{Name: "nodes", Namespaced: false, Kind: "Node", Verbs: metav1.Verbs{"get", "list"}},
		},
	})
}

func (rt *Router) handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain")
	w.WriteHeader(http.StatusOK)
	_, _ = fmt.Fprintln(w, "ok")
}

func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

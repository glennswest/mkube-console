use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

use crate::helpers::{human_bytes, human_time, parse_age};
use crate::models::k8s;
use crate::models::views::*;
use crate::AppState;

// --- Namespaces ---

#[derive(Template)]
#[template(path = "namespaces.html")]
struct NamespacesTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    namespaces: Vec<NamespaceView>,
}

pub async fn handle_namespaces(State(state): State<AppState>) -> Response {
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();

    let mut ns_map: std::collections::BTreeMap<String, NamespaceView> =
        std::collections::BTreeMap::new();

    for pod in &all_pods {
        let entry = ns_map
            .entry(pod.metadata.namespace.clone())
            .or_insert_with(|| NamespaceView {
                name: pod.metadata.namespace.clone(),
                ..Default::default()
            });
        entry.pod_count += 1;
        match pod.status.phase.as_str() {
            "Running" => entry.running += 1,
            "Pending" => entry.pending += 1,
            "Failed" => entry.failed += 1,
            _ => {}
        }
    }

    let namespaces: Vec<NamespaceView> = ns_map
        .into_values()
        .map(|mut ns| {
            ns.status_class = if ns.failed > 0 {
                "badge-error".to_string()
            } else if ns.pending > 0 {
                "badge-warning".to_string()
            } else {
                "badge-success".to_string()
            };
            ns
        })
        .collect();

    let tmpl = NamespacesTemplate {
        title: "Namespaces".to_string(),
        current_nav: "namespaces".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Namespaces".to_string(),
                url: "/ui/namespaces".to_string(),
            },
        ],
        namespaces,
    };

    render_template(&tmpl)
}

// --- View Builders ---

fn build_pod_view(pod: &k8s::Pod) -> PodView {
    let mut pv = PodView {
        name: pod.metadata.name.clone(),
        namespace: pod.metadata.namespace.clone(),
        node: pod
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get("mkube.io/node"))
            .cloned()
            .unwrap_or_default(),
        status: pod.status.phase.clone(),
        containers: pod.spec.containers.len(),
        ip: pod.status.pod_ip.clone(),
        age: parse_age(&pod.status.start_time),
        ..Default::default()
    };

    for cs in &pod.status.container_statuses {
        if cs.ready {
            pv.ready += 1;
        }
    }

    pv.status_class = match pv.status.as_str() {
        "Running" => "badge-success",
        "Pending" => "badge-warning",
        "Failed" => "badge-error",
        _ => "badge-info",
    }
    .to_string();

    pv
}

fn build_node_view(node: &k8s::Node) -> NodeView {
    let mut nv = NodeView {
        name: node.metadata.name.clone(),
        architecture: node.status.node_info.architecture.clone(),
        status: "Unknown".to_string(),
        status_class: "badge-warning".to_string(),
        ..Default::default()
    };

    for cond in &node.status.conditions {
        if cond.condition_type == "Ready" {
            if cond.status == "True" {
                nv.status = "Ready".to_string();
                nv.status_class = "badge-success".to_string();
            } else {
                nv.status = "NotReady".to_string();
                nv.status_class = "badge-error".to_string();
            }
        }
    }

    if let Some(cpu) = node.status.capacity.get("cpu") {
        nv.cpu = cpu.clone();
    }
    if let Some(mem) = node.status.capacity.get("memory") {
        if let Ok(bytes) = mem.parse::<i64>() {
            nv.memory = human_bytes(bytes);
        } else {
            nv.memory = mem.clone();
        }
    }
    if let Some(pods) = node.status.allocatable.get("pods") {
        nv.pods = pods.clone();
    }

    if let Some(ref annotations) = node.metadata.annotations {
        nv.uptime = annotations
            .get("mkube.io/uptime")
            .cloned()
            .unwrap_or_default();
        nv.board = annotations
            .get("mkube.io/board")
            .cloned()
            .unwrap_or_default();
        nv.cpu_load = annotations
            .get("mkube.io/cpu-load")
            .cloned()
            .unwrap_or_default();
    }

    nv
}

fn build_container_views(pod: &k8s::Pod) -> Vec<ContainerView> {
    pod.status
        .container_statuses
        .iter()
        .map(|cs| {
            let (state, reason) = if cs.state.running.is_some() {
                ("Running".to_string(), String::new())
            } else if let Some(ref w) = cs.state.waiting {
                ("Waiting".to_string(), w.reason.clone())
            } else if let Some(ref t) = cs.state.terminated {
                ("Terminated".to_string(), t.reason.clone())
            } else {
                ("Unknown".to_string(), String::new())
            };
            ContainerView {
                name: cs.name.clone(),
                image: cs.image.clone(),
                state,
                ready: cs.ready,
                reason,
            }
        })
        .collect()
}

fn build_volume_views(pod: &k8s::Pod) -> Vec<VolumeView> {
    pod.spec
        .containers
        .iter()
        .flat_map(|c| {
            c.volume_mounts.iter().map(|vm| VolumeView {
                name: vm.name.clone(),
                mount_path: vm.mount_path.clone(),
            })
        })
        .collect()
}

// --- Template Structs ---

#[derive(Debug, Clone)]
struct Breadcrumb {
    label: String,
    url: String,
}

fn render_template(tmpl: &impl Template) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

// --- Dashboard ---

// Pre-computed node summary for templates
#[derive(Debug, Clone)]
struct DashboardNodeView {
    name: String,
    healthy: bool,
    pod_count: usize,
    last_ping_display: String,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    node_count: usize,
    healthy_nodes: usize,
    pod_count: usize,
    running_pods: usize,
    nodes: Vec<DashboardNodeView>,
    recent_pods: Vec<PodView>,
}

pub async fn handle_dashboard(State(state): State<AppState>) -> Response {
    let summary = state.aggregator.get_cluster_summary().await;

    let pods = state.aggregator.list_all_pods().await.unwrap_or_default();
    let recent_pods: Vec<PodView> = pods.iter().take(10).map(build_pod_view).collect();

    let nodes: Vec<DashboardNodeView> = summary
        .nodes
        .iter()
        .map(|n| DashboardNodeView {
            name: n.name.clone(),
            healthy: n.healthy,
            pod_count: n.pod_count,
            last_ping_display: human_time(n.last_ping),
        })
        .collect();

    let tmpl = DashboardTemplate {
        title: "Dashboard".to_string(),
        current_nav: "dashboard".to_string(),
        breadcrumbs: vec![Breadcrumb {
            label: "Dashboard".to_string(),
            url: "/ui/".to_string(),
        }],
        node_count: summary.node_count,
        healthy_nodes: summary.healthy_nodes,
        pod_count: summary.pod_count,
        running_pods: summary.running_pods,
        nodes,
        recent_pods,
    };

    render_template(&tmpl)
}

// --- Pods ---

#[derive(Deserialize)]
pub struct PodQuery {
    #[serde(default)]
    pub namespace: Option<String>,
}

#[derive(Template)]
#[template(path = "pods.html")]
struct PodsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    pods: Vec<PodView>,
    namespaces: Vec<String>,
    filter: String,
}

pub async fn handle_pods(
    State(state): State<AppState>,
    Query(query): Query<PodQuery>,
) -> Response {
    let ns_filter = query.namespace.unwrap_or_default();
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();

    let mut namespaces = BTreeSet::new();
    let mut pod_views = Vec::new();

    for pod in &all_pods {
        namespaces.insert(pod.metadata.namespace.clone());
        if !ns_filter.is_empty() && pod.metadata.namespace != ns_filter {
            continue;
        }
        pod_views.push(build_pod_view(pod));
    }

    let tmpl = PodsTemplate {
        title: "Pods".to_string(),
        current_nav: "pods".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Pods".to_string(),
                url: "/ui/pods".to_string(),
            },
        ],
        pods: pod_views,
        namespaces: namespaces.into_iter().collect(),
        filter: ns_filter,
    };

    render_template(&tmpl)
}

// --- Pod Detail ---

#[derive(Template)]
#[template(path = "pod_detail.html")]
#[allow(dead_code)]
struct PodDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    pod: PodView,
    containers: Vec<ContainerView>,
    volumes: Vec<VolumeView>,
    annotations: HashMap<String, String>,
    labels: HashMap<String, String>,
    node: String,
}

pub async fn handle_pod_detail(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    let (pod, node_name) = match state.aggregator.get_pod(&namespace, &name).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Pod not found").into_response(),
    };

    let pv = build_pod_view(&pod);
    let containers = build_container_views(&pod);
    let volumes = build_volume_views(&pod);

    let tmpl = PodDetailTemplate {
        title: format!("Pod: {}", name),
        current_nav: "pods".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Pods".to_string(),
                url: "/ui/pods".to_string(),
            },
            Breadcrumb {
                label: name.clone(),
                url: String::new(),
            },
        ],
        pod: pv,
        containers,
        volumes,
        annotations: pod.metadata.annotations.unwrap_or_default(),
        labels: pod.metadata.labels.unwrap_or_default(),
        node: node_name,
    };

    render_template(&tmpl)
}

// --- Nodes ---

#[derive(Template)]
#[template(path = "nodes.html")]
struct NodesTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    nodes: Vec<NodeView>,
}

pub async fn handle_nodes(State(state): State<AppState>) -> Response {
    let all_nodes = state.aggregator.list_all_nodes().await.unwrap_or_default();
    let node_views: Vec<NodeView> = all_nodes.iter().map(build_node_view).collect();

    let tmpl = NodesTemplate {
        title: "Nodes".to_string(),
        current_nav: "nodes".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Nodes".to_string(),
                url: "/ui/nodes".to_string(),
            },
        ],
        nodes: node_views,
    };

    render_template(&tmpl)
}

// --- Node Detail ---

#[derive(Template)]
#[template(path = "node_detail.html")]
struct NodeDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    node: NodeView,
    pods: Vec<PodView>,
}

pub async fn handle_node_detail(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let k8s_node = match state.aggregator.get_node(&name).await {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Node not found").into_response(),
    };

    let nv = build_node_view(&k8s_node);

    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();
    let pod_views: Vec<PodView> = all_pods
        .iter()
        .filter(|p| {
            p.metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get("mkube.io/node"))
                .map(|n| n == &name)
                .unwrap_or(false)
        })
        .map(build_pod_view)
        .collect();

    let tmpl = NodeDetailTemplate {
        title: format!("Node: {}", name),
        current_nav: "nodes".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Nodes".to_string(),
                url: "/ui/nodes".to_string(),
            },
            Breadcrumb {
                label: name.clone(),
                url: String::new(),
            },
        ],
        node: nv,
        pods: pod_views,
    };

    render_template(&tmpl)
}

// --- Registry ---

#[derive(Debug, Clone)]
pub struct RepoView {
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Template)]
#[template(path = "registry.html")]
struct RegistryTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    available: bool,
    repos: Vec<RepoView>,
}

pub async fn handle_registry(State(state): State<AppState>) -> Response {
    let registry_url = state.config.registry_url();
    let available = !registry_url.is_empty();
    let mut repos = Vec::new();

    if available {
        if let Some(catalog) = fetch_catalog(&registry_url).await {
            for repo_name in catalog {
                let tags = fetch_tags(&registry_url, &repo_name).await;
                repos.push(RepoView {
                    name: repo_name,
                    tags,
                });
            }
        }
    }

    let tmpl = RegistryTemplate {
        title: "Registry".to_string(),
        current_nav: "registry".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Registry".to_string(),
                url: "/ui/registry".to_string(),
            },
        ],
        available,
        repos,
    };

    render_template(&tmpl)
}

async fn fetch_catalog(registry_url: &str) -> Option<Vec<String>> {
    #[derive(Deserialize)]
    struct Catalog {
        repositories: Vec<String>,
    }
    let resp: Catalog = reqwest::get(format!("{}/v2/_catalog", registry_url))
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    Some(resp.repositories)
}

async fn fetch_tags(registry_url: &str, repo: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct TagList {
        tags: Option<Vec<String>>,
    }
    let resp = match reqwest::get(format!("{}/v2/{}/tags/list", registry_url, repo)).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    match resp.json::<TagList>().await {
        Ok(t) => t.tags.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// --- Logs ---

#[derive(Template)]
#[template(path = "logs.html")]
#[allow(dead_code)]
struct LogsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    pods: Vec<PodView>,
    logs_url: String,
}

pub async fn handle_logs(State(state): State<AppState>) -> Response {
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();
    let pod_views: Vec<PodView> = all_pods.iter().map(build_pod_view).collect();

    let tmpl = LogsTemplate {
        title: "Logs".to_string(),
        current_nav: "logs".to_string(),
        breadcrumbs: vec![
            Breadcrumb {
                label: "Dashboard".to_string(),
                url: "/ui/".to_string(),
            },
            Breadcrumb {
                label: "Logs".to_string(),
                url: "/ui/logs".to_string(),
            },
        ],
        pods: pod_views,
        logs_url: state.config.logs_url(),
    };

    render_template(&tmpl)
}

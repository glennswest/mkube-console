use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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

// --- Namespace Detail ---

#[derive(Template)]
#[template(path = "namespace_detail.html")]
struct NamespaceDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    namespace_name: String,
    pod_count: usize,
    running: usize,
    pending: usize,
    failed: usize,
    pods: Vec<PodView>,
}

pub async fn handle_namespace_detail(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();

    let mut running = 0usize;
    let mut pending = 0usize;
    let mut failed = 0usize;
    let mut pod_views = Vec::new();

    for pod in &all_pods {
        if pod.metadata.namespace != name {
            continue;
        }
        match pod.status.phase.as_str() {
            "Running" => running += 1,
            "Pending" => pending += 1,
            "Failed" => failed += 1,
            _ => {}
        }
        pod_views.push(build_pod_view(pod));
    }

    let pod_count = pod_views.len();

    let tmpl = NamespaceDetailTemplate {
        title: format!("Namespace: {}", name),
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
            Breadcrumb {
                label: name.clone(),
                url: String::new(),
            },
        ],
        namespace_name: name,
        pod_count,
        running,
        pending,
        failed,
        pods: pod_views,
    };

    render_template(&tmpl)
}

// --- Container Detail ---

#[derive(Template)]
#[template(path = "container_detail.html")]
struct ContainerDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    container: ContainerDetailView,
}

pub async fn handle_container_detail(
    State(state): State<AppState>,
    Path((namespace, pod_name, container_name)): Path<(String, String, String)>,
) -> Response {
    let (pod, _node_name) = match state.aggregator.get_pod(&namespace, &pod_name).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Pod not found").into_response(),
    };

    // Find the container spec
    let container_spec = pod.spec.containers.iter().find(|c| c.name == container_name);

    // Find the container status
    let container_status = pod.status.container_statuses.iter().find(|cs| cs.name == container_name);

    if container_spec.is_none() && container_status.is_none() {
        return (StatusCode::NOT_FOUND, "Container not found").into_response();
    }

    let (state_str, state_class) = if let Some(cs) = container_status {
        if cs.state.running.is_some() {
            ("Running".to_string(), "badge-success".to_string())
        } else if let Some(ref w) = cs.state.waiting {
            (format!("Waiting: {}", w.reason), "badge-warning".to_string())
        } else if let Some(ref t) = cs.state.terminated {
            (format!("Terminated: {}", t.reason), "badge-error".to_string())
        } else {
            ("Unknown".to_string(), "badge-warning".to_string())
        }
    } else {
        ("Unknown".to_string(), "badge-warning".to_string())
    };

    let ready = container_status.map(|cs| cs.ready).unwrap_or(false);
    let image = container_status
        .map(|cs| cs.image.clone())
        .or_else(|| container_spec.map(|c| c.image.clone()))
        .unwrap_or_default();

    let volume_mounts = container_spec
        .map(|c| {
            c.volume_mounts
                .iter()
                .map(|vm| VolumeView {
                    name: vm.name.clone(),
                    mount_path: vm.mount_path.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let detail = ContainerDetailView {
        name: container_name.clone(),
        pod_name: pod_name.clone(),
        namespace: namespace.clone(),
        image,
        state: state_str,
        state_class,
        ready,
        volume_mounts,
    };

    let tmpl = ContainerDetailTemplate {
        title: format!("Container: {}", container_name),
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
            Breadcrumb {
                label: namespace.clone(),
                url: format!("/ui/namespaces/{}", namespace),
            },
            Breadcrumb {
                label: pod_name.clone(),
                url: format!("/ui/namespaces/{}/pods/{}", namespace, pod_name),
            },
            Breadcrumb {
                label: container_name,
                url: String::new(),
            },
        ],
        container: detail,
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
            Breadcrumb {
                label: namespace.clone(),
                url: format!("/ui/namespaces/{}", namespace),
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

// --- Deployments ---

#[derive(Template)]
#[template(path = "deployments.html")]
struct DeploymentsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    deployments: Vec<DeploymentView>,
}

pub async fn handle_deployments(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_deployments().await.unwrap_or_default();
    let deployments: Vec<DeploymentView> = items.iter().map(build_deployment_view).collect();

    let tmpl = DeploymentsTemplate {
        title: "Deployments".to_string(),
        current_nav: "deployments".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Deployments".to_string(), url: "/ui/deployments".to_string() },
        ],
        deployments,
    };
    render_template(&tmpl)
}

#[derive(Template)]
#[template(path = "deployment_detail.html")]
struct DeploymentDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    deploy: DeploymentView,
    pods: Vec<PodView>,
}

pub async fn handle_deployment_detail(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    let dep = match state.aggregator.get_deployment(&namespace, &name).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, "Deployment not found").into_response(),
    };

    let dv = build_deployment_view(&dep);

    // Find pods owned by this deployment
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();
    let pods: Vec<PodView> = all_pods
        .iter()
        .filter(|p| {
            p.metadata.namespace == namespace
                && p.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("vkube.io/owner-deployment"))
                    .map(|v| v == &name)
                    .unwrap_or(false)
        })
        .map(build_pod_view)
        .collect();

    let tmpl = DeploymentDetailTemplate {
        title: format!("Deployment: {}", name),
        current_nav: "deployments".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Deployments".to_string(), url: "/ui/deployments".to_string() },
            Breadcrumb { label: name.clone(), url: String::new() },
        ],
        deploy: dv,
        pods,
    };
    render_template(&tmpl)
}

fn build_deployment_view(d: &k8s::Deployment) -> DeploymentView {
    let status = if d.status.ready_replicas >= d.spec.replicas && d.spec.replicas > 0 {
        "Ready".to_string()
    } else if d.status.ready_replicas > 0 {
        "Degraded".to_string()
    } else {
        "Pending".to_string()
    };
    let status_class = match status.as_str() {
        "Ready" => "badge-success",
        "Degraded" => "badge-warning",
        _ => "badge-info",
    }
    .to_string();

    DeploymentView {
        name: d.metadata.name.clone(),
        namespace: d.metadata.namespace.clone(),
        replicas: d.spec.replicas,
        ready_replicas: d.status.ready_replicas,
        status,
        status_class,
        age: parse_age(&d.metadata.creation_timestamp),
    }
}

// --- Networks ---

#[derive(Template)]
#[template(path = "networks.html")]
struct NetworksTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    networks: Vec<NetworkView>,
}

pub async fn handle_networks(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_networks().await.unwrap_or_default();
    let networks: Vec<NetworkView> = items.iter().map(build_network_view).collect();

    let tmpl = NetworksTemplate {
        title: "Networks".to_string(),
        current_nav: "networks".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Networks".to_string(), url: "/ui/networks".to_string() },
        ],
        networks,
    };
    render_template(&tmpl)
}

#[derive(Template)]
#[template(path = "network_detail.html")]
struct NetworkDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    net: NetworkView,
    reservations: Vec<DHCPReservationView>,
    static_records: Vec<StaticRecordView>,
}

pub async fn handle_network_detail(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let net = match state.aggregator.get_network(&name).await {
        Ok(n) => n,
        Err(_) => return (StatusCode::NOT_FOUND, "Network not found").into_response(),
    };

    let nv = build_network_view(&net);

    let reservations: Vec<DHCPReservationView> = net
        .spec
        .dhcp
        .reservations
        .iter()
        .map(|r| DHCPReservationView {
            mac: r.mac.clone(),
            ip: r.ip.clone(),
            hostname: r.hostname.clone(),
        })
        .collect();

    let static_records: Vec<StaticRecordView> = net
        .spec
        .static_records
        .iter()
        .map(|r| StaticRecordView {
            name: r.name.clone(),
            ip: r.ip.clone(),
        })
        .collect();

    let tmpl = NetworkDetailTemplate {
        title: format!("Network: {}", name),
        current_nav: "networks".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Networks".to_string(), url: "/ui/networks".to_string() },
            Breadcrumb { label: name.clone(), url: String::new() },
        ],
        net: nv,
        reservations,
        static_records,
    };
    render_template(&tmpl)
}

fn build_network_view(n: &k8s::Network) -> NetworkView {
    let status = if n.status.dns_alive {
        "Active".to_string()
    } else if n.spec.external_dns {
        "External".to_string()
    } else {
        "Unknown".to_string()
    };
    let status_class = match status.as_str() {
        "Active" => "badge-success",
        "External" => "badge-info",
        _ => "badge-warning",
    }
    .to_string();

    NetworkView {
        name: n.metadata.name.clone(),
        type_field: n.spec.type_field.clone(),
        cidr: n.spec.cidr.clone(),
        gateway: n.spec.gateway.clone(),
        dns_zone: n.spec.dns.zone.clone(),
        dns_server: n.spec.dns.server.clone(),
        dhcp_enabled: n.spec.dhcp.enabled,
        managed: n.spec.managed,
        dns_alive: n.status.dns_alive,
        pod_count: n.status.pod_count,
        status,
        status_class,
        age: parse_age(&n.metadata.creation_timestamp),
    }
}

// --- PVCs ---

#[derive(Template)]
#[template(path = "pvcs.html")]
struct PVCsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    pvcs: Vec<PVCView>,
}

pub async fn handle_pvcs(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_pvcs().await.unwrap_or_default();
    let pvcs: Vec<PVCView> = items.iter().map(build_pvc_view).collect();

    let tmpl = PVCsTemplate {
        title: "PVCs".to_string(),
        current_nav: "pvcs".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "PVCs".to_string(), url: "/ui/pvcs".to_string() },
        ],
        pvcs,
    };
    render_template(&tmpl)
}

fn build_pvc_view(p: &k8s::PersistentVolumeClaim) -> PVCView {
    let status_class = match p.status.phase.as_str() {
        "Bound" => "badge-success",
        "Pending" => "badge-warning",
        "Lost" => "badge-error",
        _ => "badge-info",
    }
    .to_string();

    let capacity = p
        .status
        .capacity
        .get("storage")
        .cloned()
        .unwrap_or_default();

    PVCView {
        name: p.metadata.name.clone(),
        namespace: p.metadata.namespace.clone(),
        status: p.status.phase.clone(),
        status_class,
        capacity,
        access_modes: p.spec.access_modes.join(", "),
        age: parse_age(&p.metadata.creation_timestamp),
    }
}

// --- BareMetalHosts ---

#[derive(Template)]
#[template(path = "bmhs.html")]
struct BMHsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    bmhs: Vec<BMHView>,
}

pub async fn handle_bmhs(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_bmhs().await.unwrap_or_default();
    let bmhs: Vec<BMHView> = items.iter().map(build_bmh_view).collect();

    let tmpl = BMHsTemplate {
        title: "Bare Metal Hosts".to_string(),
        current_nav: "bmh".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Bare Metal Hosts".to_string(), url: "/ui/bmh".to_string() },
        ],
        bmhs,
    };
    render_template(&tmpl)
}

#[derive(Template)]
#[template(path = "bmh_detail.html")]
struct BMHDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    bmh: BMHView,
}

pub async fn handle_bmh_detail(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    let bmh = match state.aggregator.get_bmh(&namespace, &name).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, "BMH not found").into_response(),
    };

    let bv = build_bmh_view(&bmh);

    let tmpl = BMHDetailTemplate {
        title: format!("BMH: {}", name),
        current_nav: "bmh".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Bare Metal Hosts".to_string(), url: "/ui/bmh".to_string() },
            Breadcrumb { label: name.clone(), url: String::new() },
        ],
        bmh: bv,
    };
    render_template(&tmpl)
}

fn build_bmh_view(b: &k8s::BareMetalHost) -> BMHView {
    let status_class = match b.status.phase.as_str() {
        "Ready" => "badge-success",
        "Provisioning" | "Registering" => "badge-warning",
        "Error" => "badge-error",
        _ => "badge-info",
    }
    .to_string();

    BMHView {
        name: b.metadata.name.clone(),
        namespace: b.metadata.namespace.clone(),
        phase: b.status.phase.clone(),
        status_class,
        powered_on: b.status.powered_on,
        network: b.spec.network.clone(),
        image: b.spec.image.clone(),
        ip: b.spec.ip.clone(),
        mac: b.spec.boot_mac_address.clone(),
        bmc_address: b.spec.bmc.address.clone(),
        bmc_network: b.spec.bmc.network.clone(),
        bmc_username: b.spec.bmc.username.clone(),
        age: parse_age(&b.metadata.creation_timestamp),
    }
}

// --- iSCSI CDROMs ---

#[derive(Template)]
#[template(path = "iscsi_cdroms.html")]
struct ISCSICdromsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    cdroms: Vec<ISCSICdromView>,
}

pub async fn handle_iscsi_cdroms(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_iscsi_cdroms().await.unwrap_or_default();
    let cdroms: Vec<ISCSICdromView> = items.iter().map(build_iscsi_cdrom_view).collect();

    let tmpl = ISCSICdromsTemplate {
        title: "iSCSI CDROMs".to_string(),
        current_nav: "iscsi-cdroms".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "iSCSI CDROMs".to_string(), url: "/ui/iscsi-cdroms".to_string() },
        ],
        cdroms,
    };
    render_template(&tmpl)
}

#[derive(Template)]
#[template(path = "iscsi_cdrom_detail.html")]
struct ISCSICdromDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    cdrom: ISCSICdromView,
    subscribers: Vec<SubscriberView>,
}

pub async fn handle_iscsi_cdrom_detail(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let cdrom = match state.aggregator.get_iscsi_cdrom(&name).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "iSCSI CDROM not found").into_response(),
    };

    let cv = build_iscsi_cdrom_view(&cdrom);

    let subscribers: Vec<SubscriberView> = cdrom
        .status
        .subscribers
        .iter()
        .map(|s| SubscriberView {
            name: s.name.clone(),
            initiator_iqn: s.initiator_iqn.clone(),
            since: parse_age(&Some(s.since.clone())),
        })
        .collect();

    let tmpl = ISCSICdromDetailTemplate {
        title: format!("iSCSI CDROM: {}", name),
        current_nav: "iscsi-cdroms".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "iSCSI CDROMs".to_string(), url: "/ui/iscsi-cdroms".to_string() },
            Breadcrumb { label: name.clone(), url: String::new() },
        ],
        cdrom: cv,
        subscribers,
    };
    render_template(&tmpl)
}

fn build_iscsi_cdrom_view(c: &k8s::ISCSICdrom) -> ISCSICdromView {
    let status_class = match c.status.phase.as_str() {
        "Ready" => "badge-success",
        "Uploading" | "Pending" => "badge-warning",
        "Error" => "badge-error",
        _ => "badge-info",
    }
    .to_string();

    let iso_size_display = if c.status.iso_size == 0 {
        "-".to_string()
    } else {
        human_bytes(c.status.iso_size)
    };

    let portal = if c.status.portal_ip.is_empty() {
        String::new()
    } else {
        format!("{}:{}", c.status.portal_ip, c.status.portal_port)
    };

    ISCSICdromView {
        name: c.metadata.name.clone(),
        phase: c.status.phase.clone(),
        status_class,
        iso_file: c.spec.iso_file.clone(),
        iso_size_display,
        description: c.spec.description.clone(),
        target_iqn: c.status.target_iqn.clone(),
        portal,
        subscriber_count: c.status.subscribers.len(),
        age: parse_age(&c.metadata.creation_timestamp),
    }
}

// --- ConfigMaps ---

#[derive(Template)]
#[template(path = "configmaps.html")]
struct ConfigMapsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    configmaps: Vec<ConfigMapView>,
}

pub async fn handle_configmaps(State(state): State<AppState>) -> Response {
    // Collect configmaps from all namespaces we know about
    let all_pods = state.aggregator.list_all_pods().await.unwrap_or_default();
    let mut namespaces = BTreeSet::new();
    for pod in &all_pods {
        namespaces.insert(pod.metadata.namespace.clone());
    }

    let mut configmaps = Vec::new();
    for ns in &namespaces {
        if let Ok(cms) = state.aggregator.list_configmaps(ns).await {
            for cm in &cms {
                configmaps.push(ConfigMapView {
                    name: cm.metadata.name.clone(),
                    namespace: cm.metadata.namespace.clone(),
                    key_count: cm.data.len(),
                    age: parse_age(&cm.metadata.creation_timestamp),
                });
            }
        }
    }

    let tmpl = ConfigMapsTemplate {
        title: "ConfigMaps".to_string(),
        current_nav: "configmaps".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "ConfigMaps".to_string(), url: "/ui/configmaps".to_string() },
        ],
        configmaps,
    };
    render_template(&tmpl)
}

#[derive(Template)]
#[template(path = "configmap_detail.html")]
struct ConfigMapDetailTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    cm_name: String,
    cm_namespace: String,
    keys: Vec<String>,
    data: HashMap<String, String>,
}

pub async fn handle_configmap_detail(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    let cm = match state.aggregator.get_configmap(&namespace, &name).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "ConfigMap not found").into_response(),
    };

    let mut keys: Vec<String> = cm.data.keys().cloned().collect();
    keys.sort();

    let tmpl = ConfigMapDetailTemplate {
        title: format!("ConfigMap: {}", name),
        current_nav: "configmaps".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "ConfigMaps".to_string(), url: "/ui/configmaps".to_string() },
            Breadcrumb { label: name.clone(), url: String::new() },
        ],
        cm_name: name,
        cm_namespace: namespace,
        keys,
        data: cm.data,
    };
    render_template(&tmpl)
}

// --- Consistency ---

#[derive(Template)]
#[template(path = "consistency.html")]
struct ConsistencyTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    pass_count: usize,
    fail_count: usize,
    warn_count: usize,
    timestamp: String,
    categories: Vec<(String, Vec<CheckItemView>)>,
}

pub async fn handle_consistency(State(state): State<AppState>) -> Response {
    let report = state.aggregator.get_consistency().await.unwrap_or_default();

    let mut categories: Vec<(String, Vec<CheckItemView>)> = Vec::new();

    // Sort categories by name for stable ordering
    let mut sorted_checks: BTreeMap<String, Vec<k8s::CheckItem>> = BTreeMap::new();
    for (k, v) in &report.checks {
        sorted_checks.insert(k.clone(), v.clone());
    }

    for (cat_name, checks) in &sorted_checks {
        let items: Vec<CheckItemView> = checks
            .iter()
            .map(|c| {
                let status_class = match c.status.as_str() {
                    "pass" => "badge-success",
                    "fail" => "badge-error",
                    "warn" => "badge-warning",
                    _ => "badge-info",
                }
                .to_string();
                CheckItemView {
                    name: c.name.clone(),
                    status: c.status.clone(),
                    status_class,
                    message: c.message.clone(),
                    details: c.details.clone(),
                }
            })
            .collect();
        categories.push((cat_name.clone(), items));
    }

    let tmpl = ConsistencyTemplate {
        title: "Consistency".to_string(),
        current_nav: "consistency".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Consistency".to_string(), url: "/ui/consistency".to_string() },
        ],
        pass_count: report.summary.pass,
        fail_count: report.summary.fail,
        warn_count: report.summary.warn,
        timestamp: parse_age(&Some(report.timestamp)),
        categories,
    };
    render_template(&tmpl)
}

// --- Events ---

#[derive(Template)]
#[template(path = "events.html")]
struct EventsTemplate {
    title: String,
    current_nav: String,
    breadcrumbs: Vec<Breadcrumb>,
    events: Vec<EventView>,
}

pub async fn handle_events(State(state): State<AppState>) -> Response {
    let items = state.aggregator.list_events().await.unwrap_or_default();

    let mut events: Vec<EventView> = items
        .iter()
        .map(|e| {
            let type_class = match e.type_field.as_str() {
                "Normal" => "badge-success",
                "Warning" => "badge-warning",
                _ => "badge-info",
            }
            .to_string();

            let involved = if e.involved_object.namespace.is_empty() {
                format!("{}/{}", e.involved_object.kind, e.involved_object.name)
            } else {
                format!(
                    "{}/{}/{}",
                    e.involved_object.kind, e.involved_object.namespace, e.involved_object.name
                )
            };

            EventView {
                namespace: e.metadata.namespace.clone(),
                name: e.metadata.name.clone(),
                reason: e.reason.clone(),
                message: e.message.clone(),
                type_field: e.type_field.clone(),
                type_class,
                involved_object: involved,
                count: e.count,
                age: parse_age(&e.last_timestamp),
            }
        })
        .collect();

    // Show most recent first
    events.reverse();

    let tmpl = EventsTemplate {
        title: "Events".to_string(),
        current_nav: "events".to_string(),
        breadcrumbs: vec![
            Breadcrumb { label: "Dashboard".to_string(), url: "/ui/".to_string() },
            Breadcrumb { label: "Events".to_string(), url: "/ui/events".to_string() },
        ],
        events,
    };
    render_template(&tmpl)
}


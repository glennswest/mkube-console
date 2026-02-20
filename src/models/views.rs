use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct ClusterSummary {
    pub node_count: usize,
    pub healthy_nodes: usize,
    pub pod_count: usize,
    pub running_pods: usize,
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Clone)]
pub struct NodeSummary {
    pub name: String,
    pub healthy: bool,
    pub pod_count: usize,
    pub last_ping: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct PodView {
    pub name: String,
    pub namespace: String,
    pub node: String,
    pub status: String,
    pub status_class: String,
    pub ip: String,
    pub age: String,
    pub containers: usize,
    pub ready: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerView {
    pub name: String,
    pub image: String,
    pub state: String,
    pub ready: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct VolumeView {
    pub name: String,
    pub mount_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct NamespaceView {
    pub name: String,
    pub pod_count: usize,
    pub running: usize,
    pub pending: usize,
    pub failed: usize,
    pub status_class: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerDetailView {
    pub name: String,
    pub pod_name: String,
    pub namespace: String,
    pub image: String,
    pub state: String,
    pub state_class: String,
    pub ready: bool,
    pub volume_mounts: Vec<VolumeView>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeView {
    pub name: String,
    pub status: String,
    pub status_class: String,
    pub cpu: String,
    pub memory: String,
    pub pods: String,
    pub uptime: String,
    pub architecture: String,
    pub board: String,
    pub cpu_load: String,
}

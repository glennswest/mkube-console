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

#[derive(Debug, Clone, Default)]
pub struct DeploymentView {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub ready_replicas: i32,
    pub status: String,
    pub status_class: String,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct PVCView {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub status_class: String,
    pub capacity: String,
    pub access_modes: String,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkView {
    pub name: String,
    pub type_field: String,
    pub cidr: String,
    pub gateway: String,
    pub dns_zone: String,
    pub dns_server: String,
    pub dhcp_enabled: bool,
    pub managed: bool,
    pub dns_alive: bool,
    pub pod_count: i32,
    pub status: String,
    pub status_class: String,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct BMHView {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub status_class: String,
    pub powered_on: bool,
    pub network: String,
    pub image: String,
    pub ip: String,
    pub mac: String,
    pub bmc_address: String,
    pub bmc_network: String,
    pub bmc_username: String,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct ISCSICdromView {
    pub name: String,
    pub phase: String,
    pub status_class: String,
    pub iso_file: String,
    pub iso_size_display: String,
    pub description: String,
    pub target_iqn: String,
    pub portal: String,
    pub subscriber_count: usize,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConfigMapView {
    pub name: String,
    pub namespace: String,
    pub key_count: usize,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct CheckItemView {
    pub name: String,
    pub status: String,
    pub status_class: String,
    pub message: String,
    pub details: String,
}

#[derive(Debug, Clone, Default)]
pub struct EventView {
    pub namespace: String,
    pub name: String,
    pub reason: String,
    pub message: String,
    pub type_field: String,
    pub type_class: String,
    pub involved_object: String,
    pub count: i32,
    pub age: String,
}

#[derive(Debug, Clone, Default)]
pub struct DHCPReservationView {
    pub mac: String,
    pub ip: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Default)]
pub struct StaticRecordView {
    pub name: String,
    pub ip: String,
}

#[derive(Debug, Clone, Default)]
pub struct SubscriberView {
    pub name: String,
    pub initiator_iqn: String,
    pub since: String,
}

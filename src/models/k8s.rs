use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Lightweight K8s-compatible types that serialize to the same JSON as the real K8s API.

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_timestamp: Option<String>,
}

// --- Pod ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Pod {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: PodSpec,
    #[serde(default)]
    pub status: PodStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodSpec {
    #[serde(default)]
    pub node_name: String,
    #[serde(default)]
    pub containers: Vec<Container>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub volume_mounts: Vec<VolumeMount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mount_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default, rename = "podIP")]
    pub pod_ip: String,
    #[serde(default, rename = "hostIP")]
    pub host_ip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default)]
    pub container_statuses: Vec<ContainerStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatus {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub state: ContainerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<ContainerStateRunning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting: Option<ContainerStateWaiting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminated: Option<ContainerStateTerminated>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStateRunning {
    #[serde(default)]
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerStateWaiting {
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerStateTerminated {
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<Pod>,
}

impl Default for PodList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "PodList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- Node ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub status: NodeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    #[serde(default)]
    pub conditions: Vec<NodeCondition>,
    #[serde(default)]
    pub capacity: HashMap<String, String>,
    #[serde(default)]
    pub allocatable: HashMap<String, String>,
    #[serde(default)]
    pub node_info: NodeSystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeCondition {
    #[serde(default, rename = "type")]
    pub condition_type: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeSystemInfo {
    #[serde(default)]
    pub architecture: String,
    #[serde(default)]
    pub os_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<Node>,
}

impl Default for NodeList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "NodeList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- API Discovery ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiVersions {
    pub kind: String,
    pub versions: Vec<String>,
    pub server_address_by_client_cidrs: Vec<ServerAddressByClientCidr>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAddressByClientCidr {
    pub client_cidr: String,
    pub server_address: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResourceList {
    pub kind: String,
    pub group_version: String,
    pub api_resources: Vec<ApiResource>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResource {
    pub name: String,
    pub namespaced: bool,
    pub kind: String,
    pub verbs: Vec<String>,
}

// --- Watch Events ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WatchEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub object: Pod,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub api_version: String,
    pub kind: String,
    pub status: String,
    pub message: String,
}

// --- Deployment ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: DeploymentSpec,
    #[serde(default)]
    pub status: DeploymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentSpec {
    #[serde(default)]
    pub replicas: i32,
    #[serde(default)]
    pub template: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStatus {
    #[serde(default)]
    pub replicas: i32,
    #[serde(default)]
    pub ready_replicas: i32,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<Deployment>,
}

impl Default for DeploymentList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "apps/v1".to_string(),
                kind: "DeploymentList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- Network ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: NetworkSpec,
    #[serde(default)]
    pub status: NetworkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSpec {
    #[serde(default, rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub bridge: String,
    #[serde(default)]
    pub cidr: String,
    #[serde(default)]
    pub gateway: String,
    #[serde(default)]
    pub dns: NetworkDNSSpec,
    #[serde(default)]
    pub dhcp: NetworkDHCPSpec,
    #[serde(default)]
    pub ipam: serde_json::Value,
    #[serde(default)]
    pub external_dns: bool,
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub static_records: Vec<StaticRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDNSSpec {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub server: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDHCPSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub range_start: String,
    #[serde(default)]
    pub range_end: String,
    #[serde(default)]
    pub lease_time: i64,
    #[serde(default)]
    pub next_server: String,
    #[serde(default)]
    pub boot_file: String,
    #[serde(default)]
    pub boot_file_efi: String,
    #[serde(default)]
    pub reservations: Vec<DHCPReservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DHCPReservation {
    #[serde(default)]
    pub mac: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub next_server: String,
    #[serde(default)]
    pub boot_file: String,
    #[serde(default)]
    pub boot_file_efi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StaticRecord {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub dns_alive: bool,
    #[serde(default)]
    pub pod_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<Network>,
}

impl Default for NetworkList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "NetworkList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- PersistentVolumeClaim ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaim {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: PVCSpec,
    #[serde(default)]
    pub status: PVCStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PVCSpec {
    #[serde(default)]
    pub access_modes: Vec<String>,
    #[serde(default)]
    pub resources: ResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    #[serde(default)]
    pub requests: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PVCStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub access_modes: Vec<String>,
    #[serde(default)]
    pub capacity: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PVCList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<PersistentVolumeClaim>,
}

impl Default for PVCList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "PersistentVolumeClaimList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- BareMetalHost ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BareMetalHost {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: BMHSpec,
    #[serde(default)]
    pub status: BMHStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BMHSpec {
    #[serde(default)]
    pub boot_mac_address: String,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub bmc: BMCDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BMCDetails {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub mac: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BMHStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub powered_on: bool,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub last_boot: String,
    #[serde(default)]
    pub boot_count: i32,
    #[serde(default)]
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BMHList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<BareMetalHost>,
}

impl Default for BMHList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "BareMetalHostList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- ISCSICdrom ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ISCSICdrom {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: ISCSICdromSpec,
    #[serde(default)]
    pub status: ISCSICdromStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ISCSICdromSpec {
    #[serde(default)]
    pub iso_file: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ISCSICdromStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub iso_path: String,
    #[serde(default)]
    pub iso_size: i64,
    #[serde(default)]
    pub target_iqn: String,
    #[serde(default)]
    pub portal_ip: String,
    #[serde(default)]
    pub portal_port: i32,
    #[serde(default)]
    pub routeros_id: String,
    #[serde(default)]
    pub subscribers: Vec<ISCSISubscriber>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ISCSISubscriber {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub initiator_iqn: String,
    #[serde(default)]
    pub since: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ISCSICdromList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<ISCSICdrom>,
}

impl Default for ISCSICdromList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "ISCSICdromList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- ConfigMap ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMap {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<ConfigMap>,
}

impl Default for ConfigMapList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "ConfigMapList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

// --- Consistency Report ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyReport {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub summary: ConsistencySummary,
    #[serde(default)]
    pub checks: HashMap<String, Vec<CheckItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencySummary {
    #[serde(default)]
    pub pass: usize,
    #[serde(default)]
    pub fail: usize,
    #[serde(default)]
    pub warn: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub details: String,
}

// --- Event ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub involved_object: InvolvedObject,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub message: String,
    #[serde(default, rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub count: i32,
    #[serde(default)]
    pub first_timestamp: Option<String>,
    #[serde(default)]
    pub last_timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InvolvedObject {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventList {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub items: Vec<Event>,
}

impl Default for EventList {
    fn default() -> Self {
        Self {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "EventList".to_string(),
            },
            items: Vec::new(),
        }
    }
}

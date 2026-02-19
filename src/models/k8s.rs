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
    #[serde(default)]
    pub pod_ip: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub api_version: String,
    pub kind: String,
    pub status: String,
    pub message: String,
}

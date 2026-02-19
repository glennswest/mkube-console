use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_cluster_name")]
    pub cluster_name: String,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    #[serde(default)]
    pub nodes: Vec<NodeDef>,
    #[serde(default)]
    pub routeros: Option<RouterOsConfig>,
    #[serde(default)]
    pub mkube: Option<MkubeConfig>,
    #[serde(default)]
    pub registry: Option<RegistryConfig>,
    #[serde(default)]
    pub logs_url: Option<String>,
    #[serde(default)]
    pub networks: Vec<NetworkDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDef {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterOsConfig {
    pub base_url: String,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MkubeConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkDef {
    pub name: String,
    #[serde(default)]
    pub bridge: Option<String>,
    #[serde(default)]
    pub cidr: Option<String>,
    #[serde(default)]
    pub gateway: Option<String>,
    #[serde(default)]
    pub dns_endpoint: Option<String>,
}

fn default_cluster_name() -> String {
    "mkube".to_string()
}

fn default_listen_port() -> u16 {
    9090
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("reading config {}: {}", path.display(), e))?;
        let mut cfg: Config =
            serde_yaml::from_str(&data).map_err(|e| format!("parsing config: {}", e))?;

        // If no explicit nodes but mkube.base_url is set, derive a single node
        if cfg.nodes.is_empty() {
            if let Some(ref mkube) = cfg.mkube {
                cfg.nodes.push(NodeDef {
                    name: cfg.cluster_name.clone(),
                    address: mkube.base_url.clone(),
                });
            }
        }

        if cfg.nodes.is_empty() {
            return Err("at least one node or mkube.base_url must be configured".into());
        }

        Ok(cfg)
    }

    pub fn listen_addr(&self) -> String {
        format!("0.0.0.0:{}", self.listen_port)
    }

    pub fn registry_url(&self) -> String {
        self.registry
            .as_ref()
            .map(|r| r.base_url.clone())
            .unwrap_or_default()
    }

    pub fn logs_url(&self) -> String {
        self.logs_url.clone().unwrap_or_default()
    }
}

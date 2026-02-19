use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tracing::{info, warn};

use crate::models::k8s::{Node, Pod};
use crate::models::views::{ClusterSummary, NodeSummary};

use super::NodeClient;

pub struct Aggregator {
    clients: RwLock<HashMap<String, Arc<NodeClient>>>,
}

impl Aggregator {
    pub fn new(clients: Vec<NodeClient>) -> Self {
        let mut m = HashMap::new();
        for c in clients {
            m.insert(c.name.clone(), Arc::new(c));
        }
        Self {
            clients: RwLock::new(m),
        }
    }

    pub async fn list_all_pods(&self) -> Result<Vec<Pod>, Box<dyn std::error::Error + Send + Sync>> {
        let clients = self.snapshot().await;

        let mut all_pods = Vec::new();
        let mut handles = Vec::new();

        for client in clients {
            let c = client.clone();
            handles.push(tokio::spawn(async move {
                match c.list_pods().await {
                    Ok(list) => Some((c.name.clone(), list)),
                    Err(e) => {
                        warn!("error listing pods from {}: {}", c.name, e);
                        None
                    }
                }
            }));
        }

        for handle in handles {
            if let Ok(Some((node_name, list))) = handle.await {
                for mut pod in list.items {
                    let annotations = pod.metadata.annotations.get_or_insert_with(HashMap::new);
                    annotations.insert("mkube.io/node".to_string(), node_name.clone());
                    all_pods.push(pod);
                }
            }
        }

        Ok(all_pods)
    }

    pub async fn list_all_nodes(
        &self,
    ) -> Result<Vec<Node>, Box<dyn std::error::Error + Send + Sync>> {
        let clients = self.snapshot().await;

        let mut nodes = Vec::new();
        let mut handles = Vec::new();

        for client in clients {
            let c = client.clone();
            handles.push(tokio::spawn(async move {
                match c.get_node().await {
                    Ok(node) => Some(node),
                    Err(e) => {
                        warn!("error getting node from {}: {}", c.name, e);
                        None
                    }
                }
            }));
        }

        for handle in handles {
            if let Ok(Some(node)) = handle.await {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    pub async fn get_pod(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<(Pod, String), Box<dyn std::error::Error + Send + Sync>> {
        let clients = self.snapshot().await;

        for client in &clients {
            if let Ok(mut pod) = client.get_pod(ns, name).await {
                let annotations = pod.metadata.annotations.get_or_insert_with(HashMap::new);
                annotations.insert("mkube.io/node".to_string(), client.name.clone());
                return Ok((pod, client.name.clone()));
            }
        }
        Err(format!("pod {}/{} not found on any node", ns, name).into())
    }

    pub async fn create_pod(
        &self,
        pod: &Pod,
    ) -> Result<Pod, Box<dyn std::error::Error + Send + Sync>> {
        let clients_map = self.clients.read().await;

        // Route by nodeName if specified
        if !pod.spec.node_name.is_empty() {
            if let Some(c) = clients_map.get(&pod.spec.node_name) {
                return c.create_pod(pod).await;
            }
            return Err(format!("node {:?} not found", pod.spec.node_name).into());
        }

        // Least-pods scheduling
        let mut target: Option<Arc<NodeClient>> = None;
        let mut min_pods = usize::MAX;

        for c in clients_map.values() {
            if !c.is_healthy() {
                continue;
            }
            if let Ok(list) = c.list_pods().await {
                if list.items.len() < min_pods {
                    min_pods = list.items.len();
                    target = Some(c.clone());
                }
            }
        }

        match target {
            Some(c) => c.create_pod(pod).await,
            None => Err("no healthy nodes available".into()),
        }
    }

    pub async fn delete_pod(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_, node_name) = self.get_pod(ns, name).await?;

        let clients_map = self.clients.read().await;
        let c = clients_map
            .get(&node_name)
            .ok_or_else(|| format!("node {:?} not found", node_name))?;
        c.delete_pod(ns, name).await
    }

    pub async fn get_pod_log(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let (_, node_name) = self.get_pod(ns, name).await?;

        let clients_map = self.clients.read().await;
        let c = clients_map
            .get(&node_name)
            .ok_or_else(|| format!("node {:?} not found", node_name))?;
        c.get_pod_log(ns, name).await
    }

    pub async fn get_node(
        &self,
        name: &str,
    ) -> Result<Node, Box<dyn std::error::Error + Send + Sync>> {
        let clients_map = self.clients.read().await;
        let c = clients_map
            .get(name)
            .ok_or_else(|| format!("node {:?} not found", name))?;
        c.get_node().await
    }

    pub async fn get_cluster_summary(&self) -> ClusterSummary {
        let clients = self.snapshot().await;

        let mut summary = ClusterSummary {
            node_count: clients.len(),
            ..Default::default()
        };

        for c in &clients {
            let mut ns = NodeSummary {
                name: c.name.clone(),
                healthy: c.is_healthy(),
                pod_count: 0,
                last_ping: c.last_ping(),
            };

            if c.is_healthy() {
                summary.healthy_nodes += 1;
            }

            if let Ok(list) = c.list_pods().await {
                ns.pod_count = list.items.len();
                summary.pod_count += list.items.len();
                for pod in &list.items {
                    if pod.status.phase == "Running" {
                        summary.running_pods += 1;
                    }
                }
            }

            summary.nodes.push(ns);
        }

        summary
    }

    pub async fn run_health_checker(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<()>) {
        // Initial check
        self.ping_all().await;

        let mut interval = time::interval(Duration::from_secs(15));
        interval.tick().await; // skip first immediate tick

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.ping_all().await;
                }
                _ = shutdown.changed() => {
                    info!("health checker shutting down");
                    return;
                }
            }
        }
    }

    async fn ping_all(&self) {
        let clients = self.snapshot().await;
        for c in &clients {
            if let Err(e) = c.ping().await {
                warn!("health check failed for {}: {}", c.name, e);
            }
        }
    }

    async fn snapshot(&self) -> Vec<Arc<NodeClient>> {
        self.clients.read().await.values().cloned().collect()
    }
}

pub mod aggregator;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::Mutex;
use std::time::Duration;

use crate::models::k8s::{
    BMHList, BareMetalHost, ConfigMap, ConfigMapList, ConsistencyReport, Deployment,
    DeploymentList, EventList, ISCSICdrom, ISCSICdromList, Network, NetworkList, Node,
    PVCList, PersistentVolumeClaim, Pod, PodList,
};

pub struct NodeClient {
    pub name: String,
    pub address: String,
    http: Client,
    state: Mutex<ClientState>,
}

struct ClientState {
    healthy: bool,
    last_ping: Option<DateTime<Utc>>,
}

impl NodeClient {
    pub fn new(name: String, address: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to create HTTP client");

        Self {
            name,
            address,
            http,
            state: Mutex::new(ClientState {
                healthy: true,
                last_ping: None,
            }),
        }
    }

    pub async fn ping(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .get(format!("{}/healthz", self.address))
            .send()
            .await?;

        if resp.status().is_success() {
            let mut state = self.state.lock().unwrap();
            state.healthy = true;
            state.last_ping = Some(Utc::now());
            Ok(())
        } else {
            let mut state = self.state.lock().unwrap();
            state.healthy = false;
            Err(format!("node {} health check returned {}", self.name, resp.status()).into())
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.state.lock().unwrap().healthy
    }

    pub fn last_ping(&self) -> Option<DateTime<Utc>> {
        self.state.lock().unwrap().last_ping
    }

    pub async fn list_pods(&self) -> Result<PodList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/pods").await
    }

    pub async fn get_pod(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<Pod, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/namespaces/{}/pods/{}", ns, name))
            .await
    }

    pub async fn create_pod(
        &self,
        pod: &Pod,
    ) -> Result<Pod, Box<dyn std::error::Error + Send + Sync>> {
        self.post_json(
            &format!("/api/v1/namespaces/{}/pods", pod.metadata.namespace),
            pod,
        )
        .await
    }

    pub async fn delete_pod(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .delete(format!(
                "{}/api/v1/namespaces/{}/pods/{}",
                self.address, ns, name
            ))
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("delete pod failed: {}", body).into());
        }
        Ok(())
    }

    pub async fn get_pod_log(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .get(format!(
                "{}/api/v1/namespaces/{}/pods/{}/log",
                self.address, ns, name
            ))
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("get pod log failed: {}", body).into());
        }
        Ok(resp.text().await?)
    }

    pub async fn get_node(&self) -> Result<Node, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/nodes/{}", self.name)).await
    }

    pub async fn watch_pods(
        &self,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .get(format!("{}/api/v1/pods?watch=true", self.address))
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("watch pods failed: {}", body).into());
        }
        Ok(resp)
    }

    pub async fn get_container_log(
        &self,
        ns: &str,
        pod_name: &str,
        container_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .get(format!(
                "{}/api/v1/namespaces/{}/pods/{}/log?container={}",
                self.address, ns, pod_name, container_name
            ))
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("get container log failed: {}", body).into());
        }
        Ok(resp.text().await?)
    }

    // --- Deployments ---

    pub async fn list_deployments(
        &self,
    ) -> Result<DeploymentList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/deployments").await
    }

    pub async fn get_deployment(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<Deployment, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/namespaces/{}/deployments/{}", ns, name))
            .await
    }

    // --- Networks ---

    pub async fn list_networks(
        &self,
    ) -> Result<NetworkList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/networks").await
    }

    pub async fn get_network(
        &self,
        name: &str,
    ) -> Result<Network, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/networks/{}", name)).await
    }

    // --- PVCs ---

    pub async fn list_pvcs(
        &self,
    ) -> Result<PVCList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/persistentvolumeclaims").await
    }

    pub async fn get_pvc(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<PersistentVolumeClaim, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!(
            "/api/v1/namespaces/{}/persistentvolumeclaims/{}",
            ns, name
        ))
        .await
    }

    // --- BareMetalHosts ---

    pub async fn list_bmhs(
        &self,
    ) -> Result<BMHList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/baremetalhosts").await
    }

    pub async fn get_bmh(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<BareMetalHost, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!(
            "/api/v1/namespaces/{}/baremetalhosts/{}",
            ns, name
        ))
        .await
    }

    // --- iSCSI CDROMs ---

    pub async fn list_iscsi_cdroms(
        &self,
    ) -> Result<ISCSICdromList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/iscsi-cdroms").await
    }

    pub async fn get_iscsi_cdrom(
        &self,
        name: &str,
    ) -> Result<ISCSICdrom, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/iscsi-cdroms/{}", name))
            .await
    }

    // --- ConfigMaps ---

    pub async fn list_configmaps(
        &self,
        ns: &str,
    ) -> Result<ConfigMapList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/namespaces/{}/configmaps", ns))
            .await
    }

    pub async fn get_configmap(
        &self,
        ns: &str,
        name: &str,
    ) -> Result<ConfigMap, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json(&format!("/api/v1/namespaces/{}/configmaps/{}", ns, name))
            .await
    }

    // --- Consistency ---

    pub async fn get_consistency(
        &self,
    ) -> Result<ConsistencyReport, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/consistency").await
    }

    // --- Events ---

    pub async fn list_events(
        &self,
    ) -> Result<EventList, Box<dyn std::error::Error + Send + Sync>> {
        self.get_json("/api/v1/events").await
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .get(format!("{}{}", self.address, path))
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("GET {} returned error: {}", path, body).into());
        }
        Ok(resp.json().await?)
    }

    async fn post_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .post(format!("{}{}", self.address, path))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await?;

        if resp.status().as_u16() >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("POST {} returned error: {}", path, body).into());
        }
        Ok(resp.json().await?)
    }
}

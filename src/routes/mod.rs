pub mod api;
pub mod sse;
pub mod ui;

use axum::{
    Router,
    routing::get,
};
use tower_http::services::ServeDir;

use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // API discovery
        .route("/api", get(api::handle_api_versions))
        .route("/api/v1", get(api::handle_api_resources))
        // Pods
        .route("/api/v1/pods", get(api::handle_list_all_pods))
        .route(
            "/api/v1/namespaces/{namespace}/pods",
            get(api::handle_list_namespaced_pods).post(api::handle_create_pod),
        )
        .route(
            "/api/v1/namespaces/{namespace}/pods/{name}",
            get(api::handle_get_pod).delete(api::handle_delete_pod),
        )
        .route(
            "/api/v1/namespaces/{namespace}/pods/{name}/log",
            get(api::handle_get_pod_log),
        )
        // Nodes
        .route("/api/v1/nodes", get(api::handle_list_nodes))
        .route("/api/v1/nodes/{name}", get(api::handle_get_node))
        // Health
        .route("/healthz", get(api::handle_healthz))
        // Dashboard UI
        .route("/ui/", get(ui::handle_dashboard))
        .route("/ui/namespaces", get(ui::handle_namespaces))
        .route("/ui/namespaces/{name}", get(ui::handle_namespace_detail))
        .route("/ui/namespaces/{namespace}/pods/{name}", get(ui::handle_pod_detail))
        .route("/ui/namespaces/{namespace}/pods/{pod}/containers/{name}", get(ui::handle_container_detail))
        // SSE events
        .route("/ui/events/pods", get(sse::handle_pod_events))
        .route("/ui/pods", get(ui::handle_pods))
        .route("/ui/pods/{namespace}/{name}", get(ui::handle_pod_detail))
        .route("/ui/nodes", get(ui::handle_nodes))
        .route("/ui/nodes/{name}", get(ui::handle_node_detail))
        .route("/ui/registry", get(ui::handle_registry))
        // Deployments
        .route("/ui/deployments", get(ui::handle_deployments))
        .route("/ui/deployments/{namespace}/{name}", get(ui::handle_deployment_detail))
        // Networks
        .route("/ui/networks", get(ui::handle_networks))
        .route("/ui/networks/{name}", get(ui::handle_network_detail))
        // PVCs
        .route("/ui/pvcs", get(ui::handle_pvcs))
        // BareMetalHosts
        .route("/ui/bmh", get(ui::handle_bmhs))
        .route("/ui/bmh/{namespace}/{name}", get(ui::handle_bmh_detail))
        // iSCSI CDROMs
        .route("/ui/iscsi-cdroms", get(ui::handle_iscsi_cdroms))
        .route("/ui/iscsi-cdroms/{name}", get(ui::handle_iscsi_cdrom_detail))
        // ConfigMaps
        .route("/ui/configmaps", get(ui::handle_configmaps))
        .route("/ui/configmaps/{namespace}/{name}", get(ui::handle_configmap_detail))
        // Operations
        .route("/ui/consistency", get(ui::handle_consistency))
        .route("/ui/events", get(ui::handle_events))
        // Static files
        .nest_service("/ui/static", ServeDir::new("static"))
        // Root redirect
        .route(
            "/",
            get(|| async {
                axum::response::Redirect::to("/ui/")
            }),
        )
        .with_state(state)
}

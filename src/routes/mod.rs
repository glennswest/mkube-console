pub mod api;
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
        .route("/ui/pods", get(ui::handle_pods))
        .route("/ui/pods/{namespace}/{name}", get(ui::handle_pod_detail))
        .route("/ui/nodes", get(ui::handle_nodes))
        .route("/ui/nodes/{name}", get(ui::handle_node_detail))
        .route("/ui/registry", get(ui::handle_registry))
        .route("/ui/logs", get(ui::handle_logs))
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

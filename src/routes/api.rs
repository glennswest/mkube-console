use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::models::k8s::*;
use crate::AppState;

pub async fn handle_api_versions(State(state): State<AppState>) -> Json<ApiVersions> {
    Json(ApiVersions {
        kind: "APIVersions".to_string(),
        versions: vec!["v1".to_string()],
        server_address_by_client_cidrs: vec![ServerAddressByClientCidr {
            client_cidr: "0.0.0.0/0".to_string(),
            server_address: state.config.listen_addr(),
        }],
    })
}

pub async fn handle_api_resources() -> Json<ApiResourceList> {
    Json(ApiResourceList {
        kind: "APIResourceList".to_string(),
        group_version: "v1".to_string(),
        api_resources: vec![
            ApiResource {
                name: "pods".to_string(),
                namespaced: true,
                kind: "Pod".to_string(),
                verbs: vec![
                    "get".to_string(),
                    "list".to_string(),
                    "create".to_string(),
                    "delete".to_string(),
                ],
            },
            ApiResource {
                name: "pods/log".to_string(),
                namespaced: true,
                kind: "Pod".to_string(),
                verbs: vec!["get".to_string()],
            },
            ApiResource {
                name: "pods/status".to_string(),
                namespaced: true,
                kind: "Pod".to_string(),
                verbs: vec!["get".to_string()],
            },
            ApiResource {
                name: "namespaces".to_string(),
                namespaced: false,
                kind: "Namespace".to_string(),
                verbs: vec!["get".to_string(), "list".to_string()],
            },
            ApiResource {
                name: "nodes".to_string(),
                namespaced: false,
                kind: "Node".to_string(),
                verbs: vec!["get".to_string(), "list".to_string()],
            },
        ],
    })
}

pub async fn handle_list_all_pods(State(state): State<AppState>) -> Response {
    match state.aggregator.list_all_pods().await {
        Ok(pods) => Json(PodList {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "PodList".to_string(),
            },
            items: pods,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn handle_list_namespaced_pods(
    State(state): State<AppState>,
    Path(namespace): Path<String>,
) -> Response {
    match state.aggregator.list_all_pods().await {
        Ok(pods) => {
            let items: Vec<Pod> = pods
                .into_iter()
                .filter(|p| p.metadata.namespace == namespace)
                .collect();
            Json(PodList {
                type_meta: TypeMeta {
                    api_version: "v1".to_string(),
                    kind: "PodList".to_string(),
                },
                items,
            })
            .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn handle_get_pod(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    match state.aggregator.get_pod(&namespace, &name).await {
        Ok((pod, _)) => Json(pod).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn handle_create_pod(
    State(state): State<AppState>,
    Path(namespace): Path<String>,
    Json(mut pod): Json<Pod>,
) -> Response {
    pod.metadata.namespace = namespace;
    match state.aggregator.create_pod(&pod).await {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn handle_delete_pod(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    match state.aggregator.delete_pod(&namespace, &name).await {
        Ok(()) => Json(Status {
            api_version: "v1".to_string(),
            kind: "Status".to_string(),
            status: "Success".to_string(),
            message: format!("pod {:?} deleted", name),
        })
        .into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn handle_get_pod_log(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Response {
    match state.aggregator.get_pod_log(&namespace, &name).await {
        Ok(logs) => (
            StatusCode::OK,
            [("content-type", "text/plain; charset=utf-8")],
            logs,
        )
            .into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn handle_list_nodes(State(state): State<AppState>) -> Response {
    match state.aggregator.list_all_nodes().await {
        Ok(nodes) => Json(NodeList {
            type_meta: TypeMeta {
                api_version: "v1".to_string(),
                kind: "NodeList".to_string(),
            },
            items: nodes,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn handle_get_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    match state.aggregator.get_node(&name).await {
        Ok(node) => Json(node).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn handle_healthz() -> &'static str {
    "ok\n"
}

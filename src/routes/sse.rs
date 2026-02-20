use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::pin::Pin;
use std::time::Duration;

use crate::AppState;

type SseStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

/// SSE endpoint that streams pod state changes to the browser.
/// Opens watch connections to mkube nodes when available, falls back to polling.
pub async fn handle_pod_events(State(state): State<AppState>) -> Response {
    let clients = state.aggregator.snapshot_clients().await;

    if clients.is_empty() {
        // Return an empty SSE stream that just sends keepalives
        let empty: SseStream = Box::pin(stream::pending());
        return Sse::new(empty)
            .keep_alive(KeepAlive::default().interval(Duration::from_secs(15)))
            .into_response();
    }

    // Try to open watch connections to nodes
    let mut watch_lines: Vec<String> = Vec::new();
    let mut has_watch = false;

    for client in &clients {
        match client.watch_pods().await {
            Ok(resp) => {
                // Read the initial batch of watch events
                if let Ok(body) = resp.text().await {
                    for line in body.lines() {
                        if !line.trim().is_empty() {
                            watch_lines.push(line.to_string());
                        }
                    }
                    has_watch = true;
                }
            }
            Err(e) => {
                tracing::warn!("watch not available on {}: {}", client.name, e);
            }
        }
    }

    // If watch worked, send initial events then poll for updates
    // If watch didn't work, just poll
    let agg = state.aggregator.clone();

    let poll_stream = stream::unfold(
        (agg, has_watch, watch_lines, true),
        move |(agg, _has_watch, mut initial_lines, is_first)| async move {
            if is_first && !initial_lines.is_empty() {
                // Send initial watch events
                let line = initial_lines.remove(0);
                let done = initial_lines.is_empty();
                let event = Event::default().event("pod-update").data(line);
                return Some((
                    Ok::<_, Infallible>(event),
                    (agg, _has_watch, initial_lines, done),
                ));
            }

            // Poll for full state every 3 seconds
            tokio::time::sleep(Duration::from_secs(3)).await;
            let pods = agg.list_all_pods().await.unwrap_or_default();
            let data = serde_json::to_string(&pods).unwrap_or_default();
            let event = Event::default().event("pod-list").data(data);
            Some((Ok(event), (agg, _has_watch, Vec::new(), false)))
        },
    );

    Sse::new(poll_stream)
        .keep_alive(KeepAlive::default().interval(Duration::from_secs(15)))
        .into_response()
}

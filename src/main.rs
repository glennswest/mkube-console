mod clients;
mod config;
mod helpers;
mod models;
mod routes;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use clients::aggregator::Aggregator;
use clients::NodeClient;

#[derive(Clone)]
pub struct AppState {
    pub aggregator: Arc<Aggregator>,
    pub config: Arc<config::Config>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mkube_console=info".parse().unwrap()),
        )
        .init();

    let config_path = std::env::args()
        .nth(1)
        .or_else(|| {
            std::env::args().skip(1).zip(std::env::args().skip(2)).find_map(|(k, v)| {
                if k == "-config" || k == "--config" {
                    Some(v)
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "/etc/mkube-console/config.yaml".to_string());

    let cfg = config::Config::load(&PathBuf::from(&config_path)).unwrap_or_else(|e| {
        eprintln!("error loading config: {}", e);
        std::process::exit(1);
    });

    let mut node_clients = Vec::new();
    for n in &cfg.nodes {
        node_clients.push(NodeClient::new(n.name.clone(), n.address.clone()));
    }

    if node_clients.is_empty() {
        eprintln!("no nodes configured");
        std::process::exit(1);
    }

    let aggregator = Arc::new(Aggregator::new(node_clients));
    let cfg = Arc::new(cfg);

    // Shutdown signal
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

    // Start health checker
    let agg_clone = aggregator.clone();
    tokio::spawn(async move {
        agg_clone.run_health_checker(shutdown_rx).await;
    });

    let state = AppState {
        aggregator,
        config: cfg.clone(),
    };

    let router = routes::build_router(state);

    let listen_addr = cfg.listen_addr();
    let listener = TcpListener::bind(&listen_addr).await.unwrap_or_else(|e| {
        eprintln!("failed to bind {}: {}", listen_addr, e);
        std::process::exit(1);
    });

    info!("mkube-console listening on {}", listen_addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(());
        })
        .await
        .unwrap_or_else(|e| {
            eprintln!("server error: {}", e);
            std::process::exit(1);
        });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

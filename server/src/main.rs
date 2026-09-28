use proof_battle_server::config::Config;
use proof_battle_server::create_app;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Configuration error: {e}");
            tracing::error!("{e}");
            std::process::exit(1);
        }
    };

    let app = create_app(config.clone());

    println!("🚀 ProofBattle server running on ws://{}/ws", config.bind_addr);
    tracing::info!("🚀 Running server on ws://{}/ws", config.bind_addr);
    let listener = match TcpListener::bind(&config.bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", config.bind_addr, e);
            tracing::error!("Failed to bind to {}: {}", config.bind_addr, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {e}");
        tracing::error!("Server error: {e}");
    }
}

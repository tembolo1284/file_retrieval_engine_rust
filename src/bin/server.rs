// src/bin/server.rs

use std::sync::Arc;
use tokio::sync::mpsc;
use file_retrieval_engine::server::{
    engine::ServerProcessingEngine,
    app_interface::ServerAppInterface,
    index_store::IndexStore,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <server_port>", args[0]);
        eprintln!("Note: Use a non-privileged port number (1024-65535)");
        return Err("Invalid arguments".to_string());
    }

    let server_port = args[1].parse::<u16>()
        .map_err(|_| "Invalid port number".to_string())?;

    if server_port < 1024 || server_port > 65535 {
        return Err("Port must be in the range 1024-65535".to_string());
    }

    let store = Arc::new(IndexStore::new());
    let engine = Arc::new(ServerProcessingEngine::new(store, 8)?);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    let interface = ServerAppInterface::new(Arc::clone(&engine), shutdown_tx);

    println!("Starting File Retrieval Engine Server on port {}", server_port);
    engine.initialize(server_port).await?;

    println!("Server initialized. Type 'list' to see connected clients or 'quit' to shutdown.");

    // Run interface in separate task
    let interface_handle = tokio::spawn(async move {
        interface.read_commands().await
    });

    // Wait for shutdown signal
    shutdown_rx.recv().await;
    
    // Wait for interface to complete
    if let Err(e) = interface_handle.await {
        eprintln!("Interface error: {}", e);
    }

    Ok(())
}

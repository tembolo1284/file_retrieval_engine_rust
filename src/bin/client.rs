// src/bin/client.rs

use std::sync::Arc;
use file_retrieval_engine::client::{
    engine::ClientProcessingEngine,
    app_interface::ClientAppInterface,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let engine = Arc::new(ClientProcessingEngine::new());
    let interface = ClientAppInterface::new(Arc::clone(&engine));

    println!("File Retrieval Engine Client");
    println!("Use 'connect <server_ip> <port>' to connect to a server");
    println!("Type 'help' for list of available commands\n");

    interface.read_commands()?;

    Ok(())
}

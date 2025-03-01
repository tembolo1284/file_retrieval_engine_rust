// src/server/app_interface.rs

use std::sync::Arc;
use std::io::{self, Write};
use tokio::sync::mpsc;
use crate::server::engine::ServerProcessingEngine;

pub struct ServerAppInterface {
    engine: Arc<ServerProcessingEngine>,
    shutdown_sender: mpsc::Sender<()>,
}

impl ServerAppInterface {
    pub fn new(engine: Arc<ServerProcessingEngine>, shutdown_sender: mpsc::Sender<()>) -> Self {
        ServerAppInterface { 
            engine,
            shutdown_sender,
        }
    }

    pub async fn read_commands(&self) -> Result<(), String> {
        println!("Server interface ready. Available commands: 'list', 'quit', 'help'");

        loop {
            print!("> ");
            io::stdout().flush().map_err(|e| e.to_string())?;

            let mut command = String::new();
            if io::stdin().read_line(&mut command).is_err() {
                continue;
            }

            let command = command.trim();
            if command.is_empty() {
                continue;
            }

            match self.handle_command(command).await {
                Ok(should_quit) => {
                    if should_quit {
                        break;
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Ok(())
    }

    async fn handle_command(&self, command: &str) -> Result<bool, String> {
        match command {
            "quit" => {
                println!("Shutting down server...");
                self.engine.shutdown().await;
                let _ = self.shutdown_sender.send(()).await;
                println!("Server shutdown complete.");
                Ok(true)
            }

            "list" => {
                let clients = self.engine.get_connected_clients();
                if clients.is_empty() {
                    println!("No clients currently connected.");
                } else {
                    println!("\nConnected Clients:");
                    println!("----------------------------------------");
                    for client_info in &clients {
                        println!("{}", client_info);
                    }
                    println!("----------------------------------------");
                    println!("Total connected clients: {}", clients.len());
                }
                Ok(false)
            }

            "help" => {
                println!("Available commands:");
                println!("  list - List all connected clients");
                println!("  help - Show this help message");
                println!("  quit - Shutdown the server");
                Ok(false)
            }

            _ => {
                println!("Unrecognized command! Available commands: 'list', 'help', 'quit'");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::index_store::IndexStore;

    #[tokio::test]
    async fn test_help_command() {
        let store = Arc::new(IndexStore::new());
        let engine = Arc::new(ServerProcessingEngine::new(store, 4).unwrap());
        let (shutdown_tx, _) = mpsc::channel(1);
        let interface = ServerAppInterface::new(engine, shutdown_tx);
        
        assert!(interface.handle_command("help").await.unwrap() == false);
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let store = Arc::new(IndexStore::new());
        let engine = Arc::new(ServerProcessingEngine::new(store, 4).unwrap());
        let (shutdown_tx, _) = mpsc::channel(1);
        let interface = ServerAppInterface::new(engine, shutdown_tx);
        
        assert!(interface.handle_command("invalid_command").await.unwrap() == false);
    }
}

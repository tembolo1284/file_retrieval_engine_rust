// src/client/app_interface.rs

use std::sync::Arc;
use std::io::{self, Write};
use crate::client::engine::ClientProcessingEngine;

pub struct ClientAppInterface {
    engine: Arc<ClientProcessingEngine>,
}

impl ClientAppInterface {
    pub fn new(engine: Arc<ClientProcessingEngine>) -> Self {
        println!("File Retrieval Engine Client");
        println!("Available commands: connect, get_info, index, search, help, quit");
        ClientAppInterface { engine }
    }

    pub fn read_commands(&self) -> Result<(), String> {
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

            match self.handle_command(command) {
                Ok(should_quit) => {
                    if should_quit {
                        println!("Client shutdown complete.");
                        break;
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Ok(())
    }

    fn handle_command(&self, command: &str) -> Result<bool, String> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "quit" => {
                println!("Disconnecting from server...");
                self.engine.disconnect()?;
                println!("Disconnected. Goodbye!");
                Ok(true)
            }

            "connect" => {
                if parts.len() != 3 {
                    return Err("Usage: connect <server IP> <server port>".to_string());
                }
                let server_ip = parts[1];
                let server_port = parts[2];

                self.engine.connect(server_ip, server_port)?;
                println!("Successfully connected to server at {}:{}", server_ip, server_port);
                Ok(false)
            }

            "get_info" => {
                let client_id = self.engine.get_client_id();
                if client_id > 0 {
                    println!("Client ID: {}", client_id);
                } else {
                    println!("Not connected to server");
                }
                Ok(false)
            }

            "index" => {
                if parts.len() != 2 {
                    return Err("Usage: index <folder_path>".to_string());
                }
                let folder_path = parts[1];

                match self.engine.index_folder(folder_path) {
                    Ok(result) => {
                        println!("Indexing completed:");
                        println!("Total bytes processed: {}", result.total_bytes_read);
                        println!("Execution time: {:.2} seconds", result.execution_time);
                        let throughput = (result.total_bytes_read as f64 / (1024.0 * 1024.0)) / result.execution_time;
                        println!("Throughput: {:.2} MB/s", throughput);
                    }
                    Err(e) => eprintln!("Indexing failed: {}", e),
                }
                Ok(false)
            }

            "search" => {
                if parts.len() < 2 {
                    return Err("Usage: search <term1> [AND <term2> ...]".to_string());
                }
            
                let terms: Vec<String> = parts[1..]
                    .iter()
                    .filter(|&&term| term != "AND")
                    .map(|&term| term.to_string())
                    .collect();
            
                if terms.is_empty() {
                    return Err("No valid search terms provided".to_string());
                }
            
                match self.engine.search(terms) {
                    Ok(result) => {
                        println!("\nSearch completed");
            
                        if result.document_frequencies.is_empty() {
                            println!("No matches found.");
                        } else {
                            println!("\n{} matches found:", result.document_frequencies.len());
                            println!("----------------------------------------");
                            for doc in &result.document_frequencies {
                                println!("* {} (frequency: {})",
                                       doc.document_path, doc.word_frequency);
                            }
                            println!("----------------------------------------");
                        }
                    }
                    Err(e) => eprintln!("Search failed: {}", e),
                }
                Ok(false)
            }
            
            "help" => {
                println!("Available commands:");
                println!("  connect <server IP> <server port> - Connect to server");
                println!("  get_info                         - Display client ID");
                println!("  index <folder path>              - Index documents in folder");
                println!("  search <term1> AND <term2> ...   - Search indexed documents");
                println!("  help                             - Show this help message");
                println!("  quit                             - Exit the program");
                Ok(false)
            }

            _ => {
                println!("Unrecognized command! Type 'help' for available commands.");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_help_command() {
        let engine = Arc::new(ClientProcessingEngine::new());
        let interface = ClientAppInterface::new(engine);
        
        assert!(interface.handle_command("help").unwrap() == false);
    }

    #[test]
    fn test_invalid_command() {
        let engine = Arc::new(ClientProcessingEngine::new());
        let interface = ClientAppInterface::new(engine);
        
        assert!(interface.handle_command("invalid_command").unwrap() == false);
    }
}

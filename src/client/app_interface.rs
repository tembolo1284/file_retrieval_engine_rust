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
            
                let start_time = std::time::Instant::now();
            
                match self.engine.search(terms) {
                    Ok(result) => {
                        let duration = start_time.elapsed();
                        println!("\nSearch completed in {:.2} seconds", duration.as_secs_f64());
            
                        if result.document_frequencies.is_empty() {
                            println!("Search results (top 10 out of 0):");
                        } else {
                            println!("Search results (top 10 out of {}):",
                                   result.document_frequencies.len());
            
                            // Take up to 10 results and sort by frequency
                            let mut results = result.document_frequencies.clone();
                            results.sort_by(|a, b| b.word_frequency.cmp(&a.word_frequency));
            
                            for doc in results.iter().take(10) {
                                let path = &doc.document_path;
                                
                                // Let's look for "client_X" in the path and extract X
                                if path.starts_with("Client s:") {
                                    let content = &path[9..]; // Skip "Client s:"
                                    
                                    if let Some(client_pos) = content.find("/client_") {
                                        let client_id_start = client_pos + 8; // Skip "client_"
                                        if client_id_start < content.len() {
                                            let mut client_id = String::new();
                                            for c in content[client_id_start..].chars() {
                                                if c.is_ascii_digit() {
                                                    client_id.push(c);
                                                } else {
                                                    break;
                                                }
                                            }
                                            
                                            if !client_id.is_empty() {
                                                println!("* Client {}:{}: {}", 
                                                        client_id, content, doc.word_frequency);
                                                continue;
                                            }
                                        }
                                    }
                                }
                                
                                // If we couldn't extract a client ID, just print the original
                                println!("* {}: {}", path, doc.word_frequency);
                            }
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

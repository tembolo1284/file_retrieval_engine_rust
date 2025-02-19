// src/client/engine.rs

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::fs;
use std::time::Instant;
use parking_lot::Mutex;
use walkdir::WalkDir;
use std::collections::HashMap;

const BATCH_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct IndexResult {
    pub execution_time: f64,
    pub total_bytes_read: i64,
}

#[derive(Debug, Clone)]
pub struct DocPathFreqPair {
    pub document_path: String,
    pub word_frequency: i64,
}

#[derive(Debug, Clone)]
pub struct DocFreqPair {
    pub document_number: i64,
    pub word_frequency: i64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub execution_time: f64,
    pub document_frequencies: Vec<DocPathFreqPair>,
}

pub struct ClientProcessingEngine {
    client_socket: Mutex<Option<TcpStream>>,
    client_id: AtomicI32,
    is_connected: AtomicBool,
}

impl ClientProcessingEngine {
    pub fn new() -> Self {
        ClientProcessingEngine {
            client_socket: Mutex::new(None),
            client_id: AtomicI32::new(-1),
            is_connected: AtomicBool::new(false),
        }
    }

    pub fn get_document_path(&self, doc_id: i64) -> Option<String> {
        let request = format!("GET_DOC_PATH {}", doc_id);
        match self.send_message(&request) {
            Ok(response) => {
                let parts: Vec<&str> = response.splitn(2, ' ').collect();
                if parts.len() == 2 && parts[0] == "DOC_PATH" {
                    Some(parts[1].trim().to_string())
                } else {
                    None
                }
            }
            Err(_) => None
        }
    }

    pub fn index_folder(&self, folder_path: &str) -> Result<IndexResult, String> {
        let start_time = Instant::now();
        let mut total_bytes = 0i64;

        // Ensure we're connected
        if !self.is_connected.load(Ordering::Relaxed) {
            return Err("Not connected to server".to_string());
        }

        // Walk directory and collect files
        let files: Vec<_> = WalkDir::new(folder_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_owned())
            .collect();

        println!("Found {} files to process", files.len());

        for file_path in files {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;

            total_bytes += content.len() as i64;
            let word_freqs = self.process_document(&content);

            self.send_index_request(&file_path.to_string_lossy(), &word_freqs)?;
        }

        let duration = start_time.elapsed();
        Ok(IndexResult {
            execution_time: duration.as_secs_f64(),
            total_bytes_read: total_bytes,
        })
    }

    fn process_document(&self, content: &str) -> HashMap<String, i64> {
        let mut word_freqs = HashMap::new();
        let mut current_word = String::with_capacity(1024);
        let mut in_word = false;

        for c in content.chars() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                current_word.push(c);
                in_word = true;
            } else if in_word {
                if !current_word.is_empty() {
                    *word_freqs.entry(current_word.clone()).or_insert(0) += 1;
                }
                current_word.clear();
                in_word = false;
            }
        }

        if !current_word.is_empty() {
            *word_freqs.entry(current_word).or_insert(0) += 1;
        }

        word_freqs
  }        
     
    pub fn search(&self, terms: Vec<String>) -> Result<SearchResult, String> {
        let start_time = Instant::now();
    
        if !self.is_connected.load(Ordering::Relaxed) {
            return Err("Not connected to server".to_string());
        }
    
        let request = format!("SEARCH_REQUEST {}", terms.join(" "));
        let response = self.send_message(&request)?;
        
        // Debug print
        println!("Raw server response: '{}'", response);
    
        let results = self.handle_search_reply(&response)?;
        let duration = start_time.elapsed();
    
        Ok(SearchResult {
            execution_time: duration.as_secs_f64(),
            document_frequencies: results,
        })
    }
    
    pub fn connect(&self, server_ip: &str, server_port: &str) -> Result<(), String> {
        let addr = format!("{}:{}", server_ip, server_port);
        println!("Attempting to connect to {}", addr);  // Debug print
        
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;
        println!("TCP connection established");  // Debug print

        stream.set_nodelay(true)
            .map_err(|e| format!("Failed to set TCP_NODELAY: {}", e))?;
        println!("TCP_NODELAY set");  // Debug print

        *self.client_socket.lock() = Some(stream);
        println!("Sending REGISTER_REQUEST");  // Debug print
        
        let response = self.send_message("REGISTER_REQUEST")?;
        println!("Received response: {}", response);  // Debug print
        
        let mut parts = response.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("REGISTER_REPLY"), Some(client_id)) => {
                let id = client_id.parse::<i32>()
                    .map_err(|_| "Invalid client ID received")?;
                self.client_id.store(id, Ordering::Relaxed);
                self.is_connected.store(true, Ordering::Relaxed);
                Ok(())
            }
            _ => {
                *self.client_socket.lock() = None;
                Err("Invalid register reply".to_string())
            }
        }
    }        
     
    pub fn disconnect(&self) -> Result<(), String> {
        if self.is_connected.load(Ordering::Relaxed) {
            if let Some(mut socket) = self.client_socket.lock().take() {
                let _ = socket.write_all(b"QUIT_REQUEST");
            }
            self.is_connected.store(false, Ordering::Relaxed);
            self.client_id.store(-1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn is_server_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub fn get_client_id(&self) -> i32 {
        self.client_id.load(Ordering::Relaxed)
    }

    // Helper methods
    fn send_message(&self, message: &str) -> Result<String, String> {
        println!("Attempting to send message: '{}'", message);
        let mut socket = self.client_socket.lock();
        let socket = socket.as_mut()
            .ok_or("Not connected to server")?;
    
        let message = format!("{}\n", message);
        println!("Formatted message with newline: '{}'", message);
        
        socket.write_all(message.as_bytes())
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        println!("Message sent, flushing socket...");
        socket.flush()
            .map_err(|e| format!("Failed to flush socket: {}", e))?;
    
        println!("Socket flushed, waiting for response...");
        let mut buffer = vec![0u8; 1024 * 1024];
        let n = socket.read(&mut buffer)
            .map_err(|e| format!("Failed to receive response: {}", e))?;
    
        println!("Received {} bytes", n);
        if n == 0 {
            return Err("Server closed connection".to_string());
        }
    
        let response = String::from_utf8(buffer[..n].to_vec())
            .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;
        
        println!("Received response: '{}'", response);
        Ok(response)
    }
    
    fn send_index_request(&self, file_path: &str, word_freqs: &HashMap<String, i64>) -> Result<(), String> {
        let mut current_batch = format!("INDEX_REQUEST {} ", file_path);
        let mut batch_word_count = 0;

        for (word, freq) in word_freqs {
            let word_entry = format!("{} {} ", word, freq);

            // If adding this word would exceed batch size, send current batch
            if current_batch.len() + word_entry.len() > BATCH_SIZE {
                let mut final_request = current_batch.trim().to_string();
                final_request.push('\n'); // Add newline at end of request
                self.send_message(&final_request)?;

                // Start new batch
                current_batch = format!("INDEX_REQUEST {} ", file_path);
                batch_word_count = 0;
            }

            current_batch.push_str(&word_entry);
            batch_word_count += 1;
        }

        // Send final batch if not empty
        if batch_word_count > 0 {
            let mut final_request = current_batch.trim().to_string();
            final_request.push('\n'); // Add newline at end of request
            self.send_message(&final_request)?;
        }

        Ok(())
    }    
     
    fn handle_search_reply(&self, reply: &str) -> Result<Vec<DocPathFreqPair>, String> {
        let lines: Vec<&str> = reply.split('\n').collect();
        if lines.is_empty() {
            return Err("Empty response".to_string());
        }
    
        let first_line = lines[0];
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() < 2 || parts[0] != "SEARCH_REPLY" {
            return Err("Invalid reply format".to_string());
        }
    
        let num_results: usize = parts[1].parse()
            .map_err(|_| "Invalid result count".to_string())?;
    
        if num_results == 0 {
            return Ok(Vec::new());
        }
    
        let mut results = Vec::new();
        // Process remaining lines for results
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(freq) = parts.last().unwrap().parse::<i64>() {
                    // The path is everything except the last part (frequency)
                    let doc_path = parts[..parts.len()-1].join(" ");
                    results.push(DocPathFreqPair {
                        document_path: doc_path,
                        word_frequency: freq,
                    });
                }
            }
        }
    
        Ok(results)
    }    
     
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_process_document() {
        let engine = ClientProcessingEngine::new();
        let content = "test-word another_word simple test-word";
        let freqs = engine.process_document(content);

        assert_eq!(freqs.get("test-word"), Some(&2));
        assert_eq!(freqs.get("another_word"), Some(&1));
        assert!(freqs.get("simple").is_none()); // too short
    }

    #[test]
    fn test_document_processing() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path)?;
        writeln!(file, "test-word another_word simple test-word")?;

        let engine = ClientProcessingEngine::new();
        let content = fs::read_to_string(&file_path)?;
        let freqs = engine.process_document(&content);

        assert_eq!(freqs.get("test-word"), Some(&2));
        Ok(())
    }
}

// src/client/engine.rs

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::fs;
use std::time::Instant;
use parking_lot::Mutex;
use walkdir::WalkDir;
use std::collections::HashMap;

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
        let mut current_word = String::with_capacity(64);
    
        for c in content.chars() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                current_word.push(c.to_ascii_lowercase());  // Convert to lowercase
            } else if !current_word.is_empty() {
                // Only index words between 3 and 20 chars to avoid extremes
                if current_word.len() >= 3 && current_word.len() <= 20 {
                    *word_freqs.entry(current_word.clone()).or_insert(0) += 1;
                }
                current_word.clear();
            }
        }
    
        // Handle last word
        if !current_word.is_empty() && current_word.len() >= 3 && current_word.len() <= 20 {
            *word_freqs.entry(current_word).or_insert(0) += 1;
        }
    
        word_freqs
    }
    
    pub fn search(&self, terms: Vec<String>) -> Result<SearchResult, String> {
        let start_time = Instant::now();

        if !self.is_connected.load(Ordering::Relaxed) {
            return Err("Not connected to server".to_string());
        }

        let request = format!("SEARCH_REQUEST {}", terms.join(" AND "));
        let response = self.send_message(&request)?;

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
        // Chunk word frequencies into batches of 100 words each
        const BATCH_SIZE: usize = 100;
        let word_freq_vec: Vec<(&String, &i64)> = word_freqs.iter().collect();
        let total_batches = (word_freq_vec.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        
        for (batch_num, chunk) in word_freq_vec.chunks(BATCH_SIZE).enumerate() {
            // Create batch request
            let mut request = format!("INDEX_REQUEST {} {}", file_path, chunk.len());
            for (word, freq) in chunk {
                request.push_str(&format!(" {} {}", word, freq));
            }
            
            // Send batch and check response
            println!("Sending batch {}/{} ({} words)...", 
                    batch_num + 1, total_batches, chunk.len());
            
            let response = self.send_message(&request)?;
            if response.trim() != "INDEX_REPLY SUCCESS" {
                return Err(format!("Failed to index batch: {}", response));
            }
        }
    
        println!("Successfully indexed {} words for {}", word_freqs.len(), file_path);
        Ok(())
    }
    
    fn handle_search_reply(&self, reply: &str) -> Result<Vec<DocPathFreqPair>, String> {
        let mut lines = reply.lines();
        let header = lines.next()
            .ok_or("Empty search reply")?;

        let mut parts = header.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("SEARCH_REPLY"), Some(count)) => {
                let count: usize = count.parse()
                    .map_err(|_| "Invalid result count")?;

                let mut results = Vec::with_capacity(count);
                for _ in 0..count {
                    if let Some(line) = lines.next() {
                        let mut parts = line.rsplitn(2, ' ');
                        if let (Some(freq_str), Some(path)) = (parts.next(), parts.next()) {
                            let word_frequency = freq_str.parse()
                                .map_err(|_| "Invalid frequency")?;
                            results.push(DocPathFreqPair {
                                document_path: path.to_string(),
                                word_frequency,
                            });
                        }
                    }
                }
                Ok(results)
            }
            _ => Err("Invalid search reply format".to_string())
        }
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

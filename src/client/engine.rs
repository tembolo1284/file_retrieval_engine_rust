// src/client/engine.rs

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::net::TcpStream;
use std::io::{BufReader, Read, Write};
use std::fs;
use std::time::Instant;
use parking_lot::Mutex;
use walkdir::WalkDir;
use std::collections::HashMap;
use crate::server::index_store::IndexStore;

const BATCH_SIZE: usize = 1024 * 1024;
const BUFFER_SIZE: usize = 64 * 1024;

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
    #[allow(dead_code)]
    store: Arc<IndexStore>,
}

impl ClientProcessingEngine {
    pub fn new() -> Self {
        ClientProcessingEngine {
            client_socket: Mutex::new(None),
            client_id: AtomicI32::new(-1),
            is_connected: AtomicBool::new(false),
            store: Arc::new(IndexStore::new()),
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
        let total_bytes = std::sync::atomic::AtomicI64::new(0);

        // Ensure we're connected
        if !self.is_connected.load(Ordering::Relaxed) {
            return Err("Not connected to server".to_string());
        }

        // Collect files
        let files: Vec<_> = WalkDir::new(folder_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        println!("Found {} files to process", files.len());

        // Process files in chunks
        let chunk_size = std::cmp::max(1, files.len() / 4); // Use 4 chunks or reasonable number
        
        for chunk in files.chunks(chunk_size) {
            let mut word_freq_buffer = HashMap::with_capacity(10000);
            
            for entry in chunk {
                // Read file content
                let content = fs::read_to_string(entry.path())
                    .map_err(|e| format!("Failed to read file {}: {}", entry.path().display(), e))?;
                
                // Update total bytes processed
                total_bytes.fetch_add(content.len() as i64, Ordering::Relaxed);
                
                // Process document and get word frequencies
                word_freq_buffer.clear();
                self.process_document_efficient(&content, &mut word_freq_buffer);
                
                // Send index request
                self.send_index_request(&entry.path().to_string_lossy(), &word_freq_buffer)?;
            }
        }

        let duration = start_time.elapsed();
        Ok(IndexResult {
            execution_time: duration.as_secs_f64(),
            total_bytes_read: total_bytes.load(Ordering::Relaxed),
        })
    }

    fn process_document_efficient(&self, content: &str, word_freqs: &mut HashMap<String, i64>) {
        let mut current_word = String::with_capacity(64);
        let mut in_word = false;

        for c in content.chars() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                current_word.push(c);
                in_word = true;
            } else if in_word {
                if current_word.len() >= 3 {  // Minimum word length threshold
                    *word_freqs.entry(current_word.clone()).or_insert(0) += 1;
                }
                current_word.clear();
                in_word = false;
            }
        }

        if in_word && current_word.len() >= 3 {
            *word_freqs.entry(current_word).or_insert(0) += 1;
        }
    }

    pub fn search(&self, terms: Vec<String>) -> Result<SearchResult, String> {
        let start_time = Instant::now();
    
        if !self.is_connected.load(Ordering::Relaxed) {
            return Err("Not connected to server".to_string());
        }
    
        let request = format!("SEARCH_REQUEST {}", terms.join(" "));
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
        println!("Attempting to connect to {}", addr);
        
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

        stream.set_nodelay(true)
            .map_err(|e| format!("Failed to set TCP_NODELAY: {}", e))?;

        *self.client_socket.lock() = Some(stream);
        
        let response = self.send_message("REGISTER_REQUEST")?;
        
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
        let mut socket = self.client_socket.lock();
        let socket = socket.as_mut()
            .ok_or("Not connected to server")?;
    
        let message = format!("{}\n", message);
        
        socket.write_all(message.as_bytes())
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        socket.flush()
            .map_err(|e| format!("Failed to flush socket: {}", e))?;
    
        // Use a buffered reader for more efficient reading
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, socket);
        let mut _response = String::with_capacity(BUFFER_SIZE);
        
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut read_buffer = Vec::with_capacity(BUFFER_SIZE);
        
        loop {
            let n = reader.read(&mut buffer)
                .map_err(|e| format!("Failed to receive response: {}", e))?;
                
            if n == 0 {
                return Err("Server closed connection".to_string());
            }
            
            read_buffer.extend_from_slice(&buffer[..n]);
            
            // Check if response is complete (ends with newline)
            if buffer[n-1] == b'\n' {
                break;
            }
        }
        
        String::from_utf8(read_buffer)
            .map_err(|e| format!("Invalid UTF-8 in response: {}", e))
    }
    
    fn send_index_request(&self, file_path: &str, word_freqs: &HashMap<String, i64>) -> Result<(), String> {
        // Pre-calculate capacity needed
        let mut total_len = format!("INDEX_REQUEST {} ", file_path).len();
        for (word, freq) in word_freqs {
            total_len += word.len() + 1 + freq.to_string().len() + 1;
        }
        
        // Use this to pre-allocate
        let mut current_batch = String::with_capacity(total_len.min(BATCH_SIZE));
        current_batch.push_str(&format!("INDEX_REQUEST {} ", file_path));
        
        let mut batch_word_count = 0;
        let mut batch_size = current_batch.len();

        // Sort by word length to optimize batch packing
        let mut sorted_entries: Vec<_> = word_freqs.iter().collect();
        sorted_entries.sort_by_key(|(word, _)| word.len());

        for (word, freq) in sorted_entries {
            let word_entry = format!("{} {} ", word, freq);
            let entry_size = word_entry.len();

            // If adding this word would exceed batch size, send current batch
            if batch_size + entry_size > BATCH_SIZE {
                current_batch.push('\n'); // Add newline at end of request
                self.send_message(&current_batch)?;

                // Start new batch
                current_batch = format!("INDEX_REQUEST {} ", file_path);
                batch_word_count = 0;
                batch_size = current_batch.len();
            }

            current_batch.push_str(&word_entry);
            batch_size += entry_size;
            batch_word_count += 1;
        }

        // Send final batch if not empty
        if batch_word_count > 0 {
            current_batch.push('\n'); // Add newline at end of request
            self.send_message(&current_batch)?;
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

        let mut results = Vec::with_capacity(num_results);
        
        // Process remaining lines for results
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                continue;
            }
            
            // Parse the line format: "Client N:path freq"
            if line.starts_with("Client ") {
                // Find the position of the last space (before frequency)
                if let Some(last_space) = line.rfind(' ') {
                    if let Ok(freq) = line[last_space+1..].parse::<i64>() {
                        // Extract client information and path
                        let path_and_client = &line[..last_space];
                        
                        results.push(DocPathFreqPair {
                            document_path: path_and_client.to_string(),
                            word_frequency: freq,
                        });
                    }
                }
            } else {
                // Handle old format for backward compatibility
                if let Some(last_space) = line.rfind(' ') {
                    if let Ok(freq) = line[last_space+1..].parse::<i64>() {
                        let doc_path = line[..last_space].to_string();
                        results.push(DocPathFreqPair {
                            document_path: doc_path,
                            word_frequency: freq,
                        });
                    }
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
        let mut freqs = HashMap::new();
        engine.process_document_efficient(content, &mut freqs);

        assert_eq!(freqs.get("test-word"), Some(&2));
        assert_eq!(freqs.get("another_word"), Some(&1));
        assert_eq!(freqs.get("simple"), Some(&1));
    }

    #[test]
    fn test_document_processing() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path)?;
        writeln!(file, "test-word another_word simple test-word")?;

        let engine = ClientProcessingEngine::new();
        let content = fs::read_to_string(&file_path)?;
        let mut freqs = HashMap::new();
        engine.process_document_efficient(&content, &mut freqs);

        assert_eq!(freqs.get("test-word"), Some(&2));
        Ok(())
    }
}

// src/server/engine.rs

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::common::thread_pool::ThreadPool;
use crate::server::index_store::IndexStore;

#[derive(Debug, Clone)]
pub struct ClientInfo {
    client_id: i32,
    ip_address: String,
    port: i32,
}

pub struct ServerProcessingEngine {
    store: Arc<IndexStore>,
    worker_pool: Arc<ThreadPool>,
    connected_clients: Arc<Mutex<HashMap<i32, ClientInfo>>>,
    next_client_id: AtomicI32,
    should_stop: AtomicBool,
    batch_sender: mpsc::Sender<IndexBatch>,
    server_socket: Arc<Mutex<Option<TcpListener>>>,
}

#[derive(Debug)]
struct IndexBatch {
    document_number: i64,
    word_frequencies: HashMap<String, i64>,
}

impl ServerProcessingEngine {
    pub fn clone(&self) -> Self {
        ServerProcessingEngine {
            store: Arc::clone(&self.store),
            worker_pool: Arc::clone(&self.worker_pool),
            connected_clients: Arc::clone(&self.connected_clients),
            next_client_id: AtomicI32::new(self.next_client_id.load(Ordering::Relaxed)),
            should_stop: AtomicBool::new(self.should_stop.load(Ordering::Relaxed)),
            batch_sender: self.batch_sender.clone(),
            server_socket: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new(store: Arc<IndexStore>, num_threads: usize) -> Result<Self, String> {
        let worker_pool = Arc::new(ThreadPool::new(num_threads)?);
        let (batch_sender, batch_receiver) = mpsc::channel(1000);

        let engine = ServerProcessingEngine {
            store,
            worker_pool,
            connected_clients: Arc::new(Mutex::new(HashMap::new())),
            next_client_id: AtomicI32::new(1),
            should_stop: AtomicBool::new(false),
            batch_sender,
            server_socket: Arc::new(Mutex::new(None)),
        };

        engine.start_batch_processor(batch_receiver);
        Ok(engine)
    }

    pub async fn initialize(&self, server_port: u16) -> Result<(), String> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", server_port))
            .map_err(|e| format!("Failed to bind to port {}: {}", server_port, e))?;

        listener.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking mode: {}", e))?;

        *self.server_socket.lock() = Some(listener);

        let engine = Arc::new(self.clone());
        tokio::spawn(async move {
            if let Err(e) = engine.run_dispatcher().await {
                eprintln!("Dispatcher error: {}", e);
            }
        });

        Ok(())
    }


    async fn run_dispatcher(&self) -> Result<(), String> {
        loop {
            if self.should_stop.load(Ordering::Relaxed) {
                break Ok(());
            }
    
            // Only hold the lock briefly to get the listener
            let listener = {
                let socket_guard = self.server_socket.lock();
                match &*socket_guard {
                    Some(l) => l.try_clone()
                        .map_err(|e| format!("Failed to clone listener: {}", e))?,
                    None => return Ok(()),
                }
            }; // Lock is dropped here
    
            match listener.accept() {
                Ok((stream, addr)) => {
                    let client_id = self.next_client_id.fetch_add(1, Ordering::SeqCst);
                    let client_info = ClientInfo {
                        client_id,
                        ip_address: addr.ip().to_string(),
                        port: addr.port() as i32,
                    };
    
                    self.connected_clients.lock().insert(client_id, client_info.clone());
    
                    let engine = Arc::new(self.clone());
                    self.worker_pool.execute(move || {
                        engine.handle_client(stream, client_info);
                    })?;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(e) => {
                    if !self.should_stop.load(Ordering::Relaxed) {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }
        }
    }    
    
    fn handle_client(&self, mut stream: TcpStream, client_info: ClientInfo) {
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer

        while !self.should_stop.load(Ordering::Relaxed) {
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if let Ok(message) = String::from_utf8(buffer[..n].to_vec()) {
                        let response = self.process_message(&message, &client_info);
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Failed to send response: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client: {}", e);
                    break;
                }
            }
        }

        // Clean up client connection
        self.connected_clients.lock().remove(&client_info.client_id);
    }

    fn process_message(&self, message: &str, client_info: &ClientInfo) -> String {
        let parts: Vec<&str> = message.split_whitespace().collect();
        if parts.is_empty() {
            return "ERROR Empty request".to_string();
        }

        match parts[0] {
            "REGISTER_REQUEST" => {
                format!("REGISTER_REPLY {}", client_info.client_id)
            }
            "INDEX_REQUEST" => {
                // Expected format: INDEX_REQUEST <doc_id> <word> <frequency> [<word> <frequency>...]
                if parts.len() < 4 || parts.len() % 2 != 0 {
                    return "ERROR Invalid index request format".to_string();
                }

                let doc_id = match parts[1].parse::<i64>() {
                    Ok(id) => id,
                    Err(_) => return "ERROR Invalid document ID".to_string(),
                };

                let mut word_frequencies = HashMap::new();
                for chunk in parts[2..].chunks(2) {
                    if chunk.len() != 2 {
                        return "ERROR Malformed word-frequency pair".to_string();
                    }

                    let word = chunk[0].to_string();
                    let frequency = match chunk[1].parse::<i64>() {
                        Ok(f) => f,
                        Err(_) => return "ERROR Invalid frequency value".to_string(),
                    };

                    word_frequencies.insert(word, frequency);
                }

                // Send the batch for processing
                let batch = IndexBatch {
                    document_number: doc_id,
                    word_frequencies,
                };

                match self.batch_sender.try_send(batch) {
                    Ok(_) => "INDEX_REPLY SUCCESS".to_string(),
                    Err(_) => "ERROR Failed to process index request".to_string(),
                }
            }
            "SEARCH_REQUEST" => {
                // Expected format: SEARCH_REQUEST <word1> [word2 word3...]
                if parts.len() < 2 {
                    return "ERROR No search terms provided".to_string();
                }

                let search_terms: Vec<String> = parts[1..].iter().map(|&s| s.to_string()).collect();
                let results = (*self.store).search(&search_terms);

                match results {
                    Ok(docs) => {
                        if docs.is_empty() {
                            "SEARCH_REPLY NO_RESULTS".to_string()
                        } else {
                            let mut reply = String::from("SEARCH_REPLY");
                            for (doc_id, score) in docs {
                                reply.push_str(&format!(" {} {:.2}", doc_id, score));
                            }
                            reply
                        }
                    }
                    Err(_) => "ERROR Failed to execute search".to_string(),
                }
            }
            _ => "ERROR Invalid request type".to_string(),
        }
    }    

    fn start_batch_processor(&self, mut batch_receiver: mpsc::Receiver<IndexBatch>) {
        let store = Arc::clone(&self.store);
        let should_stop = Arc::new(AtomicBool::new(false));

        tokio::spawn(async move {
            let mut current_batch = Vec::new();

            while !should_stop.load(Ordering::Relaxed) {
                while let Ok(batch) = batch_receiver.try_recv() {
                    current_batch.push((batch.document_number, batch.word_frequencies));
                    if current_batch.len() >= 100 {
                        store.batch_update_index(current_batch);
                        current_batch = Vec::new();
                    }
                }

                if !current_batch.is_empty() {
                    store.batch_update_index(current_batch);
                    current_batch = Vec::new();
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    pub async fn shutdown(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
        let mut socket_guard = self.server_socket.lock();
        if socket_guard.is_some() {
            *socket_guard = None; // This effectively drops the listener
        }
    }

    pub fn get_connected_clients(&self) -> Vec<String> {
        self.connected_clients
            .lock()
            .iter()
            .map(|(_, info)| {
                format!(
                    "Client ID: {}, IP: {}, Port: {}",
                    info.client_id, info.ip_address, info.port
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_initialization() {
        let store = Arc::new(IndexStore::new());
        let engine = ServerProcessingEngine::new(store, 4).unwrap();

        assert!(engine.initialize(0).await.is_ok()); // Port 0 lets OS choose port
        engine.shutdown().await;
    }
}

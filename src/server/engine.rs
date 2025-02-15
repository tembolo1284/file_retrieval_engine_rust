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
        let socket_clone = Arc::new(Mutex::new(None));  // Start with None
        if let Some(listener) = &*self.server_socket.lock() {
            if let Ok(cloned_listener) = listener.try_clone() {
                *socket_clone.lock() = Some(cloned_listener);
            }
        }
        
        ServerProcessingEngine {
            store: Arc::clone(&self.store),
            worker_pool: Arc::clone(&self.worker_pool),
            connected_clients: Arc::clone(&self.connected_clients),
            next_client_id: AtomicI32::new(self.next_client_id.load(Ordering::Relaxed)),
            should_stop: AtomicBool::new(self.should_stop.load(Ordering::Relaxed)),
            batch_sender: self.batch_sender.clone(),
            server_socket: socket_clone,
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
        println!("Initializing server on port {}", server_port);
        
        let listener = TcpListener::bind(format!("0.0.0.0:{}", server_port))
            .map_err(|e| format!("Failed to bind to port {}: {}", server_port, e))?;

        println!("Successfully bound to port {}", server_port);

        listener.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking mode: {}", e))?;

        println!("Set listener to non-blocking mode");

        // Store the listener
        {
            let mut guard = self.server_socket.lock();
            *guard = Some(listener);
            println!("Stored listener in server_socket. is_some: {:?}", guard.is_some());
        }

        println!("Starting dispatcher...");
        let engine = Arc::new(self.clone());
        
        tokio::spawn(async move {
            println!("Dispatcher task started");
            if let Err(e) = engine.run_dispatcher().await {
                eprintln!("Dispatcher error: {}", e);
            }
        });

        println!("Server initialization complete");
        Ok(())
    }

    async fn run_dispatcher(&self) -> Result<(), String> {
        loop {
            if self.should_stop.load(Ordering::Relaxed) {
                println!("Dispatcher stopping");
                break Ok(());
            }
    
            // Get the listener
            let listener = {
                let guard = self.server_socket.lock();
                match &*guard {
                    Some(l) => {
                        match l.try_clone() {
                            Ok(listener) => listener,
                            Err(e) => {
                                eprintln!("Failed to clone listener: {}", e);
                                continue;
                            }
                        }
                    },
                    None => {
                        println!("No listener found in socket_guard");
                        return Ok(());
                    }
                }
            };
    
            match listener.accept() {
                Ok((stream, addr)) => {
                    println!("Accepted new connection from: {}", addr);
                    
                    // Set the client stream to blocking mode
                    if let Err(e) = stream.set_nonblocking(false) {
                        eprintln!("Failed to set client stream to blocking mode: {}", e);
                        continue;
                    }
    
                    let client_id = self.next_client_id.fetch_add(1, Ordering::SeqCst);
                    let client_info = ClientInfo {
                        client_id,
                        ip_address: addr.ip().to_string(),
                        port: addr.port() as i32,
                    };
    
                    self.connected_clients.lock().insert(client_id, client_info.clone());
    
                    let engine = Arc::new(self.clone());
                    if let Err(e) = self.worker_pool.execute(move || {
                        engine.handle_client(stream, client_info);
                    }) {
                        eprintln!("Failed to spawn client handler: {}", e);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Increase sleep time to 1 second to reduce CPU usage and log spam
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
        println!("Client handler started for ID: {}", client_info.client_id);
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer

        while !self.should_stop.load(Ordering::Relaxed) {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    println!("Client {} disconnected", client_info.client_id);
                    break;
                }
                Ok(n) => {
                    if let Ok(message) = String::from_utf8(buffer[..n].to_vec()) {
                        println!("Received from client {}: '{}'", client_info.client_id, message.trim());
                        let response = self.process_message(&message, &client_info);
                        println!("Sending to client {}: '{}'", client_info.client_id, response.trim());
                        
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Failed to send response to client {}: {}", client_info.client_id, e);
                            break;
                        }
                        if let Err(e) = stream.flush() {
                            eprintln!("Failed to flush stream for client {}: {}", client_info.client_id, e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client {}: {}", client_info.client_id, e);
                    break;
                }
            }
        }

        println!("Client handler ending for ID: {}", client_info.client_id);
        self.connected_clients.lock().remove(&client_info.client_id);
    }

    fn process_message(&self, message: &str, client_info: &ClientInfo) -> String {
        let mut parts = message.split_whitespace();
        let response = match parts.next() {
            Some("REGISTER_REQUEST") => {
                format!("REGISTER_REPLY {}\n", client_info.client_id)
            }
            Some("INDEX_REQUEST") => {
                if let Some(_doc_id) = parts.next() {
                    let mut word_frequencies = HashMap::new();
                    while let (Some(word), Some(freq_str)) = (parts.next(), parts.next()) {
                        if let Ok(freq) = freq_str.parse::<i64>() {
                            word_frequencies.insert(word.to_string(), freq);
                        }
                    }
                    "INDEX_REPLY SUCCESS\n".to_string()
                } else {
                    "ERROR Invalid index request format\n".to_string()
                }
            }
            Some("SEARCH_REQUEST") => {
                let search_terms: Vec<String> = parts.map(String::from).collect();
                match (*self.store).search(&search_terms) {
                    Ok(results) => {
                        if results.is_empty() {
                            "SEARCH_REPLY NO_RESULTS\n".to_string()
                        } else {
                            let mut reply = String::from("SEARCH_REPLY");
                            for (doc_id, score) in results {
                                reply.push_str(&format!(" {} {:.2}", doc_id, score));
                            }
                            reply.push('\n');
                            reply
                        }
                    }
                    Err(_) => "ERROR Failed to execute search\n".to_string(),
                }
            }
            _ => "ERROR Invalid request\n".to_string()
        };

        println!("Processing message from client {}: '{}' -> '{}'", 
                 client_info.client_id, message.trim(), response.trim());
        response
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

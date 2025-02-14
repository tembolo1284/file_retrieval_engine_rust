// src/bin/benchmark.rs

use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;
use file_retrieval_engine::client::engine::{ClientProcessingEngine, IndexResult};

#[derive(Debug)]
struct BenchmarkResults {
    total_time: f64,
    total_bytes_processed: i64,
    client_results: Vec<IndexResult>,
}

async fn run_worker(
    client_id: i32,
    engine: Arc<ClientProcessingEngine>,
    server_ip: String,
    server_port: String,
    dataset_path: String,
) -> Result<IndexResult, String> {
    // Connect to server
    engine.connect(&server_ip, &server_port)?;
    println!("Client {} connected to server", client_id);

    // Index the dataset
    let result = engine.index_folder(&dataset_path)?;
    
    println!("Client {} finished indexing.", client_id);
    println!("Bytes processed: {}", result.total_bytes_read);
    println!("Time: {:.2}s", result.execution_time);

    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <server_ip> <server_port> <num_clients> <dataset_path1> [dataset_path2 ...]",
            args[0]
        );
        return Err("Invalid arguments".to_string());
    }

    let server_ip = args[1].clone();
    let server_port = args[2].clone();
    let num_clients = args[3].parse::<usize>()
        .map_err(|_| "Invalid number of clients")?;

    if num_clients <= 0 {
        return Err("Number of clients must be positive".to_string());
    }

    let mut dataset_paths = Vec::new();
    for i in 0..num_clients {
        if i + 4 >= args.len() {
            return Err("Not enough dataset paths provided".to_string());
        }
        dataset_paths.push(args[i + 4].clone());
    }

    let start_time = Instant::now();
    let mut benchmark_results = BenchmarkResults {
        total_time: 0.0,
        total_bytes_processed: 0,
        client_results: Vec::new(),
    };

    let mut join_set = JoinSet::new();
    let mut engines = Vec::new();

    // Create and start benchmark workers
    for i in 0..num_clients {
        let engine = Arc::new(ClientProcessingEngine::new());
        engines.push(Arc::clone(&engine));
        
        let server_ip = server_ip.clone();
        let server_port = server_port.clone();
        let dataset_path = dataset_paths[i].clone();

        join_set.spawn(async move {
            run_worker(i as i32 + 1, engine, server_ip, server_port, dataset_path).await
        });
    }

    // Wait for all workers to complete
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(result)) => {
                benchmark_results.total_bytes_processed += result.total_bytes_read;
                benchmark_results.client_results.push(result);
            }
            Ok(Err(e)) => eprintln!("Worker error: {}", e),
            Err(e) => eprintln!("Join error: {}", e),
        }
    }

    benchmark_results.total_time = start_time.elapsed().as_secs_f64();

    // Print benchmark results
    println!("\n----------------------------------------");
    println!("Benchmark Results:");
    println!("Number of clients: {}", num_clients);
    println!("Total execution time: {:.2} seconds", benchmark_results.total_time);
    println!("Total data processed: {} bytes", benchmark_results.total_bytes_processed);
    
    let throughput = (benchmark_results.total_bytes_processed as f64 / (1024.0 * 1024.0)) 
        / benchmark_results.total_time;
    println!("Overall throughput: {:.2} MB/s", throughput);

    // Per-client statistics
    println!("\nPer-client Statistics:");
    for (i, result) in benchmark_results.client_results.iter().enumerate() {
        let client_throughput = (result.total_bytes_read as f64 / (1024.0 * 1024.0)) 
            / result.execution_time;
        println!("Client {}:", i + 1);
        println!("  Time: {:.2}s", result.execution_time);
        println!("  Data: {} bytes", result.total_bytes_read);
        println!("  Throughput: {:.2} MB/s", client_throughput);
    }
    println!("----------------------------------------");

    // Run search queries on the first client
    if !engines.is_empty() {
        println!("\nRunning benchmark searches on first client...");
        let test_queries = vec![
            vec!["test".to_string()],
            vec!["test".to_string(), "AND".to_string(), "example".to_string()],
            vec!["performance".to_string(), "AND".to_string(), "benchmark".to_string(), 
                 "AND".to_string(), "test".to_string()],
        ];

        for query in test_queries {
            let query_str: String = query.join(" ");
            println!("\nExecuting query: {}", query_str);

            match engines[0].search(query) {
                Ok(result) => {
                    println!("Search completed in {:.3} seconds", result.execution_time);
                    println!("Found {} results", result.document_frequencies.len());
                }
                Err(e) => eprintln!("Search error: {}", e),
            }
        }
    }

    // Disconnect all clients
    for engine in engines {
        if let Err(e) = engine.disconnect() {
            eprintln!("Error disconnecting client: {}", e);
        }
    }

    Ok(())
}

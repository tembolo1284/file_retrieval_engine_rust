# File Retrieval Engine

A distributed file indexing and retrieval system implemented in Rust, supporting multiple concurrent clients and efficient search capabilities.

## Features

- Distributed architecture with client-server model
- Multi-threaded document processing
- Sharded index store for better performance
- Concurrent client connections
- Batch processing of index updates
- Real-time search capabilities
- Performance benchmarking tools

## System Components

### Server
- Multi-threaded document indexing
- Sharded index store (256 shards)
- Connection management
- Search processing
- Batch update processing

### Client
- Document processing and indexing
- Search functionality
- Connection management
- Command-line interface

### Benchmark Tool
- Multi-client performance testing
- Throughput measurements
- Search performance testing

## Building the Project

### Prerequisites

- Rust 1.70.0 or later
- Cargo (comes with Rust)
- Linux/Unix environment (for POSIX socket support)

### Build Instructions

1. Clone the repository:
```bash
git clone https://github.com/yourusername/file-retrieval-engine.git
cd file-retrieval-engine

```
2. Build the project:

```
cargo build --release

This will create three executables in `target/release`:

- file-retrieval-server
- file-retrieval-client
- file-retrieval-benchmark

```

### Running the System

### Starting the Server

```
./target/release/file-retrieval-server <port>

# Example:
./target/release/file-retrieval-server 12345

```

### Server commands:

- list - Show connected clients

- quit - Shutdown the server

- help - Show available commands

### Running a Client

```
./target/release/file-retrieval-client

``` 

### Then use the following commands:

```
- connect <server_ip> <port>
- index <folder_path>
- search <term1> AND <term2> ...
- get_info
- quit
```

### Client Commands

- connect <server_ip> <port> - Connect to server

- get_info - Display client ID

- index <folder_path> - Index documents in folder

- search <term1> AND <term2> ... - Search indexed documents

- help - Show available commands

- quit - Exit client

### Running Benchmarks

```
./target/release/file-retrieval-benchmark <server_ip> <port> <num_clients> <dataset_path1> [dataset_path2 ...]

# Example:
./target/release/file-retrieval-benchmark localhost 8080 4 ./dataset1 ./dataset2 ./dataset3 ./dataset4
```

### Architecture Details

### Index Store

- Sharded architecture for better concurrency

- Separate shards for document mapping and term index

- Thread-safe operations using RwLock

- Efficient batch update processing

### Network Protocol

#### Messages supported:

- REGISTER_REQUEST/REPLY

- INDEX_REQUEST/REPLY

- SEARCH_REQUEST/REPLY

- QUIT_REQUEST

### Threading Model

- Server uses a thread pool for client connections

- Batch processing thread for index updates

- Client uses async I/O for network operations

### Performance Considerations

- The index store uses 256 shards to minimize lock contention

- Batch processing reduces index update overhead

- Non-blocking I/O for network operations

- Efficient memory usage with smart pointers

- Thread pool for managing client connections

### Development

### Running Tests

```
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Building Documentation

```
cargo doc --no-deps --open
```



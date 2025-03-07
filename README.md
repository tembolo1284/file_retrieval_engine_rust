# File Retrieval Engine
A distributed file indexing and retrieval system implemented in Rust, supporting multiple concurrent clients and efficient search capabilities. Now with Python bindings!

## Features
- Distributed architecture with client-server model
- Multi-threaded document processing
- Sharded index store for better performance
- Concurrent client connections
- Batch processing of index updates
- Real-time search capabilities
- Performance benchmarking tools
- **Python bindings for cross-language usage**

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

### Python Integration
- PyO3-based Python bindings
- Persistent index storage
- Python CLI mirroring Rust functionality
- Cross-language interoperability

## Building the Project

### Prerequisites
- Rust 1.70.0 or later
- Cargo (comes with Rust)
- Python 3.7 or later (for Python bindings)
- Linux/Unix environment (for POSIX socket support)

### Build Instructions

1. Clone the repository:
```bash
git clone https://github.com/tembolo1284/file-retrieval-engine.git
cd file-retrieval-engine
```

2. Build the project with the automated build script:
```bash
# Run the complete build script (Rust + Python bindings)
./rebuild.sh

# Activate the Python environment
source ./activate_env.sh
```

This will create:
- Three executables in `target/release`:
  - file-retrieval-server
  - file-retrieval-client
  - file-retrieval-benchmark
- Python bindings installed in a virtual environment

## Running the System

### Starting the Server
```bash
./target/release/file-retrieval-server <port>
# Example:
./target/release/file-retrieval-server 12345
```

### Server commands:
- `list` - Show connected clients
- `quit` - Shutdown the server
- `help` - Show available commands

### Running a Client (Rust)
```bash
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

### Running a Client (Python)
```bash
# Ensure the Python environment is activated
source ./activate_env.sh

# Use the Python CLI
python file_retrieval_cli.py <command>
```

### Python CLI Commands
```
# Index documents locally (persistent)
python file_retrieval_cli.py index <folder_path>

# Search locally (uses persistent index)
python file_retrieval_cli.py search <term1> <term2> ...

# Clear local index
python file_retrieval_cli.py clear-index

# Connect to server and index documents 
python file_retrieval_cli.py client-index <server_ip> <port> <folder_path>

# Connect to server and search
python file_retrieval_cli.py client-search <server_ip> <port> <term1> <term2> ...

# Run benchmark
python file_retrieval_cli.py benchmark <server_ip> <port> <num_threads> <folder1> [<folder2> ...]
```

### Client Commands (Rust)
- `connect <server_ip> <port>` - Connect to server
- `get_info` - Display client ID
- `index <folder_path>` - Index documents in folder
- `search <term1> AND <term2> ...` - Search indexed documents
- `help` - Show available commands
- `quit` - Exit client

### Running Benchmarks (Rust)
```bash
./target/release/file-retrieval-benchmark <server_ip> <port> <num_clients> <dataset_path1> [dataset_path2 ...]
# Example:
./target/release/file-retrieval-benchmark localhost 8080 4 ./dataset1 ./dataset2 ./dataset3 ./dataset4
```

## Python Integration Details

### Python Bindings
The system provides Python bindings using PyO3, allowing Python applications to leverage the speed and efficiency of the Rust implementation.

### Key Python Files
- `python/file_retrieval_engine/__init__.py` - Package initialization
- `file_retrieval_cli.py` - Command-line interface mirroring Rust functionality
- `persistent_index.py` - Index persistence implementation
- `rebuild.sh` - Complete build script for Rust and Python components
- `activate_env.sh` - Helper script to activate Python environment

### Building Python Bindings Manually
```bash
# Install maturin (builds Rust into Python packages)
pip install "maturin>=0.14,<0.15"

# Build and install the Python package
maturin develop --release
```

### Python Classes
- `PyIndexStore` - Python wrapper for the Rust IndexStore
- `PyClient` - Python wrapper for the Rust Client
- `PersistentIndex` - Python implementation of persistent index storage

## Architecture Details

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

## Development

### Running Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Building Documentation
```bash
cargo doc --no-deps --open
```
This will build the Rust documentation and open it in your default web browser. The documentation is at:
- `target/doc/file_retrieval_engine/index.html`

### Python Development
For Python development, an editable install is recommended:
```bash
source .venv/bin/activate
maturin develop
```

This allows you to modify the Python code without rebuilding the Rust components.

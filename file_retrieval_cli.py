#!/usr/bin/env python3
"""
Command-line interface for the file retrieval engine using Python bindings.
This script mimics the functionality of the Rust CLI but uses Python bindings.
"""

import argparse
import os
import sys
import time
from file_retrieval_engine import PyClient, PyIndexStore


def cmd_index(args):
    """Index a folder locally."""
    print(f"Indexing folder: {args.folder}")
    
    start_time = time.time()
    index = PyIndexStore()
    
    # Walk through the directory and index each file
    file_count = 0
    total_bytes = 0
    
    for root, _, files in os.walk(args.folder):
        for file in files:
            if file.endswith(".txt"):
                file_path = os.path.join(root, file)
                doc_id = index.put_document(file_path)
                
                # Index the content
                try:
                    with open(file_path, 'r', encoding='utf-8') as f:
                        content = f.read()
                    
                    total_bytes += len(content)
                    
                    # Simple word frequency calculation
                    words = content.lower().split()
                    word_freq = {}
                    for word in words:
                        word = word.strip('.,!?;:()[]{}"\'-')
                        if word:
                            word_freq[word] = word_freq.get(word, 0) + 1
                    
                    # Update the index with word frequencies
                    index.update_index(doc_id, word_freq)
                    file_count += 1
                    
                    if file_count % 100 == 0:
                        current_time = time.time()
                        elapsed = current_time - start_time
                        if elapsed > 0:
                            current_throughput = total_bytes / elapsed
                            print(f"Indexed {file_count} files... {current_throughput:.2f} bytes/second")
                        else:
                            print(f"Indexed {file_count} files...")
                except Exception as e:
                    print(f"Error indexing {file_path}: {e}")
    
    end_time = time.time()
    duration = end_time - start_time
    
    print("\n=== Indexing Complete ===")
    print(f"Total files indexed: {file_count}")
    print(f"Total bytes processed: {total_bytes:,} bytes")
    print(f"Total time: {duration:.2f} seconds")
    print(f"Overall throughput: {total_bytes/duration:.2f} bytes/second")
    if file_count > 0:
        print(f"Average file size: {total_bytes/file_count:,.2f} bytes")


def cmd_search(args):
    """Search for terms locally."""
    index = PyIndexStore()
    
    terms_str = " ".join(args.terms)
    print(f"> search {terms_str}")
    start_time = time.time()
    
    results = index.search(args.terms)
    
    end_time = time.time()
    duration = end_time - start_time
    
    # Sort results by score in descending order
    results.sort(key=lambda x: x[1], reverse=True)
    
    # Limit to top 10 results for display
    top_results = results[:10]
    
    print(f"Search completed in {duration:.2f} seconds")
    print(f"Search results (top {len(top_results)} out of {len(results)}):")
    
    if not results:
        print("No matching documents found.")
    else:
        for doc_path, score in top_results:
            # Extract client and folder information from path
            path_parts = doc_path.split('/')
            client_info = ""
            
            # Look for client pattern in path
            for part in path_parts:
                if part.startswith("client_"):
                    client_idx = path_parts.index(part)
                    if client_idx > 0 and path_parts[client_idx-1].endswith("clients"):
                        client_info = f"Client {part.split('_')[1]}:"
                    break
            
            # Create path without the full prefix
            short_path = "/".join(path_parts[path_parts.index(client_info.split(':')[0]) if client_info else 0:])
            if not client_info:
                short_path = doc_path
                
            print(f"* {short_path}:{score}")


def cmd_client_index(args):
    """Connect to a server and index a folder."""
    client = PyClient()
    
    try:
        print(f"Connecting to server at {args.host}:{args.port}")
        client.connect(args.host, args.port)
        print("Connected to server")
        
        print(f"Indexing folder: {args.folder}")
        execution_time, bytes_read = client.index_folder(args.folder)
        
        print(f"Folder indexed in {execution_time:.2f} seconds")
        print(f"Processed {bytes_read} bytes at {bytes_read/execution_time:.2f} bytes/second")
    except Exception as e:
        print(f"Error: {e}")


def cmd_client_search(args):
    """Connect to a server and search for terms."""
    client = PyClient()
    
    try:
        print(f"Connecting to server at {args.host}:{args.port}")
        client.connect(args.host, args.port)
        print("Connected to server")
        
        terms_str = " ".join(args.terms)
        print(f"> search {terms_str}")
        execution_time, results = client.search(args.terms)
        
        # Sort results by score in descending order
        results.sort(key=lambda x: x[1], reverse=True)
        
        # Limit to top 10 results for display
        top_results = results[:10]
        
        print(f"Search completed in {execution_time:.2f} seconds")
        print(f"Search results (top {len(top_results)} out of {len(results)}):")
        
        if not results:
            print("No matching documents found.")
        else:
            for doc_path, score in top_results:
                # Extract client and folder information from path
                path_parts = doc_path.split('/')
                client_info = ""
                
                # Look for client pattern in path
                for part in path_parts:
                    if part.startswith("client_"):
                        client_idx = path_parts.index(part)
                        if client_idx > 0 and path_parts[client_idx-1].endswith("clients"):
                            client_info = f"Client {part.split('_')[1]}:"
                        break
                
                # Create path without the full prefix
                if client_info:
                    try:
                        start_idx = path_parts.index(client_info.split(':')[0])
                        short_path = client_info + "/".join(path_parts[start_idx+1:])
                    except ValueError:
                        short_path = doc_path
                else:
                    short_path = doc_path
                    
                print(f"* {short_path}:{score}")
    except Exception as e:
        print(f"Error: {e}")


def cmd_benchmark(args):
    """Run a benchmark with multiple clients."""
    print(f"Running benchmark with {len(args.folders)} clients")
    print(f"Server: {args.host}:{args.port}")
    print(f"Thread count: {args.threads}")
    
    clients = []
    results = []
    
    # Create and connect clients
    for i, folder in enumerate(args.folders):
        try:
            client = PyClient()
            client.connect(args.host, args.port)
            clients.append((client, folder))
            print(f"Client {i+1} connected and will index {folder}")
        except Exception as e:
            print(f"Error connecting client {i+1}: {e}")
    
    # Start benchmark
    start_time = time.time()
    
    # Run indexing in series (for simplicity)
    for i, (client, folder) in enumerate(clients):
        try:
            print(f"Client {i+1} indexing {folder}...")
            execution_time, bytes_read = client.index_folder(folder)
            results.append((execution_time, bytes_read))
            print(f"Client {i+1} completed in {execution_time:.2f} seconds, {bytes_read} bytes")
        except Exception as e:
            print(f"Error with client {i+1}: {e}")
    
    end_time = time.time()
    total_duration = end_time - start_time
    total_bytes = sum(bytes for _, bytes in results)
    
    print("\nBenchmark Results:")
    print(f"Total time: {total_duration:.2f} seconds")
    print(f"Total bytes: {total_bytes}")
    print(f"Throughput: {total_bytes/total_duration:.2f} bytes/second")
    
    for i, (duration, bytes_read) in enumerate(results):
        print(f"Client {i+1}: {duration:.2f} seconds, {bytes_read} bytes, "
              f"{bytes_read/duration:.2f} bytes/second")


def main():
    """Main entry point for the CLI."""
    parser = argparse.ArgumentParser(
        description="File Retrieval Engine CLI (Python version)"
    )
    subparsers = parser.add_subparsers(dest="command", help="Commands")
    
    # Local index command
    index_parser = subparsers.add_parser("index", help="Index a folder locally")
    index_parser.add_argument("folder", help="Folder to index")
    
    # Local search command
    search_parser = subparsers.add_parser("search", help="Search locally")
    search_parser.add_argument("terms", nargs="+", help="Terms to search for")
    
    # Client index command
    client_index_parser = subparsers.add_parser(
        "client-index", help="Connect to a server and index a folder"
    )
    client_index_parser.add_argument("host", help="Server hostname or IP")
    client_index_parser.add_argument("port", help="Server port")
    client_index_parser.add_argument("folder", help="Folder to index")
    
    # Client search command
    client_search_parser = subparsers.add_parser(
        "client-search", help="Connect to a server and search"
    )
    client_search_parser.add_argument("host", help="Server hostname or IP")
    client_search_parser.add_argument("port", help="Server port")
    client_search_parser.add_argument("terms", nargs="+", help="Terms to search for")
    
    # Benchmark command
    benchmark_parser = subparsers.add_parser(
        "benchmark", help="Run a benchmark with multiple clients"
    )
    benchmark_parser.add_argument("host", help="Server hostname or IP")
    benchmark_parser.add_argument("port", help="Server port")
    benchmark_parser.add_argument(
        "threads", type=int, help="Number of threads per client"
    )
    benchmark_parser.add_argument(
        "folders", nargs="+", help="Folders to index (one per client)"
    )
    
    args = parser.parse_args()
    
    if args.command == "index":
        cmd_index(args)
    elif args.command == "search":
        cmd_search(args)
    elif args.command == "client-index":
        cmd_client_index(args)
    elif args.command == "client-search":
        cmd_client_search(args)
    elif args.command == "benchmark":
        cmd_benchmark(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()

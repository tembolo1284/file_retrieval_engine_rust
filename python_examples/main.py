#!/usr/bin/env python3
"""
Example of using the file retrieval engine from Python.
"""

from file_retrieval_engine import PyClient, PyIndexStore

def local_index_example():
    """Example using the local index store."""
    print("Creating a local index store...")
    index = PyIndexStore()
    
    # Add a document to the index
    doc_path = "./dataset1_client_server/1_client/client_1"
    print(f"Adding document: {doc_path}")
    doc_id = index.put_document(doc_path)
    print(f"Document added with ID: {doc_id}")
    
    # Search for terms in the indexed documents
    search_terms = ["child-like", "search", "terms"]
    print(f"Searching for terms: {search_terms}")
    results = index.search(search_terms)
    print(f"Search results: {results}")

def client_example():
    """Example using the client to connect to a server."""
    print("Creating a client...")
    client = PyClient()
    
    try:
        # Connect to the server
        server_ip = "127.0.0.1"
        server_port = "12345"
        print(f"Connecting to server at {server_ip}:{server_port}")
        client.connect(server_ip, server_port)
        print("Connected to server")
        
        # Index a folder
        folder_path = "./dataset1_client_server/1_client/client_1"
        print(f"Indexing folder: {folder_path}")
        execution_time, bytes_read = client.index_folder(folder_path)
        print(f"Folder indexed in {execution_time:.2f} seconds, {bytes_read} bytes read")
        
        # Search for terms
        search_terms = ["example", "search", "terms"]
        print(f"Searching for terms: {search_terms}")
        execution_time, results = client.search(search_terms)
        print(f"Search completed in {execution_time:.2f} seconds")
        print(f"Search results: {results}")
    
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    print("File Retrieval Engine Python Example")
    print("------------------------------------")
    
    # Uncomment the example you want to run:
    local_index_example()
    # client_example()

#!/usr/bin/env python3

import os
import time
from file_retrieval_engine import PyClient, PyIndexStore

def local_index_example():
    """Example using the local index store."""
    print("Creating a local index store...")
    index = PyIndexStore()
    
    # Add a document to the index
    doc_path = "./dataset1_client_server/1_client/client_1/folder3/Document10379.txt"
    print(f"Adding document: {doc_path}")
    doc_id = index.put_document(doc_path)
    print(f"Document added with ID: {doc_id}")
    
    # Create word frequencies manually and update the index
    # This step is crucial - without it, the search won't find any terms
    with open(doc_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Simple word frequency calculation
    words = content.lower().split()
    word_freq = {}
    for word in words:
        # Clean the word (remove punctuation)
        word = word.strip('.,!?;:()[]{}"\'-')
        if word:
            word_freq[word] = word_freq.get(word, 0) + 1
    
    print(f"Found {len(word_freq)} unique words in the document")
    
    # Update the index with word frequencies
    index.update_index(doc_id, word_freq)
    print("Index updated with word frequencies")
    
    # Search for terms in the indexed documents
    search_terms = ["child-like", "search", "terms"]
    print(f"Searching for terms: {search_terms}")
    results = index.search(search_terms)
    print(f"Search results: {results}")

def folder_index_example():
    """Example using local index store to index a full folder."""
    print("Creating a local index store...")
    index = PyIndexStore()
    
    folder_path = "./dataset1_client_server/1_client/client_1"
    if not os.path.exists(folder_path):
        print(f"Folder {folder_path} does not exist!")
        return
    
    print(f"Indexing folder: {folder_path}")
    start_time = time.time()
    
    # Walk through the directory and index each file
    file_count = 0
    for root, _, files in os.walk(folder_path):
        for file in files:
            if file.endswith(".txt"):
                file_path = os.path.join(root, file)
                doc_id = index.put_document(file_path)
                
                # Index the content
                try:
                    with open(file_path, 'r', encoding='utf-8') as f:
                        content = f.read()
                    
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
                    
                    if file_count % 10 == 0:
                        print(f"Indexed {file_count} files...")
                except Exception as e:
                    print(f"Error indexing {file_path}: {e}")
    
    end_time = time.time()
    print(f"Indexed {file_count} files in {end_time - start_time:.2f} seconds")
    
    # Search for some terms
    search_terms = ["child-like", "search", "computer"]
    print(f"Searching for terms: {search_terms}")
    results = index.search(search_terms)
    
    # Pretty print the results
    print(f"Found {len(results)} matching documents")
    for doc_path, score in results:
        print(f"  {doc_path}: score {score}")

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
        search_terms = ["child-like", "search", "computer"]
        print(f"Searching for terms: {search_terms}")
        execution_time, results = client.search(search_terms)
        print(f"Search completed in {execution_time:.2f} seconds")
        
        # Pretty print the results
        print(f"Found {len(results)} matching documents")
        for doc_path, score in results:
            print(f"  {doc_path}: score {score}")
    
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    print("File Retrieval Engine Python Example")
    print("------------------------------------")
    
    # Uncomment the example you want to run:
    local_index_example()
    # folder_index_example()
    # client_example()

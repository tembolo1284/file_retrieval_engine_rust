import file_retrieval_engine as fre
import sys
import time

def main():
    print("File Retrieval Engine Python Client")
    
    try:
        # Create a client and connect to the server
        print("Connecting to server...")
        client = fre.PyClient()
        client.connect("127.0.0.1", "12345")
        print("Connected successfully!")
        
        # Index a folder
        folder_path = "./dataset1_client_server/2_clients/client_1/"
        print(f"Indexing folder: {folder_path}")
        execution_time, bytes_read = client.index_folder(folder_path)
        print(f"Indexing completed in {execution_time:.2f} seconds")
        print(f"Bytes processed: {bytes_read}")
        
        # Search for a term
        search_term = "child-like"
        print(f"\nSearching for: '{search_term}'")
        execution_time, results = client.search([search_term])
        print(f"Search completed in {execution_time:.2f} seconds")
        print(f"Found {len(results)} results:")
        
        for path, freq in results:
            print(f"* {path}: {freq}")
    
    except Exception as e:
        print(f"Error: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())

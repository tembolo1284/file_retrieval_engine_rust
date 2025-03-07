#!/usr/bin/env python3
"""
Module for providing persistence to the PyIndexStore.
"""

import os
import json
import pickle
import tempfile
from pathlib import Path
from file_retrieval_engine import PyIndexStore

# Directory to store index data
INDEX_DIR = os.path.expanduser("~/.file_retrieval_engine")

class PersistentIndex:
    """Wrapper around PyIndexStore that provides persistence."""
    
    def __init__(self, index_name="default"):
        """Initialize with an optional index name."""
        self.index_name = index_name
        self.index_path = Path(INDEX_DIR) / f"{index_name}_index"
        self.documents_path = Path(INDEX_DIR) / f"{index_name}_documents.json"
        
        # Create directory if it doesn't exist
        os.makedirs(INDEX_DIR, exist_ok=True)
        
        # Initialize index and document mapping
        self.index = PyIndexStore()
        self.documents = {}
        
        # Load existing data if available
        self._load()
    
    def put_document(self, document_path):
        """Add a document to the index and store its path."""
        doc_id = self.index.put_document(document_path)
        self.documents[doc_id] = document_path
        self._save_documents()
        return doc_id
    
    def update_index(self, doc_id, word_freqs):
        """Update the index with word frequencies."""
        self.index.update_index(doc_id, word_freqs)
        self._save_index()
    
    def search(self, terms):
        """Search the index for terms."""
        return self.index.search(terms)
    
    def _save_documents(self):
        """Save document mapping to JSON file."""
        with open(self.documents_path, 'w') as f:
            json.dump(self.documents, f)
    
    def _save_index(self):
        """Save index to a temporary file."""
        # Currently PyIndexStore doesn't have a built-in save/load, so
        # this is a placeholder. The index will be rebuilt from documents.
        pass
    
    def _load(self):
        """Load index and documents if they exist."""
        if self.documents_path.exists():
            try:
                with open(self.documents_path, 'r') as f:
                    self.documents = json.load(f)
                
                # Currently, just populate the index from the saved documents
                # In a real implementation, we'd directly load the index state
                print(f"Loading index with {len(self.documents)} documents...")
                for doc_id, doc_path in self.documents.items():
                    try:
                        # Re-add the document to the index
                        self.index.put_document(doc_path)
                        
                        # Re-calculate word frequencies and update the index
                        with open(doc_path, 'r', encoding='utf-8') as f:
                            content = f.read()
                        
                        # Simple word frequency calculation
                        words = content.lower().split()
                        word_freq = {}
                        for word in words:
                            word = word.strip('.,!?;:()[]{}"\'-')
                            if word:
                                word_freq[word] = word_freq.get(word, 0) + 1
                        
                        # Update the index with word frequencies
                        self.index.update_index(int(doc_id), word_freq)
                    except Exception as e:
                        print(f"Error loading document {doc_path}: {e}")
                
                print("Index loaded successfully")
            except Exception as e:
                print(f"Error loading index: {e}")
    
    def clear(self):
        """Clear the index and remove saved files."""
        # Initialize a new index
        self.index = PyIndexStore()
        self.documents = {}
        
        # Remove saved files
        if self.documents_path.exists():
            os.remove(self.documents_path)

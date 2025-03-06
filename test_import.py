print("Attempting to import file_retrieval_engine...")
try:
    import file_retrieval_engine
    print("Success! Module imported.")
    print("Available objects:", dir(file_retrieval_engine))
except ImportError as e:
    print("Import failed:", e)
    
    # Try alternative import paths
    print("\nTrying alternative imports:")
    try:
        from file_retrieval_engine import file_retrieval_engine
        print("Success with from file_retrieval_engine import file_retrieval_engine")
    except ImportError as e:
        print("Failed:", e)

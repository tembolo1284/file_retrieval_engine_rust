#!/bin/bash
set -e

echo "=== Debugging Python bindings ==="

# 1. Clean up previous builds
echo "Cleaning up previous builds..."
rm -rf target/release/build/file-retrieval-engine-*
rm -rf python/file_retrieval_engine/*.so
rm -rf python/file_retrieval_engine/*.pyd
rm -rf python/file_retrieval_engine/*.dylib

# 2. Run maturin with verbose output
echo "Running maturin with debug output..."
RUST_BACKTRACE=1 maturin build --release -v

# 3. Examine the library with nm
echo "Examining the built library for PyInit symbols..."
find target -name "*.so" -o -name "*.dylib" | while read lib; do
    echo "Checking $lib for PyInit symbols..."
    nm -gD "$lib" | grep -i pyinit || echo "  No PyInit symbols found"
done

# 4. List the Python package contents
echo "Python package contents:"
find python -type f | sort

# 5. Create a test script
echo "Creating test script..."
cat > test_import.py << EOF
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
EOF

# 6. Install and test
echo "Installing and testing..."
maturin develop --release
python test_import.py

echo "=== Debug complete ==="

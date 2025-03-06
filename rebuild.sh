#!/bin/bash
set -e  # Exit immediately if any command fails

# Print header
echo "===================================================="
echo "File Retrieval Engine - Clean Rebuild Script"
echo "===================================================="

# Function to display steps
function step() {
    echo ""
    echo "→ $1"
    echo "----------------------------------------------------"
}

# Step 1: Clean everything
step "Cleaning project"
echo "Removing previous builds and virtual environments..."
rm -rf target/
rm -rf .venv/
rm -rf *.egg-info/
rm -rf dist/
rm -rf build/
rm -rf python/file_retrieval_engine/*.so
rm -rf python/file_retrieval_engine/*.pyd
rm -rf python/file_retrieval_engine/*.dylib

# Step 2: Update Cargo.toml and other configurations
step "Configuring project"
cat > pyproject.toml << 'EOF'
[build-system]
requires = ["maturin>=0.14,<0.15"]
build-backend = "maturin"

[project]
name = "file_retrieval_engine"
version = "0.1.0"
description = "Python bindings for the file retrieval engine"
requires-python = ">=3.7"

[tool.maturin]
# Simple configuration
python-source = "python"
EOF

cat > python/file_retrieval_engine/__init__.py << 'EOF'
# __init__.py
"""Python bindings for the file retrieval engine"""
from .file_retrieval_engine import PyClient, PyIndexStore

__all__ = ["PyClient", "PyIndexStore"]
EOF

# Step 3: Build Rust library
step "Building Rust library"
cargo clean
cargo build --release

# Step 4: Set up Python environment
step "Setting up Python environment"
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install "maturin>=0.14,<0.15"

# Step 5: Build Python wheel
step "Building Python wheel with Maturin"
maturin build --release -v

# Step 6: Install the wheel
step "Installing the wheel"
maturin develop --release

# Step 7: Verify installation
step "Verifying installation"
echo "Testing Python bindings..."
python3 -c "
try:
    import file_retrieval_engine
    print('SUCCESS: Module imported successfully!')
    print('Available objects:', dir(file_retrieval_engine))
except Exception as e:
    print('ERROR:', e)
"

echo ""
echo "===================================================="
echo "Build process completed!"
echo "===================================================="

#!/bin/bash
set -e  # Exit immediately if any command fails

# Configuration
PROJECT_ROOT=$(pwd)
VENV_DIR=".venv"
BUILD_TYPE="release"  # Options: debug, release

# Print header
echo "===================================================="
echo "File Retrieval Engine - Build Script"
echo "===================================================="

# Function to display steps
function step() {
    echo ""
    echo "→ $1"
    echo "----------------------------------------------------"
}

# Check for required tools
step "Checking required tools"
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 not found. Please install Python 3."
    exit 1
fi

# Build Rust binaries
step "Building Rust executables"
if [ "$BUILD_TYPE" == "release" ]; then
    echo "Building in release mode..."
    cargo build --release
    BINARY_DIR="target/release"
else
    echo "Building in debug mode..."
    cargo build
    BINARY_DIR="target/debug"
fi

echo "Rust binaries built successfully:"
ls -l $BINARY_DIR/file-retrieval-*

# Set up Python environment
step "Setting up Python environment"
echo "Creating Python virtual environment..."
python3 -m venv $VENV_DIR
source $VENV_DIR/bin/activate

# Install uv (optional)
step "Installing uv package manager"
pip install uv

# Install build dependencies
step "Installing build dependencies"
# Fixed command - add quotes around version specifier
uv pip install "maturin>=0.14,<0.15"

# Build Python bindings
step "Building Python bindings"
echo "Building Python package with Maturin..."
if [ "$BUILD_TYPE" == "release" ]; then
    maturin develop --release
else
    maturin develop
fi

# Install dependencies for examples
step "Installing additional Python dependencies"
uv pip install pytest     # Add any other dependencies you need

# Verify the installation
step "Verifying installation"
echo "Testing Python bindings..."
python -c "import file_retrieval_engine; print('Python bindings imported successfully!')"

# Create symbolic links to executables (optional)
step "Creating symbolic links to executables"
mkdir -p bin
ln -sf $PROJECT_ROOT/$BINARY_DIR/file-retrieval-client bin/
ln -sf $PROJECT_ROOT/$BINARY_DIR/file-retrieval-server bin/
ln -sf $PROJECT_ROOT/$BINARY_DIR/file-retrieval-benchmark bin/

echo ""
echo "===================================================="
echo "Build completed successfully!"
echo ""
echo "Rust binaries: $BINARY_DIR/"
echo "Python environment: $VENV_DIR/"
echo "Symlinked executables: bin/"
echo ""
echo "To activate the Python environment:"
echo "  source $VENV_DIR/bin/activate"
echo ""
echo "To run the example:"
echo "  python python_examples/main.py"
echo "===================================================="

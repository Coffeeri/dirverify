#!/bin/bash
# Build script for dirverify - cross-platform static binaries

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building dirverify for multiple platforms...${NC}"

# Create output directory
mkdir -p target/release/binaries

# Function to build for a target
build_target() {
    local target=$1
    local output_name=$2
    
    echo -e "${YELLOW}Building for ${target}...${NC}"
    
    # Install target if not already installed
    rustup target add ${target} 2>/dev/null || true
    
    # Build with static linking
    if [[ "$target" == *"linux"* ]]; then
        # For Linux, use musl for true static binaries
        if [[ "$target" == *"musl"* ]]; then
            RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target ${target}
        else
            cargo build --release --target ${target}
        fi
    else
        cargo build --release --target ${target}
    fi
    
    # Copy and rename binary
    if [[ "$target" == *"windows"* ]]; then
        cp target/${target}/release/dirverify.exe target/release/binaries/${output_name}
    else
        cp target/${target}/release/dirverify target/release/binaries/${output_name}
    fi
    
    echo -e "${GREEN}✓ Built ${output_name}${NC}"
}

# Install required tools
echo -e "${YELLOW}Installing required tools...${NC}"
cargo install cross --quiet 2>/dev/null || true

# Build for all targets
build_target "x86_64-unknown-linux-musl" "dirverify-linux-x64"
build_target "aarch64-unknown-linux-musl" "dirverify-linux-arm64"
build_target "x86_64-pc-windows-gnu" "dirverify-windows-x64.exe"
build_target "aarch64-apple-darwin" "dirverify-macos-arm64"

# Additional x86_64 macOS build (if on macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
    build_target "x86_64-apple-darwin" "dirverify-macos-x64"
fi

# Create checksums
echo -e "${YELLOW}Creating checksums...${NC}"
cd target/release/binaries
sha256sum * > checksums.sha256

echo -e "${GREEN}Build complete! Binaries are in target/release/binaries/${NC}"
ls -la target/release/binaries/

#!/bin/bash

# Verification script for Alpenglow setup
# This script checks if your environment is ready to build and test Alpenglow

set -e

echo "🔍 Verifying Alpenglow Setup..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check Rust installation
echo "1. Checking Rust installation..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✓${NC} Rust installed: $RUST_VERSION"
    
    # Check version (need 1.90+)
    RUST_VER_NUM=$(rustc --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
    RUST_MAJOR=$(echo $RUST_VER_NUM | cut -d. -f1)
    RUST_MINOR=$(echo $RUST_VER_NUM | cut -d. -f2)
    
    if [ "$RUST_MAJOR" -gt 1 ] || ([ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -ge 90 ]); then
        echo -e "${GREEN}✓${NC} Rust version is 1.90 or later"
    else
        echo -e "${YELLOW}⚠${NC} Rust version may be too old (need 1.90+, have $RUST_VER_NUM)"
        echo "   Run: rustup update stable"
    fi
else
    echo -e "${RED}✗${NC} Rust not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Cargo
echo ""
echo "2. Checking Cargo..."
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    echo -e "${GREEN}✓${NC} Cargo installed: $CARGO_VERSION"
else
    echo -e "${RED}✗${NC} Cargo not found"
    exit 1
fi

# Check if we're in the right directory
echo ""
echo "3. Checking project directory..."
if [ -f "Cargo.toml" ] && [ -f "src/lib.rs" ]; then
    echo -e "${GREEN}✓${NC} In Alpenglow project directory"
else
    echo -e "${RED}✗${NC} Not in Alpenglow project directory"
    echo "   Please run this script from the alpenglow root directory"
    exit 1
fi

# Check for optional tools
echo ""
echo "4. Checking optional tools..."
MISSING_TOOLS=0

if command -v cargo-nextest &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-nextest installed"
else
    echo -e "${YELLOW}⚠${NC} cargo-nextest not found (optional, but recommended for tests)"
    echo "   Install with: cargo install cargo-nextest"
    MISSING_TOOLS=1
fi

if command -v cargo-machete &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-machete installed"
else
    echo -e "${YELLOW}⚠${NC} cargo-machete not found (optional, for check.sh)"
    echo "   Install with: cargo install cargo-machete"
    MISSING_TOOLS=1
fi

# Check if project builds
echo ""
echo "5. Testing project build..."
if cargo check --quiet 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Project builds successfully"
else
    echo -e "${YELLOW}⚠${NC} Project doesn't build yet (this is normal on first run)"
    echo "   Run: cargo build"
    echo "   This will download dependencies and may take several minutes"
fi

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $MISSING_TOOLS -eq 0 ]; then
    echo -e "${GREEN}✓ Setup looks good!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Build the project: cargo build"
    echo "  2. Run tests: ./test.sh"
    echo "  3. Run demo: ./run.sh"
else
    echo -e "${YELLOW}⚠ Setup is mostly ready, but some optional tools are missing${NC}"
    echo ""
    echo "You can still proceed, but for full functionality:"
    echo "  - Install cargo-nextest for better test output"
    echo "  - Install cargo-machete for check.sh"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"


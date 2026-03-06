#!/bin/bash
# QBNex Installer Build Script (Linux/macOS)
# This script builds the QBNex release binary

set -e

echo "========================================"
echo "  QBNex Build Script"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Build the project
echo -e "${GREEN}Building QBNex...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}ERROR Build failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Build completed successfully!${NC}"
echo ""

# Check if executable exists
if [ ! -f "target/release/qb" ]; then
    echo -e "${RED}ERROR Executable not found at target/release/qb${NC}"
    exit 1
fi

# Make executable
chmod +x target/release/qb

echo "========================================"
echo -e "${GREEN}  Build successful!${NC}"
echo "========================================"
echo ""
echo -e "${YELLOW}Executable location target/release/qb${NC}"
echo ""
echo -e "${CYAN}To install system-wide${NC}"
echo -e "${NC}  sudo cp target/release/qb /usr/local/bin/${NC}"
echo ""
echo -e "${CYAN}To test${NC}"
echo -e "${NC}  ./target/release/qb --version${NC}"
echo ""

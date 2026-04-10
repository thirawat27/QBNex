#!/bin/bash
# =============================================================================
# QBNex macOS Setup Script
# Copyright © 2026 thirawat27
# Version: 1.0.0
# Description: Builds and configures QBNex compiler for macOS systems
# =============================================================================

set -e

# Configuration
QBNEX_VERSION="1.0.0"
QBNEX_OWNER="thirawat27"
QBNEX_YEAR="2026"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print banner
print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                   QBNex Setup for macOS                   ║"
    echo "║              Modern BASIC to Native Compiler              ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo -e "${GREEN}Version:${NC} $QBNEX_VERSION"
    echo -e "${GREEN}Owner:${NC} $QBNEX_OWNER"
    echo -e "${GREEN}Year:${NC} $QBNEX_YEAR"
    echo ""
}

# Print status message
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

# Print warning message
print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

# Print error message
print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Check Xcode command line tools
check_xcode() {
    print_status "Checking Xcode command line tools..."
    
    if ! xcode-select -v >/dev/null 2>&1; then
        print_warning "Xcode command line tools not found"
        echo ""
        echo "Installing Xcode command line tools..."
        xcode-select --install
        print_status "Please complete the installation and re-run this script"
        exit 0
    else
        print_status "Xcode command line tools installed"
    fi
}

# Create necessary directories
create_directories() {
    print_status "Creating directory structure..."
    
    mkdir -p "$PROJECT_ROOT/bin"
    mkdir -p "$PROJECT_ROOT/tmp"
    mkdir -p "$PROJECT_ROOT/logs"
    mkdir -p "$PROJECT_ROOT/cache"
    
    print_status "Directory structure created"
}

# Build the compiler
build_compiler() {
    print_status "Building QBNex compiler..."
    
    cd "$PROJECT_ROOT"
    
    # Check if source exists
    if [ -d "internal/c" ]; then
        print_status "Compiling C++ runtime libraries..."
        
        # Compile runtime library
        cd internal/c
        
        # Compile with optimizations
        clang++ -O2 -std=c++11 -c libqb.cpp -o libqb.o \
            -DQB64_MACOSX \
            -DDEPENDENCY_CONSOLE_ONLY \
            -I. \
            -Iparts/core/gl_headers
            
        if [ $? -eq 0 ]; then
            print_status "Runtime library compiled successfully"
        else
            print_error "Failed to compile runtime library"
            exit 1
        fi
        
        cd "$PROJECT_ROOT"
    else
        print_warning "Source directory not found, skipping build"
    fi
}

# Setup environment
setup_environment() {
    print_status "Setting up environment..."
    
    # Create version file
    echo "From git $QBNEX_VERSION" > "$PROJECT_ROOT/internal/version.txt"
    
    print_status "Environment configured"
}

# Main function
main() {
    print_banner
    check_xcode
    create_directories
    build_compiler
    setup_environment
    
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}QBNex setup completed successfully!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "To use QBNex, run:"
    echo "  ./bin/qb yourfile.bas"
    echo ""
    echo "Or add to your PATH:"
    echo "  export PATH=\"\$PATH:$PROJECT_ROOT/bin\""
    echo ""
}

# Run main function
main "$@"

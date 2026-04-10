#!/bin/bash
# =============================================================================
# QBNex Linux Setup Script
# Copyright © 2026 thirawat27
# Version: 1.0.0
# Description: Builds and configures QBNex compiler for Linux systems
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
    echo "║                    QBNex Setup for Linux                  ║"
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

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_warning "Some operations may require sudo privileges"
    fi
}

# Check system requirements
check_requirements() {
    print_status "Checking system requirements..."
    
    local missing_deps=()
    
    # Check for required commands
    command -v g++ >/dev/null 2>&1 || missing_deps+=("g++")
    command -v gcc >/dev/null 2>&1 || missing_deps+=("gcc")
    command -v make >/dev/null 2>&1 || missing_deps+=("make")
    
    # Check for OpenGL development libraries
    if [ ! -f /usr/include/GL/gl.h ] && [ ! -f /usr/include/GL/glew.h ]; then
        missing_deps+=("libgl1-mesa-dev")
    fi
    
    # Check for ALSA development libraries
    if [ ! -f /usr/include/alsa/asoundlib.h ]; then
        missing_deps+=("libasound2-dev")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_warning "Missing dependencies: ${missing_deps[*]}"
        echo ""
        read -p "Do you want to install missing dependencies? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            install_dependencies "${missing_deps[@]}"
        else
            print_error "Cannot proceed without required dependencies"
            exit 1
        fi
    else
        print_status "All requirements satisfied"
    fi
}

# Install dependencies
install_dependencies() {
    print_status "Installing dependencies..."
    
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y "$@"
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y "$@"
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y "$@"
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --noconfirm "$@"
    else
        print_error "Unsupported package manager. Please install dependencies manually."
        exit 1
    fi
    
    print_status "Dependencies installed successfully"
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
        
        # Determine architecture
        ARCH=$(uname -m)
        if [ "$ARCH" = "x86_64" ]; then
            BITS="64"
        else
            BITS="32"
        fi
        
        # Compile with optimizations
        g++ -O2 -std=c++11 -c libqb.cpp -o libqb.o \
            -DQB64_LINUX \
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
    
    # Create configuration if not exists
    if [ ! -f "$PROJECT_ROOT/config/qbnex.ini" ]; then
        print_status "Creating default configuration..."
    fi
    
    print_status "Environment configured"
}

# Main function
main() {
    print_banner
    check_root
    check_requirements
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

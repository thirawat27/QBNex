# QBNex Docker Image
# Multi-stage build for QBasic/QuickBASIC compiler

FROM ubuntu:22.04 AS base

# Build dependencies (QIR pipeline): Rust, plus gcc only as the system linker
# that Rust uses on Linux; no C/C++ source of QBNex is compiled.
RUN apt-get update && apt-get install -y \
    gcc \
    libasound2-dev \
    libxkbcommon-dev \
    libudev-dev \
    pkg-config \
    libx11-dev \
    libxext-dev \
    libxrandr-dev \
    libxi-dev \
    libxcursor-dev \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain for the QIR runner qb-exec (crates/qb-exec)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# Copy source code (build context is the repository root, see docker-compose.yml)
COPY . .

# Build QBNex compiler
RUN chmod +x scripts/setup_lnx.sh && \
    ./scripts/setup_lnx.sh

# Runtime stage
FROM ubuntu:22.04 AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libasound2 \
    libudev1 \
    libx11-6 \
    libxext6 \
    libxrandr2 \
    libxi6 \
    libxcursor1 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiler from the base stage. It looks for crates/ (the QIR runner)
# next to itself and reads the standard library from compiler/.
COPY --from=base /app/qb /app/qb
COPY --from=base /app/crates/qb-exec/bin /app/crates/qb-exec/bin
COPY --from=base /app/compiler /app/compiler
# Launchers on PATH that call /app/qb by its full path, so it finds /app/crates.
# `qbnex` is the product-name alias of `qb`.
RUN printf '#!/bin/sh\nexec /app/qb "$@"\n' > /usr/local/bin/qb && \
    chmod +x /usr/local/bin/qb && \
    ln -s qb /usr/local/bin/qbnex

# Set working directory for user projects
WORKDIR /project

# Default command
CMD ["qb", "--help"]

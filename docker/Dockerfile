# QBNex Docker Image
# Multi-stage build for QBasic/QuickBASIC compiler

FROM ubuntu:22.04 AS base

# Install build dependencies
RUN apt-get update && apt-get install -y \
    g++ \
    make \
    libasound2-dev \
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

# Rust toolchain for the qb-runtime library (crates/qb-runtime)
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

# Copy built compiler from base stage
COPY --from=base /app/qb /usr/local/bin/qb
COPY --from=base /app/internal /app/internal
COPY --from=base /app/crates /app/crates
COPY --from=base /app/compiler /app/compiler
# Product-name alias: `qbnex` runs the same compiler as `qb`
RUN ln -s qb /usr/local/bin/qbnex

# Set working directory for user projects
WORKDIR /project

# Default command
CMD ["qb", "--help"]

# QBNex Docker Image
# Multi-stage build for QBasic/QuickBASIC compiler

FROM ubuntu:22.04 AS base

# Install build dependencies
RUN apt-get update && apt-get install -y \
    g++ \
    make \
    libglu1-mesa-dev \
    libasound2-dev \
    libx11-dev \
    libgl-dev \
    libglu-dev \
    libxext-dev \
    libxrandr-dev \
    libxi-dev \
    libxcursor-dev \
    zlib1g-dev \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Build QBNex compiler
RUN chmod +x setup_lnx.sh && \
    ./setup_lnx.sh

# Runtime stage
FROM ubuntu:22.04 AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libglu1-mesa \
    libasound2 \
    libx11-6 \
    libxext6 \
    libxrandr2 \
    libxi6 \
    libxcursor1 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

# Copy built compiler from base stage
COPY --from=base /app/qb /usr/local/bin/qb
COPY --from=base /app/internal /app/internal
COPY --from=base /app/source /app/source

# Set working directory for user projects
WORKDIR /project

# Default command
CMD ["qb", "--help"]

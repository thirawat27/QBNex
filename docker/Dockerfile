# QBNex Docker Image
# Multi-stage build for QBasic/QuickBASIC compiler

# Base images are pinned by digest (the multi-arch index of ubuntu:22.04 when
# this was written), so a rebuild uses the same system. Update the digest on
# purpose: docker buildx imagetools inspect ubuntu:22.04
FROM ubuntu:22.04@sha256:74b12b11ffa550fba1929ad4d8d16533c34f6612728469671afba1b03b66881c AS base

# Build dependencies (QIR pipeline): Rust, plus gcc only as the system linker
# that Rust uses on Linux.
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
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain for the QIR runner qb-exec (crates/engine/exec): a pinned
# rustup-init, checked against its SHA-256 before it runs (no `curl | sh`).
# rustup then installs the toolchain rust-toolchain.toml names (1.100.0).
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN set -eu; \
    case "$(uname -m)" in \
      x86_64) triple=x86_64-unknown-linux-gnu; sum=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55 ;; \
      aarch64) triple=aarch64-unknown-linux-gnu; sum=9f608c142e0e38a78957d0661ba1dfaf0b6fb37184cfaa4b9a0aa57af6184282 ;; \
      *) echo "no pinned rustup-init for $(uname -m)" >&2; exit 1 ;; \
    esac; \
    curl --proto '=https' --tlsv1.2 -sSfo /tmp/rustup-init "https://static.rust-lang.org/rustup/archive/1.29.3/$triple/rustup-init"; \
    echo "$sum  /tmp/rustup-init" | sha256sum -c -; \
    chmod +x /tmp/rustup-init; \
    /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.100.0; \
    rm /tmp/rustup-init

# Set working directory
WORKDIR /app

# Copy source code (build context is the repository root, see docker-compose.yml)
COPY . .

# Build QBNex compiler
RUN chmod +x scripts/setup_lnx.sh && \
    ./scripts/setup_lnx.sh

# Runtime stage
FROM ubuntu:22.04@sha256:74b12b11ffa550fba1929ad4d8d16533c34f6612728469671afba1b03b66881c AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libasound2 \
    libudev1 \
    libx11-6 \
    libxext6 \
    libxrandr2 \
    libxi6 \
    libxcursor1 \
    libxkbcommon-x11-0 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiler from the base stage. It looks for crates/ (the QIR runner)
# next to itself and reads the standard library from compiler/.
COPY --from=base /app/qb /app/qb
COPY --from=base /app/crates/engine/exec/bin /app/crates/engine/exec/bin
COPY --from=base /app/compiler /app/compiler
COPY --from=base /app/LICENSE /app/LICENSE
COPY --from=base /app/licenses /app/licenses
# Launchers on PATH that call /app/qb by its full path, so it finds /app/crates.
# `qbnex` is the product-name alias of `qb`.
RUN printf '#!/bin/sh\nexec /app/qb "$@"\n' > /usr/local/bin/qb && \
    chmod +x /usr/local/bin/qb && \
    ln -s qb /usr/local/bin/qbnex

# Run as a non-root user. The compiler writes /app/target (build folders,
# qb-config.ini) next to itself, so that folder and /project belong to the user.
RUN useradd --create-home --uid 1000 qbnex && \
    mkdir -p /app/target /project && \
    chown qbnex:qbnex /app/target /project
USER qbnex

# Set working directory for user projects
WORKDIR /project

# Default command
CMD ["qb", "--help"]

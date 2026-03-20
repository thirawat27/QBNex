FROM rust:1.81-bookworm AS builder

WORKDIR /app

COPY . .

RUN cargo build --release -p cli_tool

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY --from=builder /app/target/release/qb /usr/local/bin/qb
COPY --from=builder /app/examples /opt/qbnex/examples
COPY --from=builder /app/README.md /opt/qbnex/README.md

ENV QBNEX_EXAMPLES=/opt/qbnex/examples

ENTRYPOINT ["qb"]
CMD ["--help"]

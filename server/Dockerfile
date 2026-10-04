# Multi-stage Dockerfile for proof-battle-server

FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Copy root workspace and server manifests
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/

# Create dummy src to cache dependencies
RUN mkdir -p server/src && \
    echo "fn main() {}" > server/src/main.rs && \
    cargo build --release --manifest-path server/Cargo.toml && \
    rm -rf server/src

# Copy actual source code
COPY server/src server/src
COPY server/migrations server/migrations

# Touch main.rs to invalidate dummy build and compile release binary
RUN touch server/src/main.rs && \
    cargo build --release --manifest-path server/Cargo.toml --bin proof-battle-server

# Runtime stage
FROM debian:bookworm-slim AS runner

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Non-root user with uid 10001
RUN groupadd -g 10001 proofbattle && \
    useradd -u 10001 -g proofbattle -m -s /bin/false proofbattle

WORKDIR /home/proofbattle

COPY --from=builder --chown=proofbattle:proofbattle /app/target/release/proof-battle-server /usr/local/bin/proof-battle-server

USER 10001:10001

ENV BIND_ADDR=0.0.0.0:3000
EXPOSE 3000

HEALTHCHECK --interval=5s --timeout=3s --retries=5 --start-period=5s \
    CMD curl -f http://127.0.0.1:3000/readyz || exit 1

ENTRYPOINT ["/usr/local/bin/proof-battle-server"]

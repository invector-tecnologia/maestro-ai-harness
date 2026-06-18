# Build Stage
FROM rust:1.77-slim-bookworm AS builder
WORKDIR /usr/src/maestro
COPY . .
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

# Install certificates for any external HTTPS calls to LLM providers
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/maestro/target/release/maestro /usr/local/bin/maestro

# The TUI requires an interactive terminal, so you must run it with `docker run -it`
ENTRYPOINT ["maestro"]
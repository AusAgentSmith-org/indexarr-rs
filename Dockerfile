# =============================================================================
# Stage 1: Build Vue 3 UI
# =============================================================================
FROM node:22-alpine AS ui-builder

WORKDIR /ui
COPY ui/package.json ui/package-lock.json* ./
RUN npm install

COPY ui/ ./
RUN npm run build

# =============================================================================
# Stage 2: Build Rust binary
# =============================================================================
FROM rust:1-bookworm AS rust-builder

WORKDIR /build

# Configure git for private dependencies (persists in /root/.gitconfig)
ARG GIT_AUTH_TOKEN=""
ARG PLUGIN_PASSWORD=""
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN TOKEN="${GIT_AUTH_TOKEN:-$PLUGIN_PASSWORD}" && \
    git config --global url."http://x-access-token:${TOKEN}@100.92.54.45:3002/".insteadOf "http://100.92.54.45:3002/" && \
    printf '[registries.forgejo]\nindex = "sparse+https://repo.indexarr.net/api/packages/indexarr/cargo/"\ncredential-provider = "cargo:token"\n\n[registry]\ndefault = "forgejo"\n' > $CARGO_HOME/config.toml && \
    printf '[registries.forgejo]\ntoken = "Bearer %s"\n' "$TOKEN" > $CARGO_HOME/credentials.toml

# Cache dependencies by building a dummy project first
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true

# Build the real binary
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# Remove git credentials from final layer
RUN rm -f /root/.gitconfig

# =============================================================================
# Stage 3: Runtime image (minimal)
# =============================================================================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /build/target/release/indexarr /app/indexarr

# Copy Vue UI build
COPY --from=ui-builder /ui/dist /app/ui/dist

# Copy classifier rules if present
COPY classifier.yml* /app/

# Data directory
RUN mkdir -p /app/data

ENV INDEXARR_HOST=0.0.0.0
ENV INDEXARR_PORT=8080
ENV INDEXARR_DATA_DIR=/app/data

EXPOSE 8080
EXPOSE 6881-6884/udp
EXPOSE 6890
EXPOSE 6895/udp

ENTRYPOINT ["/app/indexarr"]
CMD ["--all"]

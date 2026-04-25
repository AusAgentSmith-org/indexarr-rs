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

# GIT_AUTH_TOKEN is the Forgejo bearer token used to fetch librtbit-* deps from
# the Forgejo cargo registry. Passed in as a build_arg from .woodpecker.yml.
# PLUGIN_PASSWORD is the fallback when running under woodpeckerci/plugin-docker-buildx
# without an explicit build_args entry (the plugin auto-exposes its `password:`
# field as $PLUGIN_PASSWORD inside the build context).
ARG GIT_AUTH_TOKEN=""
ARG PLUGIN_PASSWORD=""

WORKDIR /build

# Wire up the Forgejo cargo registry inside the builder. Mirrors the project's
# .cargo/config.toml so deps with `registry = "forgejo"` resolve at build time.
RUN printf '[registries.forgejo]\nindex = "sparse+https://repo.indexarr.net/api/packages/indexarr/cargo/"\ncredential-provider = "cargo:token"\n\n[registry]\ndefault = "forgejo"\n' > $CARGO_HOME/config.toml && \
    TOKEN="${GIT_AUTH_TOKEN:-$PLUGIN_PASSWORD}" && \
    printf '[registries.forgejo]\ntoken = "Bearer %s"\n' "$TOKEN" > $CARGO_HOME/credentials.toml

# Cache dependencies by building a dummy project first
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true

# Build the real binary
COPY src/ src/
RUN touch src/main.rs && cargo build --release

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

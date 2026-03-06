# ============================================================
# FILE:        Dockerfile
# MODULE:      Phase 5 — Cloud Backend > Container Build
# TASK:        T-060 (Session 8 hardening)
# PLATFORM:    linux (server)
# AUTHOR:      FocusMe Co-Pilot (Claude Opus)
# GENERATED:   Session 8
# PURPOSE:     Multi-stage Docker build for production deployment
# KNOWN LIMITATIONS: Requires Rust 1.76+ for compilation.
#   Binary is statically linked against OpenSSL (libssl3).
# ============================================================

# ── Stage 1: Builder ────────────────────────────────────────
FROM rust:1.76-bookworm AS builder

WORKDIR /app

# Cache dependencies by building a dummy project first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy full source and build
COPY . .
RUN cargo build --release -p focusme-backend

# ── Stage 2: Runtime ────────────────────────────────────────
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash focusme

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/focusme-backend .

# Copy migrations for runtime migration execution
COPY --from=builder /app/migrations ./migrations

# Switch to non-root user
USER focusme

# Expose the HTTP port
EXPOSE 8080

# Health check — verifies the server is responding
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the backend server
ENTRYPOINT ["./focusme-backend"]

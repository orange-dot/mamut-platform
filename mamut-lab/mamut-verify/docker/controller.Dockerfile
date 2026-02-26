# =============================================================================
# Mamut Controller Dockerfile
# =============================================================================
# Multi-stage build for the mamut-controller binary.
# The controller orchestrates distributed verification tests, coordinating
# agents and managing test execution.
#
# Build: docker build -f docker/controller.Dockerfile -t mamut-controller .
# Run:   docker run -p 8080:8080 -p 50051:50051 mamut-controller
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Environment
# -----------------------------------------------------------------------------
# Using rust:1.83-bookworm as the build base provides:
# - Official Rust toolchain with Cargo
# - Debian Bookworm's build tools and libraries
# - Consistent build environment across CI and local development
FROM rust:1.83-bookworm AS builder

# Install build dependencies
# - protobuf-compiler: Required for compiling .proto files (gRPC/tonic-build)
# - pkg-config: Helps locate system libraries during compilation
# - libssl-dev: OpenSSL development files for TLS support
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory for the build
WORKDIR /build

# Copy workspace configuration files first for better layer caching
# This allows Docker to cache dependency downloads when only source changes
COPY Cargo.toml Cargo.lock* ./

# Copy the workspace member Cargo.toml files to establish dependency graph
# Order matches workspace.members in root Cargo.toml
COPY crates/mamut-core/Cargo.toml crates/mamut-core/
COPY crates/mamut-orchestrator/Cargo.toml crates/mamut-orchestrator/
COPY crates/mamut-generator/Cargo.toml crates/mamut-generator/
COPY crates/mamut-nemesis/Cargo.toml crates/mamut-nemesis/
COPY crates/mamut-checker/Cargo.toml crates/mamut-checker/
COPY crates/mamut-history/Cargo.toml crates/mamut-history/
COPY crates/mamut-minimizer/Cargo.toml crates/mamut-minimizer/
COPY crates/mamut-report/Cargo.toml crates/mamut-report/
COPY crates/mamut-transport/Cargo.toml crates/mamut-transport/
COPY crates/mamut-ffi/Cargo.toml crates/mamut-ffi/
COPY binaries/mamut-controller/Cargo.toml binaries/mamut-controller/
COPY binaries/mamut-agent/Cargo.toml binaries/mamut-agent/

# Create dummy source files to build dependencies first (layer caching)
# This technique separates dependency compilation from source compilation
RUN mkdir -p crates/mamut-core/src && echo "pub fn dummy() {}" > crates/mamut-core/src/lib.rs && \
    mkdir -p crates/mamut-orchestrator/src && echo "pub fn dummy() {}" > crates/mamut-orchestrator/src/lib.rs && \
    mkdir -p crates/mamut-generator/src && echo "pub fn dummy() {}" > crates/mamut-generator/src/lib.rs && \
    mkdir -p crates/mamut-nemesis/src && echo "pub fn dummy() {}" > crates/mamut-nemesis/src/lib.rs && \
    mkdir -p crates/mamut-checker/src && echo "pub fn dummy() {}" > crates/mamut-checker/src/lib.rs && \
    mkdir -p crates/mamut-history/src && echo "pub fn dummy() {}" > crates/mamut-history/src/lib.rs && \
    mkdir -p crates/mamut-minimizer/src && echo "pub fn dummy() {}" > crates/mamut-minimizer/src/lib.rs && \
    mkdir -p crates/mamut-report/src && echo "pub fn dummy() {}" > crates/mamut-report/src/lib.rs && \
    mkdir -p crates/mamut-transport/src && echo "pub fn dummy() {}" > crates/mamut-transport/src/lib.rs && \
    mkdir -p crates/mamut-ffi/src && echo "pub fn dummy() {}" > crates/mamut-ffi/src/lib.rs && \
    mkdir -p binaries/mamut-controller/src && echo "fn main() {}" > binaries/mamut-controller/src/main.rs && \
    mkdir -p binaries/mamut-agent/src && echo "fn main() {}" > binaries/mamut-agent/src/main.rs

# Copy proto files (needed for tonic-build)
COPY proto/ proto/

# Build dependencies only (this layer will be cached)
RUN cargo build --release --package mamut-controller 2>/dev/null || true

# Remove dummy source files
RUN find crates binaries -name "*.rs" -delete

# Copy actual source code
COPY crates/ crates/
COPY binaries/ binaries/

# Touch all source files to invalidate the cached build of our code
# (but keep cached dependencies)
RUN find crates binaries -name "*.rs" -exec touch {} +

# Build the controller binary in release mode
# Release mode enables optimizations: LTO, single codegen unit (from Cargo.toml)
RUN cargo build --release --package mamut-controller

# -----------------------------------------------------------------------------
# Stage 2: Runtime Environment
# -----------------------------------------------------------------------------
# Using debian:bookworm-slim provides:
# - Minimal Debian base (~30MB smaller than full Debian)
# - Compatible glibc version with build stage
# - Security updates from Debian stable
FROM debian:bookworm-slim AS runtime

# Labels for container metadata (OCI image spec)
LABEL org.opencontainers.image.title="Mamut Controller"
LABEL org.opencontainers.image.description="Central controller for orchestrating distributed verification tests"
LABEL org.opencontainers.image.vendor="Mamut Lab"
LABEL org.opencontainers.image.source="https://github.com/mamut-lab/mamut-verify"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

# Install runtime dependencies
# - ca-certificates: Required for HTTPS/TLS connections to external services
# - tini: Minimal init system that properly handles signals and reaps zombies
#         (addresses PID 1 zombie reaping problem in containers)
# - libssl3: OpenSSL runtime library for TLS support
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
# Running as non-root prevents privilege escalation attacks
RUN groupadd --gid 1000 mamut && \
    useradd --uid 1000 --gid mamut --shell /bin/false --create-home mamut

# Create directories for runtime data
# - /data: Persistent storage for test histories, checkpoints
# - /config: Configuration files (mounted at runtime)
RUN mkdir -p /data /config && \
    chown -R mamut:mamut /data /config

# Copy the compiled binary from the build stage
COPY --from=builder /build/target/release/mamut-controller /usr/local/bin/mamut-controller

# Ensure binary is executable
RUN chmod +x /usr/local/bin/mamut-controller

# Switch to non-root user
USER mamut

# Set working directory
WORKDIR /home/mamut

# Environment variables for runtime configuration
# These can be overridden at container runtime
ENV MAMUT_LOG_LEVEL=info
ENV MAMUT_LOG_FORMAT=json
ENV MAMUT_GRPC_PORT=50051
ENV MAMUT_HTTP_PORT=8080
ENV MAMUT_DATA_DIR=/data
ENV MAMUT_CONFIG_DIR=/config

# Expose ports
# - 50051: gRPC port for agent communication
# - 8080: HTTP port for health checks, metrics, and REST API
EXPOSE 50051 8080

# Health check configuration
# Checks the /health endpoint every 30 seconds
# - interval: Time between health checks
# - timeout: Maximum time to wait for response
# - start-period: Grace period for container startup
# - retries: Number of consecutive failures before marking unhealthy
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Volume mount points for persistent data
VOLUME ["/data", "/config"]

# Use tini as the init process
# Tini ensures proper signal handling (SIGTERM, SIGINT) and zombie reaping
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command to run the controller
# Arguments can be overridden at runtime
CMD ["mamut-controller", "--config", "/config/controller.toml"]

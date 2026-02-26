# =============================================================================
# Mamut Agent Dockerfile
# =============================================================================
# Multi-stage build for the mamut-agent binary.
# The agent executes test operations on target systems and injects faults
# to simulate network partitions, latency, clock drift, and process failures.
#
# Build: docker build -f docker/agent.Dockerfile -t mamut-agent .
# Run:   docker run --cap-add=NET_ADMIN --cap-add=SYS_TIME mamut-agent
#
# REQUIRED CAPABILITIES:
# - NET_ADMIN: Required for network fault injection (tc, netem, iptables)
# - SYS_TIME: Required for clock manipulation (time jumps, drift simulation)
#
# SECURITY NOTE:
# These capabilities grant significant system access. The agent should only
# run in isolated test environments, never in production.
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
RUN cargo build --release --package mamut-agent 2>/dev/null || true

# Remove dummy source files
RUN find crates binaries -name "*.rs" -delete

# Copy actual source code
COPY crates/ crates/
COPY binaries/ binaries/

# Touch source files to invalidate cached build while keeping dependencies
RUN find crates binaries -name "*.rs" -exec touch {} +

# Build the agent binary in release mode
RUN cargo build --release --package mamut-agent

# -----------------------------------------------------------------------------
# Stage 2: Runtime Environment
# -----------------------------------------------------------------------------
# Using debian:bookworm-slim as the runtime base.
# This stage includes fault injection tools required by mamut-nemesis.
FROM debian:bookworm-slim AS runtime

# Labels for container metadata (OCI image spec)
LABEL org.opencontainers.image.title="Mamut Agent"
LABEL org.opencontainers.image.description="Distributed agent for executing test operations and fault injection"
LABEL org.opencontainers.image.vendor="Mamut Lab"
LABEL org.opencontainers.image.source="https://github.com/mamut-lab/mamut-verify"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

# Install runtime dependencies and fault injection tools
#
# Core runtime dependencies:
# - ca-certificates: Required for HTTPS/TLS connections
# - tini: Minimal init system for proper signal handling
# - libssl3: OpenSSL runtime library
#
# Fault injection tools (required by mamut-nemesis):
# - iproute2: Provides 'tc' (traffic control) and 'ip' commands
#   * tc qdisc: Create queuing disciplines for traffic shaping
#   * tc netem: Network emulation (latency, loss, corruption, reordering)
#   * Examples: tc qdisc add dev eth0 root netem delay 100ms 10ms
#
# - iptables: Packet filtering and NAT framework
#   * Network partition simulation via DROP rules
#   * Selective packet filtering by port, protocol, address
#   * Examples: iptables -A INPUT -s 10.0.0.5 -j DROP
#
# - procps: Process management utilities
#   * ps: List running processes
#   * kill: Send signals to processes
#   * Used for process kill fault injection
#
# - iputils-ping: Network diagnostics
#   * Useful for verifying network state during tests
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    libssl3 \
    iproute2 \
    iptables \
    procps \
    iputils-ping \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for the application
# Note: The agent still needs elevated capabilities, but we avoid running
# as root to minimize attack surface. Capabilities are granted via Docker.
RUN groupadd --gid 1000 mamut && \
    useradd --uid 1000 --gid mamut --shell /bin/false --create-home mamut

# Create directories for runtime data
# - /data: Agent's local data storage
# - /config: Configuration files
RUN mkdir -p /data /config && \
    chown -R mamut:mamut /data /config

# Copy the compiled binary from the build stage
COPY --from=builder /build/target/release/mamut-agent /usr/local/bin/mamut-agent

# Ensure binary is executable
RUN chmod +x /usr/local/bin/mamut-agent

# Grant specific file capabilities for fault injection tools
# This allows the agent to use tc/iptables without full root privileges
# Note: These capabilities are in addition to runtime --cap-add flags
RUN setcap cap_net_admin+ep /usr/sbin/tc 2>/dev/null || true && \
    setcap cap_net_admin+ep /usr/sbin/iptables 2>/dev/null || true

# Switch to non-root user
# Note: Fault injection requires capabilities to be granted at runtime
USER mamut

# Set working directory
WORKDIR /home/mamut

# Environment variables for runtime configuration
ENV MAMUT_LOG_LEVEL=info
ENV MAMUT_LOG_FORMAT=json
ENV MAMUT_CONTROLLER_URL=http://controller:50051
ENV MAMUT_AGENT_ID=
ENV MAMUT_DATA_DIR=/data
ENV MAMUT_CONFIG_DIR=/config

# The agent exposes a gRPC port for controller communication
# and an HTTP port for metrics/health
EXPOSE 50052 8081

# Health check configuration
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8081/health || exit 1

# Volume mount points
VOLUME ["/data", "/config"]

# Use tini as init
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command
CMD ["mamut-agent", "--config", "/config/agent.toml"]

# =============================================================================
# REQUIRED LINUX CAPABILITIES DOCUMENTATION
# =============================================================================
#
# This container requires elevated capabilities to perform fault injection.
# Run with: docker run --cap-add=NET_ADMIN --cap-add=SYS_TIME mamut-agent
#
# NET_ADMIN (required for network fault injection):
# - Modify routing tables
# - Configure traffic control (tc/netem)
# - Manage iptables rules
# - Change network interface settings
# - Required for: partition, latency, packet loss, bandwidth throttling
#
# SYS_TIME (required for clock fault injection):
# - Set the system clock
# - Adjust clock frequency (NTP-like drift)
# - Required for: clock jumps, clock drift simulation
#
# OPTIONAL CAPABILITIES:
#
# SYS_PTRACE (optional, for advanced process fault injection):
# - Trace arbitrary processes
# - Required for: syscall injection, debugging
#
# SYS_ADMIN (optional, for container-level faults):
# - Various system administration operations
# - Required for: cgroup manipulation, namespace operations
# - WARNING: Grants extensive privileges, use with caution
#
# SECURITY CONSIDERATIONS:
# - Only grant the minimum capabilities required
# - Run in isolated networks, never expose to untrusted systems
# - Consider using user namespaces for additional isolation
# - Audit all fault injection operations
#
# =============================================================================

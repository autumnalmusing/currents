# Multi-stage build for Currents weather monitoring system
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY currents-core/ ./currents-core/
COPY currents-daemon/ ./currents-daemon/
COPY currents-forecast/ ./currents-forecast/
COPY currents-history/ ./currents-history/
COPY currents-storage/ ./currents-storage/
COPY currents-orchestrator/ ./currents-orchestrator/

# Build all packages
RUN cargo build --release --workspace

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false currents

# Copy binaries from builder stage
COPY --from=builder /app/target/release/currents /usr/local/bin/
COPY --from=builder /app/target/release/currents-forecast /usr/local/bin/
COPY --from=builder /app/target/release/currents-history /usr/local/bin/
COPY --from=builder /app/target/release/currents-orchestrator /usr/local/bin/
COPY --from=builder /app/target/release/simple-collector /usr/local/bin/

# Create directories
RUN mkdir -p /var/lib/currents /etc/currents /var/log/currents
RUN chown -R currents:currents /var/lib/currents /var/log/currents

# Copy default configuration
COPY config.toml /etc/currents/config.toml
COPY config.orchestrator.toml.example /etc/currents/orchestrator.toml

# Set permissions
RUN chmod 644 /etc/currents/*.toml
RUN chmod 755 /usr/local/bin/currents*

# Switch to non-root user
USER currents

# Set working directory
WORKDIR /var/lib/currents

# Expose port for health checks
EXPOSE 8080

# Default command (can be overridden)
CMD ["currents", "--foreground"]

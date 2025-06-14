# Multi-stage build for production
FROM rust:1.75-bookworm as builder

WORKDIR /app

# Install protoc
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY . .

# Setup proto dependencies
RUN make proto-deps

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/syla-execution-service /app/

# Create non-root user
RUN useradd -m -u 1000 syla && chown -R syla:syla /app
USER syla

EXPOSE 8083 8081

CMD ["./syla-execution-service"]
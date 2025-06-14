.PHONY: all build clean test proto proto-deps run dev

# Build targets
all: proto build

build:
	cargo build --release

clean:
	cargo clean
	rm -rf proto/google proto/common

test:
	cargo test

# Proto management
proto-deps:
	@echo "Setting up proto dependencies..."
	@mkdir -p proto
	@cd proto && ln -sf ../../../../../proto-deps/googleapis/google google 2>/dev/null || true
	@cd proto && ln -sf ../../../../../proto-deps/common common 2>/dev/null || true

proto: proto-deps
	@echo "Proto dependencies ready"

# Development
run: proto
	cargo run

dev: proto
	cargo watch -x run

# Docker
docker-build:
	docker build -t syla-execution-service .

docker-run:
	docker run -p 8083:8083 -p 8081:8081 \
		-e REDIS_URL=redis://host.docker.internal:6380 \
		--name syla-execution-service \
		syla-execution-service

# gRPC testing
grpc-health:
	grpcurl -plaintext localhost:8081 syla.execution.v1.ExecutionService/HealthCheck

grpc-list:
	grpcurl -plaintext localhost:8081 list

# Integration
integration-test: proto
	@echo "Running integration tests..."
	cargo test --test '*' -- --test-threads=1
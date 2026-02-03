#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_NAME="kabu-integration-test"

cd "$PROJECT_ROOT"

# Get host UID/GID for rootless Docker compatibility
USER_UID=$(id -u)
USER_GID=$(id -g)

echo "Building test container..."
docker build \
    -t "$IMAGE_NAME" \
    -f docker/Dockerfile.test \
    .

echo "Running tests in Docker container..."

# Construct test arguments
TEST_ARGS=("cargo" "test" "--features" "impure-test")

if [ -n "$1" ]; then
    # If specific test name provided, add it
    TEST_ARGS+=("$1")
fi

# Add -- --nocapture for verbose output if requested
if [ "$VERBOSE" = "1" ]; then
    TEST_ARGS+=("--" "--nocapture")
fi

docker run --rm \
    --user "${USER_UID}:${USER_GID}" \
    -v "$PROJECT_ROOT:/workspace:rw" \
    -e CI="${CI:-false}" \
    -e RUST_BACKTRACE="${RUST_BACKTRACE:-1}" \
    -e CARGO_TARGET_DIR=/tmp/target \
    "$IMAGE_NAME" \
    "${TEST_ARGS[@]}"

echo "Tests completed successfully."

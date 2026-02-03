#!/bin/bash
set -e

# Ensure trust directory exists
mkdir -p "${KABU_TRUST_DIR:-/tmp/kabu-trusted}"

# Execute the command
exec "$@"

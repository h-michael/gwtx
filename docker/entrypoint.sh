#!/bin/bash
set -e

# Ensure trust directory exists
mkdir -p "${GWTX_TRUST_DIR:-/tmp/gwtx-trusted}"

# Execute the command
exec "$@"

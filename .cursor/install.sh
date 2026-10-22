#!/usr/bin/env bash
# Cloud Agent environment bootstrap for shallgraph.
# Idempotent: safe to re-run. Mirrors the toolchain used by .github/workflows/test.yml.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

# Rust 1.83.0 comes from the base image (rust-toolchain.toml pins the channel).
# Add the WASM target and the pinned wasm-bindgen-cli used by scripts/build-wasm-core.sh.
rustup target add wasm32-unknown-unknown
bash ./scripts/install-wasm-bindgen.sh

# package.json requires Node >= 24; CI uses Node 25. Install the pinned Node to $HOME/.local.
INSTALL_PREFIX="${HOME}/.local" bash ./scripts/install-node.sh
export PATH="${HOME}/.local/node/bin:${PATH}"

# Install JS dependencies and build the WASM core + workspaces (gitignored artifacts).
npm ci
npm run build

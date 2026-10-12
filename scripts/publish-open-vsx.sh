#!/usr/bin/env bash
# GRD-VSC-009: Publish the packaged shallgraph-vscode VSIX to Open VSX so Cursor can install it.
# Uses the same versioned VSIX that the release workflow uploads to GitHub (GRD-DEVOPS-002 / GRD-DEVOPS-003).
# Requires OVSX_PAT (Open VSX access token). The shallgraph namespace must already exist.
# A VSIX packaged with `vsce package --pre-release` keeps that pre-release bit; Open VSX
# reads it from the package. `--pre-release` on `ovsx publish` is ignored for a prepackaged VSIX.
# Usage: ./scripts/publish-open-vsx.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -z "${OVSX_PAT:-}" ]]; then
  echo "ERROR: OVSX_PAT is not set. Create an Open VSX access token at https://open-vsx.org/user-settings/tokens after signing the Publisher Agreement and export it before publishing."
  exit 1
fi

VSIX_DIR="${SHALLGRAPH_VSIX_DIR:-${REPO_ROOT}/packages/vscode}"
shopt -s nullglob
vsix_files=("${VSIX_DIR}"/shallgraph-vscode-*.vsix)
if [[ "${#vsix_files[@]}" -ne 1 ]]; then
  echo "ERROR: expected exactly one VSIX matching ${VSIX_DIR}/shallgraph-vscode-*.vsix, found ${#vsix_files[@]}."
  echo "Package first with: npm run package -w shallgraph-vscode"
  exit 1
fi

VSIX="${vsix_files[0]}"
echo "Publishing ${VSIX} to Open VSX ..."
cd "${REPO_ROOT}/packages/vscode"
npx ovsx publish "${VSIX}" --pat "${OVSX_PAT}" --skip-duplicate
echo "Done. Extension: https://open-vsx.org/extension/shallgraph/shallgraph-vscode"

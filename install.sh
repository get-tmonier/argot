#!/usr/bin/env bash
set -euo pipefail

REPO="get-tmonier/argot"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="argot"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  linux-x86_64)  TARGET="linux-x64" ;;
  darwin-x86_64) TARGET="darwin-x64" ;;
  darwin-arm64)  TARGET="darwin-arm64" ;;
  *)
    echo "Unsupported platform: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

# Fetch latest release tag
echo "Fetching latest argot release…"
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$TAG" ]; then
  echo "Failed to fetch latest release tag" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${TAG}/argot-${TARGET}"

# Download binary
TMP=$(mktemp)
echo "Downloading argot ${TAG} for ${TARGET}…"
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

# Install
mkdir -p "$INSTALL_DIR"
mv "$TMP" "${INSTALL_DIR}/${BINARY_NAME}"
echo "Installed argot ${TAG} to ${INSTALL_DIR}/${BINARY_NAME}"

# Check uv
if ! command -v uv &>/dev/null; then
  echo ""
  echo "uv not found — installing (required for the Python engine)…"
  curl -LsSf https://astral.sh/uv/install.sh | sh
  echo "uv installed. You may need to restart your shell."
fi

# PATH reminder
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
  echo ""
  echo "Add ${INSTALL_DIR} to your PATH:"
  echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

echo ""
echo "Getting started:"
echo "  cd your-repo"
echo "  argot extract    # parse git history"
echo "  argot train      # train style model (~2GB download first time)"
echo "  argot check      # detect style anomalies"
echo "  argot explain    # AI analysis (requires 'claude' CLI in PATH)"

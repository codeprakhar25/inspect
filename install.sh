#!/usr/bin/env sh
# inspect installer
# Usage: curl -fsSL https://raw.githubusercontent.com/codeprakhar25/inspect/main/install.sh | sh

set -e

REPO="codeprakhar25/inspect"
BIN="inspect"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and arch
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  os="linux" ;;
  Darwin) os="macos" ;;
  *)
    echo "error: unsupported OS: $OS"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)          arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *)
    echo "error: unsupported architecture: $ARCH"
    exit 1
    ;;
esac

ASSET="${BIN}-${os}-${arch}"

# Get latest release tag
TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"

if [ -z "$TAG" ]; then
  echo "error: could not fetch latest release tag"
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

echo "Installing inspect ${TAG} for ${os}/${arch}..."
echo "  -> $URL"

mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/$BIN"
chmod +x "$INSTALL_DIR/$BIN"

echo ""
echo "Installed to $INSTALL_DIR/$BIN"

# Warn if not on PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "  Add to PATH by adding this to ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo ""
echo "  Run: inspect cp"
echo "  Shell widget: inspect --install-shell bash   # or zsh"

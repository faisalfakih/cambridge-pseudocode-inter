#!/bin/bash
set -e
REPO="faisalfakih/cambridge-psudocode-inter"
BINARY="cps"
INSTALL_DIR="/usr/local/bin"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

echo "🔍 Detecting platform..."

case "$OS" in
  darwin)
    case "$ARCH" in
      arm64)   PLATFORM="macos-arm64" ;;
      x86_64)  PLATFORM="macos-x86_64" ;;
      *)       echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  linux)
    case "$ARCH" in
      x86_64)  PLATFORM="linux-x86_64" ;;
      aarch64) PLATFORM="linux-aarch64" ;;
      *)       echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "❌ Unsupported OS: $OS"
    echo "   Please build from source: https://github.com/$REPO"
    exit 1
    ;;
esac

echo "📦 Platform detected: $PLATFORM"

# Get latest release version
echo "🔍 Fetching latest release..."
VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name":' \
  | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
  echo "❌ Could not fetch latest release"
  exit 1
fi

echo "📥 Downloading $BINARY $VERSION..."

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/${BINARY}-${PLATFORM}"

HTTP_STATUS=$(curl -L \
  --write-out "%{http_code}" \
  --silent \
  --output "/tmp/$BINARY" \
  "$DOWNLOAD_URL")

if [ "$HTTP_STATUS" -ne 200 ]; then
  echo "❌ No binary available for $PLATFORM yet."
  echo "   Please build from source:"
  echo "   git clone https://github.com/$REPO"
  echo "   cd cambridge-psudocode-inter"
  echo "   cargo build --release"
  exit 1
fi

chmod +x "/tmp/$BINARY"

# Install
echo "🔧 Installing to $INSTALL_DIR/$BINARY..."
if [ -w "$INSTALL_DIR" ]; then
  mv "/tmp/$BINARY" "$INSTALL_DIR/$BINARY"
else
  sudo mv "/tmp/$BINARY" "$INSTALL_DIR/$BINARY"
fi


echo ""
echo "✅ Installed $BINARY $VERSION successfully!"
echo "   Run: cps yourfile.cps"

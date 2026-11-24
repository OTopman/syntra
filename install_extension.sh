#!/usr/bin/env bash
set -e

# Resolve repository root
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_SRC="$DIR/editors/vscode/syntra-lang"
EXT_DEST="$HOME/.vscode/extensions/syntra-lang"

echo "=================================================="
echo "📦 Installing Syntra VS Code TypeScript Extension..."
echo "=================================================="

# 1. Ensure target/debug/syn binary is built
if [ ! -f "$DIR/target/debug/syn" ]; then
    echo "🔨 Building 'syn' binary..."
    cargo build --bin syn
else
    echo "✅ Found 'syn' binary at $DIR/target/debug/syn"
fi

# 2. Compile TypeScript extension source
echo "⚡ Compiling TypeScript IntelliSense extension..."
cd "$EXT_SRC"
./node_modules/.bin/tsc -p ./
cd "$DIR"

# 3. Copy extension to VS Code extensions directory
mkdir -p "$EXT_DEST"
cp -R "$EXT_SRC"/* "$EXT_DEST/"

echo "✅ Installed extension to: $EXT_DEST"
echo ""
echo "🎉 SUCCESS! The Syntra TypeScript IntelliSense extension is now compiled and installed."
echo ""
echo "Next step in VS Code / Editor:"
echo "1. Press 'Cmd+Shift+P' (macOS) or 'Ctrl+Shift+P' (Windows/Linux)"
echo "2. Type: 'Developer: Reload Window' and press Enter"
echo "3. Open any '.syn' file to experience full TypeScript IntelliSense, syntax autocomplete, diagnostics, hover, signature help, and formatting!"
echo "=================================================="

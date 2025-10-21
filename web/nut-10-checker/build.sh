#!/bin/bash
set -e

echo "🔨 Building NUT-10 Checker WASM..."

# Build WASM
wasm-pack build --target web --dev

echo "✅ Build complete!"
echo ""
echo "To run locally:"
echo "  python3 -m http.server 8000"
echo ""
echo "Then open: http://localhost:8000"

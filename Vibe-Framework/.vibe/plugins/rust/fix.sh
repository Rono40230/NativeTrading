#!/bin/bash
# Plugin Rust - Fix
if [ -f "Cargo.toml" ] || [ -f "src-tauri/Cargo.toml" ]; then
    # Détecter où est le Cargo.toml (racine ou src-tauri)
    TARGET_DIR="."
    if [ -f "src-tauri/Cargo.toml" ]; then TARGET_DIR="src-tauri"; fi
    
    (cd "$TARGET_DIR" && cargo fmt --quiet && cargo clippy --fix --allow-dirty --allow-staged --quiet -- -D warnings 2>/dev/null)
fi

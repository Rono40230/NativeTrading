#!/bin/bash
# Plugin Rust - Test
TARGET_DIR="."
if [ -f "src-tauri/Cargo.toml" ]; then TARGET_DIR="src-tauri"; fi

if (cd "$TARGET_DIR" && cargo test --quiet); then
    exit 0
else
    exit 1
fi

#!/bin/bash
set -e
echo "📦 Installation Native Trading AI"
echo "=================================="

echo "🔧 Installation Rust..."
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "🎮 Installation CUDA 11.8..."
if ! command -v nvcc &> /dev/null; then
    sudo dnf install -y cuda-11-8
fi

echo "🔥 Téléchargement LibTorch..."
if [ ! -d "$HOME/.local/libtorch" ]; then
    cd /tmp
    wget -q https://download.pytorch.org/libtorch/cu118/libtorch-cxx11-abi-shared-with-deps-2.1.0%2Bcu118.zip
    unzip -q libtorch-*.zip
    mkdir -p "$HOME/.local"
    mv libtorch "$HOME/.local/"
    echo "export LIBTORCH=$HOME/.local/libtorch" >> ~/.bashrc
    echo "export LD_LIBRARY_PATH=\$LIBTORCH/lib:\$LD_LIBRARY_PATH" >> ~/.bashrc
fi

echo "📦 Installation Node.js 20 (requis pour build Tauri uniquement)..."
sudo dnf module install -y nodejs:20

echo "🖥️ Installation dépendances système Tauri (GTK, WebKit)..."
sudo dnf install -y \
    webkit2gtk4.0-devel \
    openssl-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    patchelf \
    gtk3-devel

echo "🗄️ Installation SQLite..."
sudo dnf install -y sqlite sqlite-devel

echo "🏗️ Build backend Rust..."
cd "$(dirname "$0")/../backend"
cargo build --release

echo "🎨 Installation frontend + Tauri..."
cd "$(dirname "$0")/../frontend"
npm install

echo "✅ Installation terminée!"
echo "▶️  Lancez l'application : ./scripts/run.sh"
echo "   → Fenêtre native Tauri (aucun navigateur requis)"

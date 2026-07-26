#!/usr/bin/env bash
# Установка зависимостей Debian и сборка .deb пакета Teams Linux
set -euo pipefail

echo "==> Обновление списка пакетов"
sudo apt update

echo "==> Установка системных зависимостей сборки Tauri v2"
sudo apt install -y \
  build-essential curl wget file git \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev \
  libssl-dev \
  libnotify-dev \
  patchelf \
  pkg-config

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Rust не найден, устанавливаю через rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

if ! command -v cargo-tauri >/dev/null 2>&1; then
  echo "==> Устанавливаю Tauri CLI"
  cargo install tauri-cli --version "^2"
fi

echo "==> Сборка релиза + .deb пакета"
cargo tauri build --bundles deb

DEB_PATH=$(find target/release/bundle/deb -name "*.deb" | head -n1)
echo ""
echo "Готово! Пакет собран: ${DEB_PATH}"
echo "Установка:  sudo apt install \"${DEB_PATH}\""

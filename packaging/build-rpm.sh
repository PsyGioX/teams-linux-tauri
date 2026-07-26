#!/usr/bin/env bash
# Установка зависимостей и сборка .rpm пакета Teams Linux
# Поддерживает dnf (Fedora/RHEL/CentOS) и zypper (openSUSE) как пакетные менеджеры.
set -euo pipefail

echo "==> Определение пакетного менеджера"
if command -v dnf >/dev/null 2>&1; then
  PKG_MANAGER="dnf"
elif command -v zypper >/dev/null 2>&1; then
  PKG_MANAGER="zypper"
else
  echo "Не найден ни dnf, ни zypper. Установите зависимости вручную (см. README.md)." >&2
  exit 1
fi
echo "    Используется: ${PKG_MANAGER}"

echo "==> Установка системных зависимостей сборки Tauri v2"
if [ "$PKG_MANAGER" = "dnf" ]; then
  sudo dnf install -y \
    gcc gcc-c++ make curl wget file git \
    webkit2gtk4.1-devel \
    gtk3-devel \
    librsvg2-devel \
    libappindicator-gtk3-devel \
    openssl-devel \
    libnotify-devel \
    patchelf \
    pkgconf-pkg-config \
    rpm-build
else
  sudo zypper install -y \
    gcc gcc-c++ make curl wget file git \
    webkit2gtk3-soup2-devel \
    gtk3-devel \
    librsvg2-devel \
    libappindicator3-devel \
    libopenssl-devel \
    libnotify-devel \
    patchelf \
    pkgconf-pkg-config \
    rpm-build
fi

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

echo "==> Сборка релиза + .rpm пакета"
cargo tauri build --bundles rpm

RPM_PATH=$(find target/release/bundle/rpm -name "*.rpm" | head -n1)
echo ""
echo "Готово! Пакет собран: ${RPM_PATH}"
if [ "$PKG_MANAGER" = "dnf" ]; then
  echo "Установка:  sudo dnf install \"${RPM_PATH}\""
else
  echo "Установка:  sudo zypper install \"${RPM_PATH}\""
fi

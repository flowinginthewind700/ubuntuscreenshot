#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
ARCH="amd64"
PACKAGE_NAME="ubuntuscreenshot_${VERSION}_${ARCH}.deb"

cd "$ROOT"

echo "==> 编译 release 二进制（与 cargo run --release 相同产物目录）"
bash "$ROOT/scripts/write-cargo-env.sh" 2>/dev/null || true

if ! pkg-config --exists libpipewire-0.3 2>/dev/null; then
  if [ -f "$ROOT/.build-deps/env.sh" ]; then
    # shellcheck source=/dev/null
    source "$ROOT/.build-deps/env.sh"
  else
    echo "缺少 libpipewire-0.3-dev，正在准备本地编译依赖…" >&2
    bash "$ROOT/scripts/setup-build-deps.sh"
    # shellcheck source=/dev/null
    source "$ROOT/.build-deps/env.sh"
  fi
fi
export CARGO_TARGET_DIR="$ROOT/target"
rm -f "$ROOT/target/release/screenshot4ubuntu"
cargo build --release

BIN="$ROOT/target/release/screenshot4ubuntu"
echo "==> 编译产物: $BIN"
if [ ! -x "$BIN" ]; then
  echo "错误: 未找到 $BIN" >&2
  exit 1
fi
if [ "$(strings "$BIN" | grep -Fc "portal Screenshot requested (silent)")" -eq 0 ]; then
  echo "错误: 二进制缺少静默 portal 截屏标记" >&2
  exit 1
fi
if [ "$(strings "$BIN" | grep -Fc "fallback pipewire")" -eq 0 ]; then
  echo "错误: 二进制缺少 PipeWire 回退标记" >&2
  exit 1
fi
if [ "$(strings "$BIN" | grep -Fc "gnome-screenshot")" -gt 0 ]; then
  echo "错误: 二进制仍含 gnome-screenshot 回退" >&2
  exit 1
fi
if [ "$(strings "$BIN" | grep -Fc "layout detected")" -gt 0 ]; then
  echo "错误: 二进制仍含旧版截屏代码 (layout detected)" >&2
  exit 1
fi
echo "==> 二进制校验通过 ($(md5sum "$BIN" | awk '{print $1}'))"

echo "==> 准备打包目录"
rm -rf packaging/usr/share/doc/screenshot4ubuntu
mkdir -p packaging/usr/share/applications
mkdir -p packaging/usr/share/doc/ubuntuscreenshot
install -Dm755 "$BIN" packaging/usr/bin/ubuntuscreenshot
install -Dm644 assets/ubuntuscreenshot.desktop packaging/usr/share/applications/ubuntuscreenshot.desktop
install -Dm644 README.md packaging/usr/share/doc/ubuntuscreenshot/README.md
install -Dm644 LICENSE packaging/usr/share/doc/ubuntuscreenshot/LICENSE

if [ -f assets/icons/screenshot4ubuntu.svg ]; then
  install -Dm644 assets/icons/screenshot4ubuntu.svg \
    packaging/usr/share/icons/hicolor/scalable/apps/screenshot4ubuntu.svg
fi

echo "==> 更新 control 版本与大小"
sed -i "s/^Version:.*/Version: ${VERSION}/" packaging/DEBIAN/control
# 去掉旧的 Installed-Size 行后重写
sed -i '/^Installed-Size:/d' packaging/DEBIAN/control
INSTALLED_SIZE=$(du -sk packaging/usr packaging/DEBIAN | awk '{s+=$1} END {print s}')
echo "Installed-Size: ${INSTALLED_SIZE}" >> packaging/DEBIAN/control

echo "==> 构建 Debian 包"
mkdir -p dist
dpkg-deb --build --root-owner-group packaging "dist/${PACKAGE_NAME}"

echo "==> 验证"
dpkg-deb -I "dist/${PACKAGE_NAME}"
echo ""
dpkg-deb -c "dist/${PACKAGE_NAME}" | head -25
echo ""
echo "✓ dist/${PACKAGE_NAME}"
echo ""
echo "安装:"
echo "  sudo dpkg -i dist/${PACKAGE_NAME}"
echo "  sudo apt-get install -f"

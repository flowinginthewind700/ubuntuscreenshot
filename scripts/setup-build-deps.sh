#!/bin/bash
# 无 sudo 下载并解压 PipeWire 编译依赖到 .build-deps/，供 cargo build 使用。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPS="$ROOT/.build-deps"
PKG_DIR="$DEPS/root/usr/lib/x86_64-linux-gnu/pkgconfig"
INCLUDE_DIR="$DEPS/root/usr/include"
LIB_DIR="$DEPS/root/usr/lib/x86_64-linux-gnu"

mkdir -p "$DEPS/dl"
cd "$DEPS/dl"

echo "==> 下载 libpipewire / libspa 开发包（无需 sudo）"
apt-get download libpipewire-0.3-dev libspa-0.2-dev

echo "==> 解压到 $DEPS/root"
rm -rf "$DEPS/root"
mkdir -p "$DEPS/root"
dpkg-deb -x libpipewire-0.3-dev_*.deb "$DEPS/root"
dpkg-deb -x libspa-0.2-dev_*.deb "$DEPS/root"

echo "==> 修正 pkg-config 路径"
for pc in "$PKG_DIR"/libpipewire-0.3.pc "$PKG_DIR"/libspa-0.2.pc; do
  sed -i "s|^prefix=.*|prefix=$DEPS/root/usr|" "$pc"
  sed -i "s|^libdir=.*|libdir=$LIB_DIR|" "$pc"
done

# 链接用系统已安装的运行时 .so
PW_SO="$(ls /usr/lib/x86_64-linux-gnu/libpipewire-0.3.so.* 2>/dev/null | head -1 || true)"
if [ -n "$PW_SO" ]; then
  ln -sf "$PW_SO" "$LIB_DIR/libpipewire-0.3.so.0"
  ln -sf libpipewire-0.3.so.0 "$LIB_DIR/libpipewire-0.3.so"
else
  echo "警告: 未找到系统 libpipewire-0.3 运行时库，请安装: sudo apt install pipewire" >&2
fi

# bindgen 需要 libclang（若系统无 clang）
if ! command -v clang >/dev/null 2>&1 && [ ! -f "$DEPS/clang/usr/lib/x86_64-linux-gnu/libclang-21.so.21" ]; then
  echo "==> 下载 libclang（bindgen 用）"
  apt-get download libclang-21-dev libclang1-21 clang-21 2>/dev/null || true
  if ls libclang-21-dev_*.deb >/dev/null 2>&1; then
    rm -rf "$DEPS/clang"
    mkdir -p "$DEPS/clang"
    dpkg-deb -x libclang-21-dev_*.deb "$DEPS/clang"
    dpkg-deb -x libclang1-21_*.deb "$DEPS/clang" 2>/dev/null || true
  fi
fi

cat >"$DEPS/env.sh" <<EOF
# 由 scripts/setup-build-deps.sh 生成。用法: source .build-deps/env.sh
export PKG_CONFIG_PATH="$PKG_DIR:\${PKG_CONFIG_PATH:-}"
export BINDGEN_EXTRA_CLANG_ARGS="-isystem /usr/include -isystem /usr/include/x86_64-linux-gnu -isystem /usr/lib/gcc/x86_64-linux-gnu/\$(gcc -dumpversion 2>/dev/null || echo 13)/include"
EOF

if [ -d "$DEPS/clang/usr/lib/x86_64-linux-gnu" ]; then
  echo "export LIBCLANG_PATH=\"$DEPS/clang/usr/lib/x86_64-linux-gnu\"" >>"$DEPS/env.sh"
fi

bash "$ROOT/scripts/write-cargo-env.sh"

echo ""
echo "✓ 编译依赖已就绪"
echo ""
echo "接下来任选其一："
echo "  source .build-deps/env.sh && cargo run --release"
echo "  ./scripts/run-dev.sh"
echo ""
echo "或系统级安装（推荐长期使用）："
echo "  sudo apt install libpipewire-0.3-dev libspa-0.2-dev"

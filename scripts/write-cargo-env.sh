#!/bin/bash
# 生成 .cargo/config.toml，让 bindgen 能找到 stdbool.h 等 C 标准头。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_DIR="$ROOT/.cargo"
CONFIG="$CARGO_DIR/config.toml"
GCC_VER="$(gcc -dumpversion 2>/dev/null || echo 13)"

mkdir -p "$CARGO_DIR"

BINDGEN_ARGS="-isystem /usr/include -isystem /usr/include/x86_64-linux-gnu -isystem /usr/lib/gcc/x86_64-linux-gnu/${GCC_VER}/include"

LIBCLANG_LINE=""
if [ -d "$ROOT/.build-deps/clang/usr/lib/x86_64-linux-gnu" ]; then
  LIBCLANG_LINE="LIBCLANG_PATH = \"${ROOT}/.build-deps/clang/usr/lib/x86_64-linux-gnu\""
fi

if [ ! -f /usr/lib/gcc/x86_64-linux-gnu/"${GCC_VER}"/include/stdbool.h ]; then
  echo "错误: 未找到 gcc C 头文件（stdbool.h）。" >&2
  echo "请安装: sudo apt install build-essential" >&2
  echo "或运行: bash scripts/setup-build-deps.sh" >&2
  exit 1
fi

cat >"$CONFIG" <<EOF
# 由 scripts/write-cargo-env.sh 生成，勿手改。重新生成: bash scripts/write-cargo-env.sh
[env]
BINDGEN_EXTRA_CLANG_ARGS = "${BINDGEN_ARGS}"
EOF

if [ -n "$LIBCLANG_LINE" ]; then
  echo "$LIBCLANG_LINE" >>"$CONFIG"
fi

echo "✓ 已写入 $CONFIG (gcc ${GCC_VER})"

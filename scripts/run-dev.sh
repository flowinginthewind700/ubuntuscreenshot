#!/bin/bash
# 开发运行：自动加载本地编译依赖后执行 cargo。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# bindgen 需要 gcc 的 C 头路径；与是否用系统 pipewire 无关
if [ ! -f "$ROOT/.cargo/config.toml" ]; then
  bash "$ROOT/scripts/write-cargo-env.sh"
fi

if ! pkg-config --exists libpipewire-0.3 2>/dev/null; then
  if [ ! -f "$ROOT/.build-deps/env.sh" ]; then
    echo "缺少 libpipewire-0.3-dev，正在准备本地编译依赖…" >&2
    bash "$ROOT/scripts/setup-build-deps.sh"
  fi
  # shellcheck source=/dev/null
  source "$ROOT/.build-deps/env.sh"
elif [ -f "$ROOT/.build-deps/env.sh" ] && [ -d "$ROOT/.build-deps/clang/usr/lib/x86_64-linux-gnu" ]; then
  # 系统有 pipewire-dev 但无 clang 时，仍用本地 libclang
  # shellcheck source=/dev/null
  source "$ROOT/.build-deps/env.sh"
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
exec cargo run --release "$@"

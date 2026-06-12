#!/usr/bin/env bash
# 发布源码与 Release 资产到 GitHub。
# 前置：gh auth login 且对 flowinginthewind700/ubuntuscreenshot 有写权限。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-0.1.0}"
TAG="v${VERSION}"
REPO="flowinginthewind700/ubuntuscreenshot"
GH="${GH_BIN:-gh}"

cd "$ROOT"

if ! command -v "$GH" >/dev/null 2>&1; then
  echo "未找到 gh，请先安装并登录："
  echo "  gh auth login"
  exit 1
fi

"$GH" auth status >/dev/null
"$GH" auth setup-git
git remote set-url origin "https://github.com/${REPO}.git"

echo "==> 构建 release 二进制"
cargo build --release

echo "==> 打包"
mkdir -p dist
cp target/release/screenshot4ubuntu dist/
cp README.md LICENSE dist/
tar -czf "dist/screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz" -C dist \
  screenshot4ubuntu README.md LICENSE

echo "==> 推送代码与标签"
git push -u origin main
git tag -a "$TAG" -m "Ubuntu Screenshot ${TAG}" 2>/dev/null || git tag -f -a "$TAG" -m "Ubuntu Screenshot ${TAG}"
git push origin "$TAG" --force

echo "==> 创建 GitHub Release"
"$GH" release create "$TAG" \
  --repo "$REPO" \
  --title "Ubuntu Screenshot ${TAG}" \
  --notes "$(cat <<EOF
## Ubuntu Screenshot ${TAG}

微信式 GNOME/Wayland 截屏工具：托盘触发、双屏框选、放大镜、标注、复制/保存。

### 下载

- \`screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz\` — Linux x86_64 预编译包

### 安装

\`\`\`bash
sudo apt install -y libxcb1 libxkbcommon0 libxkbcommon-x11-0 \\
  xdg-desktop-portal xdg-desktop-portal-gnome x11-xserver-utils \\
  zenity libayatana-appindicator3-1
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
\`\`\`

详见 [README](https://github.com/${REPO}#readme)。
EOF
)" \
  "dist/screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz"

echo "==> 完成: https://github.com/${REPO}/releases/tag/${TAG}"

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

echo "==> 构建 release 二进制与 deb"
bash "$ROOT/scripts/write-cargo-env.sh" 2>/dev/null || true
bash "$ROOT/scripts/build-deb.sh"

cp target/release/screenshot4ubuntu dist/
cp README.md LICENSE dist/
tar -czf "dist/screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz" -C dist \
  screenshot4ubuntu README.md LICENSE

RELEASE_NOTES="$(mktemp)"
sed "s/{{VERSION}}/${TAG}/g" scripts/release-notes.md > "$RELEASE_NOTES"

echo "==> 推送代码与标签"
git push -u origin main
git tag -a "$TAG" -m "Ubuntu Screenshot ${TAG}" 2>/dev/null || git tag -f -a "$TAG" -m "Ubuntu Screenshot ${TAG}"
git push origin "$TAG" --force

echo "==> 创建 GitHub Release"
"$GH" release create "$TAG" \
  --repo "$REPO" \
  --title "Ubuntu Screenshot ${TAG}" \
  --notes-file "$RELEASE_NOTES" \
  "dist/screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz" \
  "dist/ubuntuscreenshot_${VERSION}_amd64.deb"

rm -f "$RELEASE_NOTES"

echo "==> 完成: https://github.com/${REPO}/releases/tag/${TAG}"

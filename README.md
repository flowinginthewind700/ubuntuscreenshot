# Ubuntu Screenshot

基于 [GPUI](https://www.gpui.rs/) 的 Ubuntu / GNOME 截屏工具。交互参考微信截屏：托盘一键触发、双屏框选、放大镜精确定位、选区内直接标注，复制或保存后退出。

支持 **简体中文 / English**，可在托盘菜单切换语言。

仓库地址：[github.com/flowinginthewind700/ubuntuscreenshot](https://github.com/flowinginthewind700/ubuntuscreenshot)

## 功能

- **系统托盘**：顶栏相机图标，菜单含「截屏」「语言」「退出」
- **多显示器**：自动识别虚拟桌面（如双屏 6000×1440），每块屏同步遮罩与框选
- **微信式框选**：全屏半透明遮罩、拖拽实时选区、圆形放大镜（4× + 十字准星）
- **选区锁定编辑**：确认选区后不可重选，在选区内直接标注
- **标注工具**：画笔、直线、矩形、椭圆、文字；可调粗细、字号、颜色
- **复制 / 保存**：编辑完成后直接复制到剪贴板或弹出保存对话框（无需预览窗）
- **国际化**：简体中文 / English

## 系统要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Ubuntu 22.04+ / Debian 12+（其他带 GNOME 的发行版亦可尝试） |
| 桌面会话 | **GNOME Wayland**（推荐）或 X11 |
| 架构 | `x86_64`（当前 Release 构建目标） |

> 预编译二进制约 35MB，仍需安装下方列出的**运行时系统包**（与是否从源码编译无关）。

## 快速安装（推荐）

### 1. 下载

在 [Releases](https://github.com/flowinginthewind700/ubuntuscreenshot/releases) 下载最新版：

- `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz`（Linux x86_64）

### 2. 安装运行时依赖

```bash
sudo apt update
sudo apt install -y \
  libxcb1 \
  libxkbcommon0 \
  libxkbcommon-x11-0 \
  xdg-desktop-portal \
  xdg-desktop-portal-gnome \
  x11-xserver-utils \
  zenity \
  libayatana-appindicator3-1
```

### 3. 解压并运行

```bash
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
```

可选：安装到用户目录，方便全局调用：

```bash
mkdir -p ~/.local/bin
cp screenshot4ubuntu ~/.local/bin/
# 确保 ~/.local/bin 在 PATH 中
```

## 使用说明

1. 运行后顶栏出现 **相机图标**
2. 点击 → **截屏** / Screenshot
3. **拖拽**框选区域（可跨双屏），松开进入编辑
4. 底部工具栏选择标注工具；文字工具在选区内点击后输入（支持中文输入法）
5. 点击 **复制图片** 或 **保存**，或按 `Ctrl+C` / `Ctrl+S`
6. 按 `Esc` 或点 **取消** 放弃本次截屏

### 快捷键

| 按键 | 框选阶段 | 编辑阶段 |
|------|----------|----------|
| `Esc` | 取消 | 取消 |
| `Enter` | 确认选区 | — |
| `Ctrl+C` | — | 复制 |
| `Ctrl+S` | — | 保存 |
| `Ctrl+Z` | — | 撤销标注 |

### 路径与配置

- 截图默认保存目录：`~/Pictures/Screenshots/`
- 语言偏好：`~/.config/screenshot4ubuntu/language`（`zh` 或 `en`）

## 从源码编译

**构建依赖**（仅编译时需要）：

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config curl \
  libxkbcommon-dev libwayland-dev libfontconfig-dev libdbus-1-dev \
  libvulkan-dev libx11-dev libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libssl-dev \
  libclang-dev libxcb1-dev libxrandr-dev libegl-dev
```

安装 [Rust](https://rustup.rs/) 后：

```bash
git clone https://github.com/flowinginthewind700/ubuntuscreenshot.git
cd ubuntuscreenshot
cargo build --release
./target/release/screenshot4ubuntu
```

## 运行时依赖说明

| 包名 | 用途 |
|------|------|
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | GNOME Wayland 整屏截屏 |
| `x11-xserver-utils` | `xrandr` 多显示器布局 |
| `zenity` | 保存文件对话框 |
| `libayatana-appindicator3-1` | 系统托盘图标 |
| `libxcb1` / `libxkbcommon*` | GPUI 窗口与输入 |

**可选**（截屏回退）：`gnome-screenshot`

**不需要**：`wl-clipboard`、Rust / Cargo（仅编译时需要）

## 常见问题

**托盘图标不显示**  
安装 `libayatana-appindicator3-1`，确认 GNOME 已启用 AppIndicator。

**截屏失败 / 只有单屏**  
确认 `xdg-desktop-portal-gnome` 已安装，且 `echo $XDG_SESSION_TYPE` 输出 `wayland`。多屏需要 `xrandr` 可用。

**保存无反应**  
安装 `zenity`。

## 技术栈

- Rust + GPUI 0.2.2（`gpui-local` 子模块）
- xdg-desktop-portal / zbus（Wayland 截屏）
- x11rb（X11 截屏）
- arboard（剪贴板）
- ksni（系统托盘）

## License

[MIT](LICENSE)

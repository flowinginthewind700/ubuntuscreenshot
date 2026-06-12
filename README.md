# Ubuntu Screenshot

[中文](#中文) · [English](#english)

---

## 中文

基于 [GPUI](https://www.gpui.rs/) 的 Ubuntu / GNOME 截屏工具。交互参考微信截屏：托盘一键触发、双屏框选、放大镜精确定位、选区内直接标注，复制或保存后退出。

应用内支持 **简体中文 / English**，可在托盘菜单切换语言。

仓库：[github.com/flowinginthewind700/ubuntuscreenshot](https://github.com/flowinginthewind700/ubuntuscreenshot)

### 功能

- **系统托盘**：顶栏相机图标，菜单含「截屏」「语言」「退出」
- **多显示器**：自动识别虚拟桌面（如双屏 6000×1440），每块屏同步遮罩与框选
- **微信式框选**：全屏半透明遮罩、拖拽实时选区、圆形放大镜（4× + 十字准星）
- **选区锁定编辑**：确认选区后不可重选，在选区内直接标注
- **标注工具**：画笔、直线、矩形、椭圆、文字；可调粗细、字号、颜色
- **复制 / 保存**：编辑完成后直接复制到剪贴板或弹出保存对话框（无需预览窗）

### 系统要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Ubuntu 22.04+ / Debian 12+（其他带 GNOME 的发行版亦可尝试） |
| 桌面会话 | **GNOME Wayland**（推荐）或 X11 |
| 架构 | `x86_64`（当前 Release 构建目标） |

> 预编译二进制约 35MB，仍需安装下方**运行时系统包**（与是否从源码编译无关）。

### 快速安装（推荐）

**1. 下载**

在 [Releases](https://github.com/flowinginthewind700/ubuntuscreenshot/releases) 下载最新版：

- `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz`（Linux x86_64）

**2. 安装运行时依赖**

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

**3. 解压并运行**

```bash
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
```

可选：安装到用户目录

```bash
mkdir -p ~/.local/bin
cp screenshot4ubuntu ~/.local/bin/
# 确保 ~/.local/bin 在 PATH 中
```

### 使用说明

1. 运行后顶栏出现 **相机图标**
2. 点击 → **截屏**
3. **拖拽**框选区域（可跨双屏），松开进入编辑
4. 底部工具栏选择标注工具；文字工具在选区内点击后输入（支持中文输入法）
5. 点击 **复制图片** 或 **保存**，或按 `Ctrl+C` / `Ctrl+S`
6. 按 `Esc` 或点 **取消** 放弃本次截屏

#### 快捷键

| 按键 | 框选阶段 | 编辑阶段 |
|------|----------|----------|
| `Esc` | 取消 | 取消 |
| `Enter` | 确认选区 | — |
| `Ctrl+C` | — | 复制 |
| `Ctrl+S` | — | 保存 |
| `Ctrl+Z` | — | 撤销标注 |

#### 路径与配置

- 截图默认保存目录：`~/Pictures/Screenshots/`
- 语言偏好：`~/.config/screenshot4ubuntu/language`（`zh` 或 `en`）

### 从源码编译

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

### 运行时依赖说明

| 包名 | 用途 |
|------|------|
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | GNOME Wayland 整屏截屏 |
| `x11-xserver-utils` | `xrandr` 多显示器布局 |
| `zenity` | 保存文件对话框 |
| `libayatana-appindicator3-1` | 系统托盘图标 |
| `libxcb1` / `libxkbcommon*` | GPUI 窗口与输入 |

**可选**（截屏回退）：`gnome-screenshot`

**不需要**：`wl-clipboard`、Rust / Cargo（仅编译时需要）

### 常见问题

**托盘图标不显示**  
安装 `libayatana-appindicator3-1`，确认 GNOME 已启用 AppIndicator。

**截屏失败 / 只有单屏**  
确认 `xdg-desktop-portal-gnome` 已安装，且 `echo $XDG_SESSION_TYPE` 输出 `wayland`。多屏需要 `xrandr` 可用。

**保存无反应**  
安装 `zenity`。

### 技术栈

- Rust + GPUI 0.2.2（`gpui-local`）
- xdg-desktop-portal / zbus（Wayland 截屏）
- x11rb（X11 截屏）
- arboard（剪贴板）
- ksni（系统托盘）

### 许可证

[MIT](LICENSE)

---

## English

A WeChat-style screenshot tool for Ubuntu / GNOME, built with [GPUI](https://www.gpui.rs/). Launch from the system tray, select regions across multiple monitors with a magnifier, annotate in-place, then copy or save.

The app UI supports **Simplified Chinese / English** — switch via the tray menu.

Repository: [github.com/flowinginthewind700/ubuntuscreenshot](https://github.com/flowinginthewind700/ubuntuscreenshot)

### Features

- **System tray**: camera icon in the top bar — Screenshot, Language, Quit
- **Multi-monitor**: detects virtual desktop layout (e.g. dual 6000×1440), synced overlay on every display
- **WeChat-style selection**: semi-transparent mask, live selection, circular magnifier (4× + crosshair)
- **Locked region editing**: after confirming the selection, annotate directly inside it
- **Annotation tools**: brush, line, rectangle, ellipse, text; adjustable stroke, font size, color
- **Copy / Save**: copy to clipboard or save via file dialog — no separate preview window

### Requirements

| Item | Requirement |
|------|-------------|
| OS | Ubuntu 22.04+ / Debian 12+ (other GNOME distros may work) |
| Session | **GNOME Wayland** (recommended) or X11 |
| Arch | `x86_64` (current release build target) |

> The prebuilt binary is ~35 MB but still requires the **runtime packages** below (whether you download or build from source).

### Quick Install (Recommended)

**1. Download**

Get the latest build from [Releases](https://github.com/flowinginthewind700/ubuntuscreenshot/releases):

- `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64)

**2. Install runtime dependencies**

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

**3. Extract and run**

```bash
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
```

Optional: install to your user bin directory

```bash
mkdir -p ~/.local/bin
cp screenshot4ubuntu ~/.local/bin/
# ensure ~/.local/bin is on your PATH
```

### Usage

1. A **camera icon** appears in the system tray after launch
2. Click → **Screenshot**
3. **Drag** to select a region (works across dual monitors), release to enter edit mode
4. Use the bottom toolbar; for text, click inside the selection and type (IME supported)
5. Click **Copy** or **Save**, or press `Ctrl+C` / `Ctrl+S`
6. Press `Esc` or **Cancel** to discard

#### Keyboard Shortcuts

| Key | Selecting | Editing |
|-----|-----------|---------|
| `Esc` | Cancel | Cancel |
| `Enter` | Confirm selection | — |
| `Ctrl+C` | — | Copy |
| `Ctrl+S` | — | Save |
| `Ctrl+Z` | — | Undo annotation |

#### Paths & Config

- Default save directory: `~/Pictures/Screenshots/`
- Language preference: `~/.config/screenshot4ubuntu/language` (`zh` or `en`)

### Build from Source

**Build dependencies** (compile-time only):

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config curl \
  libxkbcommon-dev libwayland-dev libfontconfig-dev libdbus-1-dev \
  libvulkan-dev libx11-dev libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libssl-dev \
  libclang-dev libxcb1-dev libxrandr-dev libegl-dev
```

With [Rust](https://rustup.rs/) installed:

```bash
git clone https://github.com/flowinginthewind700/ubuntuscreenshot.git
cd ubuntuscreenshot
cargo build --release
./target/release/screenshot4ubuntu
```

### Runtime Dependencies

| Package | Purpose |
|---------|---------|
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | Full-screen capture on GNOME Wayland |
| `x11-xserver-utils` | `xrandr` for multi-monitor layout |
| `zenity` | Save file dialog |
| `libayatana-appindicator3-1` | System tray icon |
| `libxcb1` / `libxkbcommon*` | GPUI window & input |

**Optional** (capture fallback): `gnome-screenshot`

**Not required**: `wl-clipboard`, Rust / Cargo (compile-time only)

### FAQ

**Tray icon missing**  
Install `libayatana-appindicator3-1` and ensure GNOME AppIndicator is enabled.

**Capture fails / single monitor only**  
Install `xdg-desktop-portal-gnome` and verify `echo $XDG_SESSION_TYPE` prints `wayland`. Multi-monitor needs `xrandr`.

**Save button does nothing**  
Install `zenity`.

### Tech Stack

- Rust + GPUI 0.2.2 (`gpui-local`)
- xdg-desktop-portal / zbus (Wayland capture)
- x11rb (X11 capture)
- arboard (clipboard)
- ksni (system tray)

### License

[MIT](LICENSE)

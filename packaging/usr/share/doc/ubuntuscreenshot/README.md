# ubuntuscreenshot

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
| 架构 | `x86_64` |

> 预编译包约 9MB（`.deb`）/ 35MB（二进制），安装后仍需系统运行时依赖（见下文）。

### 快速安装（推荐）

在 [Releases](https://github.com/flowinginthewind700/ubuntuscreenshot/releases) 下载 **`ubuntuscreenshot_0.2.8_amd64.deb`**，然后：

```bash
sudo dpkg -i ubuntuscreenshot_0.2.8_amd64.deb
sudo apt-get install -f   # 自动补齐缺失依赖
```

安装完成后：

1. 在应用菜单搜索 **Ubuntu 截屏** / **Ubuntu Screenshot** 并启动
2. 顶栏出现 **相机图标**
3. 点击图标 → **截屏**

> `.deb` 会自动声明依赖；`apt-get install -f` 会安装 `pipewire`、`xdg-desktop-portal-gnome` 等必需组件。

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
- 诊断日志：`~/.cache/ubuntuscreenshot/capture.log`

### 截屏原理（v0.2.8）

点击「截屏」后的流程：

```
托盘触发 → 后台静默抓取全屏 → 立刻打开自有 overlay（遮罩 / 放大镜 / 框选 / 标注）
```

Wayland 下采用**双层取帧**，保证命令行与安装版体验一致：

| 优先级 | 方式 | 说明 |
|--------|------|------|
| 1 | `portal Screenshot`（`interactive=false`） | 静默截全屏，**不弹出** GNOME 自带截屏/选屏界面 |
| 2 | ScreenCast + PipeWire | 静默截屏失败时的回退（常见于安装版首次无权限）；授权后通常不再弹窗 |

**不会**调用 `gnome-screenshot`，也**不会**用 GNOME 自带截屏 UI 替代本应用 overlay。

### 运行时依赖

| 包名 | 用途 |
|------|------|
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | Wayland 截屏 portal |
| `pipewire` + `libpipewire-0.3-0` | PipeWire 回退取帧 |
| `x11-xserver-utils` | `xrandr` 多显示器布局 |
| `zenity` | 保存文件对话框 |
| `libayatana-appindicator3-1` | 系统托盘图标 |
| `libxcb1` / `libxkbcommon*` | GPUI 窗口与输入 |

**不需要**：`gnome-screenshot`、`wl-clipboard`、Rust / Cargo（仅编译时需要）

手动安装依赖（非 `.deb` 时）：

```bash
sudo apt update
sudo apt install -y \
  libxcb1 libxkbcommon0 libxkbcommon-x11-0 \
  xdg-desktop-portal xdg-desktop-portal-gnome \
  pipewire libpipewire-0.3-0 \
  x11-xserver-utils zenity libayatana-appindicator3-1
```

### 从源码编译

**构建依赖**：

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev \
  libxkbcommon-dev libwayland-dev libfontconfig-dev libdbus-1-dev \
  libvulkan-dev libx11-dev libx11-xcb-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libssl-dev \
  libxcb1-dev libxrandr-dev libegl-dev
```

若无 sudo，可准备本地编译依赖：

```bash
bash scripts/setup-build-deps.sh
bash scripts/write-cargo-env.sh
source .build-deps/env.sh   # 或直接用 ./scripts/run-dev.sh
```

安装 [Rust](https://rustup.rs/) 后：

```bash
git clone https://github.com/flowinginthewind700/ubuntuscreenshot.git
cd ubuntuscreenshot
./scripts/run-dev.sh        # 开发运行
# 或
cargo build --release
./target/release/screenshot4ubuntu
```

打 `.deb` 包：

```bash
bash scripts/build-deb.sh
# 产物: dist/ubuntuscreenshot_<version>_amd64.deb
```

### 常见问题

**托盘图标不显示**  
安装 `libayatana-appindicator3-1`，确认 GNOME 已启用 AppIndicator。

**截屏失败 / 提示需要权限**  
在「设置 → 应用程序 → Ubuntu 截屏 → 截屏」中开启权限。确认 `xdg-desktop-portal-gnome` 已安装，且 `echo $XDG_SESSION_TYPE` 输出 `wayland`。查看 `~/.cache/ubuntuscreenshot/capture.log`。

**出现「共享屏幕」系统弹窗**  
说明静默截屏失败、走了 PipeWire 回退。在弹窗中选择显示器并点「分享」一次；之后应直接进入自有 overlay。

**截屏失败 / 只有单屏**  
多屏需要 `xrandr` 可用（`x11-xserver-utils`）。

**保存无反应**  
安装 `zenity`。

### 技术栈

- Rust + GPUI 0.2.2（`gpui-local`）
- xdg-desktop-portal Screenshot + ScreenCast / PipeWire（Wayland）
- x11rb（X11 回退）
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

- **System tray**: camera icon — Screenshot, Language, Quit
- **Multi-monitor**: virtual desktop layout (e.g. dual 6000×1440), synced overlay on every display
- **WeChat-style selection**: semi-transparent mask, live selection, circular magnifier (4× + crosshair)
- **Locked region editing**: annotate directly inside the confirmed selection
- **Annotation tools**: brush, line, rectangle, ellipse, text; adjustable stroke, font size, color
- **Copy / Save**: clipboard or file dialog — no separate preview window

### Requirements

| Item | Requirement |
|------|-------------|
| OS | Ubuntu 22.04+ / Debian 12+ |
| Session | **GNOME Wayland** (recommended) or X11 |
| Arch | `x86_64` |

### Quick Install (Recommended)

Download **`ubuntuscreenshot_0.2.8_amd64.deb`** from [Releases](https://github.com/flowinginthewind700/ubuntuscreenshot/releases):

```bash
sudo dpkg -i ubuntuscreenshot_0.2.8_amd64.deb
sudo apt-get install -f
```

Then launch **Ubuntu Screenshot** from the app menu and click **Screenshot** from the tray camera icon.

### Usage

1. Camera icon appears in the system tray
2. Click → **Screenshot**
3. **Drag** to select (works across dual monitors), release to edit
4. Use the bottom toolbar; for text, click inside the selection and type (IME supported)
5. **Copy** or **Save**, or `Ctrl+C` / `Ctrl+S`
6. `Esc` or **Cancel** to discard

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
- Language: `~/.config/screenshot4ubuntu/language` (`zh` or `en`)
- Debug log: `~/.cache/ubuntuscreenshot/capture.log`

### Capture Architecture (v0.2.8)

```
Tray → silent full-screen capture → own overlay (mask / magnifier / selection / annotations)
```

| Priority | Method | Notes |
|----------|--------|-------|
| 1 | `portal Screenshot` (`interactive=false`) | Silent capture, no GNOME picker UI |
| 2 | ScreenCast + PipeWire | Fallback when silent capture fails; usually one-time permission |

Does **not** use `gnome-screenshot` or GNOME's built-in screenshot UI.

### Runtime Dependencies

| Package | Purpose |
|---------|---------|
| `xdg-desktop-portal` + `xdg-desktop-portal-gnome` | Wayland capture portal |
| `pipewire` + `libpipewire-0.3-0` | PipeWire fallback capture |
| `x11-xserver-utils` | `xrandr` multi-monitor layout |
| `zenity` | Save file dialog |
| `libayatana-appindicator3-1` | System tray |
| `libxcb1` / `libxkbcommon*` | GPUI window & input |

**Not required**: `gnome-screenshot`, `wl-clipboard`, Rust / Cargo (build-time only)

### Build from Source

```bash
sudo apt install -y build-essential pkg-config \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev \
  libxkbcommon-dev libwayland-dev libfontconfig-dev libdbus-1-dev \
  libvulkan-dev libx11-dev libxcb1-dev libxrandr-dev libegl-dev

git clone https://github.com/flowinginthewind700/ubuntuscreenshot.git
cd ubuntuscreenshot
./scripts/run-dev.sh
```

Build `.deb`:

```bash
bash scripts/build-deb.sh
```

### FAQ

**Tray icon missing** — Install `libayatana-appindicator3-1`.

**Capture fails** — Enable screen capture for Ubuntu Screenshot under Settings → Apps. Check `~/.cache/ubuntuscreenshot/capture.log`.

**"Share screen" system dialog** — Silent capture failed; grant permission once, then the app's own overlay should appear.

**Single monitor only** — Install `x11-xserver-utils` for `xrandr`.

**Save does nothing** — Install `zenity`.

### Tech Stack

- Rust + GPUI 0.2.2 (`gpui-local`)
- xdg-desktop-portal Screenshot + ScreenCast / PipeWire
- x11rb, arboard, ksni

### License

[MIT](LICENSE)

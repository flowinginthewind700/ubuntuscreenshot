## Ubuntu Screenshot {{VERSION}}

[中文](#中文) · [English](#english)

---

### 中文

微信式 GNOME/Wayland 截屏工具：托盘触发、双屏框选、放大镜、选区内标注、复制/保存。

#### 本版亮点（0.2.6）

- **推荐安装 `.deb` 包**，依赖自动补齐，从应用菜单启动与命令行体验一致
- **双层 Wayland 取帧**：优先静默 portal 截全屏 → 直接进入自有 overlay；失败时 PipeWire 回退
- 修复安装版托盘截屏权限与焦点问题（`app_id`、capture gate）
- 不依赖 `gnome-screenshot`，不使用 GNOME 自带截屏 UI

#### 下载

| 文件 | 说明 |
|------|------|
| `ubuntuscreenshot_0.2.6_amd64.deb` | **推荐** — Ubuntu/Debian 一键安装 |
| `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` | 预编译二进制 + README |

#### 安装（.deb）

```bash
sudo dpkg -i ubuntuscreenshot_0.2.6_amd64.deb
sudo apt-get install -f
```

从应用菜单启动 **Ubuntu 截屏**，顶栏点相机图标 → **截屏**。

#### 使用

1. 顶栏相机图标 → **截屏**
2. 拖拽框选（可跨双屏）→ 标注 → **复制** 或 **保存**
3. `Esc` 取消

完整说明见 [README（中英文）](https://github.com/flowinginthewind700/ubuntuscreenshot#readme)。

---

### English

WeChat-style GNOME/Wayland screenshot tool: tray launcher, dual-monitor selection, magnifier, in-place annotations, copy/save.

#### Highlights (0.2.6)

- **`.deb` package recommended** — auto-installs dependencies; app menu launch matches CLI behavior
- **Dual-layer Wayland capture**: silent portal screenshot first → own overlay; PipeWire fallback on failure
- Fixed tray/permission issues for installed builds
- No `gnome-screenshot` or GNOME built-in screenshot UI

#### Download

| File | Description |
|------|-------------|
| `ubuntuscreenshot_0.2.6_amd64.deb` | **Recommended** — one-click install for Ubuntu/Debian |
| `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` | Prebuilt binary + README |

#### Install (.deb)

```bash
sudo dpkg -i ubuntuscreenshot_0.2.6_amd64.deb
sudo apt-get install -f
```

Launch **Ubuntu Screenshot** from the app menu, then **Screenshot** from the tray.

#### Usage

1. Tray camera icon → **Screenshot**
2. Drag to select (across monitors) → annotate → **Copy** or **Save**
3. `Esc` to cancel

Full docs: [README (中文 / English)](https://github.com/flowinginthewind700/ubuntuscreenshot#readme).

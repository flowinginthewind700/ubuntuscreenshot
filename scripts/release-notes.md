## Ubuntu Screenshot {{VERSION}}

[中文](#中文) · [English](#english)

---

### 中文

微信式 GNOME/Wayland 截屏工具：托盘触发、双屏框选、放大镜、选区内标注、复制/保存。

#### 本版亮点（0.2.8）

- **文字工具可靠性**：选区顶层透明点击层统一处理单击，单击即可稳定出现输入光标
- **文字输入流畅**：打字时不再每键重绘全屏截图，双屏下输入明显更顺滑
- **焦点更稳定**：空文字单击只移动位置，焦点延迟一帧设置，减少「点了没反应」
- 保留 v0.2.7 双屏主屏修复、放大镜性能优化与 v0.2.6 Wayland 双层取帧

#### 下载

| 文件 | 说明 |
|------|------|
| `ubuntuscreenshot_0.2.8_amd64.deb` | **推荐** — Ubuntu/Debian 一键安装 |
| `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` | 预编译二进制 + README |

#### 安装（.deb）

```bash
sudo dpkg -i ubuntuscreenshot_0.2.8_amd64.deb
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

#### Highlights (0.2.8)

- **Text tool reliability**: transparent click layer over the selection — single-click cursor placement works consistently
- **Smoother typing**: no full-screen repaint on every keystroke; much less lag on dual monitors
- **Stable focus**: empty text relocates in place; deferred focus reduces missed clicks
- Retains v0.2.7 primary-monitor fix, magnifier performance, and v0.2.6 dual-layer Wayland capture

#### Download

| File | Description |
|------|-------------|
| `ubuntuscreenshot_0.2.8_amd64.deb` | **Recommended** — one-click install for Ubuntu/Debian |
| `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` | Prebuilt binary + README |

#### Install (.deb)

```bash
sudo dpkg -i ubuntuscreenshot_0.2.8_amd64.deb
sudo apt-get install -f
```

Launch **Ubuntu Screenshot** from the app menu, then **Screenshot** from the tray.

#### Usage

1. Tray camera icon → **Screenshot**
2. Drag to select (across monitors) → annotate → **Copy** or **Save**
3. `Esc` to cancel

Full docs: [README (中文 / English)](https://github.com/flowinginthewind700/ubuntuscreenshot#readme).

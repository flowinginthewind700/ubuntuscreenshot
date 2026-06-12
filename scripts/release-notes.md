## Ubuntu Screenshot {{VERSION}}

[中文](#中文) · [English](#english)

---

### 中文

微信式 GNOME/Wayland 截屏工具：托盘触发、双屏框选、放大镜、选区内标注、复制/保存。

#### 下载

- `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` — Linux x86_64 预编译包

#### 安装

```bash
sudo apt install -y libxcb1 libxkbcommon0 libxkbcommon-x11-0 \
  xdg-desktop-portal xdg-desktop-portal-gnome x11-xserver-utils \
  zenity libayatana-appindicator3-1
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
```

#### 使用

1. 顶栏点击相机图标 → **截屏**
2. 拖拽框选（可跨双屏）→ 标注 → **复制** 或 **保存**
3. `Esc` 取消

完整说明见 [README（中英文）](https://github.com/flowinginthewind700/ubuntuscreenshot#readme)。

---

### English

WeChat-style GNOME/Wayland screenshot tool: tray launcher, dual-monitor selection, magnifier, in-place annotations, copy/save.

#### Download

- `screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz` — prebuilt Linux x86_64 package

#### Install

```bash
sudo apt install -y libxcb1 libxkbcommon0 libxkbcommon-x11-0 \
  xdg-desktop-portal xdg-desktop-portal-gnome x11-xserver-utils \
  zenity libayatana-appindicator3-1
tar -xzf screenshot4ubuntu-x86_64-unknown-linux-gnu.tar.gz
chmod +x screenshot4ubuntu
./screenshot4ubuntu
```

#### Usage

1. Click the camera icon in the tray → **Screenshot**
2. Drag to select (across monitors) → annotate → **Copy** or **Save**
3. `Esc` to cancel

Full docs: [README (中文 / English)](https://github.com/flowinginthewind700/ubuntuscreenshot#readme).

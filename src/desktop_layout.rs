use gpui::{DisplayId, PlatformDisplay};
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct MonitorRect {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Clone, Debug)]
pub struct VirtualDesktop {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitors: Vec<MonitorRect>,
}

impl VirtualDesktop {
    pub fn detect() -> Option<Self> {
        parse_xrandr(&Command::new("xrandr").arg("--query").output().ok()?.stdout)
    }

    pub fn from_monitors(mut monitors: Vec<MonitorRect>) -> Self {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for m in &monitors {
            min_x = min_x.min(m.x);
            min_y = min_y.min(m.y);
            max_x = max_x.max(m.x + m.width as i32);
            max_y = max_y.max(m.y + m.height as i32);
        }
        // 主屏排在前面，方便 UI 默认落在用户常用显示器
        monitors.sort_by(|a, b| {
            b.is_primary
                .cmp(&a.is_primary)
                .then_with(|| a.x.cmp(&b.x))
                .then_with(|| a.y.cmp(&b.y))
        });
        Self {
            x: min_x,
            y: min_y,
            width: (max_x - min_x).max(1) as u32,
            height: (max_y - min_y).max(1) as u32,
            monitors,
        }
    }

    pub fn primary_monitor(&self) -> Option<&MonitorRect> {
        self.monitors.iter().find(|m| m.is_primary)
    }

    /// Wayland 下 GPUI 显示器 bounds 通常为各屏本地 (0,0)，按尺寸匹配未占用的输出。
    pub fn match_display_by_size(
        displays: &[Rc<dyn PlatformDisplay>],
        width: f32,
        height: f32,
        used: &HashSet<DisplayId>,
    ) -> Option<DisplayId> {
        displays
            .iter()
            .find(|d| {
                if used.contains(&d.id()) {
                    return false;
                }
                let b = d.bounds();
                let bw = f32::from(b.size.width);
                let bh = f32::from(b.size.height);
                (bw - width).abs() < 8.0 && (bh - height).abs() < 8.0
            })
            .map(|d| d.id())
    }
}

fn parse_xrandr(stdout: &[u8]) -> Option<VirtualDesktop> {
    let text = String::from_utf8_lossy(stdout);
    let mut monitors = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if !line.contains(" connected") {
            continue;
        }
        // eDP-1 connected primary 2560x1440+0+0 ...
        // HDMI-1 connected 3440x1440+2560+0 ...
        let parts: Vec<&str> = line.split_whitespace().collect();
        let geom = parts.iter().find(|p| p.contains('x') && p.contains('+'))?;
        let (size, pos) = geom.split_once('+')?;
        let (w, h) = size.split_once('x')?;
        let (x, y) = pos.split_once('+')?;
        let is_primary = parts.iter().any(|p| *p == "primary");
        let name = parts.first().copied().unwrap_or("unknown").to_string();
        monitors.push(MonitorRect {
            name,
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            width: w.parse().ok()?,
            height: h.parse().ok()?,
            is_primary,
        });
    }

    if monitors.is_empty() {
        return None;
    }
    Some(VirtualDesktop::from_monitors(monitors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dual_monitor_layout() {
        let sample = r"
Screen 0: current 6000 x 1440
HDMI-1 connected 3440x1440+2560+0
eDP-1 connected primary 2560x1440+0+0
";
        let vd = parse_xrandr(sample.as_bytes()).unwrap();
        assert_eq!(vd.width, 6000);
        assert_eq!(vd.height, 1440);
        assert_eq!(vd.monitors.len(), 2);
        assert!(vd.monitors[0].is_primary);
        assert_eq!(vd.monitors[0].width, 2560);
    }
}

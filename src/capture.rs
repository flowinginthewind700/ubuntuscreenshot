use crate::desktop_layout::VirtualDesktop;
use crate::model::CaptureFrame;
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub fn capture_primary_screen() -> anyhow::Result<CaptureFrame> {
    let layout = VirtualDesktop::detect();
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        capture_wayland(layout.as_ref()).or_else(|wayland_err| {
            capture_x11(layout.as_ref()).map_err(|x11_err| {
                anyhow::anyhow!("Wayland 截屏失败: {wayland_err}; X11 回退失败: {x11_err}")
            })
        })
    } else {
        capture_x11(layout.as_ref()).or_else(|_| capture_wayland(layout.as_ref()))
    }
}

fn capture_wayland(layout: Option<&VirtualDesktop>) -> anyhow::Result<CaptureFrame> {
    // GNOME Wayland 下第三方应用通常无法直接调用 Shell.Screenshot（AccessDenied），
    // xdg-desktop-portal 是最可靠的整屏截屏方式。
    if let Some(layout) = layout {
        if let Ok(frame) = capture_freedesktop_portal(layout) {
            return Ok(frame);
        }
        if let Ok(frame) = capture_gnome_desktop(layout) {
            return Ok(frame);
        }
    }
    capture_gnome_shell(layout).or_else(|gnome_err| {
        capture_freedesktop_portal_legacy(layout).map_err(|portal_err| {
            anyhow::anyhow!("GNOME Shell 截屏失败: {gnome_err}; Portal 截屏失败: {portal_err}")
        })
    })
}

fn capture_gnome_desktop(layout: &VirtualDesktop) -> anyhow::Result<CaptureFrame> {
    if let Ok(img) = capture_full_desktop_image(layout) {
        if img.width() == layout.width && img.height() == layout.height {
            return Ok(frame_from_image(img, layout));
        }
        eprintln!(
            "整屏截屏尺寸 {}x{} 与虚拟桌面 {}x{} 不一致",
            img.width(),
            img.height(),
            layout.width,
            layout.height
        );
    }

    if layout.monitors.len() > 1 {
        if let Ok(frame) = capture_gnome_stitch(layout) {
            return Ok(frame);
        }
    }

    if let Ok(img) = capture_gnome_area(layout.x, layout.y, layout.width, layout.height, layout) {
        if img.width() == layout.width && img.height() == layout.height {
            return Ok(frame_from_image(img, layout));
        }
    }

    anyhow::bail!("GNOME 虚拟桌面截屏失败")
}

fn capture_full_desktop_image(layout: &VirtualDesktop) -> anyhow::Result<RgbaImage> {
    if let Ok(img) = capture_gnome_screenshot_cli() {
        if img.width() == layout.width && img.height() == layout.height {
            return Ok(img);
        }
    }

    if let Ok(img) = capture_gnome_shell_image() {
        if img.width() == layout.width && img.height() == layout.height {
            return Ok(img);
        }
    }

    // 关键：portal 在 GNOME Wayland 下可获取完整虚拟桌面，供逐屏裁剪拼接使用
    if let Ok(img) = capture_portal_image() {
        if img.width() == layout.width && img.height() == layout.height {
            return Ok(img);
        }
    }

    anyhow::bail!("无法获取完整虚拟桌面截图")
}

fn capture_gnome_screenshot_cli() -> anyhow::Result<RgbaImage> {
    let path = temp_png_path()?;
    let output = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(&path)
        .output()
        .map_err(|e| anyhow::anyhow!("无法执行 gnome-screenshot: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gnome-screenshot 失败: {stderr}");
    }
    let img = image::open(&path)?.into_rgba8();
    let _ = std::fs::remove_file(&path);
    Ok(img)
}

fn capture_gnome_shell_image() -> anyhow::Result<RgbaImage> {
    let conn = zbus::blocking::Connection::session()?;
    let proxy = zbus::blocking::Proxy::new(
        &conn,
        "org.gnome.Shell.Screenshot",
        "/org/gnome/Shell/Screenshot",
        "org.gnome.Shell.Screenshot",
    )?;

    let path = temp_png_path()?;
    let filename = path.to_string_lossy().to_string();
    proxy.call_method("Screenshot", &(false, false, filename))?;
    let img = image::open(&path)?.into_rgba8();
    let _ = std::fs::remove_file(&path);
    Ok(img)
}

fn capture_gnome_stitch(layout: &VirtualDesktop) -> anyhow::Result<CaptureFrame> {
    let mut canvas = RgbaImage::new(layout.width, layout.height);
    for monitor in &layout.monitors {
        let piece = capture_gnome_area(monitor.x, monitor.y, monitor.width, monitor.height, layout)
            .map_err(|e| {
                anyhow::anyhow!(
                    "显示器 {}x{}@({},{}) 截屏失败: {e}",
                    monitor.width,
                    monitor.height,
                    monitor.x,
                    monitor.y
                )
            })?;
        if piece.width() != monitor.width || piece.height() != monitor.height {
            anyhow::bail!(
                "显示器 {}x{}@({},{}) 截图片尺寸 {}x{} 不匹配",
                monitor.width,
                monitor.height,
                monitor.x,
                monitor.y,
                piece.width(),
                piece.height()
            );
        }
        image::imageops::overlay(
            &mut canvas,
            &piece,
            (monitor.x - layout.x) as i64,
            (monitor.y - layout.y) as i64,
        );
    }
    Ok(frame_from_image(canvas, layout))
}

fn capture_gnome_area(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    layout: &VirtualDesktop,
) -> anyhow::Result<RgbaImage> {
    match capture_gnome_area_dbus(x, y, width, height) {
        Ok(img) => Ok(img),
        Err(e) if e.to_string().contains("AccessDenied") => {
            let full = capture_full_desktop_image(layout)?;
            Ok(crop_desktop_region(&full, layout, x, y, width, height))
        }
        Err(e) => Err(e),
    }
}

fn capture_gnome_area_dbus(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> anyhow::Result<RgbaImage> {
    let conn = zbus::blocking::Connection::session()?;
    let proxy = zbus::blocking::Proxy::new(
        &conn,
        "org.gnome.Shell.Screenshot",
        "/org/gnome/Shell/Screenshot",
        "org.gnome.Shell.Screenshot",
    )?;

    let path = temp_png_path()?;
    let filename = path.to_string_lossy().to_string();
    proxy.call_method(
        "ScreenshotArea",
        &(x, y, width as i32, height as i32, false, filename),
    )?;
    let img = image::open(&path)?.into_rgba8();
    let _ = std::fs::remove_file(&path);
    Ok(img)
}

fn crop_desktop_region(
    full: &RgbaImage,
    layout: &VirtualDesktop,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> RgbaImage {
    let ox = (x - layout.x).max(0) as u32;
    let oy = (y - layout.y).max(0) as u32;
    let w = width.min(full.width().saturating_sub(ox));
    let h = height.min(full.height().saturating_sub(oy));
    image::imageops::crop_imm(full, ox, oy, w, h).to_image()
}

fn capture_gnome_shell(layout: Option<&VirtualDesktop>) -> anyhow::Result<CaptureFrame> {
    let img = capture_gnome_shell_image()?;
    let (origin_x, origin_y) = layout.map(|l| (l.x, l.y)).unwrap_or((0, 0));
    Ok(CaptureFrame {
        width: img.width(),
        height: img.height(),
        origin_x,
        origin_y,
        pixels: img.into_raw(),
    })
}

#[derive(zbus::zvariant::DeserializeDict, zbus::zvariant::Type, Debug)]
#[zvariant(signature = "dict")]
struct PortalScreenshotResponse {
    uri: String,
}

fn capture_freedesktop_portal(layout: &VirtualDesktop) -> anyhow::Result<CaptureFrame> {
    let img = capture_portal_image()?;
    if img.width() == layout.width && img.height() == layout.height {
        return Ok(frame_from_image(img, layout));
    }

    eprintln!(
        "Portal 截屏尺寸 {}x{} 与虚拟桌面 {}x{} 不一致，尝试逐屏拼接",
        img.width(),
        img.height(),
        layout.width,
        layout.height
    );

    if layout.monitors.len() > 1 {
        return capture_gnome_stitch(layout);
    }

    Ok(frame_from_image(img, layout))
}

fn capture_freedesktop_portal_legacy(layout: Option<&VirtualDesktop>) -> anyhow::Result<CaptureFrame> {
    let img = capture_portal_image()?;
    let (origin_x, origin_y) = layout.map(|l| (l.x, l.y)).unwrap_or((0, 0));
    Ok(CaptureFrame {
        width: img.width(),
        height: img.height(),
        origin_x,
        origin_y,
        pixels: img.into_raw(),
    })
}

fn capture_portal_image() -> anyhow::Result<RgbaImage> {
    let path = capture_portal_png_path()?;
    let img = image::open(&path)?.into_rgba8();
    let _ = std::fs::remove_file(&path);
    Ok(img)
}

fn capture_portal_png_path() -> anyhow::Result<PathBuf> {
    let conn = zbus::blocking::Connection::session()?;
    let handle_token = format!("s4u_{}", std::process::id());

    let unique = conn
        .unique_name()
        .ok_or_else(|| anyhow::anyhow!("无法获取 DBus unique name"))?
        .trim_start_matches(':')
        .replace('.', "_");
    let request_path = format!(
        "/org/freedesktop/portal/desktop/request/{unique}/{handle_token}"
    );

    let portal = zbus::blocking::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )?;

    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::from(handle_token.as_str()));
    options.insert("modal", zbus::zvariant::Value::from(false));
    options.insert("interactive", zbus::zvariant::Value::from(false));

    portal.call_method("Screenshot", &("", options))?;

    let rule = format!(
        "type='signal',interface='org.freedesktop.portal.Request',member='Response',path='{request_path}'"
    );
    let rule = zbus::MatchRule::try_from(rule.as_str())?;
    let mut iter = zbus::blocking::MessageIterator::for_match_rule(rule, &conn, None)?;

    let response = loop {
        let Some(msg) = iter.next() else {
            anyhow::bail!("portal 未返回 Response 信号");
        };
        let msg = msg?;
        let body = msg.body();
        let (code, body): (u32, PortalScreenshotResponse) = body.deserialize()?;
        match code {
            0 => break body,
            1 => anyhow::bail!("portal 截屏已取消"),
            c => anyhow::bail!("portal 截屏失败，错误码: {c}"),
        }
    };

    uri_to_path(&response.uri)
}

fn uri_to_path(uri: &str) -> anyhow::Result<PathBuf> {
    if let Ok(url) = url::Url::parse(uri) {
        return url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("无法解析截屏 URI: {uri}"));
    }
    Ok(PathBuf::from(uri))
}

fn temp_png_path() -> anyhow::Result<PathBuf> {
    let dir = std::env::temp_dir().join("screenshot4ubuntu");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("cap_{}.png", std::process::id()));
    Ok(path)
}

fn frame_from_image(img: RgbaImage, layout: &VirtualDesktop) -> CaptureFrame {
    CaptureFrame {
        width: img.width(),
        height: img.height(),
        origin_x: layout.x,
        origin_y: layout.y,
        pixels: img.into_raw(),
    }
}

fn capture_x11(layout: Option<&VirtualDesktop>) -> anyhow::Result<CaptureFrame> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, ImageFormat};

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let (width, height, origin_x, origin_y) = if let Some(layout) = layout {
        (layout.width as u16, layout.height as u16, layout.x, layout.y)
    } else {
        (
            screen.width_in_pixels,
            screen.height_in_pixels,
            0,
            0,
        )
    };

    let depth = screen.root_depth;
    let visual_type = screen
        .allowed_depths
        .iter()
        .find(|d| d.depth == depth)
        .and_then(|d| d.visuals.first())
        .ok_or_else(|| anyhow::anyhow!("无法获取 X11 visual"))?;

    let image = conn.get_image(
        ImageFormat::Z_PIXMAP,
        screen.root,
        origin_x as i16,
        origin_y as i16,
        width,
        height,
        u32::MAX,
    )?;
    let reply = image.reply()?;

    let rgba = x11_image_to_rgba(
        &reply.data,
        width,
        height,
        depth,
        visual_type.red_mask,
        visual_type.green_mask,
        visual_type.blue_mask,
    )?;

    Ok(CaptureFrame {
        width: width as u32,
        height: height as u32,
        origin_x,
        origin_y,
        pixels: rgba.into_raw(),
    })
}

fn x11_image_to_rgba(
    data: &[u8],
    width: u16,
    height: u16,
    depth: u8,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
) -> anyhow::Result<RgbaImage> {
    let w = width as u32;
    let h = height as u32;
    let mut out = RgbaImage::new(w, h);

    let (r_shift, g_shift, b_shift) = (
        mask_shift(red_mask),
        mask_shift(green_mask),
        mask_shift(blue_mask),
    );

    match depth {
        24 | 32 => {
            let bpp = (depth / 8) as usize;
            for y in 0..h {
                for x in 0..w {
                    let idx = ((y * w + x) as usize) * bpp;
                    if idx + 2 >= data.len() {
                        continue;
                    }
                    let pixel = u32::from_le_bytes([
                        data[idx],
                        data[idx + 1],
                        data[idx + 2],
                        if bpp == 4 { data[idx + 3] } else { 0 },
                    ]);
                    let r = ((pixel & red_mask) >> r_shift) as u8;
                    let g = ((pixel & green_mask) >> g_shift) as u8;
                    let b = ((pixel & blue_mask) >> b_shift) as u8;
                    out.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
        }
        16 => {
            for y in 0..h {
                for x in 0..w {
                    let idx = ((y * w + x) as usize) * 2;
                    if idx + 1 >= data.len() {
                        continue;
                    }
                    let pixel = u16::from_le_bytes([data[idx], data[idx + 1]]) as u32;
                    let r = ((pixel & red_mask) >> r_shift) as u8;
                    let g = ((pixel & green_mask) >> g_shift) as u8;
                    let b = ((pixel & blue_mask) >> b_shift) as u8;
                    out.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
        }
        other => anyhow::bail!("不支持的 X11 色深: {other}"),
    }

    Ok(out)
}

fn mask_shift(mask: u32) -> u32 {
    mask.trailing_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_capture_works() {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return;
        }
        let layout = VirtualDesktop::detect();
        let frame = capture_wayland(layout.as_ref()).expect("wayland capture should work");
        assert!(frame.width > 0);
        assert!(frame.height > 0);
        if let Some(layout) = layout {
            assert_eq!(frame.width, layout.width, "截屏宽度应覆盖虚拟桌面");
            assert_eq!(frame.height, layout.height, "截屏高度应覆盖虚拟桌面");
        }
    }

    #[test]
    fn probe_capture_from_background_thread() {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return;
        }
        let layout = VirtualDesktop::detect().expect("layout");
        let handle = std::thread::spawn(capture_primary_screen);
        let frame = handle.join().unwrap().expect("capture from thread");
        assert_eq!(frame.width, layout.width);
        assert_eq!(frame.height, layout.height);
    }
}

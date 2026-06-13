use crate::desktop_layout::VirtualDesktop;
use crate::model::CaptureFrame;
use crate::screencast;
use crate::util::debug_log;
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;

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
    debug_log(&format!("capture_wayland: layout={layout:?}"));
    let layout = layout.ok_or_else(|| anyhow::anyhow!("无法检测显示器布局"))?;

    // 1) 与 v0.1.0 相同：静默 portal 截全屏，不弹 GNOME 选屏/共享界面，直接进入自有 overlay
    let img = match capture_portal_screenshot_silent() {
        Ok(img) => {
            debug_log(&format!(
                "capture: silent portal ok {}x{}",
                img.width(),
                img.height()
            ));
            img
        }
        Err(portal_err) => {
            debug_log(&format!(
                "capture: silent portal failed ({portal_err:#}), fallback pipewire"
            ));
            screencast::capture_desktop_with_retry(layout)?
        }
    };

    if img.width() == layout.width && img.height() == layout.height {
        return Ok(frame_from_image(img, layout));
    }

    eprintln!(
        "截屏尺寸 {}x{} 与虚拟桌面 {}x{} 不一致，逐屏裁剪",
        img.width(),
        img.height(),
        layout.width,
        layout.height
    );

    if layout.monitors.len() > 1 {
        return Ok(stitch_from_full(&img, layout));
    }

    Ok(frame_from_image(img, layout))
}

#[derive(zbus::zvariant::DeserializeDict, zbus::zvariant::Type, Debug)]
#[zvariant(signature = "dict")]
struct PortalScreenshotResponse {
    uri: String,
}

/// v0.1.0 静默截屏：`interactive=false`，不唤起 GNOME 自带截屏/共享界面。
fn capture_portal_screenshot_silent() -> anyhow::Result<RgbaImage> {
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
    let request_path = format!("/org/freedesktop/portal/desktop/request/{unique}/{handle_token}");

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

    debug_log(&format!("portal Screenshot requested (silent): {request_path}"));
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

    debug_log(&format!("portal screenshot uri: {}", response.uri));
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

fn stitch_from_full(full: &RgbaImage, layout: &VirtualDesktop) -> CaptureFrame {
    let mut canvas = RgbaImage::new(layout.width, layout.height);
    for monitor in &layout.monitors {
        let piece = crop_desktop_region(
            full,
            layout,
            monitor.x,
            monitor.y,
            monitor.width,
            monitor.height,
        );
        image::imageops::overlay(
            &mut canvas,
            &piece,
            (monitor.x - layout.x) as i64,
            (monitor.y - layout.y) as i64,
        );
    }
    frame_from_image(canvas, layout)
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

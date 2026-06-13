pub mod portal;
pub mod pipewire;

use portal::ScreencastStream;
use crate::util::debug_log;
use crate::desktop_layout::VirtualDesktop;
use image::RgbaImage;
use std::os::fd::OwnedFd;
use std::time::Duration;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(8);

pub fn capture_desktop(layout: &VirtualDesktop) -> anyhow::Result<RgbaImage> {
    debug_log("pipewire capture: starting (ScreenCast portal, single frame)");
    let (fd, streams) = portal::capture_via_screencast()?;
    composite_streams(fd, &streams, layout)
}

pub fn capture_desktop_with_retry(layout: &VirtualDesktop) -> anyhow::Result<RgbaImage> {
    match capture_desktop(layout) {
        Ok(img) => Ok(img),
        Err(e) if portal::is_permission_denied(&e) => {
            debug_log(&format!("pipewire capture denied: {e:#}"));
            let _ = portal::open_applications_settings();
            Err(anyhow::anyhow!("{}\n({e:#})", portal::permission_hint()))
        }
        Err(e) => Err(e),
    }
}

fn composite_streams(
    fd: OwnedFd,
    streams: &[ScreencastStream],
    layout: &VirtualDesktop,
) -> anyhow::Result<RgbaImage> {
    let specs: Vec<(u32, (i32, i32), (u32, u32))> = streams
        .iter()
        .map(|s| (s.node_id, s.position, s.size))
        .collect();

    debug_log(&format!(
        "screencast: capturing {} pipewire stream(s)",
        specs.len()
    ));
    let frames = pipewire::capture_streams(fd, &specs, CAPTURE_TIMEOUT)?;

    let mut canvas = RgbaImage::new(layout.width, layout.height);
    if frames.len() == 1
        && frames[0].width == layout.width
        && frames[0].height == layout.height
    {
        return Ok(RgbaImage::from_raw(layout.width, layout.height, frames[0].pixels.clone())
            .ok_or_else(|| anyhow::anyhow!("单流图像缓冲无效"))?);
    }

    for (stream, frame) in streams.iter().zip(frames.into_iter()) {
        let ox = (stream.position.0 - layout.x).max(0) as u32;
        let oy = (stream.position.1 - layout.y).max(0) as u32;
        let piece = RgbaImage::from_raw(frame.width, frame.height, frame.pixels)
            .ok_or_else(|| anyhow::anyhow!("流图像缓冲无效"))?;
        image::imageops::overlay(&mut canvas, &piece, ox as i64, oy as i64);
    }
    Ok(canvas)
}

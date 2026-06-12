use crate::model::{Annotation, CaptureFrame};
use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont, point as ab_point};
use crate::util::px_val;
use gpui::{Bounds, Image, ImageFormat, Pixels, Point, point, px, size};
use image::{Rgba, RgbaImage};
use std::path::Path;

pub fn frame_to_gpui_image(frame: &CaptureFrame) -> anyhow::Result<Image> {
    let img =
        RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone()).ok_or_else(|| {
            anyhow::anyhow!("截屏图像数据无效")
        })?;
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )?;
    Ok(Image::from_bytes(ImageFormat::Png, buf))
}

pub fn crop_frame_to_monitor(
    frame: &CaptureFrame,
    monitor: &crate::desktop_layout::MonitorRect,
) -> anyhow::Result<CaptureFrame> {
    let bounds = Bounds::new(
        point(
            px((monitor.x - frame.origin_x) as f32),
            px((monitor.y - frame.origin_y) as f32),
        ),
        size(px(monitor.width as f32), px(monitor.height as f32)),
    );
    let rgba = crop_frame(frame, bounds)?;
    Ok(CaptureFrame {
        width: monitor.width,
        height: monitor.height,
        origin_x: monitor.x,
        origin_y: monitor.y,
        pixels: rgba.into_raw(),
    })
}

pub fn crop_frame(frame: &CaptureFrame, bounds: Bounds<Pixels>) -> anyhow::Result<RgbaImage> {
    let full = RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
        .ok_or_else(|| anyhow::anyhow!("截屏图像数据无效"))?;

    let x = px_val(bounds.origin.x).max(0.0) as u32;
    let y = px_val(bounds.origin.y).max(0.0) as u32;
    let w = px_val(bounds.size.width).max(1.0) as u32;
    let h = px_val(bounds.size.height).max(1.0) as u32;

    let x = x.min(frame.width.saturating_sub(1));
    let y = y.min(frame.height.saturating_sub(1));
    let w = w.min(frame.width - x);
    let h = h.min(frame.height - y);

    Ok(image::imageops::crop_imm(&full, x, y, w, h).to_image())
}

pub fn render_result(
    base: &RgbaImage,
    annotations: &[Annotation],
    selection: Bounds<Pixels>,
) -> RgbaImage {
    let mut result = base.clone();
    let offset_x = px_val(selection.origin.x);
    let offset_y = px_val(selection.origin.y);

    for ann in annotations {
        match ann {
            Annotation::Brush { points, color, width } => {
                draw_brush(&mut result, points, *color, *width, offset_x, offset_y);
            }
            Annotation::Line { from, to, color, width } => {
                draw_line(
                    &mut result,
                    from,
                    to,
                    *color,
                    *width,
                    offset_x,
                    offset_y,
                );
            }
            Annotation::Rect { bounds, color, width } => {
                draw_rect(&mut result, bounds, *color, *width, offset_x, offset_y);
            }
            Annotation::Ellipse { bounds, color, width } => {
                draw_ellipse(&mut result, bounds, *color, *width, offset_x, offset_y);
            }
            Annotation::Text {
                position,
                content,
                color,
                size,
            } => {
                draw_text(
                    &mut result,
                    position,
                    content,
                    *color,
                    *size,
                    offset_x,
                    offset_y,
                );
            }
        }
    }
    result
}

pub fn rgba_to_png_bytes(img: &RgbaImage) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )?;
    Ok(buf)
}

/// 将图片写入系统剪贴板（优先 arboard，不依赖 wl-copy）。
pub fn copy_image_to_clipboard(img: &RgbaImage) -> anyhow::Result<()> {
    use arboard::ImageData;

    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_image(ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: img.as_raw().clone().into(),
    })?;
    Ok(())
}

pub fn auto_save_png(img: &RgbaImage) -> anyhow::Result<std::path::PathBuf> {
    let path = default_save_path();
    save_png(img, &path)?;
    Ok(path)
}

pub fn default_save_dir() -> std::path::PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
        .join("Screenshots")
}

pub fn default_save_path() -> std::path::PathBuf {
    let dir = default_save_dir();
    let name = chrono::Local::now().format("Screenshot_%Y%m%d_%H%M%S.png");
    dir.join(name.to_string())
}

pub fn prompt_save_png(
    img: &RgbaImage,
    default_path: &Path,
    title: &str,
    filter: &str,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let output = std::process::Command::new("zenity")
        .arg("--file-selection")
        .arg("--save")
        .arg("--confirm-overwrite")
        .arg(format!("--file-filter={filter}"))
        .arg(format!("--filename={}", default_path.display()))
        .arg(format!("--title={title}"))
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path_str.is_empty() {
                Ok(None)
            } else {
                let path = std::path::PathBuf::from(path_str);
                save_png(img, &path)?;
                Ok(Some(path))
            }
        }
        Ok(_) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

pub fn save_png(img: &RgbaImage, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(path)?;
    Ok(())
}

fn to_local(point: Point<Pixels>, ox: f32, oy: f32) -> (i32, i32) {
    ((px_val(point.x) - ox) as i32, (px_val(point.y) - oy) as i32)
}

fn rgba(color: u32) -> Rgba<u8> {
    Rgba([
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
        255,
    ])
}

fn draw_brush(
    img: &mut RgbaImage,
    points: &[Point<Pixels>],
    color: u32,
    width: f32,
    ox: f32,
    oy: f32,
) {
    if points.len() < 2 {
        return;
    }
    let c = rgba(color);
    let radius = (width / 2.0).max(1.0) as i32;
    for pair in points.windows(2) {
        draw_thick_line(
            img,
            to_local(pair[0], ox, oy),
            to_local(pair[1], ox, oy),
            radius,
            c,
        );
    }
}

fn draw_line(
    img: &mut RgbaImage,
    from: &Point<Pixels>,
    to: &Point<Pixels>,
    color: u32,
    width: f32,
    ox: f32,
    oy: f32,
) {
    let radius = (width / 2.0).max(1.0) as i32;
    draw_thick_line(
        img,
        to_local(*from, ox, oy),
        to_local(*to, ox, oy),
        radius,
        rgba(color),
    );
}

fn draw_rect(
    img: &mut RgbaImage,
    bounds: &Bounds<Pixels>,
    color: u32,
    width: f32,
    ox: f32,
    oy: f32,
) {
    let c = rgba(color);
    let t = width.max(1.0) as i32;
    let x0 = (px_val(bounds.origin.x) - ox) as i32;
    let y0 = (px_val(bounds.origin.y) - oy) as i32;
    let x1 = x0 + px_val(bounds.size.width) as i32;
    let y1 = y0 + px_val(bounds.size.height) as i32;
    for x in x0..=x1 {
        for dy in 0..t {
            plot(img, x, y0 + dy, c);
            plot(img, x, y1 - dy, c);
        }
    }
    for y in y0..=y1 {
        for dx in 0..t {
            plot(img, x0 + dx, y, c);
            plot(img, x1 - dx, y, c);
        }
    }
}

fn draw_ellipse(
    img: &mut RgbaImage,
    bounds: &Bounds<Pixels>,
    color: u32,
    width: f32,
    ox: f32,
    oy: f32,
) {
    let c = rgba(color);
    let cx = px_val(bounds.origin.x) - ox + px_val(bounds.size.width) / 2.0;
    let cy = px_val(bounds.origin.y) - oy + px_val(bounds.size.height) / 2.0;
    let rx = px_val(bounds.size.width) / 2.0;
    let ry = px_val(bounds.size.height) / 2.0;
    let steps = 360;
    let mut prev = None;
    for i in 0..=steps {
        let angle = (i as f32 / steps as f32) * std::f32::consts::TAU;
        let x = cx + rx * angle.cos();
        let y = cy + ry * angle.sin();
        if let Some((px, py)) = prev {
            draw_thick_line(
                img,
                (px as i32, py as i32),
                (x as i32, y as i32),
                (width / 2.0).max(1.0) as i32,
                c,
            );
        }
        prev = Some((x, y));
    }
}

fn system_font() -> Option<FontArc> {
    // 优先 CJK 字体（支持中文），否则回退到 DejaVu
    const FONT_PATHS: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    ];
    for path in FONT_PATHS {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(font) = FontArc::try_from_vec(data) {
                return Some(font);
            }
        }
    }
    None
}

fn draw_text(
    img: &mut RgbaImage,
    position: &Point<Pixels>,
    content: &str,
    color: u32,
    size: f32,
    ox: f32,
    oy: f32,
) {
    let Some(font) = system_font() else {
        return;
    };
    let scale = PxScale::from(size);
    let scaled = font.as_scaled(scale);
    let c = rgba(color);
    let line_height = size * 1.25;
    let base_x = px_val(position.x) - ox;
    let mut base_y = px_val(position.y) - oy + scaled.ascent();
    for line in content.split('\n') {
        let mut x = base_x;
        for ch in line.chars() {
            let glyph_id = scaled.glyph_id(ch);
            let glyph = Glyph {
                id: glyph_id,
                scale,
                position: ab_point(x, base_y),
            };
            if let Some(outline) = scaled.outline_glyph(glyph) {
                outline.draw(|gx, gy, v| {
                    let px = x + gx as f32;
                    let py = base_y + gy as f32;
                    blend(img, px as i32, py as i32, blend_rgba(c, v));
                });
            }
            x += scaled.h_advance(glyph_id);
        }
        base_y += line_height;
    }
}

fn draw_thick_line(
    img: &mut RgbaImage,
    from: (i32, i32),
    to: (i32, i32),
    radius: i32,
    color: Rgba<u8>,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let steps = dx.abs().max(dy.abs()).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = from.0 as f32 + dx as f32 * t;
        let y = from.1 as f32 + dy as f32 * t;
        for ox in -radius..=radius {
            for oy in -radius..=radius {
                if (ox * ox + oy * oy) as f32 <= (radius * radius) as f32 {
                    plot(img, (x + ox as f32) as i32, (y + oy as f32) as i32, color);
                }
            }
        }
    }
}

fn plot(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, color);
    }
}

fn blend(img: &mut RgbaImage, x: i32, y: i32, fg: Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        let base = *img.get_pixel(x as u32, y as u32);
        let a = fg[3] as f32 / 255.0;
        let inv = 1.0 - a;
        let out = Rgba([
            (base[0] as f32 * inv + fg[0] as f32 * a) as u8,
            (base[1] as f32 * inv + fg[1] as f32 * a) as u8,
            (base[2] as f32 * inv + fg[2] as f32 * a) as u8,
            255,
        ]);
        img.put_pixel(x as u32, y as u32, out);
    }
}

fn blend_rgba(fg: Rgba<u8>, coverage: f32) -> Rgba<u8> {
    // fg 是前景色，coverage 是像素覆盖率 (0.0-1.0)
    // 返回带正确 alpha 的前景颜色
    Rgba([fg[0], fg[1], fg[2], (fg[3] as f32 * coverage.clamp(0.0, 1.0)) as u8])
}

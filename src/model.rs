use gpui::{Bounds, Pixels, Point, px};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Selecting,
    Editing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Brush,
    Line,
    Rect,
    Ellipse,
    Text,
}

#[derive(Clone, Debug)]
pub enum Annotation {
    Brush {
        points: Vec<Point<Pixels>>,
        color: u32,
        width: f32,
    },
    Line {
        from: Point<Pixels>,
        to: Point<Pixels>,
        color: u32,
        width: f32,
    },
    Rect {
        bounds: Bounds<Pixels>,
        color: u32,
        width: f32,
    },
    Ellipse {
        bounds: Bounds<Pixels>,
        color: u32,
        width: f32,
    },
    Text {
        position: Point<Pixels>,
        content: String,
        color: u32,
        size: f32,
    },
}

impl Annotation {
    pub fn color(&self) -> u32 {
        match self {
            Self::Brush { color, .. }
            | Self::Line { color, .. }
            | Self::Rect { color, .. }
            | Self::Ellipse { color, .. }
            | Self::Text { color, .. } => *color,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaptureFrame {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub pixels: Vec<u8>,
}

impl CaptureFrame {
    pub fn selection_bounds(&self, start: Point<Pixels>, end: Point<Pixels>) -> Bounds<Pixels> {
        let x1 = start.x.min(end.x);
        let y1 = start.y.min(end.y);
        let x2 = start.x.max(end.x);
        let y2 = start.y.max(end.y);
        gpui::bounds(
            gpui::point(x1, y1),
            gpui::size((x2 - x1).max(px(1.0)), (y2 - y1).max(px(1.0))),
        )
    }
}

pub const DEFAULT_STROKE: f32 = 2.5;
pub const DEFAULT_TEXT_SIZE: f32 = 18.0;

/// 画笔/形状线条粗细预设（像素）
pub const STROKE_WIDTHS: [f32; 5] = [1.5, 2.5, 4.0, 6.0, 10.0];

/// 文字字号预设（像素）
pub const TEXT_SIZES: [f32; 4] = [14.0, 18.0, 24.0, 32.0];

pub const COLORS: [u32; 8] = [
    0xFF3B30, // 红
    0xFF9500, // 橙
    0xFFCC00, // 黄
    0x34C759, // 绿
    0x007AFF, // 蓝
    0xAF52DE, // 紫
    0xFFFFFF, // 白
    0x1C1C1E, // 黑
];

pub fn stroke_width_label(w: f32) -> String {
    if (w - w.round()).abs() < 0.01 {
        format!("{}", w as i32)
    } else {
        format!("{w:.1}")
    }
}

pub fn text_size_label(s: f32) -> String {
    format!("{}", s as i32)
}

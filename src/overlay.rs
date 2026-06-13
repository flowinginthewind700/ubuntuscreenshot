use crate::desktop_layout::{MonitorRect, VirtualDesktop};
use crate::i18n::{self, MessageKey, overlay_status, tool_label, tr, tr_app};
use crate::image_util::{
    copy_image_to_clipboard, crop_frame, frame_to_gpui_image, prompt_save_png, render_result,
};
use crate::inline_text::{InlineTextEditor, InlineTextEvent};
use crate::model::{
    Annotation, CaptureFrame, Phase, Tool, COLORS, DEFAULT_STROKE, DEFAULT_TEXT_SIZE,
    STROKE_WIDTHS, TEXT_SIZES, stroke_width_label, text_size_label,
};
use crate::util::{debug_log, desktop_app_id, px_val};
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Corners, Entity, FocusHandle, Focusable, Global,
    Hsla, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, PathBuilder, SharedString,
    TextRun, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, canvas, div, point, prelude::*, px, quad, rgb, rgba, size,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MASK: Hsla = Hsla {
    h: 0.,
    s: 0.,
    l: 0.,
    a: 0.45,
};

const MIN_SELECTION: f32 = 8.0;
const MAG_SIZE: f32 = 140.0;
const MAG_ZOOM: f32 = 2.0;
const POINTER_REFRESH: Duration = Duration::from_millis(16);
const ESC_DEBOUNCE: Duration = Duration::from_millis(350);

#[derive(Default)]
pub struct OverlaySession {
    active: Option<WeakEntity<OverlayCore>>,
}

impl Global for OverlaySession {}

#[derive(Default)]
pub struct CaptureInProgress(pub bool);

impl Global for CaptureInProgress {}

/// 记录截屏开始时间，用于清除卡死的 busy 状态（见 capture_flow）。
pub(crate) struct CaptureBusy {
    pub active: bool,
    pub since: Option<std::time::Instant>,
}

impl Default for CaptureBusy {
    fn default() -> Self {
        Self {
            active: false,
            since: None,
        }
    }
}

impl Global for CaptureBusy {}

pub struct OverlayCore {
    frame: CaptureFrame,
    gpui_image: Arc<gpui::Image>,
    window_handles: Vec<(AnyWindowHandle, gpui::Point<gpui::Pixels>)>,
    monitors: Vec<(f32, f32, f32, f32)>,
    multi_window: bool,
    phase: Phase,
    tool: Tool,
    color: u32,
    stroke_width: f32,
    text_size: f32,
    selection_start: Option<gpui::Point<gpui::Pixels>>,
    selection_end: Option<gpui::Point<gpui::Pixels>>,
    dragging: bool,
    drawing: bool,
    draw_start: Option<gpui::Point<gpui::Pixels>>,
    annotations: Vec<Annotation>,
    status_override: Option<SharedString>,
    editing_text: Option<usize>,
    editing_window: Option<AnyWindowHandle>,
    inline_text: Entity<InlineTextEditor>,
    locale_bound: bool,
    mouse_position: Option<gpui::Point<gpui::Pixels>>,
    focus_handle: FocusHandle,
    expected_windows: usize,
    windows_ready: bool,
    ready_at: Option<Instant>,
    last_pointer_refresh: Option<Instant>,
}

impl OverlayCore {
    pub fn new(frame: CaptureFrame, cx: &mut Context<Self>) -> Self {
        let gpui_image = Arc::new(frame_to_gpui_image(&frame).expect("截屏图像转换失败"));
        let inline_text = cx.new(|cx| InlineTextEditor::new(cx));
        cx.subscribe(&inline_text, |this, _, event, cx| match event {
            InlineTextEvent::Changed => this.sync_editing_text(cx),
            InlineTextEvent::Commit => this.commit_text_edit(cx),
        })
        .detach();

        Self {
            frame,
            gpui_image,
            window_handles: Vec::new(),
            monitors: Vec::new(),
            multi_window: false,
            phase: Phase::Selecting,
            tool: Tool::Brush,
            color: COLORS[0],
            stroke_width: DEFAULT_STROKE,
            text_size: DEFAULT_TEXT_SIZE,
            selection_start: None,
            selection_end: None,
            dragging: false,
            drawing: false,
            draw_start: None,
            annotations: Vec::new(),
            status_override: None,
            editing_text: None,
            editing_window: None,
            inline_text,
            locale_bound: false,
            mouse_position: None,
            focus_handle: cx.focus_handle(),
            expected_windows: 1,
            windows_ready: false,
            ready_at: None,
            last_pointer_refresh: None,
        }
    }

    fn register_window(&mut self, handle: AnyWindowHandle, offset: gpui::Point<gpui::Pixels>) {
        if self
            .window_handles
            .iter()
            .any(|(h, _)| h.window_id() == handle.window_id())
        {
            return;
        }
        self.window_handles.push((handle, offset));
        self.check_windows_ready();
    }

    fn ensure_window_registered(&mut self, handle: AnyWindowHandle, offset: gpui::Point<gpui::Pixels>) {
        self.register_window(handle, offset);
    }

    fn check_windows_ready(&mut self) {
        if self.windows_ready || self.window_handles.len() < self.expected_windows {
            return;
        }
        self.windows_ready = true;
        self.ready_at = Some(Instant::now());
    }

    fn mark_windows_ready(&mut self, cx: &mut Context<Self>) {
        self.check_windows_ready();
        self.notify_all_windows(cx);
    }

    fn get_monitor_offset(&self, window: &Window) -> gpui::Point<gpui::Pixels> {
        let id = window.window_handle().window_id();
        if let Some((_, o)) = self
            .window_handles
            .iter()
            .find(|(h, _)| h.window_id() == id)
        {
            return *o;
        }
        let w = px_val(window.bounds().size.width);
        let h = px_val(window.bounds().size.height);
        let scale = window.scale_factor();
        for (mx, my, mw, mh) in &self.monitors {
            if (w - mw).abs() < 4.0 && (h - mh).abs() < 4.0 {
                return point(px(*mx), px(*my));
            }
            let scaled_w = w * scale;
            let scaled_h = h * scale;
            if (scaled_w - mw).abs() < 8.0 && (scaled_h - mh).abs() < 8.0 {
                return point(px(*mx), px(*my));
            }
        }
        point(px(0.0), px(0.0))
    }

    fn map_pointer(
        &self,
        pos: gpui::Point<gpui::Pixels>,
        monitor_offset: gpui::Point<gpui::Pixels>,
        window: &Window,
    ) -> gpui::Point<gpui::Pixels> {
        let scale = window.scale_factor();
        if self.multi_window {
            point(
                px(px_val(pos.x) * scale + px_val(monitor_offset.x)),
                px(px_val(pos.y) * scale + px_val(monitor_offset.y)),
            )
        } else {
            point(px(px_val(pos.x) * scale), px(px_val(pos.y) * scale))
        }
    }

    fn notify_all_windows(&self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn close_overlay(&mut self, cx: &mut Context<Self>) {
        self.close_all(cx);
    }

    fn close_all(&mut self, cx: &mut Context<Self>) {
        let handles: Vec<AnyWindowHandle> = self.window_handles.iter().map(|(h, _)| *h).collect();
        self.window_handles.clear();
        self.windows_ready = false;
        self.ready_at = None;
        cx.update_global::<OverlaySession, _>(|session, _| session.active = None);
        cx.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = false);
        cx.update_global::<CaptureBusy, _>(|state, _| *state = CaptureBusy::default());
        // 须在帧结束后关闭，否则当前窗口仍在 update 栈上会关不掉
        cx.defer(move |cx| {
            for handle in handles {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        });
    }

    fn sync_editing_text(&mut self, cx: &mut Context<Self>) {
        if let Some(idx) = self.editing_text {
            let content = self.inline_text.read(cx).content().to_string();
            if let Some(Annotation::Text { content: c, .. }) = self.annotations.get_mut(idx) {
                *c = content;
            }
            // 打字时不必刷新整层 overlay（会重绘全屏截图并可能抢焦点），
            // inline_text 自身的 cx.notify 已足够更新光标。
        }
    }

    fn commit_text_edit(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.editing_text.take() else {
            return;
        };
        self.editing_window = None;
        let content = self.inline_text.read(cx).content().trim().to_string();
        if content.is_empty() {
            self.annotations.remove(idx);
        } else if let Some(Annotation::Text { content: c, .. }) = self.annotations.get_mut(idx) {
            *c = content;
        }
        self.inline_text.update(cx, |editor, cx| editor.deactivate(cx));
        self.notify_all_windows(cx);
    }

    fn selection(&self) -> Option<Bounds<gpui::Pixels>> {
        match (self.selection_start, self.selection_end) {
            (Some(a), Some(b)) => Some(self.frame.selection_bounds(a, b)),
            _ => None,
        }
    }

    fn clamp_to_selection(&self, pos: gpui::Point<gpui::Pixels>) -> gpui::Point<gpui::Pixels> {
        if let Some(sel) = self.selection() {
            gpui::point(
                pos.x.max(sel.origin.x).min(sel.origin.x + sel.size.width),
                pos.y.max(sel.origin.y).min(sel.origin.y + sel.size.height),
            )
        } else {
            pos
        }
    }

    fn hit_text_at(&self, pos: gpui::Point<gpui::Pixels>) -> Option<usize> {
        for (idx, ann) in self.annotations.iter().enumerate().rev() {
            let Annotation::Text {
                position,
                content,
                size: font_size,
                ..
            } = ann
            else {
                continue;
            };
            let char_w = font_size * 0.55;
            let w = (content.chars().count() as f32 * char_w).max(char_w * 2.0) + 12.0;
            let h = font_size + 10.0;
            let bounds = gpui::bounds(*position, size(px(w), px(h)));
            if bounds.contains(&pos) {
                return Some(idx);
            }
        }
        None
    }

    fn start_text_editing(
        &mut self,
        idx: usize,
        local_pos: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Annotation::Text {
            position: _,
            content,
            color,
            size,
            ..
        }) = self.annotations.get(idx).cloned()
        else {
            return;
        };
        self.editing_text = Some(idx);
        self.editing_window = Some(window.window_handle());
        self.inline_text.update(cx, |editor, cx| {
            editor.begin_at(local_pos, color, size, content, cx);
        });
        window.activate_window();
        let focus = self.inline_text.read(cx).focus_handle(cx).clone();
        window.defer(cx, move |window, _| {
            focus.focus(window);
        });
        self.notify_all_windows(cx);
    }

    fn relocate_empty_text(
        &mut self,
        idx: usize,
        frame_pos: gpui::Point<gpui::Pixels>,
        local_pos: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Annotation::Text { color, size, .. }) = self.annotations.get(idx).cloned() else {
            return;
        };
        if let Some(Annotation::Text { position, .. }) = self.annotations.get_mut(idx) {
            *position = frame_pos;
        }
        self.editing_window = Some(window.window_handle());
        self.inline_text.update(cx, |editor, cx| {
            editor.begin_at(local_pos, color, size, "", cx);
        });
        window.activate_window();
        let focus = self.inline_text.read(cx).focus_handle(cx).clone();
        window.defer(cx, move |window, _| {
            focus.focus(window);
        });
        self.notify_all_windows(cx);
    }

    fn place_new_text(
        &mut self,
        frame_pos: gpui::Point<gpui::Pixels>,
        local_pos: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let idx = self.annotations.len();
        self.annotations.push(Annotation::Text {
            position: frame_pos,
            content: String::new(),
            color: self.color,
            size: self.text_size,
        });
        self.start_text_editing(idx, local_pos, window, cx);
    }

    fn handle_text_click(
        &mut self,
        clamped: gpui::Point<gpui::Pixels>,
        local_pos: gpui::Point<gpui::Pixels>,
        monitor_offset: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editing_idx) = self.editing_text {
            let empty = self.inline_text.read(cx).content().trim().is_empty();
            if empty {
                self.relocate_empty_text(editing_idx, clamped, local_pos, window, cx);
                return;
            }
            self.commit_text_edit(cx);
        }

        if let Some(idx) = self.hit_text_at(clamped) {
            if let Some(Annotation::Text { position, content, .. }) = self.annotations.get(idx) {
                if !content.trim().is_empty() {
                    let edit_local = self.frame_to_local(*position, monitor_offset, window);
                    self.start_text_editing(idx, edit_local, window, cx);
                    return;
                }
            }
        }

        self.place_new_text(clamped, local_pos, window, cx);
    }

    fn set_stroke_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.stroke_width = width;
        self.notify_all_windows(cx);
    }

    fn set_text_size(&mut self, size: f32, cx: &mut Context<Self>) {
        self.text_size = size;
        if let Some(idx) = self.editing_text {
            if let Some(Annotation::Text { size: s, .. }) = self.annotations.get_mut(idx) {
                *s = size;
            }
            self.inline_text.update(cx, |editor, cx| editor.set_font_size(size, cx));
        }
        self.notify_all_windows(cx);
    }

    fn set_color(&mut self, color: u32, cx: &mut Context<Self>) {
        self.color = color;
        if let Some(idx) = self.editing_text {
            if let Some(Annotation::Text { color: c, .. }) = self.annotations.get_mut(idx) {
                *c = color;
            }
            self.inline_text.update(cx, |editor, cx| editor.set_color(color, cx));
        }
        self.notify_all_windows(cx);
    }

    fn tool_uses_stroke(tool: Tool) -> bool {
        matches!(tool, Tool::Brush | Tool::Line | Tool::Rect | Tool::Ellipse)
    }

    fn push_shape_annotation(&mut self, start: gpui::Point<gpui::Pixels>) {
        let ann = match self.tool {
            Tool::Line => Annotation::Line {
                from: start,
                to: start,
                color: self.color,
                width: self.stroke_width,
            },
            Tool::Rect => Annotation::Rect {
                bounds: self.frame.selection_bounds(start, start),
                color: self.color,
                width: self.stroke_width,
            },
            Tool::Ellipse => Annotation::Ellipse {
                bounds: self.frame.selection_bounds(start, start),
                color: self.color,
                width: self.stroke_width,
            },
            _ => return,
        };
        self.annotations.push(ann);
    }

    fn update_last_shape(&mut self, to: gpui::Point<gpui::Pixels>) {
        let Some(start) = self.draw_start else {
            return;
        };
        let Some(last) = self.annotations.last_mut() else {
            return;
        };
        match (self.tool, last) {
            (Tool::Line, Annotation::Line { to: end, .. }) => *end = to,
            (Tool::Rect, Annotation::Rect { bounds, .. })
            | (Tool::Ellipse, Annotation::Ellipse { bounds, .. }) => {
                *bounds = self.frame.selection_bounds(start, to);
            }
            _ => {}
        }
    }

    fn confirm_selection(&mut self, cx: &mut Context<Self>) {
        let Some(sel) = self.selection() else {
            return;
        };
        if px_val(sel.size.width) < MIN_SELECTION || px_val(sel.size.height) < MIN_SELECTION {
            self.selection_start = None;
            self.selection_end = None;
            self.notify_all_windows(cx);
            return;
        }
        self.phase = Phase::Editing;
        self.tool = Tool::Brush;
        self.status_override = None;
        self.notify_all_windows(cx);
    }

    fn export_image(&self, lang: i18n::Language) -> anyhow::Result<image::RgbaImage> {
        let sel = self
            .selection()
            .ok_or_else(|| anyhow::anyhow!("{}", tr(lang, MessageKey::ErrNoSelection)))?;
        let base = crop_frame(&self.frame, sel)?;
        Ok(render_result(&base, &self.annotations, sel))
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        self.commit_text_edit(cx);
        let lang = i18n::language(cx);
        match self.export_image(lang) {
            Ok(img) => match copy_image_to_clipboard(&img) {
                Ok(()) => self.close_all(cx),
                Err(e) => {
                    self.status_override = Some(format!("{e}").into());
                    self.notify_all_windows(cx);
                }
            },
            Err(e) => {
                self.status_override = Some(format!("{e}").into());
                self.notify_all_windows(cx);
            }
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        self.commit_text_edit(cx);
        let lang = i18n::language(cx);
        match self.export_image(lang) {
            Ok(img) => {
                let default_path = crate::image_util::default_save_path();
                match prompt_save_png(
                    &img,
                    &default_path,
                    tr(lang, MessageKey::ZenitySaveTitle),
                    tr(lang, MessageKey::ZenityPngFilter),
                ) {
                    Ok(Some(_)) => self.close_all(cx),
                    Ok(None) => {}
                    Err(e) => {
                        self.status_override = Some(format!("{e}").into());
                        self.notify_all_windows(cx);
                    }
                }
            }
            Err(e) => {
                self.status_override = Some(format!("{e}").into());
                self.notify_all_windows(cx);
            }
        }
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        if !self.windows_ready {
            return;
        }
        if self
            .ready_at
            .is_some_and(|t| t.elapsed() < ESC_DEBOUNCE)
        {
            return;
        }
        self.close_overlay(cx);
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        if self.editing_text.is_some() {
            self.commit_text_edit(cx);
        }
        self.annotations.pop();
        self.notify_all_windows(cx);
    }

    fn frame_to_local(
        &self,
        frame_pos: gpui::Point<gpui::Pixels>,
        monitor_offset: gpui::Point<gpui::Pixels>,
        window: &Window,
    ) -> gpui::Point<gpui::Pixels> {
        let scale = window.scale_factor();
        if self.multi_window {
            point(
                px((px_val(frame_pos.x) - px_val(monitor_offset.x)) / scale),
                px((px_val(frame_pos.y) - px_val(monitor_offset.y)) / scale),
            )
        } else {
            point(px(px_val(frame_pos.x) / scale), px(px_val(frame_pos.y) / scale))
        }
    }

    fn handle_mouse_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ev.button != MouseButton::Left {
            return;
        }
        let monitor_offset = self.get_monitor_offset(window);
        let pos = self.map_pointer(ev.position, monitor_offset, window);
        let text_click = self.phase == Phase::Editing && self.tool == Tool::Text;
        if !text_click {
            window.focus(&self.focus_handle);
        }
        match self.phase {
            Phase::Selecting => {
                self.dragging = true;
                self.selection_start = Some(pos);
                self.selection_end = Some(pos);
                self.notify_all_windows(cx);
            }
            Phase::Editing => match self.tool {
                Tool::Text => {
                    let clamped = self.clamp_to_selection(pos);
                    self.handle_text_click(clamped, ev.position, monitor_offset, window, cx);
                }
                Tool::Line | Tool::Rect | Tool::Ellipse => {
                    self.commit_text_edit(cx);
                    let clamped = self.clamp_to_selection(pos);
                    self.drawing = true;
                    self.draw_start = Some(clamped);
                    self.push_shape_annotation(clamped);
                    self.notify_all_windows(cx);
                }
                Tool::Brush => {
                    self.commit_text_edit(cx);
                    let clamped = self.clamp_to_selection(pos);
                    self.drawing = true;
                    self.draw_start = Some(clamped);
                    self.annotations.push(Annotation::Brush {
                        points: vec![clamped],
                        color: self.color,
                        width: self.stroke_width,
                    });
                    self.notify_all_windows(cx);
                }
                Tool::Select => {}
            },
        }
    }

    fn handle_mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        monitor_offset: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let pos = self.map_pointer(ev.position, monitor_offset, window);
        self.mouse_position = Some(pos);

        if self.phase == Phase::Selecting {
            if self.dragging {
                self.selection_end = Some(pos);
                self.notify_all_windows(cx);
                return;
            }
            let now = Instant::now();
            let due = self
                .last_pointer_refresh
                .map(|t| now.duration_since(t) >= POINTER_REFRESH)
                .unwrap_or(true);
            if due {
                self.last_pointer_refresh = Some(now);
                self.notify_all_windows(cx);
            }
            return;
        }

        if !self.drawing {
            return;
        }

        let clamped = self.clamp_to_selection(pos);
        match self.tool {
            Tool::Brush => {
                if let Some(Annotation::Brush { points, .. }) = self.annotations.last_mut() {
                    points.push(clamped);
                    self.notify_all_windows(cx);
                }
            }
            Tool::Line | Tool::Rect | Tool::Ellipse => {
                self.update_last_shape(clamped);
                self.notify_all_windows(cx);
            }
            _ => {}
        }
    }

    fn handle_mouse_up(&mut self, cx: &mut Context<Self>) {
        if self.dragging {
            self.dragging = false;
            if self.phase == Phase::Selecting {
                self.confirm_selection(cx);
            } else {
                self.notify_all_windows(cx);
            }
            return;
        }
        if self.drawing {
            self.drawing = false;
            self.draw_start = None;
            self.notify_all_windows(cx);
        }
    }

    fn color_to_hsla(color: u32) -> Hsla {
        rgb(color).into()
    }

    fn paint_text_lines(
        window: &mut Window,
        cx: &mut App,
        origin: gpui::Point<gpui::Pixels>,
        content: &str,
        color: u32,
        font_size: f32,
    ) {
        let style = window.text_style();
        let size_px = style.font_size.to_pixels(window.rem_size()).max(px(font_size));
        let line_height = size_px * 1.25;
        let text_color = Self::color_to_hsla(color);
        for (idx, line_text) in content.split('\n').enumerate() {
            let run = TextRun {
                len: line_text.len(),
                font: style.font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line = window.text_system().shape_line(
                SharedString::from(line_text.to_string()),
                size_px,
                &[run],
                None,
            );
            let y = origin.y + line_height * idx as f32;
            let _ = line.paint(point(origin.x, y), line_height, window, cx);
        }
    }

    fn paint_annotation(window: &mut Window, cx: &mut App, ann: &Annotation) {
        match ann {
            Annotation::Brush { points, color, width } => {
                if points.len() < 2 {
                    return;
                }
                let mut builder = PathBuilder::stroke(px(*width));
                for (i, p) in points.iter().enumerate() {
                    if i == 0 {
                        builder.move_to(*p);
                    } else {
                        builder.line_to(*p);
                    }
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, Self::color_to_hsla(*color));
                }
            }
            Annotation::Line { from, to, color, width } => {
                let mut builder = PathBuilder::stroke(px(*width));
                builder.move_to(*from);
                builder.line_to(*to);
                if let Ok(path) = builder.build() {
                    window.paint_path(path, Self::color_to_hsla(*color));
                }
            }
            Annotation::Rect { bounds, color, width } => {
                let mut builder = PathBuilder::stroke(px(*width));
                let tl = bounds.origin;
                let tr = bounds.top_right();
                let br = bounds.bottom_right();
                let bl = bounds.bottom_left();
                builder.add_polygon(&[tl, tr, br, bl], true);
                if let Ok(path) = builder.build() {
                    window.paint_path(path, Self::color_to_hsla(*color));
                }
            }
            Annotation::Ellipse { bounds, color, width } => {
                let center_x = bounds.origin.x + bounds.size.width / 2.;
                let center_y = bounds.origin.y + bounds.size.height / 2.;
                let rx = bounds.size.width / 2.;
                let ry = bounds.size.height / 2.;
                let steps = 64;
                let mut points = Vec::with_capacity(steps);
                for i in 0..steps {
                    let angle = (i as f32 / steps as f32) * std::f32::consts::TAU;
                    points.push(point(
                        center_x + rx * angle.cos(),
                        center_y + ry * angle.sin(),
                    ));
                }
                let mut builder = PathBuilder::stroke(px(*width));
                builder.add_polygon(&points, true);
                if let Ok(path) = builder.build() {
                    window.paint_path(path, Self::color_to_hsla(*color));
                }
            }
            Annotation::Text {
                position,
                content,
                color,
                size: font_size,
            } => {
                Self::paint_text_lines(window, cx, *position, content, *color, *font_size);
            }
        }
    }
}

fn tool_button(
    id: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    cx: &mut Context<OverlayCore>,
    on_click: std::rc::Rc<dyn Fn(&mut OverlayCore, &mut Context<OverlayCore>)>,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(SharedString::from(id))
        .px_3()
        .py_1p5()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .when(active, |d| d.bg(rgb(0x007AFF)).text_color(rgb(0xffffff)))
        .when(!active, |d| d.bg(rgb(0x2c2c2e)).text_color(rgb(0xe5e5ea)))
        .child(label.clone())
        .on_click(cx.listener(move |this, _, _, cx| {
            on_click(this, cx);
        }))
}

fn toolbar_label(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(rgb(0x999999))
        .mr_1()
        .child(text.into())
}

fn annotation_for_window(ann: &Annotation, off_x: f32, off_y: f32, scale: f32) -> Annotation {
    match ann {
        Annotation::Brush { points, color, width } => Annotation::Brush {
            points: points
                .iter()
                .map(|p| {
                    point(
                        px((px_val(p.x) - off_x) / scale),
                        px((px_val(p.y) - off_y) / scale),
                    )
                })
                .collect(),
            color: *color,
            width: *width,
        },
        Annotation::Line { from, to, color, width } => Annotation::Line {
            from: point(
                px((px_val(from.x) - off_x) / scale),
                px((px_val(from.y) - off_y) / scale),
            ),
            to: point(
                px((px_val(to.x) - off_x) / scale),
                px((px_val(to.y) - off_y) / scale),
            ),
            color: *color,
            width: *width,
        },
        Annotation::Rect { bounds, color, width } => Annotation::Rect {
            bounds: gpui::bounds(
                point(
                    px((px_val(bounds.origin.x) - off_x) / scale),
                    px((px_val(bounds.origin.y) - off_y) / scale),
                ),
                size(
                    px(px_val(bounds.size.width) / scale),
                    px(px_val(bounds.size.height) / scale),
                ),
            ),
            color: *color,
            width: *width,
        },
        Annotation::Ellipse { bounds, color, width } => Annotation::Ellipse {
            bounds: gpui::bounds(
                point(
                    px((px_val(bounds.origin.x) - off_x) / scale),
                    px((px_val(bounds.origin.y) - off_y) / scale),
                ),
                size(
                    px(px_val(bounds.size.width) / scale),
                    px(px_val(bounds.size.height) / scale),
                ),
            ),
            color: *color,
            width: *width,
        },
        Annotation::Text {
            position,
            content,
            color,
            size,
        } => Annotation::Text {
            position: point(
                px((px_val(position.x) - off_x) / scale),
                px((px_val(position.y) - off_y) / scale),
            ),
            content: content.clone(),
            color: *color,
            size: *size,
        },
    }
}

fn action_button(
    id: &'static str,
    label: impl Into<SharedString>,
    bg: u32,
    cx: &mut Context<OverlayCore>,
    on_click: std::rc::Rc<dyn Fn(&mut OverlayCore, &mut Context<OverlayCore>)>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .px_3()
        .py_1p5()
        .rounded_md()
        .bg(rgb(bg))
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .text_sm()
        .child(label.into())
        .on_click(cx.listener(move |this, _, _, cx| {
            on_click(this, cx);
        }))
}

impl Render for OverlayCore {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let monitor_offset = self.get_monitor_offset(window);
        self.ensure_window_registered(window.window_handle(), monitor_offset);

        if !self.locale_bound {
            self.locale_bound = true;
            cx.observe_global_in::<i18n::LocaleSettings>(window, |_, _, cx| cx.notify())
                .detach();
        }

        let lang = i18n::language(cx);
        let gpui_image = self.gpui_image.clone();
        let gpui_image_mag = gpui_image.clone();
        let selection = self.selection();
        let annotations = self.annotations.clone();
        let phase = self.phase;
        let tool = self.tool;
        let color = self.color;
        let stroke_width = self.stroke_width;
        let text_size = self.text_size;
        let status = self
            .status_override
            .clone()
            .unwrap_or_else(|| overlay_status(lang, phase, tool).into());
        let inline_text = self.inline_text.clone();
        let editing_text = self.editing_text;
        let editing_window = self.editing_window;
        let current_window = window.window_handle();
        let frame_w = self.frame.width as f32;
        let frame_h = self.frame.height as f32;
        let mouse_pos = self.mouse_position;
        let multi_win = self.multi_window;
        let mon_off_x = px_val(monitor_offset.x);
        let mon_off_y = px_val(monitor_offset.y);
        let display_w = px_val(window.bounds().size.width);
        let display_h = px_val(window.bounds().size.height);
        let window_scale = window.scale_factor();

        let sel_for_paint = selection;
        let size_label = selection.map(|b| {
            format!(
                "{} × {}",
                px_val(b.size.width).round() as i32,
                px_val(b.size.height).round() as i32
            )
        });

        div()
            .size_full()
            .bg(rgb(0x000000))
            .track_focus(&self.focus_handle)
            .key_context("Overlay")
            .on_action(cx.listener(|this, _: &crate::actions::CancelCapture, _, cx| {
                this.cancel(cx);
            }))
            .on_action(cx.listener(|this, _: &crate::actions::ConfirmCapture, _, cx| {
                if this.editing_text.is_some() {
                    return;
                }
                if this.phase == Phase::Selecting {
                    this.confirm_selection(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &crate::actions::SaveCapture, _, cx| {
                if this.phase == Phase::Editing {
                    this.save(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &crate::actions::CopyCapture, _, cx| {
                if this.phase == Phase::Editing {
                    this.copy(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &crate::actions::UndoAnnotation, _, cx| {
                this.undo(cx);
            }))
            .child(
                div()
                    .size_full()
                    .child(
                        canvas(
                            move |_, _, _| {},
                            move |bounds, _, window, cx| {
                                let scale = window.scale_factor();
                                if let Some(render_image) = gpui_image.use_render_image(window, cx) {
                                    let image_bounds = if multi_win {
                                        Bounds::new(
                                            point(
                                                bounds.origin.x - px(mon_off_x / scale),
                                                bounds.origin.y - px(mon_off_y / scale),
                                            ),
                                            size(px(frame_w / scale), px(frame_h / scale)),
                                        )
                                    } else {
                                        Bounds::new(
                                            bounds.origin,
                                            size(px(frame_w / scale), px(frame_h / scale)),
                                        )
                                    };
                                    let _ = window.paint_image(
                                        image_bounds,
                                        Corners::default(),
                                        render_image,
                                        0,
                                        false,
                                    );
                                }

                                if let Some(sel) = sel_for_paint {
                                    let scaled_sel = if multi_win {
                                        gpui::bounds(
                                            point(
                                                px((px_val(sel.origin.x) - mon_off_x) / scale),
                                                px((px_val(sel.origin.y) - mon_off_y) / scale),
                                            ),
                                            size(
                                                px(px_val(sel.size.width) / scale),
                                                px(px_val(sel.size.height) / scale),
                                            ),
                                        )
                                    } else {
                                        gpui::bounds(
                                            point(
                                                px(px_val(sel.origin.x) / scale),
                                                px(px_val(sel.origin.y) / scale),
                                            ),
                                            size(
                                                px(px_val(sel.size.width) / scale),
                                                px(px_val(sel.size.height) / scale),
                                            ),
                                        )
                                    };

                                    let top = Bounds::new(
                                        bounds.origin,
                                        size(
                                            bounds.size.width,
                                            scaled_sel.origin.y - bounds.origin.y,
                                        ),
                                    );
                                    let bottom = Bounds::new(
                                        point(bounds.origin.x, scaled_sel.bottom_right().y),
                                        size(
                                            bounds.size.width,
                                            bounds.bottom_right().y - scaled_sel.bottom_right().y,
                                        ),
                                    );
                                    let left = Bounds::new(
                                        point(bounds.origin.x, scaled_sel.origin.y),
                                        size(
                                            scaled_sel.origin.x - bounds.origin.x,
                                            scaled_sel.size.height,
                                        ),
                                    );
                                    let right = Bounds::new(
                                        point(scaled_sel.top_right().x, scaled_sel.origin.y),
                                        size(
                                            bounds.top_right().x - scaled_sel.top_right().x,
                                            scaled_sel.size.height,
                                        ),
                                    );
                                    for mask_bounds in [top, bottom, left, right] {
                                        if px_val(mask_bounds.size.width) > 0.0
                                            && px_val(mask_bounds.size.height) > 0.0
                                        {
                                            window.paint_quad(quad(
                                                mask_bounds,
                                                px(0.),
                                                MASK,
                                                px(0.),
                                                gpui::transparent_black(),
                                                Default::default(),
                                            ));
                                        }
                                    }

                                    window.paint_quad(quad(
                                        scaled_sel,
                                        px(0.),
                                        gpui::transparent_black(),
                                        px(2.),
                                        rgb(0x007AFF),
                                        Default::default(),
                                    ));
                                } else if phase == Phase::Selecting {
                                    window.paint_quad(quad(
                                        bounds,
                                        px(0.),
                                        MASK,
                                        px(0.),
                                        gpui::transparent_black(),
                                        Default::default(),
                                    ));
                                }

                                for (idx, ann) in annotations.iter().enumerate() {
                                    if Some(idx) == editing_text {
                                        continue;
                                    }
                                    let scaled = if multi_win {
                                        annotation_for_window(ann, mon_off_x, mon_off_y, scale)
                                    } else {
                                        annotation_for_window(ann, 0.0, 0.0, scale)
                                    };
                                    OverlayCore::paint_annotation(window, cx, &scaled);
                                }
                            },
                        )
                        .size_full(),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, ev, window, cx| {
                        this.handle_mouse_down(ev, window, cx);
                    }))
                    .on_mouse_move(cx.listener(|this, ev, window, cx| {
                        let offset = this.get_monitor_offset(window);
                        this.handle_mouse_move(ev, offset, window, cx);
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.handle_mouse_up(cx);
                    })),
            )
            .when(phase == Phase::Selecting && mouse_pos.is_some(), |this| {
                let pos = mouse_pos.unwrap();
                let pos_x = px_val(pos.x);
                let pos_y = px_val(pos.y);
                let local_x = (pos_x - mon_off_x) / window_scale;
                let local_y = (pos_y - mon_off_y) / window_scale;
                if multi_win
                    && (local_x < 0.0
                        || local_y < 0.0
                        || local_x > display_w
                        || local_y > display_h)
                {
                    return this;
                }
                let offset = 28.0;
                let margin = 8.0;
                let mag_x = if local_x + offset + MAG_SIZE + margin < display_w {
                    local_x + offset
                } else {
                    local_x - MAG_SIZE - offset
                }
                .max(margin)
                .min(display_w - MAG_SIZE - margin);
                let mag_y = if local_y + offset + MAG_SIZE + margin < display_h {
                    local_y + offset
                } else {
                    local_y - MAG_SIZE - offset
                }
                .max(margin)
                .min(display_h - MAG_SIZE - margin);

                this.child(
                    div()
                        .absolute()
                        .left(px(mag_x))
                        .top(px(mag_y))
                        .w(px(MAG_SIZE))
                        .h(px(MAG_SIZE))
                        .rounded(px(MAG_SIZE / 2.0))
                        .border_2()
                        .border_color(rgb(0x007AFF))
                        .bg(rgb(0x111111))
                        .overflow_hidden()
                        .child(
                            canvas(
                                move |_, _, _| {},
                                move |mag_bounds, _, window, cx| {
                                    if let Some(render_image) =
                                        gpui_image_mag.use_render_image(window, cx)
                                    {
                                        let zoomed_w = frame_w * MAG_ZOOM;
                                        let zoomed_h = frame_h * MAG_ZOOM;
                                        let offset_x = mag_bounds.origin.x + px(MAG_SIZE / 2.0)
                                            - px(pos_x * MAG_ZOOM);
                                        let offset_y = mag_bounds.origin.y + px(MAG_SIZE / 2.0)
                                            - px(pos_y * MAG_ZOOM);
                                        let _ = window.paint_image(
                                            Bounds::new(
                                                point(offset_x, offset_y),
                                                size(px(zoomed_w), px(zoomed_h)),
                                            ),
                                            Corners::default(),
                                            render_image,
                                            0,
                                            false,
                                        );
                                    }
                                    let cx_pos = mag_bounds.origin.x + px(MAG_SIZE / 2.0);
                                    let cy_pos = mag_bounds.origin.y + px(MAG_SIZE / 2.0);
                                    window.paint_quad(quad(
                                        Bounds::new(
                                            point(mag_bounds.origin.x, cy_pos - px(0.5)),
                                            size(px(MAG_SIZE), px(1.0)),
                                        ),
                                        px(0.),
                                        rgba(0x007affaa),
                                        px(0.),
                                        gpui::transparent_black(),
                                        Default::default(),
                                    ));
                                    window.paint_quad(quad(
                                        Bounds::new(
                                            point(cx_pos - px(0.5), mag_bounds.origin.y),
                                            size(px(1.0), px(MAG_SIZE)),
                                        ),
                                        px(0.),
                                        rgba(0x007affaa),
                                        px(0.),
                                        gpui::transparent_black(),
                                        Default::default(),
                                    ));
                                },
                            )
                            .size_full(),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .top_4()
                    .left_4()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(rgba(0x000000aa))
                    .text_color(rgb(0xffffff))
                    .text_sm()
                    .child(status),
            )
            .when_some(size_label, |this, label| {
                this.child(
                    div()
                        .absolute()
                        .bottom_4()
                        .left_4()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(rgba(0x000000aa))
                        .text_color(rgb(0xffffff))
                        .text_xs()
                        .child(label),
                )
            })
            .when(phase == Phase::Editing, |this| {
                this.child(
                    div()
                        .absolute()
                        .bottom_6()
                        .left_8()
                        .right_8()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .px_4()
                        .py_3()
                        .rounded_lg()
                        .bg(rgba(0x1c1c1ee6))
                        .border_1()
                        .border_color(rgba(0xffffff22))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .child(tool_button(
                                    "tool-brush",
                                    tool_label(lang, Tool::Brush),
                                    tool == Tool::Brush,
                                    cx,
                                    std::rc::Rc::new(|t, cx| {
                                        t.commit_text_edit(cx);
                                        t.tool = Tool::Brush;
                                        cx.notify();
                                    }),
                                ))
                                .child(tool_button(
                                    "tool-line",
                                    tool_label(lang, Tool::Line),
                                    tool == Tool::Line,
                                    cx,
                                    std::rc::Rc::new(|t, cx| {
                                        t.commit_text_edit(cx);
                                        t.tool = Tool::Line;
                                        cx.notify();
                                    }),
                                ))
                                .child(tool_button(
                                    "tool-rect",
                                    tool_label(lang, Tool::Rect),
                                    tool == Tool::Rect,
                                    cx,
                                    std::rc::Rc::new(|t, cx| {
                                        t.commit_text_edit(cx);
                                        t.tool = Tool::Rect;
                                        cx.notify();
                                    }),
                                ))
                                .child(tool_button(
                                    "tool-ellipse",
                                    tool_label(lang, Tool::Ellipse),
                                    tool == Tool::Ellipse,
                                    cx,
                                    std::rc::Rc::new(|t, cx| {
                                        t.commit_text_edit(cx);
                                        t.tool = Tool::Ellipse;
                                        cx.notify();
                                    }),
                                ))
                                .child(tool_button(
                                    "tool-text",
                                    tool_label(lang, Tool::Text),
                                    tool == Tool::Text,
                                    cx,
                                    std::rc::Rc::new(|t, cx| {
                                        if t.tool != Tool::Text {
                                            t.commit_text_edit(cx);
                                        }
                                        t.tool = Tool::Text;
                                        t.notify_all_windows(cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .ml_auto()
                                        .flex()
                                        .gap_2()
                                        .child(action_button(
                                            "copy-btn",
                                            tr_app(cx, MessageKey::PreviewCopyImage),
                                            0x34C759,
                                            cx,
                                            std::rc::Rc::new(|t, cx| t.copy(cx)),
                                        ))
                                        .child(action_button(
                                            "save-btn",
                                            tr_app(cx, MessageKey::PreviewSave),
                                            0x007AFF,
                                            cx,
                                            std::rc::Rc::new(|t, cx| t.save(cx)),
                                        ))
                                        .child(
                                            div()
                                                .id("cancel-btn")
                                                .px_3()
                                                .py_1p5()
                                                .rounded_md()
                                                .bg(rgb(0x3a3a3c))
                                                .text_color(rgb(0xffffff))
                                                .cursor_pointer()
                                                .text_sm()
                                                .child(tr_app(cx, MessageKey::BtnCancel))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.cancel(cx);
                                                })),
                                        ),
                                ),
                        )
                        .when(OverlayCore::tool_uses_stroke(tool), |bar| {
                            bar.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(toolbar_label(tr_app(
                                        cx,
                                        MessageKey::LabelStrokeWidth,
                                    )))
                                    .children(STROKE_WIDTHS.iter().map(|&w| {
                                        let active = (stroke_width - w).abs() < 0.01;
                                        let label = stroke_width_label(w);
                                        div()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_xs()
                                            .when(active, |d| {
                                                d.bg(rgb(0x007AFF)).text_color(rgb(0xffffff))
                                            })
                                            .when(!active, |d| {
                                                d.bg(rgb(0x3a3a3c)).text_color(rgb(0xe5e5ea))
                                            })
                                            .child(SharedString::from(label))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.set_stroke_width(w, cx);
                                                }),
                                            )
                                    })),
                            )
                        })
                        .when(tool == Tool::Text, |bar| {
                            bar.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(toolbar_label(tr_app(cx, MessageKey::LabelFontSize)))
                                    .children(TEXT_SIZES.iter().map(|&s| {
                                        let active = (text_size - s).abs() < 0.01;
                                        let label = text_size_label(s);
                                        div()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_xs()
                                            .when(active, |d| {
                                                d.bg(rgb(0x007AFF)).text_color(rgb(0xffffff))
                                            })
                                            .when(!active, |d| {
                                                d.bg(rgb(0x3a3a3c)).text_color(rgb(0xe5e5ea))
                                            })
                                            .child(SharedString::from(label))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.set_text_size(s, cx);
                                                }),
                                            )
                                    })),
                            )
                        })
                        .when(tool != Tool::Select, |bar| {
                            bar.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(toolbar_label(tr_app(cx, MessageKey::LabelColor)))
                                    .children(COLORS.iter().map(|&c| {
                                        let selected = color == c;
                                        let is_light = c == 0xFFFFFF;
                                        div()
                                            .id(SharedString::from(format!("color-{c:08x}")))
                                            .size(px(22.))
                                            .rounded_full()
                                            .bg(rgb(c))
                                            .border_2()
                                            .when(selected, |d| d.border_color(rgb(0x007AFF)))
                                            .when(!selected && is_light, |d| {
                                                d.border_color(rgb(0x666666))
                                            })
                                            .when(!selected && !is_light, |d| {
                                                d.border_color(gpui::transparent_black())
                                            })
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.set_color(c, cx);
                                            }))
                                    })),
                            )
                        }),
                )
            })
            .when(
                editing_text.is_none()
                    || !multi_win
                    || editing_window.is_some_and(|h| h.window_id() == current_window.window_id()),
                |this| this.child(inline_text),
            )
            .when(tool == Tool::Text && phase == Phase::Editing, |this| {
                if let Some(sel) = sel_for_paint {
                    let scale = window_scale;
                    let scaled_sel = if multi_win {
                        gpui::bounds(
                            point(
                                px((px_val(sel.origin.x) - mon_off_x) / scale),
                                px((px_val(sel.origin.y) - mon_off_y) / scale),
                            ),
                            size(
                                px(px_val(sel.size.width) / scale),
                                px(px_val(sel.size.height) / scale),
                            ),
                        )
                    } else {
                        gpui::bounds(
                            point(
                                px(px_val(sel.origin.x) / scale),
                                px(px_val(sel.origin.y) / scale),
                            ),
                            size(
                                px(px_val(sel.size.width) / scale),
                                px(px_val(sel.size.height) / scale),
                            ),
                        )
                    };
                    this.child(
                        div()
                            .absolute()
                            .left(scaled_sel.origin.x)
                            .top(scaled_sel.origin.y)
                            .w(scaled_sel.size.width)
                            .h(scaled_sel.size.height)
                            .occlude()
                            .cursor(gpui::CursorStyle::IBeam)
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                                let monitor_offset = this.get_monitor_offset(window);
                                let pos = this.map_pointer(ev.position, monitor_offset, window);
                                let clamped = this.clamp_to_selection(pos);
                                this.handle_text_click(
                                    clamped,
                                    ev.position,
                                    monitor_offset,
                                    window,
                                    cx,
                                );
                            })),
                    )
                } else {
                    this
                }
            })
    }
}

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn register_overlay_keybindings(cx: &mut App) {
    static REGISTERED: AtomicBool = AtomicBool::new(false);
    if REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    cx.bind_keys([
        KeyBinding::new("escape", crate::actions::CancelCapture, Some("Overlay")),
        KeyBinding::new("enter", crate::actions::ConfirmCapture, Some("Overlay")),
        KeyBinding::new("ctrl-s", crate::actions::SaveCapture, Some("Overlay")),
        KeyBinding::new("ctrl-c", crate::actions::CopyCapture, Some("Overlay")),
        KeyBinding::new("ctrl-z", crate::actions::UndoAnnotation, Some("Overlay")),
    ]);
}

pub fn close_any_active_overlay(cx: &mut App) {
    let weak = cx
        .try_global::<OverlaySession>()
        .and_then(|s| s.active.clone());
    let Some(weak) = weak else {
        return;
    };
    if let Some(entity) = weak.upgrade() {
        let _ = entity.update(cx, |overlay, cx| overlay.close_all(cx));
    }
}

#[derive(Clone)]
struct MonitorOpenSpec {
    index: usize,
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    is_primary: bool,
}

fn open_monitor_window(
    cx: &mut App,
    entity: &Entity<OverlayCore>,
    spec: &MonitorOpenSpec,
    wayland: bool,
    multi: bool,
    displays: &[std::rc::Rc<dyn gpui::PlatformDisplay>],
    used_displays: &mut std::collections::HashSet<gpui::DisplayId>,
    focus: bool,
) -> bool {
    // Wayland 多屏：仅副屏绑定 display_id（须挂到对应 wl_output 才能显示）。
    // 主屏勿设 display_id，否则 GPUI 会 set_fullscreen 导致鼠标/框选失效（见 wayland/window.rs）。
    // 主屏用 Windowed + GPUI 逻辑分辨率，兼容 2x 分数缩放。
    let (window_bounds, display_id) = if wayland && multi && !spec.is_primary {
        let monitor = MonitorRect {
            name: spec.name.clone(),
            x: spec.x as i32,
            y: spec.y as i32,
            width: spec.width as u32,
            height: spec.height as u32,
            is_primary: false,
        };
        let display_id = VirtualDesktop::match_display_for_monitor(
            displays,
            &monitor,
            used_displays,
        );
        if let Some(id) = display_id {
            used_displays.insert(id);
            debug_log(&format!(
                "overlay: monitor {} ({}) -> display_id {:?}",
                spec.index, spec.name, id
            ));
        } else {
            debug_log(&format!(
                "overlay: monitor {} ({}) 未匹配 display_id",
                spec.index, spec.name
            ));
        }
        (
            Bounds::new(
                point(px(0.0), px(0.0)),
                size(px(spec.width), px(spec.height)),
            ),
            display_id,
        )
    } else if wayland && multi && spec.is_primary {
        let monitor = MonitorRect {
            name: spec.name.clone(),
            x: spec.x as i32,
            y: spec.y as i32,
            width: spec.width as u32,
            height: spec.height as u32,
            is_primary: true,
        };
        let logical = VirtualDesktop::match_display_for_monitor(
            displays,
            &monitor,
            &std::collections::HashSet::new(),
        )
        .and_then(|id| displays.iter().find(|d| d.id() == id).map(|d| d.bounds()));
        let bounds = logical.unwrap_or_else(|| {
            Bounds::new(
                point(px(spec.x), px(spec.y)),
                size(px(spec.width), px(spec.height)),
            )
        });
        debug_log(&format!(
            "overlay: primary {} ({}) windowed {:?}x{:?}",
            spec.index,
            spec.name,
            bounds.size.width,
            bounds.size.height
        ));
        (bounds, None)
    } else {
        (
            Bounds::new(
                point(px(spec.x), px(spec.y)),
                size(px(spec.width), px(spec.height)),
            ),
            None,
        )
    };

    let monitor_offset = point(px(spec.x), px(spec.y));
    let entity_ref = entity.clone();

    let bounds_kind = if wayland && multi && spec.is_primary {
        WindowBounds::Windowed(window_bounds)
    } else if wayland {
        WindowBounds::Fullscreen(window_bounds)
    } else {
        WindowBounds::Windowed(window_bounds)
    };

    cx.open_window(
        WindowOptions {
            titlebar: None,
            window_bounds: Some(bounds_kind),
            window_decorations: None,
            window_background: WindowBackgroundAppearance::Opaque,
            // PopUp 在 X11 下会被标记为 _NET_WM_WINDOW_TYPE_NOTIFICATION，
            // 导致窗口管理器把它显示成角落里的一个小方块而不是全屏覆盖。
            kind: WindowKind::Normal,
            is_movable: false,
            is_resizable: false,
            focus,
            show: true,
            display_id,
            app_id: Some(desktop_app_id().into()),
            ..Default::default()
        },
        move |window, cx| {
            entity_ref.update(cx, |overlay, _cx| {
                overlay.register_window(window.window_handle(), monitor_offset);
                window.focus(&overlay.focus_handle);
            });
            entity_ref
        },
    )
    .map(|_| ())
    .inspect_err(|e| eprintln!("打开截屏层失败 (屏 {}): {e}", spec.index))
    .is_ok()
}

pub fn open_overlay(frame: CaptureFrame, cx: &mut App) {
    close_any_active_overlay(cx);
    let layout = VirtualDesktop::detect();
    if let Some(layout) = &layout {
        if frame.width != layout.width || frame.height != layout.height {
            eprintln!(
                "警告: 截屏尺寸 {}x{} 与虚拟桌面 {}x{} 不一致",
                frame.width,
                frame.height,
                layout.width,
                layout.height
            );
        }
    }

    crate::inline_text::register_keybindings(cx);

    let monitors: Vec<(f32, f32, f32, f32, bool)> = layout
        .as_ref()
        .filter(|l| l.monitors.len() > 1)
        .map(|l| {
            l.monitors
                .iter()
                .map(|m| {
                    (
                        m.x as f32,
                        m.y as f32,
                        m.width as f32,
                        m.height as f32,
                        m.is_primary,
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![(
                frame.origin_x as f32,
                frame.origin_y as f32,
                frame.width as f32,
                frame.height as f32,
                true,
            )]
        });

    let multi = monitors.len() > 1;
    let wayland = is_wayland();
    let displays = if multi && wayland {
        cx.displays()
    } else {
        Vec::new()
    };

    let monitors_for_entity: Vec<(f32, f32, f32, f32)> = monitors
        .iter()
        .map(|(mx, my, mw, mh, _)| (*mx, *my, *mw, *mh))
        .collect();

    let specs: Vec<MonitorOpenSpec> = monitors
        .iter()
        .enumerate()
        .map(|(i, (mx, my, mw, mh, is_primary))| {
            let name = layout
                .as_ref()
                .and_then(|l| l.monitors.get(i))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| format!("monitor-{i}"));
            MonitorOpenSpec {
                index: i,
                name,
                x: *mx,
                y: *my,
                width: *mw,
                height: *mh,
                is_primary: *is_primary,
            }
        })
        .collect();

    let entity = cx.new(|cx| {
        let mut overlay = OverlayCore::new(frame, cx);
        overlay.multi_window = multi;
        overlay.monitors = monitors_for_entity;
        overlay.expected_windows = specs.len().max(1);
        overlay
    });

    cx.set_global(OverlaySession {
        active: Some(entity.downgrade()),
    });
    cx.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = false);
    cx.update_global::<CaptureBusy, _>(|state, _| *state = CaptureBusy::default());

    let mut used_displays = std::collections::HashSet::new();
    let primaries: Vec<_> = specs
        .iter()
        .filter(|s| !wayland || !multi || s.is_primary)
        .cloned()
        .collect();
    let secondaries: Vec<_> = specs
        .iter()
        .filter(|s| wayland && multi && !s.is_primary)
        .cloned()
        .collect();

    for (i, spec) in primaries.iter().enumerate() {
        open_monitor_window(
            cx,
            &entity,
            spec,
            wayland,
            multi,
            &displays,
            &mut used_displays,
            i == 0,
        );
    }

    if secondaries.is_empty() {
        let _ = entity.update(cx, |overlay, cx| overlay.mark_windows_ready(cx));
        cx.activate(true);
        return;
    }

    // 副屏晚一帧再开，避免与主屏 configure 竞态导致主屏鼠标事件失效
    let entity_deferred = entity.clone();
    cx.defer(move |cx| {
        let displays = cx.displays();
        let mut used_displays = std::collections::HashSet::new();
        for spec in &secondaries {
            open_monitor_window(
                cx,
                &entity_deferred,
                spec,
                true,
                true,
                &displays,
                &mut used_displays,
                false,
            );
        }
        let _ = entity_deferred.update(cx, |overlay, cx| overlay.mark_windows_ready(cx));
        cx.activate(true);
    });
}

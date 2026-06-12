use gpui::{App, Global};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Zh,
    En,
}

impl Language {
    pub fn index(self) -> usize {
        match self {
            Self::Zh => 0,
            Self::En => 1,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::En,
            _ => Self::Zh,
        }
    }

    pub fn detect() -> Self {
        if let Ok(lang) = std::env::var("LANG") {
            let lower = lang.to_lowercase();
            if lower.starts_with("en") {
                return Self::En;
            }
        }
        Self::Zh
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("screenshot4ubuntu")
            .join("language")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            match content.trim() {
                "en" => return Self::En,
                "zh" => return Self::Zh,
                _ => {}
            }
        }
        Self::detect()
    }

    pub fn save(self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let code = match self {
            Self::Zh => "zh",
            Self::En => "en",
        };
        let _ = std::fs::write(path, format!("{code}\n"));
    }
}

pub struct LocaleSettings {
    pub language: Language,
}

impl Global for LocaleSettings {}

impl LocaleSettings {
    pub fn load() -> Self {
        Self {
            language: Language::load(),
        }
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(LocaleSettings::load());
}

pub fn language(cx: &App) -> Language {
    cx.global::<LocaleSettings>().language
}

pub fn set_language(cx: &mut App, lang: Language) {
    cx.global_mut::<LocaleSettings>().language = lang;
    lang.save();
    crate::tray::refresh_tray_language(lang);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKey {
    TrayCapture,
    TrayQuit,
    TrayLanguage,
    LangZh,
    LangEn,
    TrayStartFailed,

    StatusSelectDrag,
    StatusEditingResize,
    StatusToolSelect,
    StatusToolText,
    ErrNoSelection,

    ToolSelect,
    ToolBrush,
    ToolLine,
    ToolRect,
    ToolEllipse,
    ToolText,
    LabelStrokeWidth,
    LabelFontSize,
    LabelColor,
    BtnDone,
    BtnCancel,

    PreviewTitle,
    PreviewReady,
    PreviewCopyImage,
    PreviewSave,
    PreviewClose,
    PreviewCopied,
    PreviewCopyFailed,
    PreviewSaveCancelled,
    PreviewSaved,
    PreviewSaveFailed,
    PreviewDialogFailed,
    ZenitySaveTitle,
    ZenityPngFilter,
}

pub fn tr(lang: Language, key: MessageKey) -> &'static str {
    match (lang, key) {
        (Language::Zh, MessageKey::TrayCapture) => "截屏",
        (Language::Zh, MessageKey::TrayQuit) => "退出",
        (Language::Zh, MessageKey::TrayLanguage) => "语言",
        (Language::Zh, MessageKey::LangZh) => "简体中文",
        (Language::Zh, MessageKey::LangEn) => "English",
        (Language::Zh, MessageKey::TrayStartFailed) => {
            "系统托盘启动失败: {error}（请确认已安装 AppIndicator 支持）"
        }

        (Language::Zh, MessageKey::StatusSelectDrag) => "拖拽鼠标框选区域",
        (Language::Zh, MessageKey::StatusEditingResize) => "在选区内标注，完成后点复制或保存",
        (Language::Zh, MessageKey::StatusToolSelect) => "拖拽框选区域",
        (Language::Zh, MessageKey::StatusToolText) => "点击选区内输入文字，点别处确认",
        (Language::Zh, MessageKey::ErrNoSelection) => "无选区",

        (Language::Zh, MessageKey::ToolSelect) => "框选",
        (Language::Zh, MessageKey::ToolBrush) => "画笔",
        (Language::Zh, MessageKey::ToolLine) => "直线",
        (Language::Zh, MessageKey::ToolRect) => "矩形",
        (Language::Zh, MessageKey::ToolEllipse) => "椭圆",
        (Language::Zh, MessageKey::ToolText) => "文字",
        (Language::Zh, MessageKey::LabelStrokeWidth) => "粗细",
        (Language::Zh, MessageKey::LabelFontSize) => "字号",
        (Language::Zh, MessageKey::LabelColor) => "颜色",
        (Language::Zh, MessageKey::BtnDone) => "完成",
        (Language::Zh, MessageKey::BtnCancel) => "取消",

        (Language::Zh, MessageKey::PreviewTitle) => "截图预览",
        (Language::Zh, MessageKey::PreviewReady) => "截图已完成，可在此复制或保存",
        (Language::Zh, MessageKey::PreviewCopyImage) => "复制图片",
        (Language::Zh, MessageKey::PreviewSave) => "保存",
        (Language::Zh, MessageKey::PreviewClose) => "关闭",
        (Language::Zh, MessageKey::PreviewCopied) => "已复制到剪贴板",
        (Language::Zh, MessageKey::PreviewCopyFailed) => "复制失败: {error}",
        (Language::Zh, MessageKey::PreviewSaveCancelled) => "保存已取消",
        (Language::Zh, MessageKey::PreviewSaved) => "已保存: {path}",
        (Language::Zh, MessageKey::PreviewSaveFailed) => "保存失败: {error}",
        (Language::Zh, MessageKey::PreviewDialogFailed) => "无法打开文件选择对话框: {error}",
        (Language::Zh, MessageKey::ZenitySaveTitle) => "保存截图",
        (Language::Zh, MessageKey::ZenityPngFilter) => "PNG 图片 | *.png",

        (Language::En, MessageKey::TrayCapture) => "Screenshot",
        (Language::En, MessageKey::TrayQuit) => "Quit",
        (Language::En, MessageKey::TrayLanguage) => "Language",
        (Language::En, MessageKey::LangZh) => "简体中文",
        (Language::En, MessageKey::LangEn) => "English",
        (Language::En, MessageKey::TrayStartFailed) => {
            "Failed to start system tray: {error} (ensure AppIndicator support is installed)"
        }

        (Language::En, MessageKey::StatusSelectDrag) => "Drag to select a region",
        (Language::En, MessageKey::StatusEditingResize) => {
            "Annotate inside the selection, then copy or save"
        }
        (Language::En, MessageKey::StatusToolSelect) => "Drag to select a region",
        (Language::En, MessageKey::StatusToolText) => {
            "Click inside the selection to type, click elsewhere to confirm"
        }
        (Language::En, MessageKey::ErrNoSelection) => "No selection",

        (Language::En, MessageKey::ToolSelect) => "Select",
        (Language::En, MessageKey::ToolBrush) => "Brush",
        (Language::En, MessageKey::ToolLine) => "Line",
        (Language::En, MessageKey::ToolRect) => "Rectangle",
        (Language::En, MessageKey::ToolEllipse) => "Ellipse",
        (Language::En, MessageKey::ToolText) => "Text",
        (Language::En, MessageKey::LabelStrokeWidth) => "Width",
        (Language::En, MessageKey::LabelFontSize) => "Size",
        (Language::En, MessageKey::LabelColor) => "Color",
        (Language::En, MessageKey::BtnDone) => "Done",
        (Language::En, MessageKey::BtnCancel) => "Cancel",

        (Language::En, MessageKey::PreviewTitle) => "Screenshot Preview",
        (Language::En, MessageKey::PreviewReady) => "Capture complete — copy or save here",
        (Language::En, MessageKey::PreviewCopyImage) => "Copy Image",
        (Language::En, MessageKey::PreviewSave) => "Save",
        (Language::En, MessageKey::PreviewClose) => "Close",
        (Language::En, MessageKey::PreviewCopied) => "Copied to clipboard",
        (Language::En, MessageKey::PreviewCopyFailed) => "Copy failed: {error}",
        (Language::En, MessageKey::PreviewSaveCancelled) => "Save cancelled",
        (Language::En, MessageKey::PreviewSaved) => "Saved: {path}",
        (Language::En, MessageKey::PreviewSaveFailed) => "Save failed: {error}",
        (Language::En, MessageKey::PreviewDialogFailed) => "Could not open file dialog: {error}",
        (Language::En, MessageKey::ZenitySaveTitle) => "Save Screenshot",
        (Language::En, MessageKey::ZenityPngFilter) => "PNG Image | *.png",
    }
}

pub fn tr_app(cx: &App, key: MessageKey) -> &'static str {
    tr(language(cx), key)
}

pub fn format_one(_lang: Language, template: &str, placeholder: &str, value: &str) -> String {
    template.replace(placeholder, value)
}

pub fn overlay_status(lang: Language, phase: crate::model::Phase, tool: crate::model::Tool) -> String {
    use crate::model::{Phase, Tool};
    match phase {
        Phase::Selecting => tr(lang, MessageKey::StatusSelectDrag).into(),
        Phase::Editing if tool == Tool::Text => tr(lang, MessageKey::StatusToolText).into(),
        Phase::Editing => tr(lang, MessageKey::StatusEditingResize).into(),
    }
}

pub fn tool_label(lang: Language, tool: crate::model::Tool) -> &'static str {
    use crate::model::Tool;
    match tool {
        Tool::Select => tr(lang, MessageKey::ToolSelect),
        Tool::Brush => tr(lang, MessageKey::ToolBrush),
        Tool::Line => tr(lang, MessageKey::ToolLine),
        Tool::Rect => tr(lang, MessageKey::ToolRect),
        Tool::Ellipse => tr(lang, MessageKey::ToolEllipse),
        Tool::Text => tr(lang, MessageKey::ToolText),
    }
}

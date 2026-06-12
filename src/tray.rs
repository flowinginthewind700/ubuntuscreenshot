use crate::capture_flow::start_capture;
use crate::i18n::{self, Language, MessageKey, tr};
use gpui::{App, Timer};
use ksni::blocking::TrayMethods;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

static TRAY_TX: OnceLock<Sender<TrayCommand>> = OnceLock::new();
static TRAY_HANDLE: OnceLock<ksni::blocking::Handle<ScreenshotTray>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub(crate) enum TrayCommand {
    Capture,
    SetLanguage(Language),
    Quit,
}

struct ScreenshotTray {
    language: Language,
}

impl ScreenshotTray {
    fn new() -> Self {
        Self {
            language: Language::load(),
        }
    }
}

impl ksni::Tray for ScreenshotTray {
    fn id(&self) -> String {
        "screenshot4ubuntu".into()
    }

    fn title(&self) -> String {
        "Screenshot4Ubuntu".into()
    }

    fn icon_name(&self) -> String {
        "camera-photo".into()
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let lang = self.language;
        let msgs = |key| tr(lang, key).to_string();

        vec![
            StandardItem {
                label: msgs(MessageKey::TrayCapture),
                icon_name: "camera-photo".into(),
                activate: Box::new(|_| send(TrayCommand::Capture)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            SubMenu {
                label: msgs(MessageKey::TrayLanguage),
                icon_name: "preferences-desktop-locale".into(),
                submenu: vec![
                    RadioGroup {
                        selected: lang.index(),
                        select: Box::new(|tray: &mut ScreenshotTray, index| {
                            let new_lang = Language::from_index(index);
                            tray.language = new_lang;
                            send(TrayCommand::SetLanguage(new_lang));
                        }),
                        options: vec![
                            RadioItem {
                                label: tr(lang, MessageKey::LangZh).into(),
                                ..Default::default()
                            },
                            RadioItem {
                                label: tr(lang, MessageKey::LangEn).into(),
                                ..Default::default()
                            },
                        ],
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: msgs(MessageKey::TrayQuit),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| send(TrayCommand::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn send(cmd: TrayCommand) {
    if let Some(tx) = TRAY_TX.get() {
        let _ = tx.send(cmd);
    }
}

pub fn refresh_tray_language(lang: Language) {
    if let Some(handle) = TRAY_HANDLE.get() {
        handle.update(|tray| {
            tray.language = lang;
        });
    }
}

pub fn spawn_tray() -> Receiver<TrayCommand> {
    let (tx, rx) = channel();
    TRAY_TX.set(tx).ok();

    std::thread::spawn(|| {
        let tray = ScreenshotTray::new();
        let lang = Language::load();
        match tray.spawn() {
            Ok(handle) => {
                TRAY_HANDLE.set(handle.clone()).ok();
                let _ = handle.update(|t| t.language = lang);
                while !handle.is_closed() {
                    std::thread::sleep(Duration::from_secs(60));
                }
            }
            Err(e) => {
                let template = tr(lang, MessageKey::TrayStartFailed);
                eprintln!(
                    "{}",
                    i18n::format_one(lang, template, "{error}", &e.to_string())
                );
            }
        }
    });

    rx
}

pub fn attach_tray_to_app(cx: &mut App, rx: Receiver<TrayCommand>) {
    cx.spawn(async move |cx| {
        loop {
            Timer::after(Duration::from_millis(150)).await;
            while let Ok(cmd) = rx.try_recv() {
                let _ = cx.update(|app| match cmd {
                    TrayCommand::Capture => start_capture(app),
                    TrayCommand::SetLanguage(lang) => i18n::set_language(app, lang),
                    TrayCommand::Quit => app.quit(),
                });
            }
        }
    })
    .detach();
}

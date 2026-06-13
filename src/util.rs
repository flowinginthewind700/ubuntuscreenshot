use gpui::Pixels;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub fn px_val(p: Pixels) -> f32 {
    f32::from(p)
}

/// 与 `ubuntuscreenshot.desktop` 一致，供 Wayland portal 识别已安装应用。
pub fn desktop_app_id() -> &'static str {
    "ubuntuscreenshot"
}

fn log_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ubuntuscreenshot")
        .join("capture.log")
}

static LOG_LOCK: Mutex<()> = Mutex::new(());

/// 记录与截屏相关的环境变量，用于诊断 portal/x11 路径选择问题。
pub fn log_capture_environment() {
    if let Ok(exe) = std::env::current_exe() {
        debug_log(&format!("exe={}", exe.display()));
    }
    for key in [
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_SESSION_TYPE",
        "XDG_CURRENT_DESKTOP",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        let value = std::env::var(key).unwrap_or_else(|_| "<unset>".to_string());
        debug_log(&format!("env {key}={value}"));
    }
}

/// 写入截屏诊断日志（~/.cache/ubuntuscreenshot/capture.log）。
pub fn debug_log(message: &str) {
    let _guard = LOG_LOCK.lock().ok();
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{ts}] {message}");
    }
}

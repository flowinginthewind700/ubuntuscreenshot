use crate::capture::capture_primary_screen;
use crate::overlay::{close_any_active_overlay, open_overlay, CaptureInProgress};
use gpui::{App, BorrowAppContext};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn start_capture(cx: &mut App) {
    if cx
        .try_global::<CaptureInProgress>()
        .is_some_and(|busy| busy.0)
    {
        return;
    }
    cx.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = true);
    close_any_active_overlay(cx);

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(120));
        let result = capture_primary_screen();
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        loop {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(frame) => {
                        let _ = cx.update(|app| {
                            open_overlay(frame, app);
                        });
                    }
                    Err(e) => {
                        eprintln!("截屏失败: {e}");
                        let _ = cx.update(|app| {
                            app.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = false);
                        });
                    }
                }
                break;
            }
            gpui::Timer::after(Duration::from_millis(50)).await;
        }
    })
    .detach();
}

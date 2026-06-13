use crate::capture::capture_primary_screen;
use crate::capture_gate::{close_capture_gate, open_capture_gate};
use crate::overlay::{
    close_any_active_overlay, open_overlay, CaptureBusy, CaptureInProgress,
};
use crate::util::debug_log;
use gpui::{App, BorrowAppContext, Timer};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CAPTURE_WAIT: Duration = Duration::from_secs(45);
const STALE_BUSY: Duration = Duration::from_secs(15);
const FOCUS_WAIT: Duration = Duration::from_millis(1500);

fn clear_stale_busy(cx: &mut App) {
    let stale = cx
        .try_global::<CaptureInProgress>()
        .filter(|busy| busy.0)
        .and_then(|_| cx.try_global::<CaptureBusy>())
        .and_then(|state| state.since)
        .is_some_and(|since| since.elapsed() > STALE_BUSY);
    if stale {
        debug_log("clear_stale_busy: resetting stuck CaptureInProgress");
        cx.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = false);
        cx.update_global::<CaptureBusy, _>(|state, _| *state = CaptureBusy::default());
    }
}

fn set_busy(cx: &mut App, active: bool) {
    cx.update_global::<CaptureInProgress, _>(|busy, _| busy.0 = active);
    cx.update_global::<CaptureBusy, _>(|state, _| {
        state.active = active;
        state.since = active.then(Instant::now);
    });
}

fn finish_capture(cx: &mut App, result: Option<anyhow::Result<crate::model::CaptureFrame>>) {
    close_capture_gate(cx);
    match result {
        Some(Ok(frame)) => {
            debug_log(&format!(
                "finish_capture: opening overlay {}x{}",
                frame.width, frame.height
            ));
            open_overlay(frame, cx);
            cx.activate(true);
        }
        Some(Err(e)) => {
            debug_log(&format!("finish_capture: capture failed: {e:#}"));
            eprintln!("截屏失败: {e}");
        }
        None => {
            debug_log(&format!("finish_capture: timed out after {CAPTURE_WAIT:?}"));
            eprintln!("截屏超时（{CAPTURE_WAIT:?}），请检查 xdg-desktop-portal-gnome 是否运行");
        }
    }
    set_busy(cx, false);
}

fn spawn_capture_work(cx: &mut App) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        debug_log("capture thread: calling capture_primary_screen (pipewire)");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            capture_primary_screen,
        ))
        .map_err(|e| {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                (*s).to_string()
            } else {
                "unknown panic".to_string()
            };
            anyhow::anyhow!("capture panicked: {msg}")
        })
        .and_then(|r| r);
        debug_log(&format!(
            "capture thread: done ({})",
            result
                .as_ref()
                .map(|f| format!("{}x{}", f.width, f.height))
                .unwrap_or_else(|e| format!("err: {e:#}"))
        ));
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        let deadline = Instant::now() + CAPTURE_WAIT;
        let mut result = None;
        while Instant::now() < deadline {
            if let Ok(frame) = rx.try_recv() {
                result = Some(frame);
                break;
            }
            Timer::after(Duration::from_millis(50)).await;
        }

        debug_log("async waiter: scheduling finish_capture on main thread");
        if cx
            .update(|app| {
                app.defer(move |app| finish_capture(app, result));
            })
            .is_err()
        {
            debug_log("async waiter: cx.update failed, will auto-reset busy after stale timeout");
        }
    })
    .detach();
}

pub fn start_capture(cx: &mut App) {
    clear_stale_busy(cx);

    if cx
        .try_global::<CaptureInProgress>()
        .is_some_and(|busy| busy.0)
    {
        debug_log("start_capture: skipped, capture already in progress");
        return;
    }

    debug_log("start_capture: begin (pipewire frame capture)");
    crate::util::log_capture_environment();
    set_busy(cx, true);
    close_any_active_overlay(cx);

    let gate_rx = open_capture_gate(cx);

    cx.spawn(async move |cx| {
        let focus_deadline = Instant::now() + FOCUS_WAIT;
        while Instant::now() < focus_deadline {
            if gate_rx.try_recv().is_ok() {
                debug_log("capture_gate: window focused");
                break;
            }
            Timer::after(Duration::from_millis(40)).await;
        }

        let _ = cx.update(|app| {
            app.defer(spawn_capture_work);
        });
    })
    .detach();
}

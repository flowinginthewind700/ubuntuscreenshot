use crate::util::debug_log;
use std::collections::HashMap;
use std::fs;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedFd as PortalFd, OwnedValue};

const PORTAL_DESKTOP: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SCREENCAST_IFACE: &str = "org.freedesktop.portal.ScreenCast";
const RESTORE_TOKEN_PATH: &str = "screencast_restore_token";

fn restore_token_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ubuntuscreenshot")
        .join(RESTORE_TOKEN_PATH)
}

fn load_restore_token() -> Option<String> {
    fs::read_to_string(restore_token_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_restore_token(token: &str) {
    let path = restore_token_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, token);
}

fn maybe_save_restore_token(results: &HashMap<String, OwnedValue>) {
    if let Some(value) = results.get("restore_token") {
        if let Ok(token) = value.clone().try_into() as Result<String, _> {
            if !token.is_empty() {
                debug_log("pipewire capture: saved restore_token");
                save_restore_token(&token);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScreencastStream {
    pub node_id: u32,
    pub position: (i32, i32),
    pub size: (u32, u32),
}

pub fn is_permission_denied(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("错误码: 2")
        || msg.contains("error code 2")
        || msg.contains("AccessDenied")
        || msg.contains("未授权")
        || msg.contains("取消了截屏授权")
        || msg.contains("Only the focused app")
}

pub fn capture_via_screencast() -> anyhow::Result<(OwnedFd, Vec<ScreencastStream>)> {
    let conn = Connection::session()?;
    let unique = dbus_unique_suffix(&conn)?;
    let pid = std::process::id();

    let session_handle = create_session(&conn, &unique, pid)?;
    select_sources(&conn, &unique, pid, &session_handle)?;
    let streams = start_session(&conn, &unique, pid, &session_handle)?;
    let fd = open_pipewire_remote(&conn, &session_handle)?;
    let _ = close_session(&conn, &session_handle);
    Ok((fd, streams))
}

fn create_session(conn: &Connection, unique: &str, pid: u32) -> anyhow::Result<String> {
    let handle_token = format!("s4u_sc_{pid}");
    let request_path = format!("/org/freedesktop/portal/desktop/request/{unique}/{handle_token}");

    let proxy = zbus::blocking::Proxy::new(conn, PORTAL_DESKTOP, PORTAL_PATH, SCREENCAST_IFACE)?;

    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::from(handle_token.as_str()));
    options.insert(
        "session_handle_token",
        zbus::zvariant::Value::from("ubuntuscreenshot"),
    );
    options.insert("persist_mode", zbus::zvariant::Value::from(2u32));

    proxy.call_method("CreateSession", &(options,))?;
    debug_log(&format!("screencast CreateSession: {request_path}"));

    let (code, results) = wait_portal_response(conn, &request_path, Duration::from_secs(120))?;
    match code {
        0 => {}
        1 => anyhow::bail!("用户取消了截屏授权"),
        c => anyhow::bail!("ScreenCast CreateSession 失败，错误码: {c}"),
    }

    string_result(&results, "session_handle")
}

fn select_sources(
    conn: &Connection,
    unique: &str,
    pid: u32,
    session_handle: &str,
) -> anyhow::Result<()> {
    let handle_token = format!("s4u_ss_{pid}");
    let request_path = format!("/org/freedesktop/portal/desktop/request/{unique}/{handle_token}");

    let proxy = zbus::blocking::Proxy::new(conn, PORTAL_DESKTOP, PORTAL_PATH, SCREENCAST_IFACE)?;

    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::from(handle_token.as_str()));
    options.insert("types", zbus::zvariant::Value::from(1u32)); // Monitor
    options.insert("multiple", zbus::zvariant::Value::from(true));
    options.insert("cursor_mode", zbus::zvariant::Value::from(2u32)); // Metadata
    let restore_token = load_restore_token();
    if let Some(ref token) = restore_token {
        options.insert("restore_token", zbus::zvariant::Value::from(token.as_str()));
        debug_log("pipewire capture: using restore_token");
    }

    proxy.call_method("SelectSources", &(session_path(session_handle)?, options))?;
    debug_log(&format!("screencast SelectSources: {request_path}"));

    let (code, results) = wait_portal_response(conn, &request_path, Duration::from_secs(120))?;
    match code {
        0 => {
            debug_log(&format!("pipewire capture SelectSources ok: {results:?}"));
            maybe_save_restore_token(&results);
            Ok(())
        }
        1 => anyhow::bail!("用户取消了截屏授权"),
        c => anyhow::bail!("ScreenCast SelectSources 失败，错误码: {c}"),
    }
}

fn start_session(
    conn: &Connection,
    unique: &str,
    pid: u32,
    session_handle: &str,
) -> anyhow::Result<Vec<ScreencastStream>> {
    let handle_token = format!("s4u_st_{pid}");
    let request_path = format!("/org/freedesktop/portal/desktop/request/{unique}/{handle_token}");

    let proxy = zbus::blocking::Proxy::new(conn, PORTAL_DESKTOP, PORTAL_PATH, SCREENCAST_IFACE)?;

    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::from(handle_token.as_str()));

    proxy.call_method("Start", &(session_path(session_handle)?, "", options))?;
    debug_log(&format!("screencast Start: {request_path}"));

    let (code, results) = wait_portal_response(conn, &request_path, Duration::from_secs(30))?;
    match code {
        0 => {
            maybe_save_restore_token(&results);
            parse_streams(&results)
        }
        1 => anyhow::bail!("用户取消了截屏授权"),
        c => anyhow::bail!("ScreenCast Start 失败，错误码: {c}"),
    }
}

fn open_pipewire_remote(conn: &Connection, session_handle: &str) -> anyhow::Result<OwnedFd> {
    let proxy = zbus::blocking::Proxy::new(conn, PORTAL_DESKTOP, PORTAL_PATH, SCREENCAST_IFACE)?;
    let reply = proxy.call_method(
        "OpenPipeWireRemote",
        &(session_path(session_handle)?, HashMap::<&str, zbus::zvariant::Value>::new()),
    )?;
    let body = reply.body();
    let portal_fd: PortalFd = body.deserialize()?;
    let fd: OwnedFd = portal_fd.into();
    debug_log("screencast OpenPipeWireRemote ok");
    Ok(fd)
}

fn close_session(conn: &Connection, session_handle: &str) -> anyhow::Result<()> {
    let proxy = zbus::blocking::Proxy::new(conn, PORTAL_DESKTOP, PORTAL_PATH, SCREENCAST_IFACE)?;
    proxy.call_method("Close", &(session_path(session_handle)?,))?;
    debug_log("screencast session closed");
    Ok(())
}

fn parse_streams(results: &HashMap<String, OwnedValue>) -> anyhow::Result<Vec<ScreencastStream>> {
    let streams_value = results
        .get("streams")
        .ok_or_else(|| anyhow::anyhow!("ScreenCast Start 缺少 streams: {results:?}"))?;

    let streams: Vec<(u32, HashMap<String, OwnedValue>)> = streams_value
        .clone()
        .try_into()
        .map_err(|e| anyhow::anyhow!("streams 解析失败: {e}"))?;

    let mut out = Vec::with_capacity(streams.len());
    for (node_id, props) in streams {
        let position = props
            .get("position")
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or((0i32, 0i32));
        let size = props
            .get("size")
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or((0i32, 0i32));
        let (w, h) = size;
        if w <= 0 || h <= 0 {
            continue;
        }
        out.push(ScreencastStream {
            node_id,
            position,
            size: (w as u32, h as u32),
        });
        debug_log(&format!(
            "screencast stream node={node_id} pos=({},{}) size={}x{}",
            position.0, position.1, w, h
        ));
    }

    if out.is_empty() {
        anyhow::bail!("ScreenCast 未返回有效视频流");
    }
    Ok(out)
}

fn wait_portal_response(
    conn: &Connection,
    request_path: &str,
    timeout: Duration,
) -> anyhow::Result<(u32, HashMap<String, OwnedValue>)> {
    let rule = format!(
        "type='signal',interface='org.freedesktop.portal.Request',member='Response',path='{request_path}'"
    );
    let rule = zbus::MatchRule::try_from(rule.as_str())?;
    let mut iter = zbus::blocking::MessageIterator::for_match_rule(rule, conn, None)?;

    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            anyhow::bail!("portal 授权对话框超时");
        }
        let Some(msg) = iter.next() else {
            anyhow::bail!("portal 未返回 Response 信号");
        };
        let msg = msg?;
        let body = msg.body();
        let (code, results): (u32, HashMap<String, OwnedValue>) = body.deserialize()?;
        return Ok((code, results));
    }
}

fn string_result(results: &HashMap<String, OwnedValue>, key: &str) -> anyhow::Result<String> {
    let value = results
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("portal results 缺少 {key}"))?;
    value
        .clone()
        .try_into()
        .map_err(|e| anyhow::anyhow!("portal {key} 解析失败: {e}"))
}

fn session_path<'a>(session_handle: &'a str) -> anyhow::Result<ObjectPath<'a>> {
    ObjectPath::try_from(session_handle).map_err(|e| anyhow::anyhow!("无效 session_handle: {e}"))
}

fn dbus_unique_suffix(conn: &Connection) -> anyhow::Result<String> {
    Ok(conn
        .unique_name()
        .ok_or_else(|| anyhow::anyhow!("无法获取 DBus unique name"))?
        .trim_start_matches(':')
        .replace('.', "_"))
}

pub fn open_applications_settings() -> anyhow::Result<()> {
    if Command::new("gnome-control-center")
        .arg("applications")
        .spawn()
        .is_ok()
    {
        debug_log("opened applications settings");
        return Ok(());
    }
    Ok(())
}

pub fn permission_hint() -> &'static str {
    "需要截屏权限：请在系统弹窗中选择「允许」，或在「设置 → 应用程序 → Ubuntu 截屏 → 截屏」中开启后重试"
}

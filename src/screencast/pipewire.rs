use crate::util::debug_log;
use pipewire as pw;
use pw::properties::properties;
use pw::spa;
use pw::stream::{StreamBox, StreamFlags};
use pw::{context::ContextRc, main_loop::MainLoopRc};
use spa::buffer::DataType;
use spa::param::video::{VideoFormat, VideoInfoRaw};
use spa::param::ParamType;
use spa::pod::serialize::PodSerializer;
use spa::pod::{Object, Pod, Property, PropertyFlags, Value};
use spa::utils::Direction;
use std::cell::RefCell;
use std::io::Cursor;
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct StreamFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

struct CaptureState {
    format: VideoInfoRaw,
    deadline: Instant,
}

pub fn capture_streams(
    fd: OwnedFd,
    streams: &[(u32, (i32, i32), (u32, u32))],
    timeout: Duration,
) -> anyhow::Result<Vec<StreamFrame>> {
    pw::init();
    streams
        .iter()
        .map(|&(node_id, _, (width, height))| capture_one_stream(fd.try_clone()?, node_id, width, height, timeout))
        .collect()
}

fn capture_one_stream(
    fd: OwnedFd,
    node_id: u32,
    width: u32,
    height: u32,
    timeout: Duration,
) -> anyhow::Result<StreamFrame> {
    let (tx, rx) = mpsc::sync_channel::<StreamFrame>(1);
    let done = Arc::new(AtomicBool::new(false));
    let deadline = Instant::now() + timeout;

    let mainloop = MainLoopRc::new(None).map_err(|e| anyhow::anyhow!("PipeWire MainLoop: {e}"))?;

    let context = ContextRc::new(&mainloop, None).map_err(|e| anyhow::anyhow!("PipeWire Context: {e}"))?;
    let core = context
        .connect_fd_rc(fd, None)
        .map_err(|e| anyhow::anyhow!("PipeWire connect_fd: {e}"))?;

    let stream = StreamBox::new(
        &core,
        "ubuntuscreenshot-capture",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|e| anyhow::anyhow!("PipeWire Stream: {e}"))?;

    let state = RefCell::new(CaptureState {
        format: VideoInfoRaw::default(),
        deadline,
    });
    let done_flag = done.clone();
    let loop_quit = mainloop.clone();

    let _listener = stream
        .add_local_listener_with_user_data(state)
        .param_changed(|stream, state, id, param| {
            let Some(param) = param else { return };
            if id != ParamType::Format.as_raw() {
                return;
            }
            let mut info = VideoInfoRaw::default();
            if info.parse(param).is_err() {
                return;
            }
            state.borrow_mut().format = info;
            if let Ok(bytes) = obj_to_bytes(buffer_params_object()) {
                if let Some(pod) = Pod::from_bytes(&bytes) {
                    let _ = stream.update_params(&mut [pod]);
                }
            }
        })
        .process({
            let tx = tx.clone();
            move |stream, state| {
                if done_flag.load(Ordering::Relaxed) || Instant::now() > state.borrow().deadline {
                    loop_quit.quit();
                    return;
                }
                let mut latest = None;
                while let Some(buffer) = stream.dequeue_buffer() {
                    latest = Some(buffer);
                }
                let Some(mut buffer) = latest else {
                    return;
                };
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }
                let format = state.borrow().format.format();
                if let Some(frame) =
                    copy_frame_buffer(datas[0].type_(), &datas[0], format, width, height)
                {
                    debug_log(&format!(
                        "pipewire frame node={node_id} {}x{}",
                        frame.width, frame.height
                    ));
                    let _ = tx.send(frame);
                    done_flag.store(true, Ordering::Relaxed);
                    loop_quit.quit();
                }
            }
        })
        .register()
        .map_err(|e| anyhow::anyhow!("PipeWire listener: {e}"))?;

    let format_bytes = obj_to_bytes(format_params_object())?;
    let mut format_pod = Pod::from_bytes(&format_bytes)
        .ok_or_else(|| anyhow::anyhow!("PipeWire format pod 无效"))?;

    stream
        .connect(
            Direction::Input,
            Some(node_id),
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut [format_pod],
        )
        .map_err(|e| anyhow::anyhow!("PipeWire connect stream: {e}"))?;

    mainloop.run();

    rx.try_recv()
        .map_err(|_| anyhow::anyhow!("PipeWire 未在时限内收到帧 (node={node_id})"))
}

fn copy_frame_buffer(
    data_type: DataType,
    data: &spa::buffer::Data,
    format: VideoFormat,
    width: u32,
    height: u32,
) -> Option<StreamFrame> {
    let stride = data.chunk().stride() as usize;
    let size = data.chunk().size() as usize;
    let bytes: Vec<u8> = match data_type {
        DataType::MemPtr => {
            let ptr = data.as_raw().data as *const u8;
            if ptr.is_null() {
                return None;
            }
            unsafe { std::slice::from_raw_parts(ptr, size) }.to_vec()
        }
        DataType::MemFd => {
            let fd = data.as_raw().fd;
            if fd < 0 {
                return None;
            }
            let file = unsafe { std::fs::File::from_raw_fd(fd as i32) };
            unsafe { memmap2::Mmap::map(&file).ok()?.to_vec() }
        }
        _ => return None,
    };

    let row_bytes = (width as usize).saturating_mul(4);
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height as usize {
        let src_off = y.saturating_mul(stride);
        let dst_off = y.saturating_mul(row_bytes);
        if src_off + row_bytes > bytes.len() || dst_off + row_bytes > rgba.len() {
            continue;
        }
        match format {
            VideoFormat::BGRA | VideoFormat::BGRx => {
                for x in 0..width as usize {
                    let si = src_off + x * 4;
                    let di = dst_off + x * 4;
                    rgba[di] = bytes[si + 2];
                    rgba[di + 1] = bytes[si + 1];
                    rgba[di + 2] = bytes[si];
                    rgba[di + 3] = if format == VideoFormat::BGRx {
                        255
                    } else {
                        bytes[si + 3]
                    };
                }
            }
            _ => {
                let copy = row_bytes.min(bytes.len().saturating_sub(src_off));
                rgba[dst_off..dst_off + copy].copy_from_slice(&bytes[src_off..src_off + copy]);
            }
        }
    }

    Some(StreamFrame {
        width,
        height,
        pixels: rgba,
    })
}

fn obj_to_bytes(obj: Object) -> anyhow::Result<Vec<u8>> {
    Ok(PodSerializer::serialize(Cursor::new(Vec::new()), &spa::pod::Value::Object(obj))?
        .0
        .into_inner())
}

fn buffer_params_object() -> Object {
    let data_types = (1 << DataType::MemFd.as_raw()) | (1 << DataType::MemPtr.as_raw());
    spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamBuffers,
        ParamType::Buffers,
        Property {
            key: spa::sys::SPA_PARAM_BUFFERS_dataType,
            flags: PropertyFlags::empty(),
            value: Value::Int(data_types as i32),
        },
    )
}

fn format_params_object() -> Object {
    spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        ParamType::EnumFormat,
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::RGBA,
            VideoFormat::RGBA,
            VideoFormat::BGRA,
            VideoFormat::RGBx,
            VideoFormat::BGRx,
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: 8192,
                height: 8192
            }
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 30, denom: 1 },
            spa::utils::Fraction { num: 1, denom: 1 },
            spa::utils::Fraction {
                num: 120,
                denom: 1
            }
        ),
    )
}

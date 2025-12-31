use std::fs;

use windows::core::{Result, PCWSTR};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;

pub fn check_hardware_authenticity() -> bool {
    unsafe { check_hardware_authenticity_inner() }
}

unsafe fn check_hardware_authenticity_inner() -> bool {
    let _guard = match MfSession::new() {
        Ok(g) => g,
        Err(_) => return false,
    };

    let has_hw_encoder = check_hw_encoders().map(|count| count > 0).unwrap_or(false);
    if !has_hw_encoder {
        return false;
    }

    try_hw_h264_pipeline().unwrap_or(false)
}

struct MfSession {
    com_initialized: bool,
    mf_started: bool,
}

impl MfSession {
    unsafe fn new() -> Result<Self> {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        match MFStartup(MF_VERSION, MFSTARTUP_FULL) {
            Ok(_) => Ok(Self {
                com_initialized: true,
                mf_started: true,
            }),
            Err(e) => {
                CoUninitialize();
                Err(e)
            }
        }
    }
}

impl Drop for MfSession {
    fn drop(&mut self) {
        unsafe {
            if self.mf_started {
                let _ = MFShutdown();
            }
            if self.com_initialized {
                CoUninitialize();
            }
        }
    }
}

unsafe fn check_hw_encoders() -> Result<u32> {
    let mut count: u32 = 0;
    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();

    let output_info = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };

    let res = MFTEnumEx(
        MFT_CATEGORY_VIDEO_ENCODER,
        MFT_ENUM_FLAG_HARDWARE,
        None,
        Some(&output_info),
        &mut activates,
        &mut count,
    );

    if res.is_ok() && !activates.is_null() {
        for i in 0..count as usize {
            std::ptr::drop_in_place(activates.add(i));
        }
        CoTaskMemFree(Some(activates as *const _));
    }

    res.map(|_| count)
}

unsafe fn try_hw_h264_pipeline() -> Result<bool> {
    let mut attributes: Option<IMFAttributes> = None;
    MFCreateAttributes(&mut attributes, 1)?;

    let Some(attributes) = attributes else {
        return Ok(false);
    };

    let _ = attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1);

    let path_to_use = pick_probe_path();
    let _delete_probe = FileDeleteGuard::new(path_to_use.clone());

    let output_path: Vec<u16> = path_to_use.encode_utf16().chain(Some(0)).collect();
    let writer = MFCreateSinkWriterFromURL(PCWSTR(output_path.as_ptr()), None, &attributes)?;

    let output_mt = create_h264_type()?;
    let stream_idx = writer.AddStream(&output_mt)?;

    let input_mt = create_input_type()?;
    let _ = writer.SetInputMediaType(stream_idx, &input_mt, None);

    Ok(writer.BeginWriting().is_ok())
}

fn pick_probe_path() -> String {
    let preferred = "C:\\Windows\\Temp\\hw_probe.mp4".to_string();
    if fs::write(&preferred, b"").is_ok() {
        let _ = fs::remove_file(&preferred);
        return preferred;
    }

    std::env::temp_dir()
        .join("hw_probe.mp4")
        .to_string_lossy()
        .to_string()
}

struct FileDeleteGuard {
    path: String,
}

impl FileDeleteGuard {
    fn new(path: String) -> Self {
        Self { path }
    }
}

impl Drop for FileDeleteGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

unsafe fn create_h264_type() -> Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
    mt.SetUINT32(&MF_MT_AVG_BITRATE, 4_000_000)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280u64) << 32) | 720u64)?;
    mt.SetUINT64(&MF_MT_FRAME_RATE, ((30u64) << 32) | 1u64)?;
    mt.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    Ok(mt)
}

unsafe fn create_input_type() -> Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280u64) << 32) | 720u64)?;
    Ok(mt)
}

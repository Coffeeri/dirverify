use clap::{Parser, ValueEnum};
use dirverify::hashing::HashAlgorithm;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AlgorithmArg {
    Sha256,
    Md5,
    Crc32,
    Blake2,
    #[value(alias = "blake")]
    Blake3,
    Xxh3,
}

impl From<AlgorithmArg> for HashAlgorithm {
    fn from(value: AlgorithmArg) -> Self {
        match value {
            AlgorithmArg::Sha256 => HashAlgorithm::Sha256,
            AlgorithmArg::Md5 => HashAlgorithm::Md5,
            AlgorithmArg::Crc32 => HashAlgorithm::Crc32,
            AlgorithmArg::Blake2 => HashAlgorithm::Blake2,
            AlgorithmArg::Blake3 => HashAlgorithm::Blake3,
            AlgorithmArg::Xxh3 => HashAlgorithm::Xxh3,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Benchmark hashing throughput (in-memory, chunked)")]
struct Args {
    /// Data size to hash (MiB)
    #[arg(long, default_value = "256")]
    size_mib: usize,

    /// Chunk size fed to hashers (KiB)
    #[arg(long, default_value = "64")]
    chunk_kib: usize,

    /// Timed iterations per algorithm (warmup run happens once per algorithm)
    #[arg(long, default_value = "5")]
    iters: usize,

    /// Algorithms to benchmark (default: all)
    #[arg(long, value_enum)]
    alg: Vec<AlgorithmArg>,
}

fn main() {
    let args = Args::parse();
    let algorithms: Vec<AlgorithmArg> = if args.alg.is_empty() {
        vec![
            AlgorithmArg::Xxh3,
            AlgorithmArg::Crc32,
            AlgorithmArg::Blake3,
            AlgorithmArg::Blake2,
            AlgorithmArg::Sha256,
            AlgorithmArg::Md5,
        ]
    } else {
        args.alg.clone()
    };

    let byte_len = args.size_mib * 1024 * 1024;
    let chunk_len = args.chunk_kib * 1024;
    let mut data = vec![0u8; byte_len];
    fill_pseudorandom(&mut data);

    eprintln!(
        "hashbench: size={} MiB, chunk={} KiB, iters={}",
        args.size_mib, args.chunk_kib, args.iters
    );

    for alg in algorithms {
        let alg_name = format!("{alg:?}").to_lowercase();
        let algo: HashAlgorithm = alg.into();

        // Warmup (not timed)
        let warmup_digest = hash_buffer_chunked(&data, chunk_len, algo);
        black_box(warmup_digest);

        let mut total = Duration::from_secs(0);
        let mut best = Duration::MAX;
        let mut last_digest = String::new();

        for _ in 0..args.iters {
            let start = Instant::now();
            last_digest = hash_buffer_chunked(&data, chunk_len, algo);
            let elapsed = start.elapsed();
            total += elapsed;
            if elapsed < best {
                best = elapsed;
            }
            black_box(&last_digest);
        }

        let avg = total / (args.iters as u32);
        let avg_mib_s = (args.size_mib as f64) / avg.as_secs_f64();
        let best_mib_s = (args.size_mib as f64) / best.as_secs_f64();

        println!("{alg_name:>7}  avg={avg_mib_s:8.1} MiB/s  best={best_mib_s:8.1} MiB/s  digest={last_digest}");
    }
}

fn fill_pseudorandom(bytes: &mut [u8]) {
    // Simple xorshift64* generator; cost is paid once (outside timed region).
    let mut x: u64 = 0x243f_6a88_85a3_08d3;
    for chunk in bytes.chunks_mut(8) {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x = x.wrapping_mul(0x2545_f491_4f6c_dd1d);
        let out = x.to_le_bytes();
        let n = chunk.len();
        chunk.copy_from_slice(&out[..n]);
    }
}

fn hash_buffer_chunked(data: &[u8], chunk_len: usize, algorithm: HashAlgorithm) -> String {
    match algorithm {
        HashAlgorithm::Sha256 => {
            use sha2::Digest as _;
            let mut hasher = sha2::Sha256::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                hasher.update(chunk);
            }
            format!("{:x}", hasher.finalize())
        }
        HashAlgorithm::Md5 => {
            let mut context = md5::Context::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                context.consume(chunk);
            }
            format!("{:x}", context.compute())
        }
        HashAlgorithm::Crc32 => {
            let mut hasher = crc32fast::Hasher::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                hasher.update(chunk);
            }
            format!("{:08x}", hasher.finalize())
        }
        HashAlgorithm::Blake2 => {
            use blake2::Digest as _;
            let mut hasher = blake2::Blake2s256::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                hasher.update(chunk);
            }
            format!("{:x}", hasher.finalize())
        }
        HashAlgorithm::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                hasher.update(chunk);
            }
            hasher.finalize().to_hex().to_string()
        }
        HashAlgorithm::Xxh3 => {
            let mut hasher = xxhash_rust::xxh3::Xxh3::new();
            for chunk in data.chunks(chunk_len.max(1)) {
                hasher.update(chunk);
            }
            format!("{:016x}", hasher.digest())
        }
    }
}

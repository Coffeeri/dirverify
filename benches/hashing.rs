use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dirverify::hashing::{hash_file, HashAlgorithm};
use std::hint::black_box;
use std::io::Write;
use std::time::Duration;

fn bench_hashing(c: &mut Criterion) {
    // In-memory throughput benchmark (minimizes filesystem noise).
    let data_len = 256usize * 1024 * 1024; // 256 MiB
    let chunk_len = 64usize * 1024; // 64 KiB (matches src/hashing.rs buffer)
    let mut data = vec![0u8; data_len];
    fill_pseudorandom(&mut data);

    let algorithms = [
        (HashAlgorithm::Xxh3, "xxh3"),
        (HashAlgorithm::Crc32, "crc32"),
        (HashAlgorithm::Blake3, "blake3"),
        (HashAlgorithm::Blake2, "blake2"),
        (HashAlgorithm::Sha256, "sha256"),
        (HashAlgorithm::Md5, "md5"),
    ];

    let mut mem_group = c.benchmark_group("hash_mem_chunked");
    mem_group.throughput(Throughput::Bytes(data_len as u64));
    mem_group.warm_up_time(Duration::from_secs(3));
    mem_group.measurement_time(Duration::from_secs(8));
    mem_group.sample_size(20);

    for (algo, name) in algorithms {
        mem_group.bench_with_input(BenchmarkId::from_parameter(name), &algo, |b, &algo| {
            b.iter(|| {
                let digest = hash_buffer_chunked(black_box(&data), chunk_len, algo);
                black_box(digest);
            })
        });
    }
    mem_group.finish();

    // File hashing benchmark (represents dirverify's actual I/O pattern).
    // Note: results will depend heavily on OS page cache and storage.
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("bench.bin");
    {
        let mut f = std::fs::File::create(&file_path).expect("create temp file");
        f.write_all(&data).expect("write benchmark data");
        f.sync_all().ok();
    }

    let mut file_group = c.benchmark_group("hash_file");
    file_group.throughput(Throughput::Bytes(data_len as u64));
    file_group.warm_up_time(Duration::from_secs(2));
    file_group.measurement_time(Duration::from_secs(6));
    file_group.sample_size(15);

    for (algo, name) in algorithms {
        file_group.bench_with_input(BenchmarkId::from_parameter(name), &algo, |b, &algo| {
            b.iter(|| {
                let digest = hash_file(black_box(&file_path), algo).expect("hash_file");
                black_box(digest);
            })
        });
    }
    file_group.finish();
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

criterion_group!(benches, bench_hashing);
criterion_main!(benches);

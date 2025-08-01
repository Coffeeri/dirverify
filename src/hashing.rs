use blake2::{Blake2s256, Digest as Blake2Digest};
use crc32fast::Hasher as Crc32Hasher;
use md5;
use sha2::Sha256;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    Sha256,
    Md5,
    Crc32,
    Blake2,
    Xxh3,
}

pub fn hash_file(path: &Path, algorithm: HashAlgorithm) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0; 65536]; // 64KB buffer

    match algorithm {
        HashAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Md5 => {
            let mut context = md5::Context::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                context.consume(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", context.compute()))
        }
        HashAlgorithm::Crc32 => {
            let mut hasher = Crc32Hasher::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:08x}", hasher.finalize()))
        }
        HashAlgorithm::Blake2 => {
            let mut hasher = Blake2s256::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Xxh3 => {
            let mut hasher = Xxh3::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:016x}", hasher.digest()))
        }
    }
}

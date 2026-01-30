# dirverify - Directory Verification Tool

A fast, cross-platform tool to verify the equality of directories across different machines using various hash algorithms.

## Features

- **Multiple hash algorithms**: SHA256, MD5, CRC32, BLAKE2, BLAKE3, XXH3
- **Cross-platform**: Static binaries for Linux (x64/ARM64), Windows (x64/ARM64), macOS (ARM64/x64/universal)
- **Parallel processing**: Utilizes all CPU cores for fast hashing
- **Flexible verification**: Compare directories across different machines
- **Exclusion patterns**: Skip unwanted files/directories using glob patterns
- **Timestamp-aware**: Option to skip files newer on target machine
- **JSON output**: Machine-readable checksum format

## Installation

Download the appropriate binary for your platform from the releases page:

- `dirverify-macos-arm64.tar.gz` - macOS Apple Silicon
- `dirverify-macos-x64.tar.gz` - macOS Intel
- `dirverify-macos-universal.tar.gz` - macOS Universal
- `dirverify-linux-x64.tar.gz` - Linux x86_64 (musl)
- `dirverify-linux-arm64.tar.gz` - Linux ARM64 (musl)
- `dirverify-windows-x64.exe.zip` - Windows x64
- `dirverify-windows-arm64.exe.zip` - Windows ARM64

## Usage

### Basic Usage

1. **Generate checksums on source machine:**
```bash
# Generate checksums for current directory
dirverify > checksums.json

# Generate checksums for specific directory
dirverify /path/to/source -o checksums.json

# Use a specific algorithm (default: sha256)
dirverify -a md5 -o checksums.json
```

2. **Transfer `checksums.json` to target machine**

3. **Verify on target machine:**
```bash
# Verify from current directory
dirverify -c checksums.json

# Verify with different root directory
dirverify -c checksums.json -r /path/to/target

# Skip files that are newer on target
dirverify -c checksums.json --skip-newer
```

### Advanced Options

#### Exclude Patterns
```bash
# Exclude specific directories
dirverify -e "*.git" -e "*node_modules*" -e "*.tmp"

# Exclude using patterns
dirverify -e "build/*" -e "*.log" -o checksums.json
```

#### Performance Tuning
```bash
# Use specific number of threads (default: all cores)
dirverify -t 4 -o checksums.json

# Verbose output
dirverify -v -c checksums.json
```

#### Algorithm Selection
```bash
# Fast algorithms for large files
dirverify -a xxh3  # Fastest
dirverify -a crc32 # Fast, simple

# Cryptographic algorithms
dirverify -a sha256 # Default, secure
dirverify -a blake  # BLAKE3 (default for "blake")
dirverify -a blake3 # BLAKE3 (explicit)
dirverify -a blake2 # BLAKE2 (explicit)

# Legacy support
dirverify -a md5    # For compatibility
```

## Examples

### Example 1: Backup Verification
```bash
# On backup source
dirverify /home/user/documents -a sha256 -o backup-checksums.json

# On backup destination (after file transfer)
dirverify -c backup-checksums.json -r /mnt/backup/documents
```

### Example 2: Deployment Verification
```bash
# On build server
cd /var/www/myapp
dirverify -e "*.log" -e "cache/*" -o deploy-checksums.json

# On production server
cd /var/www/myapp
dirverify -c deploy-checksums.json --skip-newer
```

### Example 3: Mirror Sync Verification
```bash
# Generate checksums excluding temporary files
dirverify /data/mirror -e "*.tmp" -e "*.part" -a xxh3 -o mirror.json

# Verify on remote mirror
dirverify -c mirror.json -r /mnt/remote-mirror
```

## Checksum File Format

The tool generates JSON files with the following structure:

```json
{
  "version": "1.0",
  "algorithm": "sha256",
  "entries": [
    {
      "path": "relative/path/to/file.txt",
      "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "modified": 1699564432,
      "size": 1024
    }
  ]
}
```

## Performance

Performance varies by algorithm and file size:

| Algorithm | Speed | Security | Use Case |
|-----------|-------|----------|----------|
| XXH3 | Varies (see benchmark) | Non-cryptographic | Large files, speed critical |
| CRC32 | Varies (see benchmark) | Non-cryptographic | Quick integrity checks |
| BLAKE3 | Varies (see benchmark) | Cryptographic | Secure, very fast |
| BLAKE2 | Varies (see benchmark) | Cryptographic | Secure, fast |
| SHA256 | Varies (see benchmark) | Cryptographic | Default, widely supported |
| MD5 | Varies (see benchmark) | Broken | Legacy systems only |

### Local benchmarking

Run the included benchmark helper to compare throughput on your machine:

```bash
cargo run --release --bin hashbench
```

For statistically robust results (Criterion):

```bash
cargo bench --bench hashing
```

#### Example results (Apple M1 Pro)

Ran on `MacBookPro18,3` (Apple M1 Pro), `macOS 26.2 (25C56)`, `Kernel 25.2.0` with `cargo bench --bench hashing` (256 MiB input, 64 KiB chunking). Numbers below are Criterion throughput estimates (lower / median / upper).

`hash_mem_chunked` (in-memory; minimizes filesystem noise):

| Algorithm | Throughput |
|----------|------------|
| XXH3 | 30.412 / 31.132 / 31.651 GiB/s |
| CRC32 | 7.5145 / 7.5608 / 7.6081 GiB/s |
| BLAKE3 | 1.6279 / 1.6437 / 1.6561 GiB/s |
| BLAKE2 | 573.26 / 576.91 / 580.17 MiB/s |
| SHA256 | 320.20 / 324.05 / 327.40 MiB/s |
| MD5 | 549.79 / 551.77 / 553.60 MiB/s |

`hash_file` (hashes a 256 MiB temp file via `hash_file`; sensitive to OS page cache/storage):

| Algorithm | Throughput |
|----------|------------|
| XXH3 | 7.0406 / 7.5253 / 7.9165 GiB/s |
| CRC32 | 4.1969 / 4.2870 / 4.3854 GiB/s |
| BLAKE3 | 1.3273 / 1.3701 / 1.4107 GiB/s |
| BLAKE2 | 536.40 / 540.02 / 543.32 MiB/s |
| SHA256 | 311.91 / 312.54 / 313.17 MiB/s |
| MD5 | 475.39 / 493.93 / 511.40 MiB/s |

Tune the benchmark size/chunking and optionally select specific algorithms:

```bash
cargo run --release --bin hashbench -- --size-mib 1024 --chunk-kib 64 --iters 3 --alg blake3 --alg sha256 --alg xxh3
```

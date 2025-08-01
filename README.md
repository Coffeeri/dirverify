# dirverify - Directory Verification Tool

A fast, cross-platform tool to verify the equality of directories across different machines using various hash algorithms.

## Features

- **Multiple hash algorithms**: SHA256, MD5, CRC32, BLAKE2, XXH3
- **Cross-platform**: Static binaries for Linux (x64/ARM64), Windows (x64), macOS (ARM64)
- **Parallel processing**: Utilizes all CPU cores for fast hashing
- **Flexible verification**: Compare directories across different machines
- **Exclusion patterns**: Skip unwanted files/directories using glob patterns
- **Timestamp-aware**: Option to skip files newer on target machine
- **JSON output**: Machine-readable checksum format

## Installation

Download the appropriate binary for your platform from the releases page:

- `dirverify-linux-x64` - Linux x86_64
- `dirverify-linux-arm64` - Linux ARM64
- `dirverify-windows-x64.exe` - Windows x64
- `dirverify-macos-arm64` - macOS ARM64 (Apple Silicon)

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
dirverify -a blake2 # Fast and secure

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

## Building from Source

### Prerequisites
- Rust 1.70 or later
- For cross-compilation: `cross` tool

### Build Commands
```bash
# Clone repository
git clone https://github.com/yourusername/dirverify
cd dirverify

# Build for current platform
cargo build --release

# Build all platforms (Linux/macOS)
chmod +x build.sh
./build.sh
```

### Project Structure
```
dirverify/
├── src/
│   ├── main.rs      # Main application logic
│   └── hashing.rs   # Hash algorithm implementations
├── Cargo.toml       # Dependencies and build configuration
├── build.sh         # Cross-compilation script
└── README.md        # This file
```

## Performance

Performance varies by algorithm and file size:

| Algorithm | Speed | Security | Use Case |
|-----------|-------|----------|----------|
| XXH3 | ~10 GB/s | Non-cryptographic | Large files, speed critical |
| CRC32 | ~5 GB/s | Non-cryptographic | Quick integrity checks |
| BLAKE2 | ~1 GB/s | Cryptographic | Secure, fast |
| SHA256 | ~500 MB/s | Cryptographic | Default, widely supported |
| MD5 | ~400 MB/s | Broken | Legacy systems only |

## Error Handling

The tool provides clear error messages:
- Missing files are reported
- Permission errors are logged
- Hash mismatches show expected vs actual
- Exit code 1 on verification failure

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## Troubleshooting

### "File not found" errors during verification
- Check if the root directory (-r) is correct
- Ensure relative paths match between source and target

### "Permission denied" errors
- Run with appropriate permissions
- Some system files may require elevated privileges

### Different results between algorithms
- This is expected - each algorithm produces different hash values
- Use the same algorithm for generation and verification

### Slow performance
- Use faster algorithms (xxh3, crc32) for large datasets
- Adjust thread count with -t option
- Exclude unnecessary large files

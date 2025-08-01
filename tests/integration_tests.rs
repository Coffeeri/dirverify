use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

#[test]
fn test_generate_checksums_basic() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "file1.txt", b"Hello, World!");
    create_test_file(dir.path(), "file2.txt", b"Test content");
    
    let output = Command::new("cargo")
        .args(&["run", "--", dir.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Check JSON structure
    assert!(stdout.contains("\"version\""));
    assert!(stdout.contains("\"algorithm\""));
    assert!(stdout.contains("\"entries\""));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
}

#[test]
fn test_verify_checksums() {
    let source_dir = TempDir::new().unwrap();
    let target_dir = TempDir::new().unwrap();
    
    // Create identical files in both directories
    create_test_file(source_dir.path(), "test.txt", b"Same content");
    create_test_file(target_dir.path(), "test.txt", b"Same content");
    
    // Generate checksums
    let checksum_file = source_dir.path().join("checksums.json");
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            source_dir.path().to_str().unwrap(),
            "-o", checksum_file.to_str().unwrap()
        ])
        .output()
        .expect("Failed to generate checksums");
    
    assert!(output.status.success());
    
    // Verify checksums
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            target_dir.path().to_str().unwrap(),
            "-c", checksum_file.to_str().unwrap()
        ])
        .output()
        .expect("Failed to verify checksums");
    
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("OK:"));
}

#[test]
fn test_exclude_patterns() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "include.txt", b"Include this");
    create_test_file(dir.path(), "exclude.tmp", b"Exclude this");
    create_test_file(dir.path(), ".git/config", b"Exclude git");
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            dir.path().to_str().unwrap(),
            "-e", "*.tmp",
            "-e", "**/.git/**"
        ])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Debug output
    eprintln!("Output JSON: {}", stdout);
    
    assert!(stdout.contains("include.txt"));
    assert!(!stdout.contains("exclude.tmp"));
    assert!(!stdout.contains(".git"));
}

#[test]
fn test_different_algorithms() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "test.txt", b"Test content");
    
    let algorithms = vec!["sha256", "md5", "crc32", "blake2", "xxh3"];
    
    for algo in algorithms {
        let output = Command::new("cargo")
            .args(&[
                "run", "--",
                dir.path().to_str().unwrap(),
                "-a", algo
            ])
            .output()
            .expect("Failed to execute command");
        
        assert!(output.status.success(), "Algorithm {} should work", algo);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(&format!("\"algorithm\": \"{}\"", algo)));
    }
}

#[test]
fn test_verify_failed_checksum() {
    let source_dir = TempDir::new().unwrap();
    let target_dir = TempDir::new().unwrap();
    
    // Create different content
    create_test_file(source_dir.path(), "test.txt", b"Original content");
    create_test_file(target_dir.path(), "test.txt", b"Different content");
    
    // Generate checksums
    let checksum_file = source_dir.path().join("checksums.json");
    Command::new("cargo")
        .args(&[
            "run", "--",
            source_dir.path().to_str().unwrap(),
            "-o", checksum_file.to_str().unwrap()
        ])
        .output()
        .expect("Failed to generate checksums");
    
    // Verify should fail
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            target_dir.path().to_str().unwrap(),
            "-c", checksum_file.to_str().unwrap()
        ])
        .output()
        .expect("Failed to verify checksums");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("FAILED"));
    assert!(stderr.contains("Hash mismatch"));
}

#[test]
fn test_subdirectories() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "root.txt", b"Root file");
    create_test_file(dir.path(), "sub1/file1.txt", b"Subdir 1");
    create_test_file(dir.path(), "sub1/sub2/file2.txt", b"Nested file");
    
    let output = Command::new("cargo")
        .args(&["run", "--", dir.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    assert!(stdout.contains("root.txt"));
    assert!(stdout.contains("sub1/file1.txt"));
    assert!(stdout.contains("sub1/sub2/file2.txt"));
}

#[test]
fn test_verbose_mode() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "test.txt", b"Test");
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            dir.path().to_str().unwrap(),
            "-v"
        ])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Processed:"));
}

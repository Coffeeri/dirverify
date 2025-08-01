use clap::{Parser, ValueEnum};
use glob::Pattern;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use walkdir::WalkDir;

mod hashing;
use hashing::{hash_file, HashAlgorithm};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Algorithm {
    Sha256,
    Md5,
    Crc32,
    Blake2,
    Xxh3,
}

impl From<Algorithm> for HashAlgorithm {
    fn from(algo: Algorithm) -> Self {
        match algo {
            Algorithm::Sha256 => HashAlgorithm::Sha256,
            Algorithm::Md5 => HashAlgorithm::Md5,
            Algorithm::Crc32 => HashAlgorithm::Crc32,
            Algorithm::Blake2 => HashAlgorithm::Blake2,
            Algorithm::Xxh3 => HashAlgorithm::Xxh3,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Verify equality of two root directories", long_about = None)]
struct Args {
    /// Directory to process or verify
    #[arg(default_value = ".")]
    directory: PathBuf,

    /// Checksum file to verify against
    #[arg(short, long)]
    check: Option<PathBuf>,

    /// Hash algorithm to use
    #[arg(short, long, value_enum, default_value = "sha256")]
    algorithm: Algorithm,

    /// Output file for checksums (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Exclude patterns (can be specified multiple times)
    #[arg(short, long)]
    exclude: Vec<String>,

    /// Only check files older than those on target (requires -c)
    #[arg(long)]
    skip_newer: bool,

    /// Root directory for verification (when using -c)
    #[arg(short, long)]
    root: Option<PathBuf>,

    /// Number of threads to use
    #[arg(short, long, default_value = "0")]
    threads: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChecksumEntry {
    path: String,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChecksumFile {
    version: String,
    algorithm: String,
    entries: Vec<ChecksumEntry>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set thread pool size
    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
            .unwrap();
    }

    if let Some(ref checksum_file) = args.check {
        verify_checksums(&args, checksum_file)
    } else {
        generate_checksums(&args)
    }
}

fn should_exclude(path: &Path, patterns: &[Pattern]) -> bool {
    patterns.iter().any(|pattern| {
        path.to_str()
            .map(|s| pattern.matches(s))
            .unwrap_or(false)
    })
}

fn generate_checksums(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let exclude_patterns: Vec<Pattern> = args
        .exclude
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();

    let mut entries = Vec::new();
    let processed = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    eprintln!("Scanning directory: {}", args.directory.display());

    // Collect all files
    let files: Vec<_> = WalkDir::new(&args.directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !should_exclude(e.path(), &exclude_patterns))
        .collect();

    let total_files = files.len();
    eprintln!("Found {} files to process", total_files);

    // Process files in parallel
    let results: Vec<_> = files
        .par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(&args.directory)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            match process_file(path, &relative_path, args.algorithm.into(), args.skip_newer) {
                Ok(checksum_entry) => {
                    let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    if args.verbose || count % 100 == 0 {
                        eprint!("\rProcessed: {}/{}", count, total_files);
                    }
                    Some(checksum_entry)
                }
                Err(e) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("\nError processing {}: {}", path.display(), e);
                    None
                }
            }
        })
        .collect();

    eprintln!("\rProcessed: {}/{}", total_files, total_files);

    entries.extend(results);

    // Sort entries for consistent output
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let checksum_file = ChecksumFile {
        version: "1.0".to_string(),
        algorithm: format!("{:?}", args.algorithm).to_lowercase(),
        entries,
    };

    // Write output
    let output_json = serde_json::to_string_pretty(&checksum_file)?;
    
    if let Some(output_path) = &args.output {
        fs::write(output_path, output_json)?;
        eprintln!("Checksums written to: {}", output_path.display());
    } else {
        println!("{}", output_json);
    }

    let error_count = errors.load(Ordering::Relaxed);
    if error_count > 0 {
        eprintln!("Warning: {} errors occurred during processing", error_count);
    }

    Ok(())
}

fn process_file(
    path: &Path,
    relative_path: &str,
    algorithm: HashAlgorithm,
    include_metadata: bool,
) -> Result<ChecksumEntry, Box<dyn std::error::Error>> {
    let hash = hash_file(path, algorithm)?;
    
    let (modified, size) = if include_metadata {
        let metadata = fs::metadata(path)?;
        let modified = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        (Some(modified), Some(metadata.len()))
    } else {
        (None, None)
    };

    Ok(ChecksumEntry {
        path: relative_path.to_string(),
        hash,
        modified,
        size,
    })
}

fn verify_checksums(
    args: &Args,
    checksum_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(checksum_path)?;
    let reader = BufReader::new(file);
    let checksum_file: ChecksumFile = serde_json::from_reader(reader)?;

    eprintln!("Verifying {} files using {} algorithm", 
              checksum_file.entries.len(), 
              checksum_file.algorithm);

    let root_dir = args.root.as_ref().unwrap_or(&args.directory);
    let processed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let total = checksum_file.entries.len();

    // Parse algorithm from checksum file
    let algorithm = match checksum_file.algorithm.as_str() {
        "sha256" => HashAlgorithm::Sha256,
        "md5" => HashAlgorithm::Md5,
        "crc32" => HashAlgorithm::Crc32,
        "blake2" => HashAlgorithm::Blake2,
        "xxh3" => HashAlgorithm::Xxh3,
        _ => {
            eprintln!("Warning: Unknown algorithm '{}', using SHA256", checksum_file.algorithm);
            HashAlgorithm::Sha256
        }
    };

    // Verify files in parallel
    let _results: Vec<_> = checksum_file
        .entries
        .par_iter()
        .map(|entry| {
            let full_path = root_dir.join(&entry.path);
            let result = verify_single_file(&full_path, entry, algorithm, args.skip_newer);
            
            match &result {
                VerifyResult::Ok => {
                    let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    if args.verbose {
                        eprintln!("OK: {}", entry.path);
                    } else if count % 100 == 0 {
                        eprint!("\rVerified: {}/{}", count, total);
                    }
                }
                VerifyResult::Failed(msg) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("\nFAILED: {} - {}", entry.path, msg);
                }
                VerifyResult::Skipped(msg) => {
                    skipped.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        eprintln!("SKIPPED: {} - {}", entry.path, msg);
                    }
                }
            }
            
            (entry.path.clone(), result)
        })
        .collect();

    eprintln!("\rVerified: {}/{}", total, total);

    // Summary
    let ok_count = processed.load(Ordering::Relaxed);
    let fail_count = failed.load(Ordering::Relaxed);
    let skip_count = skipped.load(Ordering::Relaxed);

    eprintln!("\nSummary:");
    eprintln!("  OK:      {}", ok_count);
    eprintln!("  Failed:  {}", fail_count);
    eprintln!("  Skipped: {}", skip_count);
    eprintln!("  Total:   {}", total);

    if fail_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

enum VerifyResult {
    Ok,
    Failed(String),
    Skipped(String),
}

fn verify_single_file(
    path: &Path,
    entry: &ChecksumEntry,
    algorithm: HashAlgorithm,
    skip_newer: bool,
) -> VerifyResult {
    if !path.exists() {
        return VerifyResult::Failed("File not found".to_string());
    }

    // Check if we should skip newer files
    if skip_newer && entry.modified.is_some() {
        match fs::metadata(path) {
            Ok(metadata) => {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                        let current_modified = duration.as_secs();
                        if current_modified > entry.modified.unwrap() {
                            return VerifyResult::Skipped("File is newer on target".to_string());
                        }
                    }
                }
            }
            Err(e) => return VerifyResult::Failed(format!("Cannot read metadata: {}", e)),
        }
    }

    match hash_file(path, algorithm) {
        Ok(hash) => {
            if hash == entry.hash {
                VerifyResult::Ok
            } else {
                VerifyResult::Failed(format!("Hash mismatch: expected {}, got {}", entry.hash, hash))
            }
        }
        Err(e) => VerifyResult::Failed(format!("Cannot compute hash: {}", e)),
    }
}

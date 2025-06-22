use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug)]
struct CullHistoryRecord {
    timestamp: String,
    retained: String,
    culled: Vec<String>,
    action: String, // "moved" or "deleted"
}

#[derive(Parser, Debug)]
#[command(name = "cullrs", version, about = "CLI for culling photos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Duplicate workflows
    Duplicates {
        #[command(subcommand)]
        command: Dups,
    },

    /// Work with cull history
    History {
        #[command(subcommand)]
        command: HistoryCmd,
    },
}

#[derive(Subcommand, Debug)]
enum Dups {
    /// Find and list duplicate groups
    Scan {
        /// Directory to scan
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },

    /// Move duplicates into `<dir>/duplicates`
    Cull {
        /// Directory to cull
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Only show what would be moved
        #[arg(long)]
        dry_run: bool,
        /// Directory to move duplicates into (default: `<dir>/duplicates`)
        #[arg(long, value_name = "DIR")]
        target_dir: Option<PathBuf>,
    },

    /// Permanently delete duplicate images
    Delete {
        /// Directory to cull
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum HistoryCmd {
    /// List all cull history records
    List {
        /// Directory containing the photos
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },

    /// Restore moved files from history (all types)
    Restore {
        /// Directory containing the photos
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Restore a specific record index
        #[arg(long, conflicts_with = "all")]
        record: Option<usize>,
        /// Restore all records
        #[arg(long, conflicts_with = "record")]
        all: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Duplicates { command } => match command {
            Dups::Scan { path } => {
                println!("▶ Scanning for duplicates in: {}", path.display());
                let groups = find_duplicates(&path)?;
                if groups.is_empty() {
                    println!("No duplicates found.");
                } else {
                    println!("Found {} duplicate group(s):", groups.len());
                    for (i, group) in groups.iter().enumerate() {
                        println!(" Group {}:", i + 1);
                        for file in group {
                            println!("   ▶ {}", file.display());
                        }
                    }
                }
            }

            Dups::Cull {
                path,
                dry_run,
                target_dir,
            } => {
                println!("▶ Culling duplicates in: {}", path.display());
                let mut groups = find_duplicates(&path)?;
                if groups.is_empty() {
                    println!("No duplicates found.");
                    return Ok(());
                }

                for group in &mut groups {
                    group.sort_by_key(|p| get_timestamp(p));
                }

                let dup_dir = target_dir.unwrap_or_else(|| path.join("duplicates"));
                if !dry_run {
                    fs::create_dir_all(&dup_dir)
                        .with_context(|| format!("Failed to create directory {:?}", dup_dir))?;
                }

                let history_file = path.join(".history.jsonl");
                let mut history_out = if dry_run {
                    None
                } else {
                    Some(
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&history_file)
                            .with_context(|| {
                                format!("Failed to open history file {:?}", history_file)
                            })?,
                    )
                };

                for (i, group) in groups.iter().enumerate() {
                    println!("\n✨ Group {}:", i + 1);
                    println!("   🏆 Keeping → {}", group[0].display());
                    let retained = group[0].to_string_lossy().into_owned();
                    let mut culled_paths = Vec::new();

                    for dup in &group[1..] {
                        culled_paths.push(dup.to_string_lossy().into_owned());
                        if dry_run {
                            println!(
                                "   📦 [dry-run] MOVE {} → {}",
                                dup.display(),
                                dup_dir.display()
                            );
                        } else {
                            let file_name = dup.file_name().unwrap();
                            let dest = dup_dir.join(file_name);
                            fs::rename(dup, &dest).with_context(|| {
                                format!("Failed to move {:?} → {:?}", dup, dest)
                            })?;
                            println!("   📦 Moved {} → {}", dup.display(), dest.display());
                        }
                    }

                    if let Some(out) = history_out.as_mut() {
                        let record = CullHistoryRecord {
                            timestamp: Utc::now().to_rfc3339(),
                            retained,
                            culled: culled_paths,
                            action: "moved".to_string(),
                        };
                        writeln!(out, "{}", serde_json::to_string(&record)?)?;
                    }
                }

                if dry_run {
                    println!("\n⚠️  Dry-run only; no files were changed.");
                } else {
                    println!(
                        "\n✅ Recorded cull history in {}",
                        path.join(".history.jsonl").display()
                    );
                }
            }

            Dups::Delete { path } => {
                println!("▶ Deleting duplicates in: {}", path.display());
                let mut groups = find_duplicates(&path)?;
                if groups.is_empty() {
                    println!("No duplicates found.");
                    return Ok(());
                }

                for group in &mut groups {
                    group.sort_by_key(|p| get_timestamp(p));
                }

                let history_file = path.join(".history.jsonl");
                let mut history_out = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&history_file)
                    .with_context(|| format!("Failed to open history file {:?}", history_file))?;

                for (i, group) in groups.iter().enumerate() {
                    println!("\n✨ Group {}:", i + 1);
                    println!("   🏆 Keeping → {}", group[0].display());
                    let retained = group[0].to_string_lossy().into_owned();
                    let mut culled_paths = Vec::new();

                    for dup in &group[1..] {
                        culled_paths.push(dup.to_string_lossy().into_owned());
                        fs::remove_file(dup)
                            .with_context(|| format!("Failed to delete {}", dup.display()))?;
                        println!("   🗑️  Deleted {}", dup.display());
                    }

                    let record = CullHistoryRecord {
                        timestamp: Utc::now().to_rfc3339(),
                        retained,
                        culled: culled_paths,
                        action: "deleted".to_string(),
                    };
                    writeln!(history_out, "{}", serde_json::to_string(&record)?)?;
                }

                println!(
                    "\n✅ Recorded cull history in {}",
                    path.join(".history.jsonl").display()
                );
            }
        },

        Commands::History { command } => match command {
            HistoryCmd::List { path } => {
                let history_file = path.join(".history.jsonl");
                let f = File::open(&history_file)
                    .with_context(|| format!("Could not open history file {:?}", history_file))?;
                let reader = BufReader::new(f);

                println!("🗂️  Cull History:");
                for (i, line) in reader.lines().enumerate() {
                    let line = line?;
                    match serde_json::from_str::<CullHistoryRecord>(&line) {
                        Ok(rec) => println!(
                            "[{}] {}\n     kept: {}\n     culled: {:?}\n     action: {}\n",
                            i, rec.timestamp, rec.retained, rec.culled, rec.action
                        ),
                        Err(err) => eprintln!("⚠️  Skipping malformed entry {}: {}", i, err),
                    }
                }
            }

            HistoryCmd::Restore { path, record, all } => {
                let history_file = path.join(".history.jsonl");
                let f = File::open(&history_file)
                    .with_context(|| format!("Could not open history file {:?}", history_file))?;
                let reader = BufReader::new(f);

                let mut stored: Vec<(CullHistoryRecord, String)> = Vec::new();
                for line in reader.lines() {
                    let line = line?;
                    if let Ok(rec) = serde_json::from_str::<CullHistoryRecord>(&line) {
                        if rec.action == "moved" {
                            stored.push((rec, line));
                        }
                    }
                }

                if stored.is_empty() {
                    anyhow::bail!("No valid 'moved' history records to restore");
                }

                let restore_indices = if all {
                    (0..stored.len()).collect()
                } else {
                    let idx = record.unwrap_or(stored.len() - 1);
                    if idx >= stored.len() {
                        anyhow::bail!(
                            "Invalid history index {}; valid range is 0..{}",
                            idx,
                            stored.len() - 1
                        );
                    }
                    vec![idx]
                };

                for &i in &restore_indices {
                    let rec = &stored[i].0;
                    println!(
                        "🔄 Restoring {} files from record {}...",
                        rec.culled.len(),
                        rec.timestamp
                    );
                    for orig in &rec.culled {
                        let fname = Path::new(orig).file_name().unwrap_or_default();
                        let src = path.join("duplicates").join(&fname);
                        let dest = Path::new(orig);

                        if !src.exists() {
                            eprintln!("⚠️ Source file {:?} does not exist; skipping", src);
                            continue;
                        }
                        if src == dest {
                            eprintln!("⚠️ Source and destination are the same; skipping {:?}", src);
                            continue;
                        }
                        fs::rename(&src, &dest)
                            .with_context(|| format!("Failed to restore {:?} → {:?}", src, dest))?;
                        println!("🔄 Restored {:?} → {:?}", src, dest);
                    }
                }

                let remaining: Vec<String> = stored
                    .into_iter()
                    .enumerate()
                    .filter(|(i, _)| !restore_indices.contains(i))
                    .map(|(_, (_, line))| line)
                    .collect();
                let new_content = if remaining.is_empty() {
                    String::new()
                } else {
                    remaining.join("\n") + "\n"
                };
                fs::write(&history_file, new_content)
                    .with_context(|| format!("Failed to update history file {:?}", history_file))?;

                println!(
                    "🧹 Updated history, removed {} record(s)",
                    restore_indices.len()
                );
            }
        },
    }

    Ok(())
}

/// Recursively walk `dir`, returning a Vec of image file paths.
fn scan_directory(dir: &Path) -> Result<Vec<PathBuf>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?);
    spinner.set_message("Scanning for images…");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let allowed_exts = ["jpg", "jpeg", "png", "gif", "bmp", "tiff"];
    let mut images = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if allowed_exts.contains(&ext.to_lowercase().as_str()) {
                    images.push(path.to_path_buf());
                }
            }
        }
        spinner.tick();
    }
    spinner.finish_with_message("Scan complete");
    Ok(images)
}

/// Find duplicate images by mean-hash similarity, returning groups of file paths.
fn find_duplicates(dir: &Path) -> Result<Vec<Vec<PathBuf>>> {
    let images = scan_directory(dir)?;
    println!("▶ Parallel hashing {} images…", images.len());

    let hasher = HasherConfig::new().hash_alg(HashAlg::Mean).to_hasher();

    let key_paths: Vec<(String, PathBuf)> = benchmark("hashing all images", || {
        images
            .par_iter()
            .map(|path| -> Result<(String, PathBuf)> {
                let img = ImageReader::open(path)
                    .with_context(|| format!("Failed to open {:?}", path))?
                    .decode()
                    .with_context(|| format!("Failed to decode {:?}", path))?;
                let key = hasher.hash_image(&img).to_base64();
                Ok((key, path.clone()))
            })
            .collect::<Result<_>>()
    })?;

    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (key, path) in key_paths {
        map.entry(key).or_default().push(path);
    }

    Ok(map.into_values().filter(|v| v.len() > 1).collect())
}

/// Run `f()`, print how long it took (with `label`), and return its result.
fn benchmark<T, F: FnOnce() -> T>(label: &str, f: F) -> T {
    let start = Instant::now();
    let result = f();
    println!("⏱ {} took {:.2?}", label, start.elapsed());
    result
}

/// Get the creation time (or modification time) of a file, falling back to UNIX_EPOCH on error.
fn get_timestamp(path: &PathBuf) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.created())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

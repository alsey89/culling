use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug)]
struct CullHistoryRecord {
    timestamp: String,
    retained: String,
    culled: Vec<String>,
    action: String, // "moved" or "deleted"
}

/// Photo Culler CLI - prototype commands
#[derive(Parser, Debug)]
#[command(name = "darwin", version, about = "CLI for culling photos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan a directory, print image files and their metadata
    Scan {
        /// Path to directory to scan
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },

    /// Run one or more analysis passes on a directory
    Analyze {
        /// Directory to analyze
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Find duplicate images
        #[arg(long)]
        duplicates: bool,
        /// (future) detect faces
        #[arg(long)]
        faces: bool,
        /// (future) measure focus
        #[arg(long)]
        focus: bool,
    },

    /// Cull images by a single chosen method
    Cull {
        /// Directory to cull
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Only show what would be deleted/moved
        #[arg(long)]
        dry: bool,
        /// Permanently delete instead of moving
        #[arg(long)]
        delete: bool,
        /// Directory to move duplicates into (default: <path>/duplicates) or blurry into (<path>/blurry)
        #[arg(long, value_name = "DIR")]
        target_dir: Option<PathBuf>,
        /// Pick exactly one cull action
        #[command(subcommand)]
        action: CullAction,
    },

    /// Show cull history
    History {
        /// Directory containing the photos
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
    },

    /// Restore moved files from cull history
    Restore {
        /// Directory containing the photos
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Either the record index (from `history`)
        #[arg(long, conflicts_with = "all")]
        record: Option<usize>,
        /// Restore all records
        #[arg(long, conflicts_with = "record")]
        all: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CullAction {
    /// Remove duplicate groups
    Duplicates {},

    /// Remove out-of-focus (blurry) images
    Focus {
        /// Minimum acceptable focus score (lower = blurrier)
        #[arg(long, default_value_t = 100.0)]
        min_focus: f64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { path } => {
            println!("▶ Scanning directory: {}", path.display());
            let images = scan_directory(&path)?;
            println!("Found {} image(s):", images.len());
            for img in &images {
                println!(" - {}", img.display());
            }
        }

        Commands::Analyze {
            path,
            duplicates,
            faces,
            focus,
        } => {
            println!("▶ Analyzing directory: {}", path.display());
            let images = scan_directory(&path)?;
            println!("Found {} image(s) to analyze.", images.len());
            if images.is_empty() {
                eprintln!("⚠️  No images found in the specified directory.");
                return Ok(());
            }
            let mut did_any = false;
            if duplicates {
                did_any = true;
                println!("🔍 Finding duplicates...");
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
            if faces {
                did_any = true;
                println!("👤 Detecting faces... (not yet implemented)");
            }
            if focus {
                did_any = true;
                println!("🔬 Measuring focus... (not yet implemented)");
            }
            if !did_any {
                eprintln!(
                    "⚠️  No analysis flags given. Try `--duplicates`, `--faces`, or `--focus`."
                );
            }
        }

        Commands::Cull {
            path,
            dry,
            delete,
            target_dir,
            action,
        } => match action {
            CullAction::Duplicates {} => {
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
                if !delete && !dry {
                    fs::create_dir_all(&dup_dir)
                        .with_context(|| format!("Failed to create directory {:?}", dup_dir))?;
                }

                let history_file = path.join(".darwin_history.jsonl");
                let mut history_out = if dry {
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
                    let retained = group[0].display().to_string();
                    let mut culled_paths = Vec::new();
                    let action_str = if delete { "deleted" } else { "moved" }.to_string();

                    for dup in &group[1..] {
                        culled_paths.push(dup.display().to_string());
                        if dry {
                            if delete {
                                println!("   🗑️ [dry-run] DELETE {}", dup.display());
                            } else {
                                println!("   📦 [dry-run] MOVE {} → {:?}", dup.display(), dup_dir);
                            }
                        } else if delete {
                            fs::remove_file(dup)
                                .with_context(|| format!("Failed to delete {}", dup.display()))?;
                            println!("   🗑️  Deleted {}", dup.display());
                        } else {
                            let file_name = dup.file_name().unwrap();
                            let dest = dup_dir.join(file_name);
                            fs::rename(dup, &dest).with_context(|| {
                                format!("Failed to move {:?} → {:?}", dup, dest)
                            })?;
                            println!("   📦 Moved {} → {:?}", dup.display(), dest);
                        }
                    }

                    if let Some(out) = history_out.as_mut() {
                        let record = CullHistoryRecord {
                            timestamp: Utc::now().to_rfc3339(),
                            retained,
                            culled: culled_paths,
                            action: action_str,
                        };
                        writeln!(out, "{}", serde_json::to_string(&record)?)?;
                    }
                }

                if dry {
                    println!("\n⚠️  Dry-run only; no files were changed.");
                } else {
                    println!(
                        "\n✅ Recorded cull history in {}",
                        path.join(".darwin_history.jsonl").display()
                    );
                }
            }

            CullAction::Focus { min_focus } => {
                println!("▶ Culling out-of-focus images in: {}", path.display());
                let blurry = find_blurry(&path, min_focus)?;
                if blurry.is_empty() {
                    println!("No blurry images found (threshold {}).", min_focus);
                    return Ok(());
                }
                let blur_dir = target_dir.unwrap_or_else(|| path.join("blurry"));
                if !delete && !dry {
                    fs::create_dir_all(&blur_dir)
                        .with_context(|| format!("Failed to create directory {:?}", blur_dir))?;
                }

                for img in blurry {
                    if dry {
                        if delete {
                            println!("   🗑️ [dry-run] DELETE {}", img.display());
                        } else {
                            println!("   📦 [dry-run] MOVE {} → {:?}", img.display(), blur_dir);
                        }
                    } else if delete {
                        fs::remove_file(&img)
                            .with_context(|| format!("Failed to delete {}", img.display()))?;
                        println!("   🗑️  Deleted {}", img.display());
                    } else {
                        let file_name = img.file_name().unwrap();
                        let dest = blur_dir.join(file_name);
                        fs::rename(&img, &dest)
                            .with_context(|| format!("Failed to move {:?} → {:?}", img, dest))?;
                        println!("   📦 Moved {} → {:?}", img.display(), dest);
                    }
                }

                if dry {
                    println!("\n⚠️  Dry-run only; no files were changed.");
                } else {
                    println!("\n✅ Focus-based cull complete.");
                }
            }
        },

        Commands::History { path } => {
            let history_file = path.join(".darwin_history.jsonl");
            let f = File::open(&history_file)
                .with_context(|| format!("Could not open history file {:?}", history_file))?;
            let reader = BufReader::new(f);
            println!("🗂️  Cull History:");
            for (i, line) in reader.lines().enumerate() {
                if let Ok(line) = line {
                    match serde_json::from_str::<CullHistoryRecord>(&line) {
                        Ok(rec) => println!(
                            "[{}] {}\n     kept: {}\n     culled: {:?}\n     action: {}\n",
                            i, rec.timestamp, rec.retained, rec.culled, rec.action
                        ),
                        Err(err) => eprintln!("⚠️  Skipping malformed entry {}: {}", i, err),
                    }
                }
            }
        }

        Commands::Restore { path, record, all } => {
            let history_file = path.join(".darwin_history.jsonl");
            let f = File::open(&history_file)
                .with_context(|| format!("Could not open history file {:?}", history_file))?;
            let reader = BufReader::new(f);

            let mut stored: Vec<(CullHistoryRecord, String)> = Vec::new();
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(rec) = serde_json::from_str::<CullHistoryRecord>(&line) {
                    if rec.action == "moved" {
                        stored.push((rec, line));
                    } else {
                        eprintln!(
                            "⚠️ Skipping record {}: action was '{}'; cannot restore.",
                            rec.timestamp, rec.action
                        );
                    }
                } else {
                    eprintln!("⚠️ Skipping malformed history entry");
                }
            }

            if stored.is_empty() {
                anyhow::bail!("No valid 'moved' history records to restore");
            }

            let restore_indices: Vec<usize> = if all {
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
                    match fs::rename(&src, &dest) {
                        Ok(_) => println!("🔄 Restored {:?} → {:?}", src, dest),
                        Err(err) => eprintln!("⚠️ Failed to restore {:?}: {}", src, err),
                    }
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
    }

    Ok(())
}

/// Recursively walk `dir`, returning a Vec of image file paths.
fn scan_directory(dir: &Path) -> Result<Vec<PathBuf>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?);
    spinner.set_message("Scanning for images...");
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

/// Find duplicate images by mean-hash similarity, returning groups of file paths
fn find_duplicates(dir: &Path) -> Result<Vec<Vec<PathBuf>>> {
    let images = scan_directory(dir)?;
    let pb = ProgressBar::new(images.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Hashing images... {pos}/{len}",
    )?);

    // start the “total” timer
    let total_start = Instant::now();
    // for calculating per-image average
    let mut sum_hash_time = Duration::ZERO;

    let hasher = HasherConfig::new().hash_alg(HashAlg::Mean).to_hasher();
    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in images {
        // start per-image timer
        let img_start = Instant::now();

        let img = ImageReader::open(&path).with_context(|| format!("Failed to open {:?}", path))?;
        let img = img
            .decode()
            .with_context(|| format!("Failed to decode {:?}", path))?;
        let key = hasher.hash_image(&img).to_base64();
        map.entry(key).or_default().push(path.clone());
        pb.inc(1);

        // record how long we spent hashing this one
        sum_hash_time += img_start.elapsed();
    }
    pb.finish_with_message("Hashing complete");

    // total elapsed wall-clock time
    let total_elapsed = total_start.elapsed();
    // compute average
    let avg = if !map.is_empty() {
        sum_hash_time / (map.values().flatten().count() as u32)
    } else {
        Duration::ZERO
    };
    println!(
        "⏱  Total hashing time: {:.2?},  avg per image: {:.2?}",
        total_elapsed, avg
    );

    Ok(map.into_values().filter(|g| g.len() > 1).collect())
}

/// Placeholder for focus-based culling; returns empty list until implemented.
fn find_blurry(dir: &Path, _threshold: f64) -> Result<Vec<PathBuf>> {
    // TODO: implement focus/blurriness detection (e.g. variance of Laplacian)
    print!(
        "🔬 Measuring focus or images in {}... (not yet implemented)",
        dir.display()
    );
    Ok(vec![])
}

/// Get the creation time (or modification time) of a file, falling back to UNIX_EPOCH on error
fn get_timestamp(path: &PathBuf) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.created())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

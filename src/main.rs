use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use image::ImageReader;
use image_hasher::{HashAlg, HasherConfig};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
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

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    auto_confirm: bool,
    selection_strategy: SelectionStrategy,
    excluded_dirs: Vec<String>,
    duplicates_hash_threshold: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_confirm: false,
            selection_strategy: SelectionStrategy::Oldest,
            excluded_dirs: vec!["duplicates".to_string()],
            duplicates_hash_threshold: 15,
        }
    }
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
enum SelectionStrategy {
    /// Keep the oldest file (by creation time)
    Oldest,
    /// Keep the newest file (by creation time)
    Newest,
    /// Keep the largest file (by size)
    Largest,
    /// Keep the smallest file (by size)
    Smallest,
}

#[derive(Parser, Debug)]
#[command(
    name = "cullrs",
    version,
    about = "CLI for culling photos with advanced duplicate detection"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Duplicate workflows
    Duplicates {
        #[command(subcommand)]
        command: DupeCMD,
    },

    /// Work with cull history
    History {
        #[command(subcommand)]
        command: HistoryCmd,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCmd,
    },
}

#[derive(Subcommand, Debug)]
enum DupeCMD {
    /// Find and list duplicate groups
    Scan {
        /// Directory to scan
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Hash similarity threshold (0-64, lower = more strict)
        #[arg(long)]
        threshold: Option<u32>,
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
        /// Selection strategy for which file to keep
        #[arg(long, value_enum)]
        strategy: Option<SelectionStrategy>,
        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
        /// Hash similarity threshold (0-64, lower = more strict)
        #[arg(long)]
        threshold: Option<u32>,
    },

    /// Permanently delete duplicate images
    Delete {
        /// Directory to cull
        #[arg(short, long, value_name = "DIR")]
        path: PathBuf,
        /// Selection strategy for which file to keep
        #[arg(long, value_enum)]
        strategy: Option<SelectionStrategy>,
        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
        /// Hash similarity threshold (0-64, lower = more strict)
        #[arg(long)]
        threshold: Option<u32>,
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

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Show current configuration
    Show,
    /// Set configuration values
    Set {
        /// Hash similarity threshold
        #[arg(long)]
        threshold: Option<u32>,
        /// Default selection strategy
        #[arg(long, value_enum)]
        strategy: Option<SelectionStrategy>,
        /// Auto-confirm destructive operations
        #[arg(long)]
        auto_confirm: Option<bool>,
    },
    /// Reset configuration to defaults
    Reset,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config { command } => handle_config_command(command),
        Commands::Duplicates { command } => handle_duplicates_command(command),
        Commands::History { command } => handle_history_command(command),
    }
}

fn handle_config_command(command: ConfigCmd) -> Result<()> {
    let config_path = get_config_path()?;

    match command {
        ConfigCmd::Show => {
            let config = load_config(&config_path).unwrap_or_default();
            println!("Current configuration:");
            println!("  [General] Auto confirm: {}", config.auto_confirm);
            println!(
                "  [General] Selection strategy: {:?}",
                config.selection_strategy
            );
            println!(
                "  [General] Excluded directories: {:?}",
                config.excluded_dirs
            );
            println!(
                "  [Duplicates] Hash threshold: {}",
                config.duplicates_hash_threshold
            );
        }
        ConfigCmd::Set {
            threshold,
            strategy,
            auto_confirm,
        } => {
            let mut config = load_config(&config_path).unwrap_or_default();

            if let Some(t) = threshold {
                if t > 64 {
                    anyhow::bail!("Threshold must be between 0 and 64");
                }
                config.duplicates_hash_threshold = t;
            }
            if let Some(s) = strategy {
                config.selection_strategy = s;
            }
            if let Some(ac) = auto_confirm {
                config.auto_confirm = ac;
            }

            save_config(&config_path, &config)?;
            println!("Configuration updated!");
        }
        ConfigCmd::Reset => {
            let config = Config::default();
            save_config(&config_path, &config)?;
            println!("Configuration reset to defaults!");
        }
    }
    Ok(())
}

fn handle_duplicates_command(command: DupeCMD) -> Result<()> {
    let config = load_config(&get_config_path()?).unwrap_or_default();

    match command {
        DupeCMD::Scan { path, threshold } => {
            validate_directory(&path)?;
            println!("▶ Scanning for duplicates in: {}", path.display());

            let threshold = threshold.unwrap_or(config.duplicates_hash_threshold);
            let groups = find_duplicates(&path, threshold)?;
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

        DupeCMD::Cull {
            path,
            dry_run,
            target_dir,
            strategy,
            force,
            threshold,
        } => {
            validate_directory(&path)?;

            let target_dir = target_dir.unwrap_or_else(|| path.join("duplicates"));
            validate_target_directory(&path, &target_dir)?;

            if !force && !config.auto_confirm && !dry_run {
                if !confirm_action(&format!("Move duplicates to '{}'?", target_dir.display()))? {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }

            println!("▶ Culling duplicates in: {}", path.display());
            let threshold = threshold.unwrap_or(config.duplicates_hash_threshold);
            let mut groups = find_duplicates(&path, threshold)?;
            if groups.is_empty() {
                println!("No duplicates found.");
                return Ok(());
            }

            let selection_strategy = strategy.unwrap_or(config.selection_strategy);
            for group in &mut groups {
                sort_group_by_strategy(group, &selection_strategy);
            }

            if !dry_run {
                fs::create_dir_all(&target_dir)
                    .with_context(|| format!("Failed to create directory {:?}", target_dir))?;
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
                            target_dir.display()
                        );
                    } else {
                        let dest = get_unique_destination(&target_dir, dup)?;
                        fs::rename(dup, &dest)
                            .with_context(|| format!("Failed to move {:?} → {:?}", dup, dest))?;
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

        DupeCMD::Delete {
            path,
            strategy,
            force,
            threshold,
        } => {
            validate_directory(&path)?;

            if !force && !config.auto_confirm {
                if !confirm_action("Permanently delete duplicate files? This cannot be undone!")? {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }

            println!("▶ Deleting duplicates in: {}", path.display());
            let threshold = threshold.unwrap_or(config.duplicates_hash_threshold);
            let mut groups = find_duplicates(&path, threshold)?;
            if groups.is_empty() {
                println!("No duplicates found.");
                return Ok(());
            }

            let selection_strategy = strategy.unwrap_or(config.selection_strategy);
            for group in &mut groups {
                sort_group_by_strategy(group, &selection_strategy);
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
    }
    Ok(())
}

fn handle_history_command(command: HistoryCmd) -> Result<()> {
    match command {
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
    }
    Ok(())
}

// Enhanced image detection using file headers when possible
fn is_image_file(path: &Path) -> bool {
    // First try to read the file header to detect image type
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0; 12];
        if file.read(&mut buffer).is_ok() {
            // Check for common image file signatures
            if buffer.starts_with(&[0xFF, 0xD8, 0xFF]) {
                // JPEG
                return true;
            }
            if buffer.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
                // PNG
                return true;
            }
            if buffer.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
                // GIF
                return true;
            }
            if buffer.starts_with(&[0x42, 0x4D]) {
                // BMP
                return true;
            }
            if buffer.starts_with(&[0x49, 0x49, 0x2A, 0x00])
                || buffer.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
            {
                // TIFF
                return true;
            }
        }
    }

    // Fallback to extension check
    let allowed_exts = [
        "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "raw", "cr2", "nef", "arw",
    ];
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        allowed_exts.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

fn scan_directory(dir: &Path) -> Result<Vec<PathBuf>> {
    let config = load_config(&get_config_path()?).unwrap_or_default();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} {msg} [{elapsed_precise}]",
    )?);
    pb.set_message("Scanning for images…");
    pb.enable_steady_tick(Duration::from_millis(100));

    let mut images = Vec::new();
    let mut file_count = 0;

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            if let Some(name) = e.file_name().to_str() {
                !config.excluded_dirs.iter().any(|excluded| name == excluded)
            } else {
                true
            }
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() {
            file_count += 1;
            if is_image_file(path) {
                images.push(path.to_path_buf());
            }
        }

        if file_count % 100 == 0 {
            pb.set_message(format!(
                "Scanned {} files, found {} images",
                file_count,
                images.len()
            ));
        }
        pb.tick();
    }

    pb.finish_with_message(format!(
        "Scan complete: {} images found from {} files",
        images.len(),
        file_count
    ));
    Ok(images)
}

fn find_duplicates(dir: &Path, threshold: u32) -> Result<Vec<Vec<PathBuf>>> {
    let images = scan_directory(dir)?;
    if images.is_empty() {
        return Ok(vec![]);
    }

    println!("▶ Parallel hashing {} images…", images.len());

    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::Gradient) // More robust than Mean for detecting similar images
        .to_hasher();

    let pb = ProgressBar::new(images.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{bar:40.cyan/blue} {pos:>7}/{len:7} {msg} [{elapsed_precise}]",
    )?);
    pb.set_message("Hashing images");

    let hashes: Vec<(u64, PathBuf)> = benchmark("hashing all images", || {
        images
            .par_iter()
            .map(|path| -> Result<(u64, PathBuf)> {
                let result = ImageReader::open(path)
                    .with_context(|| format!("Failed to open {:?}", path))?
                    .decode()
                    .with_context(|| format!("Failed to decode {:?}", path))
                    .map(|img| {
                        let hash = hasher.hash_image(&img);
                        (
                            hash.as_bytes()
                                .iter()
                                .fold(0u64, |acc, &b| acc << 8 | b as u64),
                            path.clone(),
                        )
                    });
                pb.inc(1);
                result
            })
            .collect::<Result<_>>()
    })?;

    // pb.finish();
    pb.finish_and_clear();
    println!("▶ Hashing complete");

    // Group similar hashes using Hamming distance
    println!("▶ Grouping similar hashes with threshold {}", threshold);

    let mut groups: Vec<Vec<PathBuf>> = Vec::new();
    let mut used = vec![false; hashes.len()];

    for i in 0..hashes.len() {
        if used[i] {
            continue;
        }

        let mut group = vec![hashes[i].1.clone()];
        used[i] = true;

        for j in (i + 1)..hashes.len() {
            if used[j] {
                continue;
            }

            let distance = hamming_distance(hashes[i].0, hashes[j].0);
            if distance <= threshold {
                group.push(hashes[j].1.clone());
                used[j] = true;
            }
        }

        if group.len() > 1 {
            groups.push(group);
        }
    }

    Ok(groups)
}

fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}

fn sort_group_by_strategy(group: &mut Vec<PathBuf>, strategy: &SelectionStrategy) {
    match strategy {
        SelectionStrategy::Oldest => {
            group.sort_by_key(|p| get_timestamp(p));
        }
        SelectionStrategy::Newest => {
            group.sort_by_key(|p| std::cmp::Reverse(get_timestamp(p)));
        }
        SelectionStrategy::Largest => {
            group.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).map(|m| m.len()).unwrap_or(0)));
        }
        SelectionStrategy::Smallest => {
            group.sort_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(u64::MAX));
        }
    }
}

fn get_unique_destination(target_dir: &Path, source: &Path) -> Result<PathBuf> {
    let file_name = source.file_name().unwrap();
    let mut dest = target_dir.join(file_name);

    if !dest.exists() {
        return Ok(dest);
    }

    let stem = source.file_stem().unwrap().to_string_lossy();
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        dest = target_dir.join(format!("{}_{}{}", stem, counter, ext));
        if !dest.exists() {
            return Ok(dest);
        }
        counter += 1;

        if counter > 9999 {
            anyhow::bail!("Too many files with similar names in target directory");
        }
    }
}

fn validate_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", path.display());
    }
    Ok(())
}

fn validate_target_directory(source: &Path, target: &Path) -> Result<()> {
    if target == source {
        anyhow::bail!("Target directory cannot be the same as source directory");
    }
    if target.starts_with(source) && target != source.join("duplicates") {
        anyhow::bail!("Target directory cannot be a subdirectory of source (except 'duplicates')");
    }
    Ok(())
}

fn confirm_action(message: &str) -> Result<bool> {
    print!("{} [y/N]: ", message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".config").join("cullrs").join("config.json"))
}

fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

fn save_config(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}

fn benchmark<T, F: FnOnce() -> T>(label: &str, f: F) -> T {
    let start = Instant::now();
    let result = f();
    println!("⏱ {} took {:.2?}", label, start.elapsed());
    result
}

fn get_timestamp(path: &PathBuf) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.created())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

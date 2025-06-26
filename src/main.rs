use anyhow::{Result, bail};
use blake3;
use clap::{Parser, Subcommand, ValueEnum};
use image_hasher::{HashAlg, HasherConfig};
use indicatif::{ProgressBar, ProgressStyle};
// use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "cullrs", version, about = "CLI for culling duplicate files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = FileType::All)]
        file_type: FileType,
        #[arg(long)]
        threshold: Option<u32>,
    },
    Cull {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = FileType::All)]
        file_type: FileType,
        #[arg(long)]
        strategy: Option<SelectionStrategy>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        target: Option<PathBuf>,
        #[arg(long)]
        threshold: Option<u32>,
    },
    Delete {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = FileType::All)]
        file_type: FileType,
        #[arg(long)]
        strategy: Option<SelectionStrategy>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        threshold: Option<u32>,
    },
}

#[derive(ValueEnum, Clone, PartialEq, Eq)]
enum FileType {
    All,
    Image,
}

#[derive(ValueEnum, Clone)]
enum SelectionStrategy {
    Oldest,
    Newest,
    Largest,
    Smallest,
}

/// A generic “file hasher” interface
trait Hasher {
    /// Hash a single file to a 64-bit code
    fn hash(&self, path: &Path) -> Result<u64>;
    /// Decide if two hash codes count as “duplicates”
    fn is_duplicate(&self, a: u64, b: u64) -> bool;
}

/// Exact (byte-wise) hashing via BLAKE3
struct ExactHasher;
impl Hasher for ExactHasher {
    fn hash(&self, path: &Path) -> Result<u64> {
        let mut f = fs::File::open(path)?;
        let mut h = blake3::Hasher::new();
        io::copy(&mut f, &mut h)?;
        let out = h.finalize();
        // fold 32-byte digest into one u64
        let mut code = 0u64;
        for &b in out.as_bytes().iter().take(8) {
            code = (code << 8) | (b as u64);
        }
        Ok(code)
    }

    fn is_duplicate(&self, a: u64, b: u64) -> bool {
        a == b
    }
}

/// Perceptual image hasher
struct ImageHasher {
    inner: image_hasher::Hasher,
    threshold: u32,
}
impl ImageHasher {
    fn new(threshold: u32) -> Self {
        let h = HasherConfig::new().hash_alg(HashAlg::Gradient).to_hasher();
        Self {
            inner: h,
            threshold,
        }
    }
}
impl Hasher for ImageHasher {
    fn hash(&self, path: &Path) -> Result<u64> {
        let img = image::ImageReader::open(path)?.decode()?;
        let bits = self.inner.hash_image(&img);
        // fold 8 bytes into u64
        let mut code = 0u64;
        for &b in bits.as_bytes().iter().take(8) {
            code = (code << 8) | (b as u64);
        }
        Ok(code)
    }

    fn is_duplicate(&self, a: u64, b: u64) -> bool {
        (a ^ b).count_ones() <= self.threshold
    }
}

/// Discover files
fn discover_files(dir: &PathBuf, file_type: &FileType) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            if *file_type == FileType::Image {
                is_image_file(e.path())
            } else {
                true
            }
        })
        .map(|e| e.into_path())
        .collect()
}

/// Generic duplicate-finder using our Hasher trait
fn find_duplicates(files: Vec<PathBuf>, hasher: &dyn Hasher) -> Result<Vec<Vec<PathBuf>>> {
    // 1) set up a progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} {msg}").unwrap());

    // 2) hash all files (updating the bar)
    let mut hashes = Vec::with_capacity(files.len());
    for f in files {
        let code = hasher.hash(&f)?;
        hashes.push((code, f));
        pb.inc(1);
    }
    pb.finish_and_clear();

    // 3) group “duplicate” codes together
    let mut used = vec![false; hashes.len()];
    let mut groups = Vec::new();
    for i in 0..hashes.len() {
        if used[i] {
            continue;
        }
        let (code_i, ref path_i) = hashes[i];
        used[i] = true;

        let mut group = vec![path_i.clone()];
        for j in (i + 1)..hashes.len() {
            if used[j] {
                continue;
            }
            let (code_j, ref path_j) = hashes[j];
            if hasher.is_duplicate(code_i, code_j) {
                group.push(path_j.clone());
                used[j] = true;
            }
        }
        if group.len() > 1 {
            groups.push(group);
        }
    }

    Ok(groups)
}

/// Choose which file to keep
fn select_keep(group: &[PathBuf], strategy: &SelectionStrategy) -> PathBuf {
    let mut g = group.to_vec();
    match strategy {
        SelectionStrategy::Oldest => {
            g.sort_by_key(|p| get_metadata_time(p));
        }
        SelectionStrategy::Newest => {
            g.sort_by_key(|p| std::cmp::Reverse(get_metadata_time(p)));
        }
        SelectionStrategy::Largest => {
            g.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).map(|m| m.len()).unwrap_or(0)));
        }
        SelectionStrategy::Smallest => {
            g.sort_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(u64::MAX));
        }
    }
    g.into_iter().next().unwrap()
}

fn get_metadata_time(path: &PathBuf) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.created())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("jpg")
            | Some("jpeg")
            | Some("png")
            | Some("gif")
            | Some("bmp")
            | Some("tiff")
            | Some("webp")
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            path,
            file_type,
            threshold,
        } => {
            let files = discover_files(&path, &file_type);
            if files.is_empty() {
                println!("No files found in {}", path.display());
                return Ok(());
            }
            let th = threshold.unwrap_or(0);
            let use_fuzzy = file_type == FileType::Image && th > 0;
            if use_fuzzy {
                println!(
                    "✨ Image Mode: Scanning {} files for duplicates. Threshold: {}",
                    files.len(),
                    th
                );
            } else {
                println!(
                    "✨ File Mode: Scanning {} files for exact duplicates.",
                    files.len()
                );
            }

            let hasher: Box<dyn Hasher> = if use_fuzzy {
                Box::new(ImageHasher::new(th))
            } else {
                Box::new(ExactHasher)
            };
            let groups = find_duplicates(files, hasher.as_ref())?;

            if groups.is_empty() {
                println!("No duplicates found.");
            } else {
                println!("Found {} duplicate groups:", groups.len());
                for (i, group) in groups.iter().enumerate() {
                    println!(" Group {}:", i + 1);
                    for f in group {
                        println!("   ▶ {}", f.display());
                    }
                }
            }
        }

        Commands::Cull {
            path,
            file_type,
            strategy,
            dry_run,
            force,
            target,
            threshold,
        } => {
            let files = discover_files(&path, &file_type);
            let th = threshold.unwrap_or(0);
            let use_fuzzy = file_type == FileType::Image && th > 0;

            let hasher: Box<dyn Hasher> = if use_fuzzy {
                Box::new(ImageHasher::new(th))
            } else {
                Box::new(ExactHasher)
            };
            let groups = find_duplicates(files, hasher.as_ref())?;

            let strat = strategy.unwrap_or(SelectionStrategy::Oldest);
            let tgt = target.unwrap_or_else(|| path.join("duplicates"));

            if !force {
                print!("Move duplicates to '{}' [y/N]: ", tgt.display());
                io::stdout().flush()?;
                let mut ans = String::new();
                io::stdin().read_line(&mut ans)?;
                if !matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("Aborted");
                    return Ok(());
                }
            }
            fs::create_dir_all(&tgt)?;

            for (i, g) in groups.into_iter().enumerate() {
                let keep = select_keep(&g, &strat);
                println!("\n✨ Group {}: Keeping -> {}", i + 1, keep.display());
                for f in g.iter().filter(|p| *p != &keep) {
                    let dest = get_unique_destination(&tgt, f)?;
                    if dry_run {
                        println!(" [dry] mv {} -> {}", f.display(), dest.display());
                    } else {
                        fs::rename(f, &dest)?;
                        println!("   mv {} -> {}", f.display(), dest.display());
                    }
                }
            }
            println!("Done culling.");
        }

        Commands::Delete {
            path,
            file_type,
            strategy,
            force,
            threshold,
        } => {
            let files = discover_files(&path, &file_type);
            let th = threshold.unwrap_or(0);
            let use_fuzzy = file_type == FileType::Image && th > 0;

            let hasher: Box<dyn Hasher> = if use_fuzzy {
                Box::new(ImageHasher::new(th))
            } else {
                Box::new(ExactHasher)
            };
            let groups = find_duplicates(files, hasher.as_ref())?;

            let strat = strategy.unwrap_or(SelectionStrategy::Oldest);
            if !force {
                print!("Delete duplicates permanently? [y/N]: ");
                io::stdout().flush()?;
                let mut ans = String::new();
                io::stdin().read_line(&mut ans)?;
                if !matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("Aborted");
                    return Ok(());
                }
            }
            for (i, g) in groups.into_iter().enumerate() {
                let keep = select_keep(&g, &strat);
                println!("\n🗑️  Group {}: Keeping -> {}", i + 1, keep.display());
                for f in g.iter().filter(|p| *p != &keep) {
                    fs::remove_file(f)?;
                    println!("   rm {}", f.display());
                }
            }
            println!("Done deleting.");
        }
    }

    Ok(())
}

/// Generate a unique path in `target` for `source` by appending `_1`, `_2`, … if needed
fn get_unique_destination(target: &PathBuf, source: &PathBuf) -> Result<PathBuf> {
    let file_name = source.file_name().unwrap();
    let dest = target.join(file_name);
    if !dest.exists() {
        return Ok(dest);
    }

    let stem = source.file_stem().unwrap().to_string_lossy();
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for idx in 1.. {
        let candidate = target.join(format!("{}_{}{}", stem, idx, ext));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!("Too many duplicates in target");
}

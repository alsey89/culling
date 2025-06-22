# cullrs

`cullrs` is a Rust-based CLI tool for scanning, detecting, and culling undesirable photos (currently supports duplicates with future support for other analysis passes). It provides a clear workflow for finding duplicates, previewing culls, executing moves or deletions, and managing a unified history+restore interface.

---

## 🚀 Features

- **Scan for duplicates**: Identify groups of files with identical image hashes.
- **Cull (move)**: Move duplicate files into a dedicated `duplicates/` folder (with dry-run support).
- **Delete**: Permanently remove duplicate files.
- **History**: Record every cull or delete action in a JSONL log (`.history.jsonl`).
- **Unified history & restore**: View history and restore moved files via subcommands under `history`.

### Future Extensions

- Additional analysis passes (e.g., focus/blurriness, face detection, exposure checks).
- GUI or TUI front-end (Tauri, ratatui).

---

## ⚙️ Prerequisites

- Rust toolchain (1.70+)
- `cargo` (comes with Rust)

Optional libraries (pulled via Cargo.toml):

- `anyhow`, `clap`, `chrono`
- `image`, `image-hasher`
- `walkdir`, `indicatif`, `rayon`
- `serde`, `serde_json`

---

## 📥 Installation

You can install `cullrs` locally or globally:

### From crates.io (future)

```sh
cargo install cullrs
```

### From source

```sh
git clone https://github.com/alsey89/culling.git
cd cullrs
cargo build --release
# Optionally copy the binary into your $PATH:
cp target/release/cullrs ~/.cargo/bin/
```

Run `cullrs --version` to verify installation.

---

## 📝 Usage Overview

```text
USAGE:
  cullrs <COMMAND> [OPTIONS]

COMMANDS:
  duplicates   Duplicate workflows (scan, cull, delete)
  history      Manage cull history (list, restore)
  help         Print this message or the help of the given subcommand(s)
```

---

## 📂 `duplicates` Subcommands

All duplicate workflows are grouped under:

```sh
cullrs duplicates <SUBCOMMAND> [OPTIONS] <DIR>
```

### 1. Scan for duplicate groups

List all groups of images that share the same hash (no filesystem changes).

```sh
cullrs duplicates scan --path ./photos/
```

**Output example**:

```
▶ Scanning for duplicates in: photos/
Found 2 duplicate group(s):
 Group 1:
   ▶ photos/img001.jpg
   ▶ photos/img001_copy.jpg
 Group 2:
   ▶ photos/vacation1.png
   ▶ photos/vacation1_edited.png
```

### 2. Cull (move) duplicates

Move all but the oldest file in each group into a `duplicates/` folder.

```sh
# Preview only (dry run):
cullrs duplicates cull --path ./photos/ --dry-run

# Actually move duplicates:
cullrs duplicates cull --path ./photos/
```

- **`--dry-run`**: Show what would be moved without touching files.
- **`--target-dir <DIR>`**: Override default `./photos/duplicates/` output directory.

### 3. Delete duplicates

Permanently remove all but the oldest file in each duplicate group.

```sh
cullrs duplicates delete --path ./photos/
```

---

## 📜 `history` Subcommands

The history command now encapsulates both listing and restoring from the `.history.jsonl` log:

```sh
cullrs history <SUBCOMMAND> [OPTIONS] --path <DIR>
```

### 1. List history records

Show a chronological list of every cull or delete action.

```sh
cullrs history list --path ./photos/
```

**Output example**:

```
🗂️  Cull History:
[0] 2025-06-22T14:30:00Z
     kept: photos/img001.jpg
     culled: ["photos/img001_copy.jpg"]
     action: moved
[1] 2025-06-22T14:32:10Z
     kept: photos/img002.png
     culled: ["photos/img002_copy1.png","photos/img002_copy2.png"]
     action: deleted
```

### 2. Restore moved files

Recover files that were moved (but not those deleted) from a specific record or all records at once.

```sh
# Restore the last record:
cullrs history restore --path ./photos/

# Restore a specific record index:
cullrs history restore --path ./photos/ --record 0

# Restore all records:
cullrs history restore --path ./photos/ --all
```

---

## 🛠️ Internals & Tips

- **History file**: Stored at `<dir>/.history.jsonl`, one JSON record per cull group.
- **Hash algorithm**: Uses mean-hash (`image-hasher`) for quick similarity.
- **Parallelism**: Hashing is done in parallel (`rayon`).

---

## 🤝 Contributing

Contributions welcome! Please:

1. Fork the repo
2. Create a feature branch
3. Open a PR with tests and updated docs

---

## 📄 License

[MIT](LICENSE)

# Darwin: Photo Culler CLI

Darwin is a command-line tool for scanning, analyzing, and culling photos in bulk.

## Features

- **Scan** directories for image files (jpg, jpeg, png, gif, bmp, tiff).
- **Analyze** images for duplicates (mean-hash). future: face detection and focus measurement.
- **Cull** images by moving or deleting duplicates (future: out of focus, bad exposure, etc.) with dry-run mode.
- **History** tracking of cull actions in a JSONL log.
- **Restore** moved files from history records.

## Installation

1. Ensure you have Rust and Cargo installed ([https://rustup.rs](https://rustup.rs)).
2. Clone this repository:

   ```sh
   git clone https://github.com/yourusername/darwin.git
   cd darwin
   ```

3. Build and install:

   ```sh
   cargo install --path .
   ```

4. Confirm installation:

   ```sh
   darwin --version
   ```

## Usage

Run `darwin <COMMAND> --help` for details on any command.

### Scan

Scan a directory and list all supported image files:

```sh
darwin scan -p /path/to/photos
```

### Analyze

Perform analysis passes on the image set. At least one analysis flag is required.

- **--duplicates**: find duplicate groups
- **--faces**: detect faces (not yet implemented)
- **--focus**: measure focus (not yet implemented)

Example:

```sh
darwin analyze -p /path/to/photos --duplicates
```

### Cull

Cull images by a single chosen method (duplicates or focus). Supports dry-run and deletion.

#### Duplicate culling

```sh
darwin cull -p /path/to/photos duplicates --dry
# or to actually move duplicates:
# darwin cull -p /path/to/photos duplicates
```

Options:

- `--dry`: show actions without changing files
- `--delete`: permanently delete instead of moving
- `--target-dir <DIR>`: custom destination directory for moved files

#### Focus culling

```sh
darwin cull -p /path/to/photos focus --min-focus 150.0
```

Options same as above.

### History

Display the history of all cull actions:

```sh
darwin history -p /path/to/photos
```

### Restore

Restore moved files from history. By default, restores the most recent record.

```sh
# Restore the last record:

darwin restore -p /path/to/photos

# Restore a specific record index:

darwin restore -p /path/to/photos --record 2

# Restore all records:

darwin restore -p /path/to/photos --all
```

## Files & Logging

- Cull history is stored in `.darwin_history.jsonl` inside the target directory.
- Each entry includes a timestamp, retained file, culled files list, and action type.

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature-name`)
3. Commit your changes (`git commit -m "Add feature"`)
4. Push to the branch (`git push origin feature-name`)
5. Open a pull request

## License

MIT License © alsey89

# Cullrs Cache Directory Structure

## Overview

Cullrs stores cached data (thumbnails, metadata, etc.) in a `.cullrs` hidden directory within your project's output directory (or source directory if no output directory is specified).

## Directory Structure

```
your-project-output-directory/
├── .cullrs/                    # Hidden cache directory
│   ├── thumbnails/            # Generated thumbnails
│   │   ├── ast_123abc.jpg     # Thumbnail for asset ID ast_123abc
│   │   ├── ast_456def.jpg     # Thumbnail for asset ID ast_456def
│   │   └── ...
│   └── metadata/              # Future: cached metadata
└── your-processed-images/     # Your actual processed images
```

## Benefits of This Approach

1. **Project Association**: Cache files are stored alongside your project, making them easy to find and manage
2. **Persistence**: Unlike system temp directories, these files persist until you delete them
3. **Portability**: If you move your project directory, the cache moves with it
4. **User Control**: You can easily delete the `.cullrs` directory to clear the cache
5. **Version Control**: The `.cullrs` directory can be added to `.gitignore` to exclude cache files from version control

## Recommended .gitignore Entry

If your project is under version control, add this line to your `.gitignore` file:

```gitignore
# Cullrs cache directory
.cullrs/
```

## Cache Management

- **Automatic Creation**: The `.cullrs` directory is created automatically when needed
- **Manual Cleanup**: You can safely delete the `.cullrs` directory to clear all cached data
- **Regeneration**: Thumbnails will be regenerated automatically if they're missing or outdated

## File Sizes

- Thumbnails are stored as JPEG files with 85% quality
- Maximum thumbnail size is 512px on the longest side
- Typical thumbnail file size: 20-100KB depending on image complexity

## Privacy and Security

- The `.cullrs` directory only contains derived data (thumbnails, metadata)
- No original images are copied or modified
- Cache files can be safely deleted without affecting your original images

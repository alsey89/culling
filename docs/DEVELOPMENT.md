# Development Setup Guide

Welcome to Cullrs development! This guide will help you set up your local development environment.

## Prerequisites

### Required Software

- **Rust** (latest stable) - [Install via rustup](https://rustup.rs/)
- **Node.js** (v18 or higher) - [Download from nodejs.org](https://nodejs.org/)
- **Git** - [Install Git](https://git-scm.com/downloads)

### Platform-Specific Dependencies

#### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install system dependencies
brew install pkg-config
```

#### Linux (Ubuntu/Debian)

```bash
# Install build essentials
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# Install Tauri dependencies
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev librsvg2-dev
```

#### Windows

```bash
# Install Visual Studio Build Tools or Visual Studio Community
# Download from: https://visualstudio.microsoft.com/downloads/

# Install WebView2 (usually pre-installed on Windows 11)
```

---

## Quick Setup

### 1. Clone the Repository

```bash
git clone https://github.com/alsey89/cullrs.git
cd cullrs
```

### 2. Install Dependencies

```bash
# Install JavaScript dependencies
npm install

# Install Tauri CLI globally (optional but recommended)
cargo install tauri-cli

# Or use via npm
npm install -g @tauri-apps/cli
```

### 3. Run Development Server

```bash
# Start the development server with hot reload
npm run tauri dev

# Alternative: use cargo directly
cargo tauri dev
```

This will:

- Build the Rust backend
- Start the Vue frontend with hot reload
- Open the application window
- Enable live reloading for both frontend and backend changes

---

## Project Structure

```
cullrs/
├── app/                    # Vue 3 + Nuxt frontend
│   ├── components/         # Vue components
│   ├── pages/             # Route pages
│   ├── stores/            # Pinia state management
│   └── composables/       # Vue composables
├── src-tauri/             # Rust backend
│   ├── src/               # Rust source code
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── docs/                  # Documentation
├── package.json           # Node.js dependencies and scripts
└── README.md              # Project overview
```

---

## Development Workflow

### Frontend Development

The frontend uses Vue 3 with Nuxt, TypeScript, and Tailwind CSS.

```bash
# Run frontend only (useful for UI work)
npm run dev

# Type checking
npm run typecheck

# Linting
npm run lint
npm run lint:fix

# Testing
npm run test
npm run test:watch
```

### Backend Development

The backend is written in Rust using the Tauri framework.

```bash
# Run Rust tests
cd src-tauri
cargo test

# Check Rust code
cargo check

# Format Rust code
cargo fmt

# Lint Rust code
cargo clippy
```

### Full Application Development

```bash
# Start the full application with hot reload
npm run tauri dev

# Build for production
npm run tauri build

# Generate Tauri icons
npm run tauri icon
```

---

## Code Style & Conventions

### Frontend (TypeScript/Vue)

- **Formatter**: Prettier
- **Linter**: ESLint with Vue/TypeScript rules
- **Component Naming**: PascalCase for component files
- **Composition API**: Use `<script setup>` syntax
- **State Management**: Pinia stores with composable pattern

Example component structure:

```vue
<template>
  <div class="component-name">
    <!-- Template content -->
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";

interface Props {
  title: string;
  count?: number;
}

const props = withDefaults(defineProps<Props>(), {
  count: 0,
});

const emit = defineEmits<{
  update: [value: string];
}>();

// Component logic here
</script>

<style scoped>
/* Component styles */
</style>
```

### Backend (Rust)

- **Formatter**: `cargo fmt`
- **Linter**: `cargo clippy`
- **Naming**: snake_case for functions and variables, PascalCase for types
- **Error Handling**: Use `Result<T, E>` and proper error types
- **Documentation**: Document public APIs with `///` comments

Example service structure:

```rust
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub path: PathBuf,
    // ... other fields
}

#[command]
pub async fn scan_directory(path: String) -> Result<Vec<Asset>, String> {
    // Implementation here
    Ok(vec![])
}
```

---

## Database Development

### Migrations

```bash
# Create a new migration
cd src-tauri
cargo run -- migrate create <migration_name>

# Run migrations
cargo run -- migrate run

# Rollback migrations
cargo run -- migrate rollback
```

### Schema Changes

1. Create migration file in `src-tauri/migrations/`
2. Update data models in `src-tauri/src/core/models/`
3. Update repository layer if needed
4. Test migration with sample data

---

## Testing

### Frontend Tests

```bash
# Unit tests with Vitest
npm run test

# Component tests
npm run test:components

# E2E tests with Playwright
npm run test:e2e

# Coverage report
npm run test:coverage
```

### Backend Tests

```bash
cd src-tauri

# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Doc tests
cargo test --doc

# Test with coverage
cargo tarpaulin
```

### Performance Testing

```bash
# Benchmark tests
cd src-tauri
cargo bench

# Memory profiling
cargo run --release --features profiling
```

---

## Debugging

### Frontend Debugging

- **Browser DevTools**: Open with F12 in the app window
- **Vue DevTools**: Install browser extension for Vue debugging
- **VS Code**: Use "Vetur" or "Volar" extension

### Backend Debugging

- **VS Code**: Use "rust-analyzer" extension with launch.json configuration
- **Console Logs**: Use `tracing` crate for structured logging
- **GDB/LLDB**: For low-level debugging

Example launch.json for VS Code:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Tauri Development Debug",
      "cargo": {
        "args": ["build", "--manifest-path=src-tauri/Cargo.toml"],
        "filter": {
          "name": "cullrs",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

---

## Build & Release

### Development Builds

```bash
# Debug build (fast compilation, larger binary)
npm run tauri build -- --debug

# Development build with specific target
npm run tauri build -- --target x86_64-apple-darwin
```

### Production Builds

```bash
# Production build (optimized, smaller binary)
npm run tauri build

# Build for all supported platforms
npm run build:all
```

### Code Signing & Notarization

See platform-specific guides:

- [macOS Signing Guide](docs/signing/macos.md)
- [Windows Signing Guide](docs/signing/windows.md)
- [Linux Packaging Guide](docs/signing/linux.md)

---

## Troubleshooting

### Common Issues

#### Build Failures

**Issue**: Rust compilation errors

```bash
# Solution: Update Rust toolchain
rustup update

# Clear build cache
cargo clean
cd src-tauri && cargo clean
```

**Issue**: Node.js dependency conflicts

```bash
# Solution: Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
```

#### Runtime Issues

**Issue**: Hot reload not working

```bash
# Solution: Restart development server
npm run tauri dev
```

**Issue**: Database migration errors

```bash
# Solution: Reset database and re-run migrations
rm -f src-tauri/data/app.db
npm run tauri dev
```

### Getting Help

1. **Check the logs**: Look in `~/.cullrs/logs/` for error logs
2. **Search issues**: Check [GitHub Issues](https://github.com/alsey89/cullrs/issues)
3. **Ask questions**: Use [GitHub Discussions](https://github.com/alsey89/cullrs/discussions)
4. **Debug mode**: Run with `RUST_LOG=debug npm run tauri dev`

---

## Contributing

### Before Submitting a PR

1. **Run tests**: Ensure all tests pass
2. **Check formatting**: Run linters and formatters
3. **Update docs**: Add/update documentation for new features
4. **Test manually**: Verify the feature works as expected
5. **Write tests**: Add appropriate test coverage

### PR Guidelines

- Clear, descriptive title and description
- Reference related issues with "Fixes #123"
- Include screenshots for UI changes
- Keep PRs focused and reasonably sized
- Update CHANGELOG.md if applicable

---

Ready to start developing? Run `npm run tauri dev` and start building! 🚀

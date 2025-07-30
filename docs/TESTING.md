# Testing Guide

Comprehensive testing strategy for Cullrs photo culling application.

## Overview

Our testing approach covers multiple layers:

- **Unit Tests**: Individual functions and components
- **Integration Tests**: Service interactions and data flow
- **End-to-End Tests**: Complete user workflows
- **Performance Tests**: Speed and memory benchmarks
- **Manual Tests**: User experience validation

---

## Frontend Testing (Vue 3 + TypeScript)

### Setup

```bash
# Install testing dependencies (already included)
npm install

# Run all frontend tests
npm run test

# Run tests in watch mode
npm run test:watch

# Generate coverage report
npm run test:coverage
```

### Unit Testing with Vitest

**Framework**: Vitest (fast, modern alternative to Jest)
**Location**: `app/tests/` and `*.test.ts` files

Example component test:

```typescript
// app/tests/components/AssetThumbnail.test.ts
import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import AssetThumbnail from "~/components/culling/AssetThumbnail.vue";

describe("AssetThumbnail", () => {
  const mockAsset = {
    id: "ast_001",
    path: "/test/image.jpg",
    width: 1920,
    height: 1080,
    size: 2048000,
  };

  it("renders asset information correctly", () => {
    const wrapper = mount(AssetThumbnail, {
      props: { asset: mockAsset },
    });

    expect(wrapper.find('[data-testid="asset-id"]').text()).toBe("ast_001");
    expect(wrapper.find('[data-testid="resolution"]').text()).toBe("1920×1080");
  });

  it("emits selection event when clicked", async () => {
    const wrapper = mount(AssetThumbnail, {
      props: { asset: mockAsset },
    });

    await wrapper.find('[data-testid="thumbnail"]').trigger("click");
    expect(wrapper.emitted("select")).toBeTruthy();
    expect(wrapper.emitted("select")?.[0]).toEqual([mockAsset.id]);
  });

  it("displays loading state correctly", () => {
    const wrapper = mount(AssetThumbnail, {
      props: { asset: mockAsset, loading: true },
    });

    expect(wrapper.find('[data-testid="loading-spinner"]').exists()).toBe(true);
  });
});
```

### Composable Testing

```typescript
// app/tests/composables/useTauri.test.ts
import { describe, it, expect, vi } from "vitest";
import { useTauri } from "~/composables/useTauri";

// Mock Tauri API
vi.mock("@tauri-apps/api/tauri", () => ({
  invoke: vi.fn(),
}));

describe("useTauri", () => {
  it("creates project correctly", async () => {
    const { invoke } = await import("@tauri-apps/api/tauri");
    const mockProject = { id: "proj_001", name: "Test Project" };

    vi.mocked(invoke).mockResolvedValue(mockProject);

    const { createProject } = useTauri();
    const result = await createProject({ name: "Test Project" });

    expect(invoke).toHaveBeenCalledWith("create_project", {
      config: { name: "Test Project" },
    });
    expect(result).toEqual(mockProject);
  });
});
```

### Store Testing (Pinia)

```typescript
// app/tests/stores/project.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useProjectStore } from "~/stores/project";

describe("Project Store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("initializes with correct default state", () => {
    const store = useProjectStore();

    expect(store.currentProject).toBeNull();
    expect(store.assets).toEqual([]);
    expect(store.groups).toEqual([]);
  });

  it("updates current project", () => {
    const store = useProjectStore();
    const project = { id: "proj_001", name: "Test" };

    store.setCurrentProject(project);
    expect(store.currentProject).toEqual(project);
  });
});
```

---

## Backend Testing (Rust)

### Setup

```bash
cd src-tauri

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_duplicate_detection

# Run tests with coverage
cargo tarpaulin --out html
```

### Unit Testing

**Framework**: Built-in Rust testing with `tokio-test` for async
**Location**: `src-tauri/src/` with `#[cfg(test)]` modules

Example service test:

```rust
// src-tauri/src/core/services/scanner.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_scan_directory() {
        // Setup test directory
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.jpg");
        fs::write(&test_file, b"fake jpg content").unwrap();

        let scanner = ScannerService::new();
        let assets = scanner.scan_paths(&[temp_dir.path().to_path_buf()]).await.unwrap();

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].path, test_file);
    }

    #[test]
    fn test_file_filter() {
        let scanner = ScannerService::new();

        assert!(scanner.is_supported_image("test.jpg"));
        assert!(scanner.is_supported_image("test.PNG"));
        assert!(!scanner.is_supported_image("test.txt"));
        assert!(!scanner.is_supported_image("test"));
    }
}
```

### Integration Testing

```rust
// src-tauri/tests/integration_test.rs
use cullrs::core::services::{ScannerService, SimilarityService};
use tempfile::TempDir;

#[tokio::test]
async fn test_full_duplicate_detection_workflow() {
    // Create test environment
    let temp_dir = TempDir::new().unwrap();

    // Create duplicate files
    let file1 = temp_dir.path().join("image1.jpg");
    let file2 = temp_dir.path().join("image2.jpg");
    let content = include_bytes!("../test_data/sample.jpg");

    std::fs::write(&file1, content).unwrap();
    std::fs::write(&file2, content).unwrap();

    // Scan files
    let scanner = ScannerService::new();
    let mut assets = scanner.scan_paths(&[temp_dir.path().to_path_buf()]).await.unwrap();

    // Compute hashes
    scanner.compute_hashes(&mut assets).await.unwrap();

    // Detect duplicates
    let similarity = SimilarityService::new();
    let groups = similarity.detect_exact_duplicates(&assets);

    // Verify results
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].assets.len(), 2);
    assert_eq!(groups[0].group_type, GroupType::Exact);
    assert_eq!(groups[0].similarity, 100.0);
}
```

### Database Testing

```rust
// src-tauri/tests/database_test.rs
use cullrs::storage::{Database, repositories::ProjectRepository};
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_project_crud() {
    // Create temporary database
    let db_file = NamedTempFile::new().unwrap();
    let db = Database::new(db_file.path()).await.unwrap();

    let repo = ProjectRepository::new(&db);

    // Test create
    let project = Project {
        id: "proj_001".to_string(),
        name: "Test Project".to_string(),
        source_paths: vec!["/test".into()],
        ..Default::default()
    };

    repo.create(&project).await.unwrap();

    // Test read
    let retrieved = repo.get_by_id("proj_001").await.unwrap();
    assert_eq!(retrieved.name, "Test Project");

    // Test update
    let mut updated = retrieved;
    updated.name = "Updated Project".to_string();
    repo.update(&updated).await.unwrap();

    let retrieved = repo.get_by_id("proj_001").await.unwrap();
    assert_eq!(retrieved.name, "Updated Project");

    // Test delete
    repo.delete("proj_001").await.unwrap();
    assert!(repo.get_by_id("proj_001").await.is_err());
}
```

---

## End-to-End Testing (Playwright)

### Setup

```bash
# Install Playwright
npm install -D @playwright/test

# Install browsers
npx playwright install

# Run E2E tests
npm run test:e2e

# Run E2E tests with UI
npm run test:e2e:ui
```

### E2E Test Examples

```typescript
// tests/e2e/project-creation.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Project Creation", () => {
  test("creates new project successfully", async ({ page }) => {
    // Start the application
    await page.goto("/");

    // Navigate to create project
    await page.click('[data-testid="create-project-btn"]');

    // Fill project form
    await page.fill('[data-testid="project-name"]', "My Test Project");
    await page.click('[data-testid="add-folder-btn"]');

    // Mock folder selection (Tauri API)
    await page.evaluate(() => {
      // Mock Tauri folder selection
      window.__TAURI__ = {
        dialog: {
          open: () => Promise.resolve("/Users/test/Photos"),
        },
      };
    });

    await page.click('[data-testid="create-btn"]');

    // Verify project created
    await expect(page.locator('[data-testid="project-title"]')).toHaveText(
      "My Test Project"
    );
    await expect(page.locator('[data-testid="folder-list"]')).toContainText(
      "/Users/test/Photos"
    );
  });

  test("handles scan progress correctly", async ({ page }) => {
    // Setup project and start scan
    await page.goto("/project/test-project");
    await page.click('[data-testid="start-scan-btn"]');

    // Verify progress UI
    await expect(page.locator('[data-testid="progress-bar"]')).toBeVisible();
    await expect(page.locator('[data-testid="scan-status"]')).toContainText(
      "Scanning..."
    );

    // Wait for scan completion
    await expect(page.locator('[data-testid="scan-complete"]')).toBeVisible({
      timeout: 30000,
    });

    // Verify results
    await expect(page.locator('[data-testid="asset-count"]')).not.toHaveText(
      "0"
    );
  });
});
```

### Tauri-Specific E2E Testing

```typescript
// tests/e2e/tauri-integration.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Tauri Integration", () => {
  test("file operations work correctly", async ({ page }) => {
    await page.goto("/");

    // Mock Tauri file operations
    await page.addInitScript(() => {
      window.__TAURI__ = {
        tauri: {
          invoke: async (cmd: string, args: any) => {
            if (cmd === "scan_project") {
              return { assets: [], groups: [] };
            }
            if (cmd === "create_project") {
              return { id: "proj_001", name: args.config.name };
            }
            return null;
          },
        },
      };
    });

    // Test Tauri commands
    await page.click('[data-testid="scan-btn"]');

    // Verify Tauri integration
    const result = await page.evaluate(() => {
      return window.__TAURI__.tauri.invoke("scan_project", {
        projectId: "test",
      });
    });

    expect(result).toEqual({ assets: [], groups: [] });
  });
});
```

---

## Performance Testing

### Frontend Performance

```typescript
// tests/performance/ui-performance.spec.ts
import { test, expect } from "@playwright/test";

test.describe("UI Performance", () => {
  test("grid renders large datasets efficiently", async ({ page }) => {
    await page.goto("/project/large-dataset");

    // Measure initial load time
    const startTime = Date.now();
    await page.waitForSelector('[data-testid="asset-grid"]');
    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(2000); // Should load within 2 seconds

    // Test scrolling performance
    const scrollStart = Date.now();
    await page.mouse.wheel(0, 1000);
    await page.waitForTimeout(100);
    const scrollTime = Date.now() - scrollStart;

    expect(scrollTime).toBeLessThan(100); // Smooth scrolling
  });
});
```

### Backend Performance

```rust
// src-tauri/benches/scanning_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cullrs::core::services::ScannerService;

fn bench_file_scanning(c: &mut Criterion) {
    let scanner = ScannerService::new();

    c.bench_function("scan 1000 files", |b| {
        b.iter(|| {
            // Benchmark scanning performance
            let paths = vec![black_box("/test/photos".into())];
            scanner.scan_paths(&paths)
        })
    });
}

fn bench_hash_computation(c: &mut Criterion) {
    let scanner = ScannerService::new();

    c.bench_function("compute hashes for 100 images", |b| {
        b.iter(|| {
            // Benchmark hashing performance
            let mut assets = create_test_assets(100);
            scanner.compute_hashes(black_box(&mut assets))
        })
    });
}

criterion_group!(benches, bench_file_scanning, bench_hash_computation);
criterion_main!(benches);
```

---

## Manual Testing Checklists

### Critical User Flows

#### Project Creation & Scanning

- [ ] Create new project with source and output folders
- [ ] Validate output folder doesn't conflict with source
- [ ] Start/pause/cancel scanning
- [ ] Verify progress updates correctly
- [ ] Handle large directories (10k+ files)
- [ ] Test with various file types
- [ ] Verify exclude patterns work

#### Duplicate Detection

- [ ] Exact duplicates are correctly grouped
- [ ] Similar images grouped by threshold
- [ ] Suggested keep asset is reasonable
- [ ] No false positives in grouping
- [ ] Performance acceptable for large sets

#### Culling Workflow

- [ ] Grid view displays correctly
- [ ] Compare view works properly
- [ ] Keyboard shortcuts functional
- [ ] Decision state updates immediately
- [ ] Preview output structure correctly

#### Export & Actions

- [ ] Manifest export produces valid JSON/CSV with source → output mappings
- [ ] File copy/hardlink operations work safely to output folder
- [ ] Original files remain completely untouched
- [ ] Integration exports use output paths correctly

### Platform-Specific Testing

#### macOS

- [ ] App bundle launches correctly
- [ ] File permissions requested properly
- [ ] Trash operations use macOS trash
- [ ] Keyboard shortcuts use Cmd key

#### Windows

- [ ] Installer works correctly
- [ ] App launches from Start menu
- [ ] File operations respect Windows permissions
- [ ] Keyboard shortcuts use Ctrl key

#### Linux

- [ ] AppImage runs on various distributions
- [ ] File operations work with different file systems
- [ ] Desktop integration functional

---

## Test Data & Fixtures

### Creating Test Data

```bash
# Generate test images
cd test-data
./generate-test-images.sh

# This creates:
# - Exact duplicates (same content, different names)
# - Similar images (slight variations)
# - Different formats (JPG, PNG, HEIC, etc.)
# - Various resolutions and qualities
```

### Test Image Sets

1. **Small Set** (100 images) - Quick testing
2. **Medium Set** (1,000 images) - Performance testing
3. **Large Set** (10,000 images) - Stress testing
4. **Duplicate Set** - Known duplicates for validation
5. **Similar Set** - Visually similar images for grouping tests

---

## Continuous Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: "18"
      - run: npm ci
      - run: npm run test
      - run: npm run test:e2e

  test-backend:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cd src-tauri && cargo test

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: npm ci
      - run: npm run tauri build
      - run: npm run test:integration
```

---

## Test Coverage Goals

- **Frontend**: >90% line coverage
- **Backend**: >95% line coverage
- **E2E**: Cover all critical user paths
- **Performance**: Benchmark all key operations

### Measuring Coverage

```bash
# Frontend coverage
npm run test:coverage

# Backend coverage
cd src-tauri
cargo tarpaulin --out html

# View reports
open coverage/index.html
```

---

## Debugging Test Failures

### Common Issues

1. **Async timing** - Use proper async/await and timeouts
2. **Mock setup** - Ensure mocks are correctly configured
3. **Test isolation** - Clean up state between tests
4. **Platform differences** - Account for OS-specific behavior

### Debug Tools

- VS Code Test Explorer
- Vitest UI: `npm run test:ui`
- Playwright trace viewer: `npx playwright show-trace`
- Rust test output: `cargo test -- --nocapture`

Ready to write bulletproof tests! 🧪

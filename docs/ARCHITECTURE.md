# Technical Design Document (TDD)

## Cullrs - Photo Culling & Deduplication App

**Version:** 1.0  
**Owner:** Technical Team  
**Last Updated:** 2025-07-29  
**Status:** Draft for Implementation Planning  
**Related:** [Product Requirements Document](README.md)

---

## 1. System Overview

### 1.1 Architecture Principles

- **Privacy by Design**: All processing happens locally, no cloud dependencies
- **Performance First**: Native Rust backend for CPU-intensive operations
- **Cross-Platform**: Single codebase targeting macOS, Windows, Linux
- **Modular Design**: Clean separation between core engine and UI
- **Safety & Reliability**: Non-destructive operations with comprehensive undo

### 1.2 Technology Stack

- **Backend**: Rust (core image processing, file operations, business logic)
- **Frontend**: Tauri + Vue 3 + TypeScript + Nuxt
- **UI Framework**: Tailwind CSS + shadcn/vue components
- **Database**: SQLite (local project storage)
- **Build System**: Tauri CLI + Vite
- **Testing**: Vitest (frontend) + Rust's built-in testing (backend)

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (Tauri WebView)                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │   Vue 3 + Nuxt  │  │  UI Components  │  │  State Mgmt  │ │
│  │   (TypeScript)  │  │  (shadcn/vue)   │  │   (Pinia)    │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────┬───────────────────────────────────┘
                          │ Tauri IPC Commands
┌─────────────────────────▼───────────────────────────────────┐
│                    Backend (Rust)                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  Tauri Commands │  │   Core Engine   │  │   Storage    │ │
│  │   (API Layer)   │  │ (Image Process) │  │  (SQLite)    │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  File System    │  │   Similarity    │  │  Licensing   │ │
│  │   Operations    │  │    Detection    │  │   System     │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Backend Architecture (Rust)

### 3.1 Module Structure

```
src-tauri/src/
├── main.rs                 # Tauri app initialization
├── lib.rs                  # Library exports
│   ├── commands/               # Tauri command handlers
│   ├── mod.rs
│   ├── project.rs          # Project CRUD operations
│   ├── scan.rs             # File scanning & indexing
│   ├── duplicate.rs        # Duplicate detection
│   ├── similarity.rs       # Similarity grouping
│   ├── decisions.rs        # Culling decisions
│   ├── actions.rs          # File copy/hardlink operations
│   ├── export.rs           # Manifest exports
│   ├── ai.rs               # AI analysis commands (Pro)
│   └── licensing.rs        # License validation
├── core/                   # Core business logic
│   ├── mod.rs
│   ├── models/             # Data models
│   │   ├── mod.rs
│   │   ├── asset.rs        # Asset entity
│   │   ├── project.rs      # Project entity
│   │   ├── group.rs        # VariantGroup entity
│   │   └── decision.rs     # Decision entity
│   ├── services/           # Business services
│   │   ├── mod.rs
│   │   ├── scanner.rs      # File system scanning
│   │   ├── hasher.rs       # Content hashing
│   │   ├── similarity.rs   # Perceptual hashing
│   │   ├── exif.rs         # EXIF extraction
│   │   ├── actions.rs      # File copy/hardlink operations
│   │   ├── quality.rs      # Quality analysis (Pro)
│   │   ├── ai/             # AI services (Pro)
│   │   │   ├── mod.rs
│   │   │   ├── quality_analyzer.rs  # AI-powered quality metrics
│   │   │   ├── face_detector.rs     # Face/eye detection
│   │   │   ├── aesthetic_scorer.rs  # Aesthetic scoring
│   │   │   ├── model_manager.rs     # Model loading/management
│   │   │   └── inference.rs         # ONNX inference engine
│   │   └── export.rs       # Manifest generation
│   └── utils/              # Utilities
│       ├── mod.rs
│       ├── paths.rs        # Path handling
│       ├── image.rs        # Image operations
│       └── crypto.rs       # Hashing utilities
├── storage/                # Data persistence
│   ├── mod.rs
│   ├── database.rs         # SQLite connection
│   ├── migrations.rs       # Schema migrations
│   └── repositories/       # Data access
│       ├── mod.rs
│       ├── project.rs
│       ├── asset.rs
│       └── decision.rs
├── licensing/              # License management
│   ├── mod.rs
│   ├── validator.rs        # License validation
│   ├── trial.rs            # Trial management
│   └── features.rs         # Feature gating
└── error.rs                # Error types
```

### 3.2 Key Data Models

```rust
// Asset - Represents a single image file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Asset {
    pub id: String,           // ast_000001
    pub path: PathBuf,
    pub hash: String,         // SHA-256 content hash
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub exif: Option<ExifData>,
    pub quality: Option<QualityMetrics>,
    pub ai_metrics: Option<AIMetrics>,  // Pro features
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// VariantGroup - Collection of similar/duplicate assets
#[derive(Debug, Serialize, Deserialize)]
pub struct VariantGroup {
    pub id: String,           // grp_000001
    pub group_type: GroupType,
    pub similarity: f64,      // 0-100
    pub assets: Vec<Asset>,
    pub suggested_keep: Option<String>, // asset_id
}

// Decision - User's keep/remove choice
#[derive(Debug, Serialize, Deserialize)]
pub struct Decision {
    pub asset_id: String,
    pub state: DecisionState,
    pub reason: ReasonCode,
    pub notes: Option<String>,
    pub decided_at: DateTime<Utc>,
}

// Project - Top-level container
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_paths: Vec<PathBuf>,
    pub output_path: PathBuf,
    pub exclude_patterns: Vec<String>,
    pub file_types: Vec<String>,
    pub scan_status: ScanStatus,
    pub created_at: DateTime<Utc>,
}
```

### 3.3 Core Services

#### Scanner Service

```rust
pub struct ScannerService {
    // Walks directory trees, applies filters, extracts metadata
    pub async fn scan_paths(&self, paths: &[PathBuf]) -> Result<Vec<Asset>>;
    pub async fn compute_hashes(&self, assets: &mut [Asset]) -> Result<()>;
    pub async fn extract_exif(&self, asset: &mut Asset) -> Result<()>;
}
```

#### Similarity Service

```rust
pub struct SimilarityService {
    // Perceptual hashing and grouping
    pub fn compute_perceptual_hash(&self, image_path: &Path) -> Result<String>;
    pub fn group_similar(&self, assets: &[Asset], threshold: f64) -> Vec<VariantGroup>;
    pub fn compute_similarity(&self, hash1: &str, hash2: &str) -> f64;
}
```

#### Action Service

```rust
pub struct ActionService {
    // File operations for copying/hardlinking selected assets to output folder
    pub async fn copy_assets(&self, assets: &[Asset], output_path: &Path) -> Result<Vec<CopyResult>>;
    pub async fn hardlink_assets(&self, assets: &[Asset], output_path: &Path) -> Result<Vec<CopyResult>>;
    pub fn preview_output_structure(&self, assets: &[Asset], output_path: &Path, preserve_structure: bool) -> Result<Vec<OutputMapping>>;
    pub async fn validate_output_path(&self, output_path: &Path, source_paths: &[PathBuf]) -> Result<()>;
}

pub struct CopyResult {
    pub asset_id: String,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

pub struct OutputMapping {
    pub asset_id: String,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
}
```

### 3.4 AI Services (Pro Features)

The AI module provides advanced image analysis capabilities using on-device machine learning models. All AI processing is local and privacy-preserving.

#### Technology Stack

- **ONNX Runtime**: Cross-platform ML inference
- **OpenCV**: Image preprocessing and computer vision
- **Candle**: Rust-native ML framework (alternative)
- **Model Format**: ONNX for cross-platform compatibility

#### AI Module Structure

```rust
// AI service manager
pub struct AIService {
    model_manager: ModelManager,
    quality_analyzer: Option<AIQualityAnalyzer>,
    face_detector: Option<FaceDetector>,
    aesthetic_scorer: Option<AestheticScorer>,
    license_validator: Arc<LicenseValidator>,
}

impl AIService {
    pub async fn new(license_validator: Arc<LicenseValidator>) -> Result<Self> {
        let model_manager = ModelManager::new().await?;

        Ok(Self {
            model_manager,
            quality_analyzer: None,
            face_detector: None,
            aesthetic_scorer: None,
            license_validator,
        })
    }

    pub async fn initialize_models(&mut self) -> Result<()> {
        if self.license_validator.has_pro_features() {
            self.quality_analyzer = Some(
                AIQualityAnalyzer::new(&self.model_manager).await?
            );
            self.face_detector = Some(
                FaceDetector::new(&self.model_manager).await?
            );
            self.aesthetic_scorer = Some(
                AestheticScorer::new(&self.model_manager).await?
            );
        }
        Ok(())
    }

    pub async fn analyze_asset(&self, asset: &mut Asset) -> Result<()> {
        if let Some(analyzer) = &self.quality_analyzer {
            asset.ai_metrics = Some(analyzer.analyze(&asset.path).await?);
        }
        Ok(())
    }
}
```

#### Model Manager

```rust
pub struct ModelManager {
    models_dir: PathBuf,
    loaded_models: HashMap<String, OnnxModel>,
    download_progress: Arc<Mutex<HashMap<String, f64>>>,
}

impl ModelManager {
    pub async fn new() -> Result<Self> {
        let models_dir = app_data_dir()?.join("models");
        fs::create_dir_all(&models_dir).await?;

        Ok(Self {
            models_dir,
            loaded_models: HashMap::new(),
            download_progress: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn ensure_model(&mut self, model_name: &str) -> Result<&OnnxModel> {
        if !self.loaded_models.contains_key(model_name) {
            self.download_and_load_model(model_name).await?;
        }
        Ok(self.loaded_models.get(model_name).unwrap())
    }

    async fn download_and_load_model(&mut self, model_name: &str) -> Result<()> {
        let model_path = self.models_dir.join(format!("{}.onnx", model_name));

        if !model_path.exists() {
            self.download_model(model_name, &model_path).await?;
        }

        let model = OnnxModel::load(&model_path)?;
        self.loaded_models.insert(model_name.to_string(), model);
        Ok(())
    }

    async fn download_model(&self, model_name: &str, path: &Path) -> Result<()> {
        // Download model from secure CDN with progress tracking
        // Models are digitally signed and verified
    }
}
```

#### AI Quality Analyzer

```rust
pub struct AIQualityAnalyzer {
    sharpness_model: Arc<OnnxModel>,
    exposure_model: Arc<OnnxModel>,
    noise_model: Arc<OnnxModel>,
}

impl AIQualityAnalyzer {
    pub async fn new(model_manager: &ModelManager) -> Result<Self> {
        let sharpness_model = model_manager.ensure_model("sharpness_v1").await?;
        let exposure_model = model_manager.ensure_model("exposure_v1").await?;
        let noise_model = model_manager.ensure_model("noise_v1").await?;

        Ok(Self {
            sharpness_model: Arc::new(sharpness_model.clone()),
            exposure_model: Arc::new(exposure_model.clone()),
            noise_model: Arc::new(noise_model.clone()),
        })
    }

    pub async fn analyze(&self, image_path: &Path) -> Result<AIMetrics> {
        let img = self.preprocess_image(image_path).await?;

        let sharpness_score = self.run_inference(&self.sharpness_model, &img).await?;
        let exposure_score = self.run_inference(&self.exposure_model, &img).await?;
        let noise_score = self.run_inference(&self.noise_model, &img).await?;

        Ok(AIMetrics {
            sharpness_score: (sharpness_score * 100.0) as u8,
            exposure_score: (exposure_score * 100.0) as u8,
            noise_score: (noise_score * 100.0) as u8,
            overall_quality: self.compute_overall_quality(
                sharpness_score, exposure_score, noise_score
            ),
            face_metrics: None, // Populated by face detector
            aesthetic_score: None, // Populated by aesthetic scorer
        })
    }

    async fn preprocess_image(&self, image_path: &Path) -> Result<Tensor> {
        // Load, resize, normalize image for model input
        let img = image::open(image_path)?;
        let resized = img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);

        // Convert to RGB tensor with normalization
        let rgb = resized.to_rgb8();
        let tensor = self.image_to_tensor(&rgb)?;
        Ok(tensor)
    }

    async fn run_inference(&self, model: &OnnxModel, input: &Tensor) -> Result<f32> {
        let output = model.run(vec![input.clone()])?;
        Ok(output[0].as_slice::<f32>()?[0])
    }
}
```

#### Face Detection Service

```rust
pub struct FaceDetector {
    detection_model: Arc<OnnxModel>,
    eye_model: Arc<OnnxModel>,
}

impl FaceDetector {
    pub async fn new(model_manager: &ModelManager) -> Result<Self> {
        let detection_model = model_manager.ensure_model("face_detection_v1").await?;
        let eye_model = model_manager.ensure_model("eye_detection_v1").await?;

        Ok(Self {
            detection_model: Arc::new(detection_model.clone()),
            eye_model: Arc::new(eye_model.clone()),
        })
    }

    pub async fn detect_faces(&self, image_path: &Path) -> Result<FaceMetrics> {
        let img = self.preprocess_for_detection(image_path).await?;

        let faces = self.run_face_detection(&img).await?;
        let mut face_analyses = Vec::new();

        for face_bbox in faces {
            let face_crop = self.extract_face_region(&img, &face_bbox)?;
            let eye_analysis = self.analyze_eyes(&face_crop).await?;

            face_analyses.push(FaceAnalysis {
                bbox: face_bbox,
                eyes_open_confidence: eye_analysis.eyes_open_confidence,
                eye_contact_score: eye_analysis.eye_contact_score,
            });
        }

        Ok(FaceMetrics {
            face_count: face_analyses.len() as u32,
            faces: face_analyses,
            primary_face_quality: self.compute_primary_face_quality(&face_analyses),
        })
    }
}
```

#### Data Models for AI

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIMetrics {
    pub sharpness_score: u8,        // 0-100
    pub exposure_score: u8,         // 0-100
    pub noise_score: u8,            // 0-100 (higher = less noise)
    pub overall_quality: u8,        // 0-100 composite score
    pub face_metrics: Option<FaceMetrics>,
    pub aesthetic_score: Option<u8>, // 0-100
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FaceMetrics {
    pub face_count: u32,
    pub faces: Vec<FaceAnalysis>,
    pub primary_face_quality: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FaceAnalysis {
    pub bbox: BoundingBox,
    pub eyes_open_confidence: f32,   // 0.0-1.0
    pub eye_contact_score: f32,      // 0.0-1.0
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

#### AI Command Handlers

```rust
// commands/ai.rs
use tauri::State;

#[tauri::command]
pub async fn analyze_asset_quality(
    asset_id: String,
    state: State<'_, AppState>
) -> Result<AIMetrics, String> {
    let ai_service = &state.ai_service;

    // Check Pro license
    if !state.license_validator.has_pro_features() {
        return Err("AI features require Pro license".to_string());
    }

    let asset = state.asset_repository.get_by_id(&asset_id).await
        .map_err(|e| e.to_string())?;

    ai_service.analyze_quality(&asset.path).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_faces_in_asset(
    asset_id: String,
    state: State<'_, AppState>
) -> Result<FaceMetrics, String> {
    let ai_service = &state.ai_service;

    if !state.license_validator.has_pro_features() {
        return Err("Face detection requires Pro license".to_string());
    }

    let asset = state.asset_repository.get_by_id(&asset_id).await
        .map_err(|e| e.to_string())?;

    ai_service.detect_faces(&asset.path).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_model_download_progress(
    state: State<'_, AppState>
) -> Result<HashMap<String, f64>, String> {
    Ok(state.ai_service.get_download_progress().await)
}
```

---

## 4. Frontend Architecture (Vue 3 + Nuxt)

### 4.1 Directory Structure

```
app/
├── app.vue                 # Root component
├── nuxt.config.ts          # Nuxt configuration
├── components/             # Vue components
│   ├── ui/                 # shadcn/vue components
│   ├── layout/
│   │   ├── Header.vue
│   │   ├── Sidebar.vue
│   │   └── StatusBar.vue
│   ├── project/
│   │   ├── ProjectCreate.vue
│   │   ├── ProjectList.vue
│   │   └── ScanProgress.vue
│   ├── culling/
│   │   ├── GroupGrid.vue
│   │   ├── CompareView.vue
│   │   ├── AssetThumbnail.vue
│   │   └── DecisionPanel.vue
│   └── export/
│       ├── ManifestExport.vue
│       └── IntegrationMap.vue
├── pages/                  # Route pages
│   ├── index.vue           # Dashboard
│   ├── project/
│   │   ├── [id].vue        # Project view
│   │   └── create.vue      # New project
│   └── settings/
│       └── index.vue       # App settings
├── stores/                 # Pinia stores
│   ├── project.ts          # Project state
│   ├── culling.ts          # Culling decisions
│   ├── ui.ts               # UI state
│   └── licensing.ts        # License state
├── composables/            # Vue composables
│   ├── useTauri.ts         # Tauri command wrapper
│   ├── useKeyboard.ts      # Keyboard shortcuts
│   └── useProgress.ts      # Progress tracking
├── types/                  # TypeScript types
│   ├── api.ts              # API interfaces
│   ├── models.ts           # Data models
│   └── events.ts           # Event types
└── lib/
    └── utils.ts            # Utility functions
```

### 4.2 State Management (Pinia)

```typescript
// stores/project.ts
export const useProjectStore = defineStore("project", () => {
  const currentProject = ref<Project | null>(null);
  const scanProgress = ref<ScanProgress | null>(null);
  const assets = ref<Asset[]>([]);
  const groups = ref<VariantGroup[]>([]);

  const createProject = async (config: ProjectConfig) => {
    // Call Tauri command
  };

  const startScan = async (projectId: string) => {
    // Call Tauri command with progress updates
  };

  return {
    currentProject,
    scanProgress,
    assets,
    groups,
    createProject,
    startScan,
  };
});

// stores/culling.ts
export const useCullingStore = defineStore("culling", () => {
  const decisions = ref<Map<string, Decision>>(new Map());
  const selectedAssets = ref<Set<string>>(new Set());
  const currentGroup = ref<VariantGroup | null>(null);

  const setDecision = async (
    assetId: string,
    decision: DecisionState,
    reason: ReasonCode
  ) => {
    // Update local state and call backend
  };

  return { decisions, selectedAssets, currentGroup, setDecision };
});
```

### 4.3 Tauri Integration

```typescript
// composables/useTauri.ts
import { invoke } from "@tauri-apps/api/tauri";

export const useTauri = () => {
  const createProject = async (config: ProjectConfig): Promise<Project> => {
    return await invoke("create_project", { config });
  };

  const scanProject = async (projectId: string): Promise<void> => {
    return await invoke("scan_project", { projectId });
  };

  const getGroups = async (projectId: string): Promise<VariantGroup[]> => {
    return await invoke("get_groups", { projectId });
  };

  const setDecision = async (
    assetId: string,
    decision: Decision
  ): Promise<void> => {
    return await invoke("set_decision", { assetId, decision });
  };

  return { createProject, scanProject, getGroups, setDecision };
};
```

---

## 5. Database Design (SQLite)

### 5.1 Schema

```sql
-- Projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_paths TEXT NOT NULL, -- JSON array
    output_path TEXT NOT NULL,  -- Output folder path
    exclude_patterns TEXT,      -- JSON array
    file_types TEXT,           -- JSON array
    scan_status TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- Assets table
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    hash TEXT,
    size INTEGER,
    width INTEGER,
    height INTEGER,
    exif_data TEXT,            -- JSON
    quality_metrics TEXT,      -- JSON (basic quality)
    ai_metrics TEXT,           -- JSON (AI-powered metrics, Pro)
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Variant groups table
CREATE TABLE variant_groups (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    group_type TEXT NOT NULL,  -- 'exact' or 'similar'
    similarity REAL,
    suggested_keep TEXT,       -- asset_id
    created_at DATETIME NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Group membership table (many-to-many)
CREATE TABLE group_assets (
    group_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    PRIMARY KEY (group_id, asset_id),
    FOREIGN KEY (group_id) REFERENCES variant_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
);

-- Decisions table
CREATE TABLE decisions (
    asset_id TEXT PRIMARY KEY,
    state TEXT NOT NULL,       -- 'keep', 'remove', 'undecided'
    reason TEXT NOT NULL,      -- reason code
    notes TEXT,
    decided_at DATETIME NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
);

-- Output mappings table (tracks source → output path mappings)
CREATE TABLE output_mappings (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    output_path TEXT NOT NULL,
    operation_type TEXT NOT NULL, -- 'copy', 'hardlink'
    status TEXT NOT NULL,         -- 'pending', 'completed', 'failed'
    completed_at DATETIME,
    error_message TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_assets_project_id ON assets(project_id);
CREATE INDEX idx_assets_hash ON assets(hash);
CREATE INDEX idx_variant_groups_project_id ON variant_groups(project_id);
CREATE INDEX idx_group_assets_group_id ON group_assets(group_id);
CREATE INDEX idx_group_assets_asset_id ON group_assets(asset_id);
CREATE INDEX idx_output_mappings_project_id ON output_mappings(project_id);
CREATE INDEX idx_output_mappings_asset_id ON output_mappings(asset_id);
```

---

## 6. Key Algorithms & Implementation Details

### 6.1 Duplicate Detection

```rust
// Exact duplicates via SHA-256 content hashing
pub async fn detect_exact_duplicates(assets: &[Asset]) -> Vec<VariantGroup> {
    let mut hash_map: HashMap<String, Vec<&Asset>> = HashMap::new();

    // Group by hash
    for asset in assets {
        if let Some(hash) = &asset.hash {
            hash_map.entry(hash.clone()).or_default().push(asset);
        }
    }

    // Convert groups with > 1 asset to VariantGroups
    hash_map.into_iter()
        .filter(|(_, assets)| assets.len() > 1)
        .map(|(hash, assets)| VariantGroup {
            id: generate_group_id(),
            group_type: GroupType::Exact,
            similarity: 100.0,
            assets: assets.into_iter().cloned().collect(),
            suggested_keep: suggest_best_asset(&assets),
        })
        .collect()
}
```

### 6.2 Perceptual Similarity

```rust
// Using image-hash crate for perceptual hashing
use image_hash::{ImageHash, HashAlg};

pub fn compute_perceptual_hash(image_path: &Path) -> Result<String> {
    let img = image::open(image_path)?;
    let hasher = ImageHash::new();
    let hash = hasher.hash_image(&img);
    Ok(hash.to_base64())
}

pub fn compute_similarity(hash1: &str, hash2: &str) -> f64 {
    let h1 = ImageHash::from_base64(hash1)?;
    let h2 = ImageHash::from_base64(hash2)?;
    let distance = h1.dist(&h2);
    // Convert Hamming distance to similarity percentage
    let max_distance = 64; // For 64-bit hash
    ((max_distance - distance) as f64 / max_distance as f64) * 100.0
}
```

### 6.3 Quality Metrics (Pro Feature)

```rust
// Basic quality heuristics
pub struct QualityAnalyzer;

impl QualityAnalyzer {
    pub fn analyze_image(&self, image_path: &Path) -> Result<QualityMetrics> {
        let img = image::open(image_path)?;

        Ok(QualityMetrics {
            sharpness: self.compute_sharpness(&img),
            exposure: self.compute_exposure(&img),
            noise: self.compute_noise(&img),
        })
    }

    fn compute_sharpness(&self, img: &DynamicImage) -> f64 {
        // Laplacian edge detection variance
        // Higher variance = sharper image
        // Implementation details...
    }

    fn compute_exposure(&self, img: &DynamicImage) -> f64 {
        // Histogram analysis for proper exposure
        // Implementation details...
    }

    fn compute_noise(&self, img: &DynamicImage) -> f64 {
        // Noise estimation via high-frequency analysis
        // Implementation details...
    }
}
```

---

## 7. Performance Considerations

### 7.1 Scanning Performance

- **Parallel Processing**: Use `rayon` for parallel file processing
- **Progressive Scanning**: Stream results to UI, don't wait for completion
- **Efficient Hashing**: Use fast hashing (BLAKE3) for content, slower perceptual hashing only when needed
- **Caching**: Cache perceptual hashes in database to avoid recomputation

### 7.2 UI Performance

- **Virtual Scrolling**: For large image grids (10k+ images)
- **Lazy Loading**: Load thumbnails on-demand
- **Debounced Updates**: Batch decision updates to avoid UI thrashing
- **Worker Threads**: Move heavy computation off main thread

### 7.3 Memory Management

- **Streaming**: Process images in batches, not all at once
- **Thumbnail Cache**: LRU cache for image thumbnails
- **Asset Pooling**: Reuse asset objects where possible

### 7.4 AI Performance Optimization

- **Model Caching**: Keep frequently used models in memory
- **Batch Inference**: Process multiple images together when possible
- **Hardware Acceleration**: Utilize GPU/Neural Engine when available
- **Progressive Loading**: Download and load models on-demand
- **Image Preprocessing**: Optimize image loading and resizing pipelines
- **Thread Pool**: Dedicated thread pool for AI inference to avoid UI blocking

---

## 8. Security & Privacy

### 8.1 Data Protection

- **Local-Only**: All data stays on user's machine
- **Encrypted Storage**: SQLite database encryption (optional)
- **Path Redaction**: Remove sensitive paths from logs/telemetry
- **Secure Cleanup**: Overwrite sensitive data on deletion

### 8.2 License Protection

- **Offline Validation**: License validation works offline for 14 days
- **Hardware Fingerprinting**: Tie licenses to machine characteristics
- **Tamper Detection**: Protect against license key modification

---

## 9. Build & Deployment

### 9.1 Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies
npm install

# Install Tauri CLI
cargo install tauri-cli

# Development mode
npm run tauri dev
```

### 9.2 Build Pipeline

```bash
# Build for production
npm run tauri build

# Platform-specific builds
npm run tauri build -- --target x86_64-apple-darwin    # macOS Intel
npm run tauri build -- --target aarch64-apple-darwin   # macOS Apple Silicon
npm run tauri build -- --target x86_64-pc-windows-msvc # Windows x64
npm run tauri build -- --target x86_64-unknown-linux-gnu # Linux x64
```

### 9.3 Distribution

- **macOS**: Code-signed `.dmg` with notarization
- **Windows**: Signed `.msi` installer
- **Linux**: `.AppImage` and `.deb` packages
- **Auto-updates**: Tauri updater for seamless updates

---

## 10. Testing Strategy

### 10.1 Backend Testing (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_duplicate_detection() {
        // Test exact duplicate grouping
    }

    #[test]
    fn test_similarity_calculation() {
        // Test perceptual hash similarity
    }

    #[test]
    fn test_quality_metrics() {
        // Test quality analysis
    }

    #[tokio::test]
    async fn test_ai_quality_analyzer() {
        // Test AI-powered quality scoring
        let analyzer = AIQualityAnalyzer::mock();
        let metrics = analyzer.analyze("test_image.jpg").await.unwrap();
        assert!(metrics.sharpness_score <= 100);
        assert!(metrics.overall_quality <= 100);
    }

    #[tokio::test]
    async fn test_face_detection() {
        // Test face detection accuracy
        let detector = FaceDetector::mock();
        let result = detector.detect_faces("portrait.jpg").await.unwrap();
        assert!(result.face_count > 0);
    }

    #[test]
    fn test_model_loading() {
        // Test model management and loading
        let manager = ModelManager::new_mock();
        assert!(manager.ensure_model("test_model").is_ok());
    }
}
```

### 10.2 Frontend Testing (Vitest)

```typescript
// tests/components/AssetThumbnail.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AssetThumbnail from "~/components/culling/AssetThumbnail.vue";

describe("AssetThumbnail", () => {
  it("renders asset correctly", () => {
    const asset = { id: "test", path: "/test.jpg", width: 1000, height: 1000 };
    const wrapper = mount(AssetThumbnail, { props: { asset } });
    expect(wrapper.find("img").attributes("src")).toContain("test.jpg");
  });
});
```

### 10.3 Integration Testing

- **End-to-End**: Playwright tests for critical user flows
- **Performance**: Benchmark tests for large image sets
- **Cross-Platform**: Test on all target platforms

---

## 11. Monitoring & Observability

### 11.1 Logging

```rust
use tracing::{info, warn, error, debug};

// Structured logging throughout the application
info!(
    project_id = %project.id,
    assets_scanned = assets.len(),
    duration_ms = scan_duration.as_millis(),
    "Scan completed successfully"
);
```

### 11.2 Metrics Collection

- **Performance Metrics**: Scan times, UI responsiveness
- **Usage Analytics**: Feature usage, error rates (opt-in)
- **Quality Metrics**: Crash reports, stability metrics

---

## 12. Migration & Compatibility

### 12.1 Data Migration

```rust
pub struct MigrationManager;

impl MigrationManager {
    pub fn migrate_database(&self, from_version: &str, to_version: &str) -> Result<()> {
        // Handle schema migrations between versions
    }

    pub fn migrate_project_format(&self, project_path: &Path) -> Result<()> {
        // Handle project file format changes
    }
}
```

### 12.2 Backward Compatibility

- **Schema Versioning**: Support reading older project formats
- **Graceful Degradation**: Handle missing features in older versions
- **Export Compatibility**: Maintain export format compatibility

---

## 13. Future Considerations

### 13.1 Extensibility

- **Plugin System**: Load additional image processing modules
- **Custom Rules**: User-defined auto-selection rules
- **Integration APIs**: Support for new photo management tools

### 13.2 Scalability

- **Distributed Processing**: Multi-machine scanning for studios
- **Cloud Sync**: Optional cloud backup of decisions (not images)
- **Team Features**: Shared decision workflows

---

This technical design provides a solid foundation for implementing the photo culling application while maintaining the privacy-first, performance-oriented goals outlined in the PRD.

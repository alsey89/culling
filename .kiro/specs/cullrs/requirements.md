# Requirements Document

## Introduction

CullRS (cullrs) is a comprehensive photo culling application built in Rust that helps users identify and manage duplicate photos across their collections. The application provides a modern desktop GUI using Tauri with Nuxt.js frontend. The system starts with basic exact duplicate detection and progressively adds advanced features like near-duplicate detection, crop detection, quality scoring, and AI-powered analysis.

## Requirements

### Requirement 1

**User Story:** As a photographer, I want to detect exact duplicate photos in my collection, so that I can free up storage space and organize my photo library efficiently.

#### Acceptance Criteria

1. WHEN the user selects a directory THEN the system SHALL scan all supported image formats (JPEG, PNG, TIFF, RAW formats)
2. WHEN scanning for exact duplicates THEN the system SHALL use file hash comparison for 100% accuracy
3. WHEN exact duplicates are found THEN the system SHALL display them grouped together with file paths and metadata
4. WHEN the user selects duplicates to delete THEN the system SHALL provide confirmation before permanent deletion
5. IF the scan is interrupted THEN the system SHALL allow resuming from the last processed file

### Requirement 2

**User Story:** As a user, I want a modern desktop interface, so that I can efficiently manage my photo collection with an intuitive and responsive user experience.

#### Acceptance Criteria

1. WHEN launching the desktop application THEN the system SHALL provide a Tauri-based GUI with Nuxt.js, Tailwind CSS, and shadcn-vue components
2. WHEN using the interface THEN the system SHALL provide drag-and-drop functionality for directory selection
3. WHEN processing operations THEN the system SHALL display real-time progress indicators and status updates
4. WHEN viewing results THEN the system SHALL provide responsive layouts that work on different screen sizes
5. WHEN performing batch operations THEN the system SHALL handle large datasets efficiently without UI freezing

### Requirement 3

**User Story:** As a power user, I want to detect near-duplicate photos and crops, so that I can identify similar images that aren't exact matches.

#### Acceptance Criteria

1. WHEN scanning for near-duplicates THEN the system SHALL use perceptual hashing algorithms
2. WHEN comparing images THEN the system SHALL detect crops, rotations, and minor edits
3. WHEN near-duplicates are found THEN the system SHALL display similarity percentages
4. WHEN the user sets similarity thresholds THEN the system SHALL only show matches above the specified threshold
5. IF images have different resolutions THEN the system SHALL still detect them as potential duplicates

### Requirement 4

**User Story:** As a professional photographer, I want advanced photo scoring and AI-powered analysis, so that I can automatically identify the best photos from similar shots.

#### Acceptance Criteria

1. WHEN analyzing photo quality THEN the system SHALL score images based on technical criteria (sharpness, exposure, composition)
2. WHEN multiple similar photos exist THEN the system SHALL rank them by quality score
3. WHEN AI models are available THEN the system SHALL use them for advanced scene and subject analysis
4. WHEN scoring is complete THEN the system SHALL recommend which photos to keep or delete
5. IF AI models are not available THEN the system SHALL fall back to technical analysis only

### Requirement 5

**User Story:** As a user managing large photo collections, I want efficient scanning and processing, so that I can handle thousands of photos without performance issues.

#### Acceptance Criteria

1. WHEN scanning large directories THEN the system SHALL process files in parallel using multiple CPU cores
2. WHEN memory usage is high THEN the system SHALL implement efficient memory management to prevent crashes
3. WHEN processing is ongoing THEN the system SHALL display real-time progress and estimated completion time
4. WHEN the user cancels an operation THEN the system SHALL stop gracefully and preserve partial results
5. IF the system encounters corrupted files THEN the system SHALL skip them and continue processing

### Requirement 6

**User Story:** As a user, I want to preview and manage detected duplicates safely, so that I can make informed decisions before deleting photos.

#### Acceptance Criteria

1. WHEN duplicates are detected THEN the system SHALL display thumbnail previews of all matches
2. WHEN viewing duplicates THEN the system SHALL show file metadata including size, date, and location
3. WHEN selecting photos to delete THEN the system SHALL provide options to move to trash or permanently delete
4. WHEN deletion is requested THEN the system SHALL require explicit confirmation with details of what will be deleted
5. IF the user wants to undo THEN the system SHALL provide recovery options for recently deleted files (when moved to trash)

### Requirement 7

**User Story:** As a user, I want flexible configuration options, so that I can customize the application behavior for my specific needs.

#### Acceptance Criteria

1. WHEN configuring the application THEN the system SHALL allow setting custom similarity thresholds
2. WHEN selecting file types THEN the system SHALL allow inclusion/exclusion of specific image formats
3. WHEN setting up scanning THEN the system SHALL allow recursive directory scanning with depth limits
4. WHEN using the application THEN the system SHALL save user preferences and restore them on restart
5. IF advanced features are enabled THEN the system SHALL allow fine-tuning of AI model parameters

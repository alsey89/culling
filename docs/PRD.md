# Product Requirements Document (PRD)

## Photo Culling & Deduplication App (Open‑Core, GUI‑Only with Optional Future CLI)

**Version:** 1.0  
**Owner:** Michael Chen  
**Last Updated:** 2025-07-27  
**Status:** Draft for Build Planning (Feature/Scope) — _Technical/system design covered separately_

---

## 0. Purpose & Positioning

A cross‑platform desktop application built in **Rust** with a **Tauri GUI** that **helps photographers and prosumers quickly cull- **Trust:** Undo rate ≤ 10% of copy operationslarge photo sets** and **remove exact/similar duplicates** with **privacy‑first, on‑device processing**.  
Model: **Open‑Core** (powerful free, open‑source core + compelling paid Pro tier).

This document defines the **feature scope, priorities, user stories, acceptance criteria, UX flows, events/telemetry, and artifacts** required to support design, QA, docs, and AI automation workflows. It intentionally **excludes** low‑level implementation details.

---

## 1. Personas & Jobs‑to‑be‑Done (JTBD)

### P1 — Event Photographer (Pro)

- **Goals:** Cull 2–10k photos per event in hours, not days; keep the best; remove dupes; export picks to Lightroom/Photo Mechanic/digiKam.
- **Pain:** Redundant bursts, soft focus, closed eyes, minor variations; manual reviewing is time‑consuming.
- **Success:** 70–90% reduction in review time; trustworthy auto‑select suggestions; reversible actions.

### P2 — Serious Hobbyist / Prosumer

- **Goals:** Periodically tidy a 100k‑photo library; remove duplicates/near‑duplicates; keep highest quality exposure or resolution.
- **Pain:** Tools either too “pro” or too generic; fears accidental deletion.
- **Success:** Confidence via previews, safe actions, and clear reporting.

### P3 — Archivist / Studio Assistant

- **Goals:** Apply repeatable rules; produce culling manifests and hand off to a lead.
- **Pain:** Inconsistent human decisions; hard to document the why.
- **Success:** Consistent, explainable results with audit trails.

---

## 2. Product Pillars

1. **Privacy by Design**: All photos and metadata are processed on the user’s machine—no cloud uploads.
2. **Open‑Core Trust**: A powerful, free, and open‑source core drives adoption and community engagement.
3. **Ownership & Flexibility**: Users choose between a perpetual license (_$299 one‑time, includes one year of updates_) or an annual subscription (_$150/year_).
4. **Modern Performance**: A lean, fast, and secure application built in Rust with a Tauri GUI, ensuring high efficiency and a small footprint across macOS, Windows, and Linux.
5. **Safety & Explainability**: Non‑destructive by default, reversible operations, clear rule definitions, and explainable automation.

---

## 3. Feature Map & Priorities

> **Priority legend:** P0 (MVP), P1 (Next), P2 (Later)

| ID    | Feature                          | Priority | Free / Pro | Summary                                                                                                                             |
| ----- | -------------------------------- | -------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| F-001 | Project & Scan                   | P0       | Free       | Create project, select source/output folders, index images, compute fingerprints for duplicates & similarity. Progress UI & cancel. |
| F-002 | Exact Duplicate Detection        | P0       | Free       | Strong hash grouping for exact duplicates. Safe actions: select keep/remove; export manifest.                                       |
| F-003 | Similarity Grouping (Perceptual) | P0       | Free       | Group visually similar images (threshold slider). Manual compare UI for groups.                                                     |
| F-004 | Culling Workspace                | P0       | Free       | Grid + compare view, keyboard shortcuts, star/flag states, sticky selection, zoom/pan.                                              |
| F-005 | Actions & Safety                 | P0       | Free       | Copy/Hardlink "Keep" files to output folder, leaving originals untouched. Optional "Remove" actions with confirmation.              |
| F-006 | Export Manifests                 | P0       | Free       | JSON/CSV export of decisions with source → output mappings, reasons/metrics.                                                        |
| F-007 | Unlimited Manual Culling         | P0       | Free       | Free tier offers unlimited manual culling and folder management.                                                                    |
| F-008 | Paywall & Licensing              | P0       | Pro        | Annual Subscription ($150/year) and Perpetual License ($299, incl. 1yr updates); offline license cache; entitlement check.          |
| F-009 | Auto‑Select Rules (Basic)        | P1       | Pro        | Rule‑based grouping: resolution, timestamp, camera/lens, file size, ISO. Preview before apply.                                      |
| F-010 | Quality Metrics (Heuristics)     | P1       | Pro        | On‑device blur/sharpness/exposure/noise scores (0–100); sortable and explainable.                                                   |
| F-011 | Face/Eyes‑Open Awareness (Local) | P2       | Pro        | Optional downloadable model for face count and closed‑eye detection; opt‑in.                                                        |
| F-012 | Integrations                     | P1       | Pro        | Export manifests for Lightroom, digiKam, Photo Mechanic with mapping presets.                                                       |
| F-013 | Presets & Rules Library          | P1       | Pro        | Save and share auto‑select rule presets.                                                                                            |
| F-014 | Multi‑Session Snapshot           | P2       | Pro        | Save/restore project states & snapshots; diff decisions across sessions.                                                            |
| F-015 | Team Mode (Seats)                | P2       | Pro        | Reviewer roles, merge manifests, conflict resolution.                                                                               |
| F-016 | General File Dedupe Mode         | P2       | Free+Pro   | Extend dedupe beyond photos to docs, videos, audio; plugin framing.                                                                 |
| F-017 | Onboarding & Education           | P0       | Free       | First‑run tour, tooltips, safety coach.                                                                                             |
| F-018 | Telemetry (Opt‑in)               | P0       | Free       | Anonymous usage events for UX improvement; strict privacy toggles.                                                                  |
| F-019 | i18n & Accessibility             | P0       | Free       | Locale files, RTL support, keyboard navigation, high‑contrast checks.                                                               |
| F-020 | Logging & Crash Reports          | P0       | Free       | Local logs; optional crash report with consent; redacted paths.                                                                     |

---

## 4. Free vs Pro Capabilities (SKU Matrix)

**Free (Open‑Core Core)**

- All P0 features (F-001–F-007, F-017–F-020)
- Unlimited manual culling and folder management
- Basic AI assists: on‑device blur/sharpness and exact duplicate detection
- Full performance for viewing, rating, tagging
- JSON/CSV export of manifests

**Pro**

- All Free features
- Advanced auto‑select rules (F-009)
- Quality metrics (F-010)
- Face/Eyes‑Open model (F-011)
- Integrations (F-012)
- Presets & Rules library (F-013)
- Workflow automation and priority support

**Trial**

- 7‑day Pro unlock on first run (no account required); reverts to Free thereafter.

---

## 5. Core User Flows (High‑Level)

### Flow A — First Project (Free)

1. Launch → Onboarding (Skip/Next).
2. New Project → Pick source folders → Pick output folder → Scan (progress modal).
3. Dashboard: tabs for **Duplicates**, **Similar Groups**, **Unreviewed**.
4. Compare view → Mark _Keep_ (star) / _Remove_ (trash).
5. Preview output structure and manifest.
6. Apply: Copy/hardlink "Keep" files to output folder.
7. Summary with source → output mapping.

**ACs:** Scan cancelable; export includes reasons; output preview accurate; originals never modified; etc.

### Flow B — Auto‑Select (Pro)

1. Open results → Click “Auto‑select”.
2. Pick preset or custom rule.
3. Preview counts and examples.
4. Stage changes (no file ops).
5. Manual overrides remain.
6. Export & apply.

**ACs:** Preview accuracy; performance budgets; separate apply step; etc.

### Flow C — Integrations (Pro)

1. Export to Lightroom/digiKam/PM.
2. Map fields; generate manifest.
3. Save mapping preset.

---

## 6. Non‑Functional Requirements (NFR)

- **Performance:** ≥50 images/sec indexing; UI latency <120 ms; auto‑select 10 k images ≤30 s.
- **Safety:** Non‑destructive by default; session undo stack; pre‑flight checks.
- **Privacy:** All compute on device; telemetry opt‑in; redacted crash reports.
- **Internationalization & Accessibility:** Runtime locale switch; keyboard‑only; high‑contrast.

---

## 7. Detailed Feature Requirements & Acceptance Criteria

### F-001 Project & Scan (P0, Free)

**Requirements**

- Create/Open project with: name, source paths (list), output folder path, exclude patterns (globs), file types (image defaults: jpg/jpeg/png/heic/tiff/webp/raw families), batch cap (respect SKU).
- Display progress (files scanned / time remaining estimate).
- Pause/Resume/Cancel.

**Acceptance**

- AC-001-1: Exclude patterns are applied before expensive operations.
- AC-001-2: Cancel leaves no partial writes beyond index cache.
- AC-001-3: Large directories (≥250k files) do not freeze UI; progress updates every ≤250ms.
- AC-001-4: Output folder validation prevents conflicts with source paths.

### F-002 Exact Duplicate Detection (P0, Free)

**Requirements**

- Group files with identical content hash.
- Display per-group total size reclaimable and suggested keep (newest by default).
- Safe operations only after confirm.

**Acceptance**

- AC-002-1: Hash collisions statistically negligible (document hash type in Tech spec).
- AC-002-2: Cross-filesystem detection supported.

### F-003 Similarity Grouping (P0, Free)

**Requirements**

- Perceptual similarity threshold (0–100); presets (Strict/Medium/Loose).
- Show visual thumbnails; open compare view to flip/zoom.

**Acceptance**

- AC-003-1: Threshold adjustments reflow groups without full rescan.
- AC-003-2: Compare view supports 1:1, 2:1, fit; synchronized pan/zoom.

### F-004 Culling Workspace (P0, Free)

**Requirements**

- Grid & Compare panes; keyboard shortcuts (documented help sheet).
- Flags: **Keep**, **Remove**, **Undecided**.
- Star ratings (0–5), color labels (optional toggle).

**Acceptance**

- AC-004-1: Shortcut latency ≤120ms.
- AC-004-2: Decisions update counts/bytes in dashboard in real time.

### F-005 Actions & Safety (P0, Free)

**Requirements**

- Output Mode: Copy/Hardlink selected "Keep" files to a new output folder, leaving originals untouched.
- Optional actions: Move to Trash/Quarantine for "Remove" files (user choice).
- Preserve folder structure in output or flatten based on user preference.
- Generate side-by-side manifest showing source → output mapping.

**Acceptance**

- AC-005-1: Output folder is completely separate from source; originals never modified.
- AC-005-2: User can preview exact output structure before applying.
- AC-005-3: Folder structure preservation maintains relative paths from source root.
- AC-005-4: Optional "Remove" actions require separate confirmation with summary.

### F-006 Export Manifests (P0, Free)

See **§8 Manifest Schemas**.

**Acceptance**

- AC-006-1: JSON and CSV outputs match schemas; include app version, timestamps, and deterministic group IDs.
- AC-006-2: “Reason” field populated for every Remove/Keep (e.g., exact_duplicate, higher_resolution, user_override).

### F-007 Batch Limits & Unlock (P0, Free+Pro)

**Acceptance**

- AC-007-1: When cap exceeded, user can select subset or see CTA to upgrade.
- AC-007-2: 7-day Pro unlock banner includes privacy note; no account required.

### F-008 Paywall & Licensing (P0, Pro)

**Requirements**

- Purchase flows (monthly/annual/lifetime), offline grace (14 days) if license server unreachable (tech detail separate).
- License viewer; transfer seat (manual deactivate).

**Acceptance**

- AC-008-1: Entitlements enforced reliably; Pro features grayed out with clear explainer.
- AC-008-2: Trial end reverts to Free with snackbar & persistent header note.

### F-009 Auto-Select Rules — Basic (P1, Pro)

**Rules (initial set)**

- Keep highest resolution.
- Keep newest or oldest taken date (EXIF).
- Keep by camera/lens preference list.
- Keep largest file size.
- Prefer lower ISO (proxy for noise) when same exposure triangle ranges.

**Acceptance**

- AC-009-1: Dry-run preview shows exact counts & list of affected assets.
- AC-009-2: User can whitelist/blacklist folders or filenames from auto-select.

### F-010 Quality Metrics (Heuristics) (P1, Pro)

**Visible Signals**

- Sharpness/Blur score (0–100), Exposure score (0–100), Noise score (0–100).
- Per-asset tooltips explain score rationale (human-readable).

**Acceptance**

- AC-010-1: Scores deterministic per image.
- AC-010-2: Sorting by score updates visible list in <300ms for 10k items.

### F-011 Face/Eyes-Open Awareness (P2, Pro)

**Requirements**

- Optional downloadable pack; indicates face count and closed eyes likelihood.
- Never auto-delete based solely on this; only as a rule input.

**Acceptance**

- AC-011-1: Model pack can be disabled/enabled at runtime.
- AC-011-2: Face data never leaves device.

### F-012 Integrations (P1, Pro)

**Initial Targets**

- Lightroom: starred/flagged CSV/JSON mapping.
- digiKam: tag/album JSON.
- Photo Mechanic: contact sheet/selection list.

**Acceptance**

- AC-012-1: Exports validated against mapping templates (see §8.3).
- AC-012-2: Docs panel: “How to import” steps for each integration.

---

## 8. Artifacts for AI/Automation

### 8.1 Canonical Domain Entities

- **Asset**: A single file with path + metadata (EXIF, size, resolution).
- **VariantGroup**: Set of Assets deemed exact or similar variants.
- **Decision**: Keep/Remove/Undecided (+ reason code).
- **Rule**: Declarative preference used in Auto-select.
- **Manifest**: Exported JSON/CSV describing project, groups, assets, and decisions.

### 8.2 Decision Reason Codes (enumeration)

    exact_duplicate
    higher_resolution
    lower_iso
    newer_timestamp
    older_timestamp
    preferred_camera
    preferred_lens
    larger_filesize
    quality_sharpness
    quality_exposure
    quality_noise
    user_override_keep
    user_override_remove
    manual_no_reason

### 8.3 Export Schemas (JSON)

#### 8.3.1 Culling Manifest (JSON)

    {
      "schema_version": "1.0",
      "app": { "name": "Culler", "version": "1.0.0" },
      "project": {
        "id": "proj_2025_07_27_001",
        "created_at": "2025-07-27T09:00:00Z",
        "source_paths": ["/Users/michael/Pictures/EventA"],
        "output_path": "/Users/michael/Pictures/EventA_Culled",
        "exclude_patterns": ["**/tmp/**"],
        "file_types": ["jpg","jpeg","png","heic","tiff","cr3","nef","arw","dng"]
      },
      "stats": {
        "total_assets": 12345,
        "duplicate_groups": 234,
        "similar_groups": 567,
        "bytes_reclaimable": 1234567890
      },
      "groups": [
        {
          "group_id": "grp_000001",
          "group_type": "exact|similar",
          "similarity": 100,
          "assets": [
            {
              "asset_id": "ast_000001",
              "source_path": "/abs/path/IMG_0001.JPG",
              "output_path": "/abs/output/IMG_0001.JPG",
              "hash": "sha256:...",
              "width": 6000,
              "height": 4000,
              "filesize": 5242880,
              "exif": {
                "taken_at": "2025-07-20T10:33:01Z",
                "camera": "Canon R6",
                "lens": "24-70mm",
                "iso": 400
              },
              "quality": {
                "sharpness": 78,
                "exposure": 65,
                "noise": 72
              },
              "decision": {
                "state": "keep|remove|undecided",
                "reason": "higher_resolution",
                "notes": "user note optional",
                "decided_at": "2025-07-27T09:12:01Z"
              }
            }
          ]
        }
      ]
    }

#### 8.3.2 Dedup Action Plan (JSON)

    {
      "schema_version": "1.0",
      "app": { "name": "Culler", "version": "1.0.0" },
      "plan": {
        "generated_at": "2025-07-27T09:13:00Z",
        "action": "copy|hardlink",
        "output_path": "/abs/output/folder"
      },
      "batches": [
        {
          "batch_id": "bat_0001",
          "group_id": "grp_000001",
          "keep_asset_id": "ast_000001",
          "keep_source_path": "/abs/source/IMG_0001.JPG",
          "keep_output_path": "/abs/output/IMG_0001.JPG",
          "remove_asset_ids": ["ast_000002","ast_000003"]
        }
      ],
      "summary": {
        "total_groups": 123,
        "files_to_copy": 456,
        "estimated_bytes_copied": 987654321
      }
    }

#### 8.3.3 Integration Mapping (Lightroom example, JSON)

    {
      "schema_version": "1.0",
      "integration": "lightroom",
      "fields": {
        "path": "output_path",
        "flagged": "decision.state == 'keep'",
        "rating": "decision.state == 'keep' ? 5 : 0",
        "color_label": "quality.sharpness >= 80 ? 'Green' : null"
      }
    }

### 8.4 CSV Columns (Culling Manifest)

    group_id,group_type,similarity,asset_id,source_path,output_path,hash,width,height,filesize,taken_at,camera,lens,iso,sharpness,exposure,noise,decision_state,decision_reason,decided_at

### 8.5 Telemetry/Analytics Events (Opt-in)

- **event_app_start** { session_id, os, version }
- **event_project_create** { project_id, sources_count, file_types }
- **event_scan_complete** { project_id, total_assets, dup_groups, sim_groups, ms_elapsed }
- **event_threshold_change** { project_id, old, new }
- **event_group_open** { project_id, group_id, group_type, asset_count }
- **event_decision_set** { project_id, group_id, asset_id, state, reason }
- **event_auto_select_preview** { project_id, rules: [ids], affected_count }
- **event_auto_select_apply** { project_id, rules: [ids], affected_count }
- **event_export_manifest** { project_id, format, path, success }
- **event_apply_actions** { project_id, action, files_count, bytes, output_path, success }
- **event_trial_started** { start_ts }
- **event_trial_expired** { end_ts }
- **event_purchase** { sku, price_usd, channel }
- **event_license_check** { status }

_All events must be anonymous; absolute paths redacted unless user consents._

---

## 9. Roadmap & Milestones

- **M1 — MVP (P0):** 8 weeks (F‑001–F‑007, F‑017–F‑020)
- **M2 — Pro Value (P1):** +6–8 weeks (F‑009, F‑010, F‑012, F‑013)
- **M3 — Depth & Teams (P2):** +10–12 weeks (F‑011, F‑014, F‑015, F‑016)

---

## 10. Success Metrics

- **Conversion:** Free → Pro trial cohort 5–7% by month 3
- **Performance:** TTFD <5 min for 10 k images
- **Trust:** Undo rate ≤ 10% of destructive batches
- **Adoption:** Tens of thousands of active Free users; NPS ≥ 40 for Pro by month 6

---

## 11. Risks & Mitigations

- Automation over‑confidence → human‑in‑loop confirmations
- Large libraries → progressive grouping, background workers
- OS quirks → capability checks, documented fallbacks
- Privacy concerns → opt‑in telemetry, open‑source transparency

---

## 12. Glossary

- **Cull:** Decide keep/remove
- **VariantGroup:** Exact or similar asset set
- **Rule:** Preference logic for auto‑select
- **Manifest:** Export of decisions & metadata
- **Auto‑Select:** Rule‑based batch proposal

---

---

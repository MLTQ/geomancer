# model.rs

## Purpose
Holds the normalized domain model shared by all task sources and the renderer. It gives the rest of the app one stable shape for tasks, source summaries, and aggregate stats.

## Components

### `TaskStatus`
- **Does**: Normalizes tracker-specific status strings into a small UI-oriented enum.
- **Interacts with**: Source adapters in `sources/beads.rs` and `sources/markdown.rs`

### `Task`
- **Does**: Represents one issue/task with status, source identity, dependency links, and optional timeline markers for claim/completion trails.
- **Interacts with**: `TaskSnapshot` in this file, globe rendering in `render.rs`

### `TrailEventKind` and `TrailEvent`
- **Does**: Represent normalized claim/completion events from tracker activity feeds.
- **Interacts with**: `beads.rs` activity ingestion, trail rendering in `render.rs`

### `LoadedSource`
- **Does**: Summarizes each scanner/adapter that contributed tasks to the current view.
- **Interacts with**: `load_repository` in `sources/mod.rs`, sidebar UI in `app.rs`

### `TaskSnapshot`
- **Does**: Bundles the current repository path, normalized task list, normalized trail events, source summaries, and warnings.
- **Interacts with**: `GeomancerApp` in `app.rs`, `layout_cells` in `layout.rs`

### `SnapshotStats`
- **Does**: Provides cheap aggregate counts and completion ratio for the UI.
- **Interacts with**: `TaskSnapshot::stats`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `sources/mod.rs` | `TaskSnapshot::empty` exists for empty/error states | Removing constructor or changing fields |
| `app.rs` | `Task`, `LoadedSource`, and `SnapshotStats` remain cloneable/cheap to inspect | Renaming fields or dropping derived traits |
| `render.rs` | `Task` dependency/status fields and `TrailEvent` activity data remain available | Removing dependency ids, claim/completion markers, trail events, or status normalization |

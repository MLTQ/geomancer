# app.rs

## Purpose
Owns the top-level egui application state: current repository path, loaded snapshot, animation bookkeeping, and the inspector UI around the globe widget.

## Components

### `GeomancerApp`
- **Does**: Stores app state, refresh cadence, globe orientation, auto-spin mode, and completion animation timestamps.
- **Interacts with**: `load_repository` in `sources/mod.rs`, `paint_globe` in `render.rs`

### `GeomancerApp::refresh`
- **Does**: Reloads tasks from disk/CLI, rebuilds cell layout, and updates error state.
- **Interacts with**: `layout_cells` in `layout.rs`

### `GeomancerApp::update_completion_animations`
- **Does**: Starts drop animations when tasks transition into done state.
- **Interacts with**: `completion_progress`

### `GeomancerApp::completion_progress`
- **Does**: Converts animation start timestamps into normalized drop progress values for the renderer.
- **Interacts with**: `GlobeVisualState` in `render.rs`

### `stats_panel` and `hover_card`
- **Does**: Render the surrounding repository summary and per-task hover details, including blocker task details for the hovered region.
- **Interacts with**: `SnapshotStats` and `Task` from `model.rs`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `main.rs` | `GeomancerApp::new` accepts an initial path and eagerly loads data | Changing constructor shape |
| `render.rs` | Completion progress map is keyed by task id and refreshed every frame | Changing animation state contract |
| Users | Enter in the repo field or the refresh button triggers a reload; drag pauses auto-spin and rotates the globe | Removing reload/inspection controls |

## Notes
- Refresh is polling-based for the MVP. A future version can swap in filesystem watchers or source-specific streaming updates without changing the renderer.

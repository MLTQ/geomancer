# render.rs

## Purpose
Contains the custom globe painter and the fake-3D projection logic that sells the Evangelion-inspired surface animation. It turns normalized tasks plus layout coordinates into painted polygons, hover hit-testing, and dependency cues.

## Components

### `GlobeVisualState`
- **Does**: Bundles the current camera orientation and animation inputs that change frame-to-frame.
- **Interacts with**: `GeomancerApp` in `app.rs`

### `GlobeRenderOutput`
- **Does**: Returns hit-test results back to the UI layer.
- **Interacts with**: Hover UI in `app.rs`

### `paint_globe`
- **Does**: Paints the globe, dependency connectors, completion drop animation, unlock markers, hover/dependency highlight states, outer timeline strands, and wireframe-only unfinished cells.
- **Interacts with**: `CellLayout` from `layout.rs`, `TaskSnapshot` from `model.rs`

### Projection and color helpers
- **Does**: Build per-cell polygons from tangent-basis sphere caps, rotate/project them, and derive branch-colored outlines.
- **Interacts with**: `Task` dependency graph in `model.rs`
- **Rationale**: Keeping these helpers local avoids leaking render-specific math into the domain layer.

### `paint_dependency_arc`
- **Does**: Draws dependency links as visible front-hemisphere arcs instead of flat chords across the viewport.
- **Interacts with**: `paint_globe`

### `paint_timeline_trails`
- **Does**: Draws pale-blue claimed strands and green completed strands as lifted ribbons that hover above the globe.
- **Interacts with**: `Task.claimed_at`, `Task.completed_at`, `TrailEvent`

### `build_trail_groups`
- **Does**: Chooses between real activity-driven trail grouping and a cleaner fallback task-state chain, keeping separate strands for parallel workers.
- **Interacts with**: `Task.assignee`, `TrailEvent.actor`

### `build_activity_trail_groups`
- **Does**: Converts normalized `bd activity` claim/completion events into per-actor trail strands.
- **Interacts with**: `TaskSnapshot.trail_events`

### `sanitize_projected_polygon`
- **Does**: Reorders projected vertices and rejects degenerate or spike-prone polygons before they are painted.
- **Interacts with**: `paint_globe`

### `paint_wireframe_cell`
- **Does**: Draws concentric outline rings for unfinished cells so they remain visible without reading as completed fill.
- **Interacts with**: `paint_globe`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `app.rs` | `paint_globe` returns hovered task index in snapshot order | Changing hover semantics or index basis |
| `layout.rs` | Cell centers remain unit vectors and angular radii remain in radians | Switching to screen-space inputs |
| Future render refactors | Completion progress map is keyed by task id | Changing keying scheme |

## Notes
- The branch coloring is intentionally approximate: it hashes local/root dependency signatures to give DAG neighborhoods a visual lineage without solving a full region-partitioning problem.
- Long stray edges from the first MVP were caused by seam-crossing longitude polygons and flat dependency chords; this renderer now avoids both by staying in sphere space until projection.
- If tangent spikes reappear, the next step is polygon clipping against the horizon rather than heuristic rejection.
- When `bd activity` is present, it drives the trail order directly. When it is empty, the renderer falls back to a simpler current-state chain so the path stays readable.

# sources/mod.rs

## Purpose
Coordinates all task adapters and merges their outputs into one normalized repository snapshot. This is the boundary that makes the globe reusable beyond beads.

## Components

### `load_repository`
- **Does**: Detects supported sources, loads them, normalizes ordering, back-fills dependents, and returns a combined snapshot.
- **Interacts with**: `beads.rs`, `markdown.rs`, `TaskSnapshot` in `../model.rs`

### `populate_dependents`
- **Does**: Reconstructs reverse dependency links after source adapters report forward edges.
- **Interacts with**: `Task.dependency_ids` and `Task.dependent_ids` in `../model.rs`

### `SourceLoadResult`
- **Does**: Common return type for adapters so warnings and tasks can be merged uniformly.
- **Interacts with**: All source modules

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `app.rs` | `load_repository` returns tasks already sorted and enriched with dependents | Removing sort or dependent reconstruction |
| Future adapters | `SourceLoadResult` holds both tasks and warnings | Changing adapter return shape |

## Notes
- The first two adapters are intentionally local-first: `beads` for structured DAGs and markdown checklists as a generic repo fallback.

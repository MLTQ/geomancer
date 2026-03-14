# beads.rs

## Purpose
Loads structured issue/task data from a local beads repository by shelling out to `bd`. This keeps Geomancer aligned with the supported CLI JSON output instead of reverse-engineering internal storage.

## Components

### `detect`
- **Does**: Treats `.beads/` as the signal that the directory is a beads repo.
- **Interacts with**: `load_repository` in `mod.rs`

### `load`
- **Does**: Runs `bd list --json --sandbox --no-daemon`, parses issues, and converts them into normalized `Task` values.
- **Interacts with**: `TaskStatus::from_raw` in `../model.rs`

### `parse_warnings`
- **Does**: Filters noisy daemon startup messages so only relevant CLI warnings reach the UI.
- **Interacts with**: Sidebar warning list in `../app.rs`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `sources/mod.rs` | `load` returns forward dependency ids in `Task.dependency_ids` | Dropping dependency extraction |
| Future source work | `bd` remains the authority for beads JSON | Replacing CLI loading with incompatible direct DB parsing |

## Notes
- This adapter is intentionally read-only and passes `--sandbox --no-daemon` to avoid surprising background behavior from the viewer app.

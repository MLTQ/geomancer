# markdown.rs

## Purpose
Provides a generic local adapter for repos that track work in markdown checklists. This is the first "works beyond beads" proof point and a template for future adapters.

## Components

### `load`
- **Does**: Walks repo markdown files and turns checklist items into normalized tasks.
- **Interacts with**: `TaskStatus` and `Task` in `../model.rs`

### `collect_markdown_files`
- **Does**: Recursively gathers candidate markdown files while skipping heavy/generated directories.
- **Interacts with**: `load`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `sources/mod.rs` | `load` never panics on unreadable files and returns best-effort tasks | Propagating hard failures for single-file read errors |
| Future adapters | Directory walking rules remain conservative | Expanding recursion into `.git`, `target`, or `.beads` |

## Notes
- This deliberately keeps semantics simple: checklist items are independent tasks. Richer markdown conventions can be added later behind another adapter or parser layer.

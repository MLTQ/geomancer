# geomancer

Geomancer is a standalone Rust + egui desktop app that turns repo task systems into a live, rotating progress globe.

Current MVP sources:
- `beads` repos via `bd list --json`
- markdown checklists (`- [ ]` / `- [x]`) inside the target directory

The globe is intentionally distorted rather than mathematically pure. Each task gets a polygon on a rotating sphere, completed tasks animate in with a falling red fill, dependency links are lightly projected across the surface, and hovering a cell shows the underlying task metadata.

## Run

```bash
cargo run -- /path/to/repo
```

If no path is provided, Geomancer starts on the current working directory.

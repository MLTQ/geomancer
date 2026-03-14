# main.rs

## Purpose
Application entrypoint for the Geomancer desktop app. It wires the initial repository path into the egui app shell and configures the native window.

## Components

### `main`
- **Does**: Starts the native `eframe` application and seeds the initial repo path from CLI args or the current working directory.
- **Interacts with**: `GeomancerApp` in `app.rs`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| OS launcher / `cargo run` | `main()` returns `eframe::Result<()>` and opens the UI | Changing startup contract or removing native app boot |

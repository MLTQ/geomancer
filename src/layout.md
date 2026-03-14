# layout.rs

## Purpose
Generates the distorted spherical tiling used by the renderer. The goal is stable, globe-like placement for an arbitrary task count without pretending to be mathematically exact.

## Components

### `CellLayout`
- **Does**: Stores one task cell's 3D unit-sphere center and angular size.
- **Interacts with**: `paint_globe` in `render.rs`

### `layout_cells`
- **Does**: Maps task count to a Fibonacci-sphere distribution with near-uniform cell density.
- **Interacts with**: `GeomancerApp` in `app.rs`
- **Rationale**: Fibonacci points avoid visible seams and row artifacts while still staying lightweight.

### `fibonacci_sphere`
- **Does**: Generates evenly spread task centers across the sphere.
- **Interacts with**: `layout_cells`

### `estimate_radii`
- **Does**: Derives a local angular radius from nearest-neighbor spacing so cells stay reasonably packed.
- **Interacts with**: `layout_cells`

## Contracts

| Dependent | Expects | Breaking changes |
|-----------|---------|------------------|
| `app.rs` | `layout_cells(n)` returns exactly `n` placements | Returning fewer/more cells |
| `render.rs` | `CellLayout.center` is a unit vector and `angular_radius` is in radians | Changing coordinate semantics |

## Notes
- This is still intentionally approximate. The goal is to look like a coherent tiled globe, not to implement a mathematically exact Goldberg mesh.

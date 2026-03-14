use std::f32::consts::PI;

#[derive(Clone, Debug)]
pub struct CellLayout {
    pub task_index: usize,
    pub center: [f32; 3],
    pub angular_radius: f32,
}

pub fn layout_cells(task_count: usize) -> Vec<CellLayout> {
    if task_count == 0 {
        return Vec::new();
    }

    let centers = fibonacci_sphere(task_count);
    let radii = estimate_radii(&centers);

    centers
        .into_iter()
        .zip(radii)
        .enumerate()
        .map(|(task_index, (center, angular_radius))| CellLayout {
            task_index,
            center: [center.x, center.y, center.z],
            angular_radius,
        })
        .collect()
}

fn fibonacci_sphere(task_count: usize) -> Vec<Vec3> {
    let golden_angle = PI * (3.0 - 5.0_f32.sqrt());

    (0..task_count)
        .map(|index| {
            let t = (index as f32 + 0.5) / task_count as f32;
            let y = 1.0 - (2.0 * t);
            let ring_radius = (1.0 - y * y).max(0.0).sqrt();
            let theta = golden_angle * index as f32;

            Vec3 {
                x: ring_radius * theta.cos(),
                y,
                z: ring_radius * theta.sin(),
            }
        })
        .collect()
}

fn estimate_radii(centers: &[Vec3]) -> Vec<f32> {
    let fallback = (2.6 / (centers.len().max(1) as f32).sqrt()).clamp(0.12, 0.5);

    centers
        .iter()
        .enumerate()
        .map(|(index, center)| {
            let mut nearest = f32::MAX;

            for (other_index, other) in centers.iter().enumerate() {
                if index == other_index {
                    continue;
                }

                let angle = center.dot(*other).clamp(-1.0, 1.0).acos();
                nearest = nearest.min(angle);
            }

            if nearest.is_finite() {
                (nearest * 0.62).clamp(0.1, 0.44)
            } else {
                fallback
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

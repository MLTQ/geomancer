use std::{
    collections::{BTreeMap, HashMap},
    f32::consts::TAU,
};

use egui::{
    epaint::Hsva, lerp, pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke,
};

use crate::{
    layout::CellLayout,
    model::{Task, TaskSnapshot, TrailEventKind},
};

pub struct GlobeRenderOutput {
    pub hovered_task: Option<usize>,
}

#[derive(Clone, Copy)]
pub struct GlobeVisualState<'a> {
    pub completion_progress: &'a HashMap<String, f32>,
    pub time: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Clone)]
struct ProjectedCell {
    task_index: usize,
    polygon: Vec<Pos2>,
    center: Pos2,
    sphere_center: Vec3,
    depth: f32,
    outline: Color32,
    fill: Color32,
    unlock_color: Color32,
    progress: f32,
    is_done: bool,
}

pub fn paint_globe(
    painter: &Painter,
    rect: Rect,
    snapshot: &TaskSnapshot,
    layout: &[CellLayout],
    visuals: GlobeVisualState<'_>,
) -> GlobeRenderOutput {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.38;
    let task_index = snapshot.task_index();
    let mut projected = Vec::with_capacity(layout.len());

    paint_background(painter, center, radius, visuals.time);

    for cell in layout {
        let Some(task) = snapshot.tasks.get(cell.task_index) else {
            continue;
        };

        let polygon3 = build_cell_points(cell);
        let mut polygon2 = Vec::with_capacity(polygon3.len());
        let mut depth_sum = 0.0f32;
        let mut visible_vertices = 0usize;
        for point in polygon3 {
            let rotated = rotate_point(point, visuals.yaw, visuals.pitch, visuals.roll);
            let screen = project_point(center, radius, rotated);

            polygon2.push(screen);
            depth_sum += rotated.z;
            visible_vertices += usize::from(rotated.z > -0.45);
        }

        let center3 = rotate_point(
            Vec3::from_array(cell.center),
            visuals.yaw,
            visuals.pitch,
            visuals.roll,
        );
        let projected_center = project_point(center, radius, center3);
        let average_depth = depth_sum / polygon2.len() as f32;

        if visible_vertices < 4 || average_depth < -0.55 {
            continue;
        }

        let Some(polygon) = sanitize_projected_polygon(polygon2, projected_center, radius) else {
            continue;
        };

        let progress = visuals
            .completion_progress
            .get(&task.id)
            .copied()
            .unwrap_or_else(|| if task.is_done() { 1.0 } else { 0.0 });
        let outline = lineage_color(task, &task_index, &snapshot.tasks, average_depth);
        let fill = fill_color(task, average_depth, progress);
        let unlock_color = brighten(outline, 0.25);

        projected.push(ProjectedCell {
            task_index: cell.task_index,
            polygon,
            center: projected_center,
            sphere_center: center3,
            depth: average_depth,
            outline,
            fill,
            unlock_color,
            progress,
            is_done: task.is_done(),
        });
    }

    projected.sort_by(|left, right| left.depth.total_cmp(&right.depth));

    let pointer = painter.ctx().pointer_latest_pos();
    let hovered = pointer.and_then(|pointer| hovered_task_at(pointer, &projected));
    let highlighted_dependencies = hovered
        .and_then(|task_index| snapshot.tasks.get(task_index))
        .map(|task| {
            task.dependency_ids
                .iter()
                .filter_map(|id| task_index.get(id.as_str()).copied())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if snapshot.tasks.len() <= 250 {
        for cell in &projected {
            let task = &snapshot.tasks[cell.task_index];
            for dependency in &task.dependency_ids {
                let Some(&dependency_index) = task_index.get(dependency.as_str()) else {
                    continue;
                };
                let Some(target) = projected.iter().find(|candidate| candidate.task_index == dependency_index) else {
                    continue;
                };
                let line_color = cell.outline.gamma_multiply(0.18);
                paint_dependency_arc(painter, center, radius, cell.sphere_center, target.sphere_center, line_color);
            }
        }
    }

    for cell in &projected {
        let is_hovered = hovered == Some(cell.task_index);
        let is_hovered_dependency = highlighted_dependencies.contains(&cell.task_index);
        let stroke_color = if is_hovered {
            brighten(cell.outline, 0.45)
        } else if is_hovered_dependency {
            Color32::from_rgb(255, 240, 176)
        } else {
            cell.outline
        };
        let stroke_width = if is_hovered {
            3.8
        } else if is_hovered_dependency {
            3.0
        } else {
            2.1
        };

        painter.add(Shape::closed_line(
            cell.polygon.clone(),
            Stroke::new(stroke_width + 2.0, stroke_color.gamma_multiply(0.22)),
        ));
        if cell.is_done {
            painter.add(Shape::convex_polygon(
                cell.polygon.clone(),
                cell.fill,
                Stroke::new(stroke_width, stroke_color),
            ));
        } else {
            paint_wireframe_cell(painter, &cell.polygon, cell.center, stroke_color, stroke_width);
        }

        if cell.is_done && cell.progress < 1.0 {
            let drop_offset = vec2(0.0, -rect.height() * 0.55 * (1.0 - cell.progress).powi(2));
            let drop_fill = Color32::from_rgba_premultiplied(245, 44, 60, 140);
            let drop_points: Vec<_> = cell.polygon.iter().map(|point| *point + drop_offset).collect();
            painter.add(Shape::convex_polygon(
                drop_points,
                drop_fill,
                Stroke::new(1.0, brighten(cell.outline, 0.15)),
            ));
        }

        let task = &snapshot.tasks[cell.task_index];
        if !task.dependent_ids.is_empty() {
            painter.circle_filled(cell.center, 3.2, cell.unlock_color);
        }
    }

    paint_timeline_trails(painter, snapshot, layout, center, radius, visuals);

    GlobeRenderOutput {
        hovered_task: hovered,
    }
}

fn paint_background(painter: &Painter, center: Pos2, radius: f32, time: f32) {
    painter.circle_filled(center, radius * 1.12, Color32::from_rgb(13, 13, 16));
    painter.circle_stroke(center, radius * 1.04, Stroke::new(1.2, Color32::from_rgb(110, 32, 24)));

    for ring in [0.48, 0.72, 0.92] {
        let alpha = (55.0 + (time * 0.9 + ring * TAU).sin() * 12.0) as u8;
        painter.circle_stroke(
            center,
            radius * ring,
            Stroke::new(0.9, Color32::from_rgba_premultiplied(255, 96, 58, alpha)),
        );
    }
}

fn paint_timeline_trails(
    painter: &Painter,
    snapshot: &TaskSnapshot,
    layout: &[CellLayout],
    center: Pos2,
    radius: f32,
    visuals: GlobeVisualState<'_>,
) {
    let claimed = build_trail_groups(snapshot, layout, TrailKind::Claimed);
    let completed = build_trail_groups(snapshot, layout, TrailKind::Completed);

    for (index, group) in claimed.iter().enumerate() {
        paint_trail_group(
            painter,
            &group.paths,
            center,
            radius,
            1.13 + index as f32 * 0.018,
            Color32::from_rgba_premultiplied(120, 214, 255, 165),
            visuals,
        );
    }

    for (index, group) in completed.iter().enumerate() {
        paint_trail_group(
            painter,
            &group.paths,
            center,
            radius,
            1.19 + index as f32 * 0.018,
            Color32::from_rgba_premultiplied(112, 255, 164, 180),
            visuals,
        );
    }
}

fn build_cell_points(cell: &CellLayout) -> [Vec3; 6] {
    let center = Vec3::from_array(cell.center);
    let (east, north) = tangent_basis(center);

    core::array::from_fn(|index| {
        let theta = index as f32 / 6.0 * TAU;
        let tangent = east.scale(theta.cos()).add(north.scale(theta.sin())).normalized();
        center
            .scale(cell.angular_radius.cos())
            .add(tangent.scale(cell.angular_radius.sin()))
            .normalized()
    })
}

fn rotate_point(point: Vec3, yaw: f32, pitch: f32, roll: f32) -> Vec3 {
    let spin = rotate_y(point, yaw);
    let tilt = rotate_x(spin, pitch);
    rotate_z(tilt, roll)
}

fn rotate_x(point: Vec3, angle: f32) -> Vec3 {
    let sin = angle.sin();
    let cos = angle.cos();
    Vec3 {
        x: point.x,
        y: point.y * cos - point.z * sin,
        z: point.y * sin + point.z * cos,
    }
}

fn rotate_y(point: Vec3, angle: f32) -> Vec3 {
    let sin = angle.sin();
    let cos = angle.cos();
    Vec3 {
        x: point.x * cos + point.z * sin,
        y: point.y,
        z: -point.x * sin + point.z * cos,
    }
}

fn rotate_z(point: Vec3, angle: f32) -> Vec3 {
    let sin = angle.sin();
    let cos = angle.cos();
    Vec3 {
        x: point.x * cos - point.y * sin,
        y: point.x * sin + point.y * cos,
        z: point.z,
    }
}

fn fill_color(task: &Task, depth: f32, progress: f32) -> Color32 {
    if !task.is_done() {
        return Color32::TRANSPARENT;
    }

    let shade = lerp(0.55..=1.0, (depth + 1.0) * 0.5);
    mix(
        Color32::from_rgb(247, 104, 48),
        Color32::from_rgb(214, 24, 48),
        progress * shade,
    )
}

fn lineage_color(
    task: &Task,
    task_index: &HashMap<&str, usize>,
    tasks: &[Task],
    depth: f32,
) -> Color32 {
    let mut seeds = Vec::new();
    collect_root_seeds(task, task_index, tasks, &mut seeds, 0);
    if seeds.is_empty() {
        seeds.push(hash_text(&task.id));
    }

    let combined = seeds.iter().fold(0u64, |acc, value| acc ^ *value);
    let hue = (combined % 360) as f32 / 360.0;
    let saturation = if task.dependency_ids.len() > 1 { 0.92 } else { 0.76 };
    let value = lerp(0.58..=0.98, (depth + 1.0) * 0.5);
    Color32::from(Hsva::new(hue, saturation, value, 1.0))
}

fn collect_root_seeds(
    task: &Task,
    task_index: &HashMap<&str, usize>,
    tasks: &[Task],
    seeds: &mut Vec<u64>,
    depth: usize,
) {
    if depth > 3 {
        return;
    }

    if task.dependency_ids.is_empty() {
        seeds.push(hash_text(&task.id));
        return;
    }

    for dependency in &task.dependency_ids {
        if let Some(&index) = task_index.get(dependency.as_str()) {
            collect_root_seeds(&tasks[index], task_index, tasks, seeds, depth + 1);
        } else {
            seeds.push(hash_text(dependency));
        }
    }
}

fn mix(left: Color32, right: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        lerp(left.r() as f32..=right.r() as f32, t) as u8,
        lerp(left.g() as f32..=right.g() as f32, t) as u8,
        lerp(left.b() as f32..=right.b() as f32, t) as u8,
    )
}

fn brighten(color: Color32, amount: f32) -> Color32 {
    mix(color, Color32::WHITE, amount)
}

fn hash_text(text: &str) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn point_in_polygon(point: Pos2, polygon: &[Pos2]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];

    for &current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y + f32::EPSILON)
                    + current.x);

        if intersects {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

fn build_trail_groups(
    snapshot: &TaskSnapshot,
    layout: &[CellLayout],
    kind: TrailKind,
) -> Vec<TrailGroup> {
    if !snapshot.trail_events.is_empty() {
        let groups = build_activity_trail_groups(snapshot, layout, kind);
        if !groups.is_empty() {
            return groups;
        }
    }

    let layout_by_index: HashMap<usize, [f32; 3]> = layout
        .iter()
        .map(|cell| (cell.task_index, cell.center))
        .collect();
    let mut grouped: BTreeMap<String, Vec<TrailNode>> = BTreeMap::new();

    for task in snapshot.tasks.iter().enumerate() {
        let (task_index, task) = task;
        let Some(center) = layout_by_index.get(&task_index).copied() else {
            continue;
        };
        if !task_matches_trail_kind(task, kind) {
            continue;
        }

        let timestamp = trail_sort_key(task, kind).to_owned();
        let owner = task
            .assignee
            .clone()
            .unwrap_or_else(|| format!("{}:shared", task.source));
        grouped
            .entry(owner)
            .or_default()
            .push(TrailNode {
                task_index,
                timestamp,
                point: Vec3::from_array(center),
            });
    }

    grouped
        .into_iter()
        .map(|(_, mut nodes)| {
            nodes.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
            let ordered = order_nodes_cleanly(&nodes);
            TrailGroup { paths: vec![ordered] }
        })
        .filter(|group| !group.paths.is_empty() && !group.paths[0].is_empty())
        .collect()
}

fn build_activity_trail_groups(
    snapshot: &TaskSnapshot,
    layout: &[CellLayout],
    kind: TrailKind,
) -> Vec<TrailGroup> {
    let task_index = snapshot.task_index();
    let layout_by_task_id: HashMap<_, _> = layout
        .iter()
        .filter_map(|cell| snapshot.tasks.get(cell.task_index).map(|task| (task.id.as_str(), cell.center)))
        .collect();
    let target_kind = match kind {
        TrailKind::Claimed => TrailEventKind::Claimed,
        TrailKind::Completed => TrailEventKind::Completed,
    };
    let mut grouped: BTreeMap<String, Vec<TrailNode>> = BTreeMap::new();

    for event in &snapshot.trail_events {
        if event.kind != target_kind {
            continue;
        }
        let Some(center) = layout_by_task_id.get(event.task_id.as_str()).copied() else {
            continue;
        };
        let Some(&task_position) = task_index.get(event.task_id.as_str()) else {
            continue;
        };

        grouped
            .entry(event.actor.clone())
            .or_default()
            .push(TrailNode {
                task_index: task_position,
                timestamp: event.timestamp.clone(),
                point: Vec3::from_array(center),
            });
    }

    grouped
        .into_iter()
        .map(|(_, mut nodes)| {
            nodes.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
            nodes.dedup_by(|left, right| {
                left.task_index == right.task_index && left.timestamp == right.timestamp
            });

            TrailGroup {
                paths: vec![nodes.into_iter().map(|node| node.point).collect()],
            }
        })
        .filter(|group| !group.paths.is_empty() && !group.paths[0].is_empty())
        .collect()
}

fn paint_trail_group(
    painter: &Painter,
    paths: &[Vec<Vec3>],
    center: Pos2,
    radius: f32,
    shell_scale: f32,
    color: Color32,
    visuals: GlobeVisualState<'_>,
) {
    for (path_index, path) in paths.iter().enumerate() {
        if path.is_empty() {
            continue;
        }

        let rotated: Vec<_> = path
            .iter()
            .map(|point| rotate_point(*point, visuals.yaw, visuals.pitch, visuals.roll))
            .collect();
        let line_color = if path_index == 0 {
            color
        } else {
            color.gamma_multiply(0.68)
        };
        let point_radius = if path_index == 0 { 2.8 } else { 2.1 };

        if rotated.len() == 1 {
            let point = rotated[0];
            if point.z > -0.15 {
                painter.circle_filled(
                    project_shell_point(center, radius, shell_scale, point),
                    point_radius,
                    line_color,
                );
            }
            continue;
        }

        for window in rotated.windows(2) {
            let from = window[0];
            let to = window[1];
            paint_outer_arc(painter, center, radius, shell_scale, from, to, line_color);
        }

        for point in rotated {
            if point.z > -0.15 {
                painter.circle_filled(
                    project_shell_point(center, radius, shell_scale, point),
                    point_radius,
                    line_color.gamma_multiply(0.9),
                );
            }
        }
    }
}

fn hovered_task_at(pointer: Pos2, projected: &[ProjectedCell]) -> Option<usize> {
    let mut hovered = None;
    let mut best_depth = f32::NEG_INFINITY;

    for cell in projected {
        if point_in_polygon(pointer, &cell.polygon) && cell.depth >= best_depth {
            best_depth = cell.depth;
            hovered = Some(cell.task_index);
        }
    }

    hovered
}

fn sanitize_projected_polygon(mut polygon: Vec<Pos2>, center: Pos2, radius: f32) -> Option<Vec<Pos2>> {
    polygon.sort_by(|left, right| {
        let left_angle = (left.y - center.y).atan2(left.x - center.x);
        let right_angle = (right.y - center.y).atan2(right.x - center.x);
        left_angle.total_cmp(&right_angle)
    });

    let area = polygon_area(&polygon).abs();
    if area < 8.0 {
        return None;
    }

    let mut max_edge = 0.0f32;
    let mut total_edge = 0.0f32;
    for index in 0..polygon.len() {
        let next = (index + 1) % polygon.len();
        let edge = polygon[index].distance(polygon[next]);
        max_edge = max_edge.max(edge);
        total_edge += edge;
    }

    let average_edge = total_edge / polygon.len() as f32;
    if max_edge > radius * 0.33 || max_edge > average_edge * 1.9 {
        return None;
    }

    Some(polygon)
}

fn polygon_area(polygon: &[Pos2]) -> f32 {
    let mut area = 0.0f32;

    for index in 0..polygon.len() {
        let next = (index + 1) % polygon.len();
        area += polygon[index].x * polygon[next].y - polygon[next].x * polygon[index].y;
    }

    area * 0.5
}

fn paint_wireframe_cell(
    painter: &Painter,
    polygon: &[Pos2],
    center: Pos2,
    stroke_color: Color32,
    stroke_width: f32,
) {
    for (index, scale) in [1.0, 0.78, 0.58].into_iter().enumerate() {
        let ring = scale_polygon(polygon, center, scale);
        let alpha = match index {
            0 => 0.95,
            1 => 0.48,
            _ => 0.28,
        };
        let width = (stroke_width - index as f32 * 0.45).max(0.8);
        painter.add(Shape::closed_line(
            ring,
            Stroke::new(width, stroke_color.gamma_multiply(alpha)),
        ));
    }
}

fn scale_polygon(polygon: &[Pos2], center: Pos2, scale: f32) -> Vec<Pos2> {
    polygon
        .iter()
        .map(|point| center + (*point - center) * scale)
        .collect()
}

fn paint_outer_arc(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    shell_scale: f32,
    from: Vec3,
    to: Vec3,
    color: Color32,
) {
    let dot = from.dot(to).clamp(-1.0, 1.0);
    let omega = dot.acos();

    if omega <= 0.001 {
        return;
    }

    let sin_omega = omega.sin();
    let mut previous = None;
    let steps = 24usize;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let a = ((1.0 - t) * omega).sin() / sin_omega;
        let b = (t * omega).sin() / sin_omega;
        let lifted = from.scale(a).add(to.scale(b)).normalized().scale(shell_scale);

        if lifted.z <= -0.05 {
            previous = None;
            continue;
        }

        let projected = project_outer_point(center, radius, lifted);
        if let Some(prev) = previous {
            painter.line_segment([prev, projected], Stroke::new(1.8, color));
        }
        previous = Some(projected);
    }
}

fn paint_dependency_arc(
    painter: &Painter,
    globe_center: Pos2,
    radius: f32,
    from: Vec3,
    to: Vec3,
    color: Color32,
) {
    let dot = from.dot(to).clamp(-1.0, 1.0);
    let omega = dot.acos();

    if omega <= 0.001 {
        return;
    }

    let sin_omega = omega.sin();
    let steps = 18usize;
    let mut previous = None;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let a = ((1.0 - t) * omega).sin() / sin_omega;
        let b = (t * omega).sin() / sin_omega;
        let point = from.scale(a).add(to.scale(b)).normalized();

        if point.z <= -0.1 {
            previous = None;
            continue;
        }

        let projected = project_point(globe_center, radius, point);
        if projected.distance(globe_center) > radius * 1.03 {
            previous = None;
            continue;
        }

        if let Some(prev) = previous {
            painter.line_segment([prev, projected], Stroke::new(1.0, color));
        }
        previous = Some(projected);
    }
}

fn project_point(center: Pos2, radius: f32, point: Vec3) -> Pos2 {
    let perspective = 0.7 + ((point.z + 1.0) * 0.15);
    pos2(
        center.x + point.x * radius * perspective,
        center.y + point.y * radius * perspective,
    )
}

fn project_shell_point(center: Pos2, radius: f32, shell_scale: f32, point: Vec3) -> Pos2 {
    project_outer_point(center, radius, point.scale(shell_scale))
}

fn project_outer_point(center: Pos2, radius: f32, point: Vec3) -> Pos2 {
    let perspective = 0.68 + ((point.z + 1.0) * 0.14);
    pos2(
        center.x + point.x * radius * perspective,
        center.y + point.y * radius * perspective,
    )
}

fn tangent_basis(center: Vec3) -> (Vec3, Vec3) {
    let reference = if center.y.abs() > 0.92 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let east = reference.cross(center).normalized();
    let north = center.cross(east).normalized();
    (east, north)
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy)]
enum TrailKind {
    Claimed,
    Completed,
}

struct TrailGroup {
    paths: Vec<Vec<Vec3>>,
}

struct TrailNode {
    task_index: usize,
    timestamp: String,
    point: Vec3,
}

fn claimed_sort_key(task: &Task) -> Option<&str> {
    task.claimed_at
        .as_deref()
        .or(task.updated_at.as_deref())
}

fn completed_sort_key(task: &Task) -> Option<&str> {
    task.completed_at
        .as_deref()
        .or(task.updated_at.as_deref())
}

fn task_matches_trail_kind(task: &Task, kind: TrailKind) -> bool {
    match kind {
        TrailKind::Claimed => matches!(task.status, crate::model::TaskStatus::InProgress),
        TrailKind::Completed => task.is_done(),
    }
}

fn trail_sort_key<'a>(task: &'a Task, kind: TrailKind) -> &'a str {
    match kind {
        TrailKind::Claimed => claimed_sort_key(task).unwrap_or(task.id.as_str()),
        TrailKind::Completed => completed_sort_key(task).unwrap_or(task.id.as_str()),
    }
}

fn order_nodes_cleanly(nodes: &[TrailNode]) -> Vec<Vec3> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let mut remaining: Vec<_> = nodes.iter().map(|node| node.point).collect();
    let mut ordered = Vec::with_capacity(remaining.len());
    let start = remaining.remove(0);
    ordered.push(start);

    while !remaining.is_empty() {
        let current = *ordered.last().unwrap_or(&start);
        let (next_index, _) = remaining
            .iter()
            .enumerate()
            .map(|(index, point)| (index, current.distance_to(*point)))
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or((0, 0.0));
        ordered.push(remaining.remove(next_index));
    }

    ordered
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn from_array(values: [f32; 3]) -> Self {
        Self::new(values[0], values[1], values[2])
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    fn scale(self, value: f32) -> Self {
        Self::new(self.x * value, self.y * value, self.z * value)
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn distance_to(self, other: Self) -> f32 {
        self.add(other.scale(-1.0)).length()
    }

    fn normalized(self) -> Self {
        let length = self.length().max(1e-6);
        self.scale(1.0 / length)
    }
}

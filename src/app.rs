use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use eframe::egui::{self, Align, Color32, Layout, RichText};

use crate::{
    layout::{layout_cells, CellLayout},
    model::{SnapshotStats, Task, TaskSnapshot},
    render::{paint_globe, GlobeVisualState},
    sources,
};

const AUTO_REFRESH_SECONDS: f32 = 2.5;
const DROP_ANIMATION_SECONDS: f32 = 1.1;

pub struct GeomancerApp {
    repo_input: String,
    snapshot: TaskSnapshot,
    layout: Vec<CellLayout>,
    last_refresh: Instant,
    rotation: f32,
    previous_done: HashMap<String, bool>,
    completion_started: HashMap<String, Instant>,
    load_error: Option<String>,
}

impl GeomancerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_path: PathBuf) -> Self {
        let repo_input = initial_path.display().to_string();
        let mut app = Self {
            repo_input,
            snapshot: TaskSnapshot::empty(initial_path),
            layout: Vec::new(),
            last_refresh: Instant::now() - Duration::from_secs(30),
            rotation: 0.0,
            previous_done: HashMap::new(),
            completion_started: HashMap::new(),
            load_error: None,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        let path = PathBuf::from(self.repo_input.trim());
        if path.as_os_str().is_empty() {
            self.load_error = Some("enter a repository path".to_owned());
            return;
        }

        match sources::load_repository(&path) {
            Ok(snapshot) => {
                let previous_done = self.previous_done.clone();
                self.snapshot = snapshot;
                self.layout = layout_cells(self.snapshot.tasks.len());
                self.last_refresh = Instant::now();
                self.load_error = None;
                self.previous_done = self
                    .snapshot
                    .tasks
                    .iter()
                    .map(|task| (task.id.clone(), task.is_done()))
                    .collect();
                self.update_completion_animations(previous_done);
            }
            Err(error) => {
                self.load_error = Some(error);
            }
        }
    }

    fn update_completion_animations(&mut self, previous_done: HashMap<String, bool>) {
        let now = Instant::now();
        self.completion_started
            .retain(|id, _| self.snapshot.tasks.iter().any(|task| &task.id == id));

        for task in &self.snapshot.tasks {
            if task.is_done() && !previous_done.get(&task.id).copied().unwrap_or(false) {
                self.completion_started.insert(task.id.clone(), now);
            } else if !task.is_done() {
                self.completion_started.remove(&task.id);
            }
        }
    }

    fn completion_progress(&mut self) -> HashMap<String, f32> {
        let now = Instant::now();
        let mut progress = HashMap::new();
        let mut finished = Vec::new();

        for task in &self.snapshot.tasks {
            if !task.is_done() {
                continue;
            }

            let value = self
                .completion_started
                .get(&task.id)
                .map(|started| (now - *started).as_secs_f32() / DROP_ANIMATION_SECONDS)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            progress.insert(task.id.clone(), value);

            if value >= 1.0 {
                finished.push(task.id.clone());
            }
        }

        for id in finished {
            self.completion_started.remove(&id);
        }

        progress
    }
}

impl eframe::App for GeomancerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let delta = ctx.input(|input| input.unstable_dt).max(1.0 / 120.0);
        self.rotation += delta * 0.22;

        if self.last_refresh.elapsed().as_secs_f32() >= AUTO_REFRESH_SECONDS {
            self.refresh();
        }

        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.heading(RichText::new("Geomancer").color(Color32::from_rgb(255, 116, 70)));
                    ui.label("Repository");
                    let response = ui.add_sized(
                        [ui.available_width() - 180.0, 28.0],
                        egui::TextEdit::singleline(&mut self.repo_input),
                    );

                    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        self.refresh();
                    }

                    if ui.button("Refresh").clicked() {
                        self.refresh();
                    }
                });
            });

        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(310.0)
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(14)))
            .show(ctx, |ui| {
                let stats = self.snapshot.stats();
                stats_panel(ui, stats);
                ui.separator();

                for source in &self.snapshot.sources {
                    ui.label(
                        RichText::new(format!("{}  {}", source.name, source.task_count))
                            .color(Color32::from_rgb(255, 150, 110)),
                    );
                    ui.small(&source.detail);
                    ui.add_space(6.0);
                }

                if let Some(error) = &self.load_error {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(255, 96, 96), error);
                }

                if !self.snapshot.warnings.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Warnings").strong());
                    for warning in &self.snapshot.warnings {
                        ui.small(warning);
                    }
                }
            });

        let completion_progress = self.completion_progress();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(8, 8, 10))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                let header = format!(
                    "{} tasks  |  {:.0}% complete  |  {}",
                    self.snapshot.tasks.len(),
                    self.snapshot.stats().completion_ratio() * 100.0,
                    self.snapshot.root.display()
                );
                ui.label(RichText::new(header).color(Color32::from_rgb(255, 172, 135)));
                ui.add_space(12.0);

                let available = ui.available_rect_before_wrap();
                let painter = ui.painter_at(available);
                let render = paint_globe(
                    &painter,
                    available.shrink2(egui::vec2(12.0, 12.0)),
                    &self.snapshot,
                    &self.layout,
                    GlobeVisualState {
                        completion_progress: &completion_progress,
                        time: ctx.input(|input| input.time) as f32,
                        rotation: self.rotation,
                    },
                );

                if let Some(task_index) = render.hovered_task.and_then(|index| self.snapshot.tasks.get(index))
                {
                    hover_card(ctx, task_index);
                }
            });

        ctx.request_repaint();
    }
}

fn stats_panel(ui: &mut egui::Ui, stats: SnapshotStats) {
    ui.label(RichText::new("Repository Completeness").strong());
    ui.add_space(6.0);

    ui.label(format!("Total      {}", stats.total));
    ui.label(format!("Done       {}", stats.done));
    ui.label(format!("Active     {}", stats.in_progress));
    ui.label(format!("Blocked    {}", stats.blocked));
    ui.label(format!("Open       {}", stats.open));
}

fn hover_card(ctx: &egui::Context, task: &Task) {
    let Some(pointer) = ctx.pointer_latest_pos() else {
        return;
    };

    egui::Area::new("task-hover-card".into())
        .order(egui::Order::Foreground)
        .fixed_pos(pointer + egui::vec2(18.0, 18.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(18, 15, 17))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(255, 106, 62)))
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.label(RichText::new(&task.title).strong());
                    ui.small(&task.id);
                    ui.small(format!("status: {}", task.status.label()));
                    ui.small(format!("source: {}", task.source));

                    if let Some(path) = &task.source_path {
                        ui.small(path);
                    }
                    if let Some(assignee) = &task.assignee {
                        ui.small(format!("owner: {assignee}"));
                    }
                    if let Some(updated_at) = &task.updated_at {
                        ui.small(format!("updated: {updated_at}"));
                    }
                    if let Some(url) = &task.url {
                        ui.small(url);
                    }

                    ui.small(format!("depends on: {}", task.dependency_ids.len()));
                    ui.small(format!("unlocks: {}", task.dependent_ids.len()));
                });
        });
}

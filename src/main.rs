mod app;
mod layout;
mod model;
mod render;
mod sources;

use std::path::PathBuf;

use app::GeomancerApp;

fn main() -> eframe::Result<()> {
    let initial_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([960.0, 720.0])
            .with_title("Geomancer"),
        ..Default::default()
    };

    eframe::run_native(
        "Geomancer",
        options,
        Box::new(move |cc| Ok(Box::new(GeomancerApp::new(cc, initial_path.clone())))),
    )
}

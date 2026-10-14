mod app;
mod launcher;
mod recommend;
mod store;
mod theme;

use std::path::PathBuf;

/// Where to look for the database: the path given as first argument, or else
/// next to the executable (downloaded release), then `data/` (repo clone).
fn data_candidates() -> Vec<PathBuf> {
    if let Some(path) = std::env::args_os().nth(1) {
        return vec![PathBuf::from(path)];
    }

    let next_to_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(launcher::DATA_FILE)));

    next_to_exe
        .into_iter()
        .chain([PathBuf::from("data").join(launcher::DATA_FILE)])
        .collect()
}

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("bundled icon.png should decode")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let launcher = launcher::Launcher::new(data_candidates());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 820.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Anime Reco",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            theme::install(&cc.egui_ctx);
            // egui scrolls 40 pt per wheel notch by default, which crawls past
            // ~360 pt tall cards; this matches what browsers do.
            cc.egui_ctx
                .options_mut(|o| o.input_options.line_scroll_speed = 100.0);
            Ok(Box::new(launcher))
        }),
    )
}

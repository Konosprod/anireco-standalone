mod app;
mod recommend;
mod store;

fn main() -> eframe::Result {
    env_logger::init();

    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/anime.db".to_string());

    eprintln!("Loading data from {db_path} ...");
    let data_store = store::load(&db_path).expect("failed to load anime database");
    eprintln!("Loaded {} anime", data_store.all.len());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1150.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Anime Reco",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::AniRecoApp::new(data_store)))
        }),
    )
}

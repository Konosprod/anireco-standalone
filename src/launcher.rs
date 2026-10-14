//! Startup wrapper: shows the app once the database is found, or a screen
//! explaining where to download it and where to put it (release binaries
//! don't ship the ~400 MB database).

use crate::app::AniRecoApp;
use crate::store::{self, LoadError};
use crate::theme;
use eframe::egui::{self, CornerRadius, Margin, OpenUrl, RichText, Stroke};
use std::path::PathBuf;

pub const DATA_FILE: &str = "anime.db";
pub const DATA_URL: &str = "https://github.com/Konosprod/anireco-standalone/raw/main/data/anime.db";

pub enum Launcher {
    Ready(AniRecoApp),
    MissingData {
        /// Where the database is looked for, in order. The first one is where
        /// users are told to put it.
        candidates: Vec<PathBuf>,
        error: LoadError,
    },
}

impl Launcher {
    pub fn new(candidates: Vec<PathBuf>) -> Self {
        match store::load_first(&candidates) {
            Ok((store, path)) => {
                eprintln!("Loaded {} anime from {}", store.all.len(), path.display());
                Self::Ready(AniRecoApp::new(store))
            }
            Err(error) => Self::MissingData { candidates, error },
        }
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let retry = match self {
            Self::Ready(app) => return eframe::App::ui(app, ui, frame),
            Self::MissingData { candidates, error } => missing_data_screen(ui, candidates, error),
        };

        if retry && let Self::MissingData { candidates, .. } = self {
            *self = Self::new(std::mem::take(candidates));
        }
    }
}

/// Returns `true` when the user asks to look for the database again.
fn missing_data_screen(ui: &mut egui::Ui, candidates: &[PathBuf], error: &LoadError) -> bool {
    let (title, explanation, path) = match error {
        LoadError::Missing => (
            "Base de données introuvable",
            format!(
                "L'application a besoin du fichier {DATA_FILE} (~400 Mo), qui n'est pas inclus \
                 dans l'exécutable. Télécharge-le et place-le ici :"
            ),
            &candidates[0],
        ),
        LoadError::LfsPointer(path) => (
            "Base de données incomplète",
            "Ce fichier n'est qu'un pointeur Git LFS : le dépôt a été cloné sans Git LFS. \
             Lance « git lfs pull » dans le dépôt, ou télécharge la base et remplace ce fichier :"
                .to_owned(),
            path,
        ),
        LoadError::Invalid(path, err) => (
            "Base de données illisible",
            format!(
                "Ce fichier n'a pas pu être lu ({err}). Il est peut-être incomplet ou corrompu : \
                 retélécharge-le et remplace ce fichier :"
            ),
            path,
        ),
    };

    let mut retry = false;

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .inner_margin(Margin::same(28)),
        )
        .show(ui, |ui| {
            ui.add_space(((ui.available_height() - 280.0) / 2.0).max(0.0));
            ui.vertical_centered(|ui| {
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(CornerRadius::same(theme::RADIUS_MD))
                    .shadow(theme::card_shadow(0.3))
                    .inner_margin(Margin::same(28))
                    .show(ui, |ui| {
                        ui.set_width(520.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.label(
                                RichText::new(title)
                                    .size(20.0)
                                    .family(theme::family(theme::FONT_SEMIBOLD))
                                    .color(theme::TEXT),
                            );
                            ui.add_space(8.0);
                            ui.label(RichText::new(explanation).color(theme::TEXT_MUTED));
                            ui.add_space(10.0);

                            egui::Frame::new()
                                .fill(theme::INPUT_BG)
                                .stroke(Stroke::new(1.0, theme::BORDER))
                                .corner_radius(CornerRadius::same(theme::RADIUS_SM))
                                .inner_margin(Margin::symmetric(12, 8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        RichText::new(path.display().to_string())
                                            .monospace()
                                            .color(theme::TEXT),
                                    );
                                });
                            ui.add_space(18.0);

                            ui.horizontal(|ui| {
                                let button_size = egui::vec2(190.0, 34.0);
                                if ui
                                    .add_sized(
                                        button_size,
                                        theme::primary_button("Télécharger la base", 14.0),
                                    )
                                    .clicked()
                                {
                                    ui.ctx().open_url(OpenUrl::new_tab(DATA_URL));
                                }
                                if ui
                                    .add_sized(
                                        button_size,
                                        theme::secondary_button("Réessayer", 14.0),
                                    )
                                    .clicked()
                                {
                                    retry = true;
                                }
                            });
                        });
                    });
            });
        });

    retry
}

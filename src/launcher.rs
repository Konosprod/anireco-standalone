//! Startup wrapper: shows the app once the database is found, or a screen
//! that downloads it (release binaries don't ship the ~400 MB database; it's
//! attached to each GitHub release instead).

use crate::app::AniRecoApp;
use crate::store::{self, LoadError, Store};
use crate::theme;
use eframe::egui::{self, CornerRadius, Margin, OpenUrl, RichText, Stroke};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

pub const DATA_FILE: &str = "anime.db";
const RELEASES_URL: &str = "https://github.com/Konosprod/anireco-standalone/releases/latest";
const DATA_URL: &str =
    "https://github.com/Konosprod/anireco-standalone/releases/latest/download/anime.db";
const CHECKSUM_URL: &str =
    "https://github.com/Konosprod/anireco-standalone/releases/latest/download/anime.db.sha256";

pub enum Launcher {
    Ready(AniRecoApp),
    MissingData(MissingData),
}

impl Launcher {
    pub fn new(candidates: Vec<PathBuf>) -> Self {
        match store::load_first(&candidates) {
            Ok((store, path)) => ready(store, &path),
            Err(error) => Self::MissingData(MissingData {
                candidates,
                error,
                download: None,
                download_error: None,
            }),
        }
    }
}

fn ready(store: Store, path: &Path) -> Launcher {
    eprintln!("Loaded {} anime from {}", store.all.len(), path.display());
    Launcher::Ready(AniRecoApp::new(store))
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let next = match self {
            Self::Ready(app) => return eframe::App::ui(app, ui, frame),
            Self::MissingData(missing) => missing.ui(ui),
        };
        if let Some(next) = next {
            *self = next;
        }
    }
}

pub struct MissingData {
    /// Where the database is looked for, in order. The first one is where a
    /// missing database gets downloaded to.
    candidates: Vec<PathBuf>,
    error: LoadError,
    download: Option<Download>,
    download_error: Option<String>,
}

struct Download {
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    result: mpsc::Receiver<Result<(Store, PathBuf), String>>,
}

impl Download {
    /// Downloads and checks the database on a background thread, then loads it.
    fn start(target: PathBuf, ctx: egui::Context) -> Self {
        let done = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let (sender, result) = mpsc::channel();

        let (thread_done, thread_total) = (done.clone(), total.clone());
        std::thread::spawn(move || {
            let outcome = download_to(&target, &thread_done, &thread_total, &ctx).and_then(|()| {
                store::load_first(std::slice::from_ref(&target))
                    .map_err(|_| "la base téléchargée n'a pas pu être chargée".to_owned())
            });
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });

        Self {
            done,
            total,
            result,
        }
    }
}

fn download_to(
    target: &Path,
    done: &AtomicU64,
    total: &AtomicU64,
    ctx: &egui::Context,
) -> Result<(), String> {
    let expected = ureq::get(CHECKSUM_URL)
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(|err| format!("impossible de récupérer l'empreinte de la base ({err})"))?;
    let expected = expected
        .split_whitespace()
        .next()
        .ok_or("le fichier d'empreinte de la release est vide")?
        .to_lowercase();

    let response = ureq::get(DATA_URL)
        .call()
        .map_err(|err| format!("impossible de télécharger la base ({err})"))?;
    total.store(
        response.body().content_length().unwrap_or(0),
        Ordering::Relaxed,
    );
    let mut reader = response.into_body().into_reader();

    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)
            .map_err(|err| format!("impossible de créer {} ({err})", dir.display()))?;
    }
    // Written under a temporary name, so an interrupted download never
    // leaves a truncated anime.db behind.
    let partial = target.with_extension("db.part");
    let write_error =
        |err: std::io::Error| format!("impossible d'écrire {} ({err})", partial.display());
    let mut file = File::create(&partial).map_err(write_error)?;

    let mut hasher = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| format!("téléchargement interrompu ({err})"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(write_error)?;
        hasher.update(&buffer[..read]);
        done.fetch_add(read as u64, Ordering::Relaxed);
        ctx.request_repaint();
    }
    file.sync_all().map_err(write_error)?;
    drop(file);

    let actual: String = hasher
        .finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != expected {
        let _ = fs::remove_file(&partial);
        return Err(
            "le fichier téléchargé est corrompu (empreinte SHA-256 différente), réessaie"
                .to_owned(),
        );
    }

    fs::rename(&partial, target)
        .map_err(|err| format!("impossible de finaliser {} ({err})", target.display()))
}

impl MissingData {
    /// Returns the launcher state to switch to, if any.
    fn ui(&mut self, ui: &mut egui::Ui) -> Option<Launcher> {
        if let Some(download) = &self.download {
            match download.result.try_recv() {
                Ok(Ok((store, path))) => return Some(ready(store, &path)),
                Ok(Err(message)) => {
                    self.download = None;
                    self.download_error = Some(message);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.download = None;
                    self.download_error =
                        Some("le téléchargement s'est arrêté de façon inattendue".to_owned());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }

        let (title, explanation, target) = match &self.error {
            LoadError::Missing => (
                "Base de données introuvable",
                format!(
                    "L'application a besoin du fichier {DATA_FILE} (~400 Mo), qui n'est pas inclus \
                     dans l'exécutable. Elle peut le télécharger pour toi ici :"
                ),
                self.candidates[0].clone(),
            ),
            LoadError::LfsPointer(path) => (
                "Base de données incomplète",
                "Ce fichier n'est qu'un pointeur Git LFS : le dépôt a été cloné sans Git LFS. \
                 Lance « git lfs pull » dans le dépôt, ou télécharge la base pour remplacer ce fichier :"
                    .to_owned(),
                path.clone(),
            ),
            LoadError::Invalid(path, err) => (
                "Base de données illisible",
                format!(
                    "Ce fichier n'a pas pu être lu ({err}). Il est peut-être incomplet ou corrompu : \
                     télécharge-le à nouveau pour le remplacer :"
                ),
                path.clone(),
            ),
        };

        let mut next = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(Margin::same(28)))
            .show(ui, |ui| {
                ui.add_space(((ui.available_height() - 320.0) / 2.0).max(0.0));
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
                                path_box(ui, &target);
                                ui.add_space(18.0);

                                if let Some(download) = &self.download {
                                    download_progress(ui, download);
                                } else {
                                    if let Some(message) = &self.download_error {
                                        ui.label(RichText::new(format!("Échec : {message}.")).color(theme::ERROR));
                                        ui.add_space(12.0);
                                    }

                                    ui.horizontal(|ui| {
                                        let button_size = egui::vec2(190.0, 34.0);
                                        if ui
                                            .add_sized(button_size, theme::primary_button("Télécharger la base", 14.0))
                                            .clicked()
                                        {
                                            self.download_error = None;
                                            self.download = Some(Download::start(target.clone(), ui.ctx().clone()));
                                        }
                                        if ui
                                            .add_sized(button_size, theme::secondary_button("Réessayer", 14.0))
                                            .on_hover_text("Chercher à nouveau le fichier, si tu l'as placé toi-même")
                                            .clicked()
                                        {
                                            next = Some(Launcher::new(self.candidates.clone()));
                                        }
                                    });
                                    ui.add_space(10.0);
                                    if ui
                                        .link(
                                            RichText::new("Ou le télécharger manuellement depuis GitHub")
                                                .size(13.0)
                                                .color(theme::ACCENT),
                                        )
                                        .clicked()
                                    {
                                        ui.ctx().open_url(OpenUrl::new_tab(RELEASES_URL));
                                    }
                                }
                            });
                        });
                });
            });

        next
    }
}

fn path_box(ui: &mut egui::Ui, path: &Path) {
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
}

fn download_progress(ui: &mut egui::Ui, download: &Download) {
    const MB: f32 = 1_000_000.0;
    let done = download.done.load(Ordering::Relaxed) as f32;
    let total = download.total.load(Ordering::Relaxed) as f32;

    let (fraction, text) = if total > 0.0 {
        (
            done / total,
            format!("{:.0} / {:.0} Mo", done / MB, total / MB),
        )
    } else {
        (0.0, "Connexion…".to_owned())
    };

    ui.add(
        egui::ProgressBar::new(fraction)
            .text(RichText::new(text).color(theme::TEXT))
            .fill(theme::ACCENT_SOFT_BG)
            .desired_height(26.0)
            .corner_radius(CornerRadius::same(theme::RADIUS_SM)),
    );
}

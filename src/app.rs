use crate::recommend;
use crate::store::Store;
use eframe::egui;
use std::collections::HashSet;

const COMMON_COLOR: egui::Color32 = egui::Color32::from_rgb(31, 166, 114);
const ACCENT_BG: egui::Color32 = egui::Color32::from_rgb(237, 236, 255);
const CARD_WIDTH: f32 = 150.0;
const COVER_SIZE: egui::Vec2 = egui::Vec2::new(140.0, 190.0);

pub struct AniRecoApp {
    store: Store,
    search_text: String,
    suggestions: Vec<usize>,
    selected: Option<usize>,
    ours: Vec<usize>,
    anilist: Vec<usize>,
}

impl AniRecoApp {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            search_text: String::new(),
            suggestions: Vec::new(),
            selected: None,
            ours: Vec::new(),
            anilist: Vec::new(),
        }
    }

    fn update_suggestions(&mut self) {
        let query = self.search_text.to_lowercase();
        if query.len() < 3 {
            self.suggestions.clear();
            return;
        }

        self.suggestions = self
            .store
            .all
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.title_english.to_lowercase().starts_with(&query)
                    || a.title_native.to_lowercase().starts_with(&query)
                    || a.title_romaji.to_lowercase().starts_with(&query)
            })
            .map(|(idx, _)| idx)
            .take(20)
            .collect();
    }

    fn select(&mut self, idx: usize) {
        let id = self.store.all[idx].id;
        self.selected = Some(idx);
        self.suggestions.clear();
        self.search_text = self.store.all[idx].display_title().to_string();
        self.ours = recommend::recommend(&self.store, id);
        self.anilist = recommend::anilist_recommendations(&self.store, id);
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("Anime Reco");
                ui.add_space(20.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_text)
                        .hint_text("Rechercher un anime...")
                        .desired_width(320.0),
                );
                if response.changed() {
                    self.update_suggestions();
                }
            });

            if !self.suggestions.is_empty() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_max_height(220.0);
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            let suggestions = self.suggestions.clone();
                            let mut pick = None;
                            for idx in suggestions {
                                let title = self.store.all[idx].display_title().to_string();
                                if ui.selectable_label(false, title).clicked() {
                                    pick = Some(idx);
                                }
                            }
                            if let Some(idx) = pick {
                                self.select(idx);
                            }
                        });
                });
            }
            ui.add_space(8.0);
        });
    }

    fn render_source_card(&self, ui: &mut egui::Ui, idx: usize) {
        let anime = &self.store.all[idx];
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                if !anime.cover.is_empty() {
                    ui.add(
                        egui::Image::new(anime.cover.clone())
                            .fit_to_exact_size(COVER_SIZE)
                            .corner_radius(6.0),
                    );
                }
                ui.vertical(|ui| {
                    ui.heading(anime.display_title());
                    ui.label(strip_html(&anime.description));
                    ui.horizontal_wrapped(|ui| {
                        for genre in &anime.genres {
                            genre_badge(ui, genre);
                        }
                    });
                });
            });
        });
    }

    fn render_card_grid(&self, ui: &mut egui::Ui, ids: &[usize], other: &HashSet<i64>, salt: &str) {
        egui::Grid::new(ui.id().with(salt))
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                for (i, &idx) in ids.iter().enumerate() {
                    self.render_card(ui, idx, other);
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
    }

    fn render_card(&self, ui: &mut egui::Ui, idx: usize, other: &HashSet<i64>) {
        let anime = &self.store.all[idx];
        let is_common = other.contains(&anime.id);

        let frame = egui::Frame::group(ui.style()).stroke(if is_common {
            egui::Stroke::new(2.0, COMMON_COLOR)
        } else {
            ui.style().visuals.widgets.noninteractive.bg_stroke
        });

        let response = frame
            .show(ui, |ui| {
                ui.set_width(CARD_WIDTH);
                ui.vertical(|ui| {
                    if !anime.cover.is_empty() {
                        ui.add(
                            egui::Image::new(anime.cover.clone())
                                .fit_to_exact_size(COVER_SIZE)
                                .corner_radius(4.0),
                        );
                    }
                    if is_common {
                        ui.colored_label(COMMON_COLOR, "Commun");
                    }
                    ui.label(anime.display_title());
                });
            })
            .response;

        if response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            && !anime.site_url.is_empty()
        {
            let _ = webbrowser::open(&anime.site_url);
        }
    }
}

impl eframe::App for AniRecoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_header(ui);

        egui::CentralPanel::default().show(ui, |ui| {
            let Some(selected_idx) = self.selected else {
                ui.centered_and_justified(|ui| {
                    ui.label("Cherche un anime ci-dessus pour voir ses recommandations.");
                });
                return;
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_source_card(ui, selected_idx);
                ui.add_space(12.0);

                let ours_ids: HashSet<i64> =
                    self.ours.iter().map(|&i| self.store.all[i].id).collect();
                let anilist_ids: HashSet<i64> =
                    self.anilist.iter().map(|&i| self.store.all[i].id).collect();
                let common = ours_ids.intersection(&anilist_ids).count();

                ui.label(format!(
                    "{common} titre(s) en commun entre notre algo et les recommandations AniList (sur {} vs {}).",
                    self.ours.len(),
                    self.anilist.len(),
                ));
                ui.add_space(12.0);

                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.strong("NOTRE ALGO");
                        ui.add_space(6.0);
                        self.render_card_grid(ui, &self.ours, &anilist_ids, "ours");
                    });
                    columns[1].vertical(|ui| {
                        ui.strong("RECOMMANDATIONS ANILIST");
                        ui.add_space(6.0);
                        self.render_card_grid(ui, &self.anilist, &ours_ids, "anilist");
                    });
                });
            });
        });
    }
}

fn genre_badge(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ACCENT_BG)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(text);
        });
}

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

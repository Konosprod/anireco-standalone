use crate::recommend;
use crate::store::Store;
use crate::theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Rect, RichText, Stroke,
    StrokeKind, UiBuilder,
    text::{LayoutJob, TextFormat, TextWrapping},
};
use std::collections::HashSet;

const CARD_GAP: f32 = 16.0;
const CARD_PADDING: f32 = 10.0;
const MAX_CARD_WIDTH: f32 = 220.0;
const COVER_RATIO: f32 = 1.42;
const TITLE_HEIGHT: f32 = 36.0;
const BADGE_HEIGHT: f32 = 22.0;
const COLUMN_GUTTER: f32 = 48.0;
const SOURCE_COVER_SIZE: egui::Vec2 = egui::Vec2::new(150.0, 213.0);
const SUGGESTION_ROW_HEIGHT: f32 = 34.0;
const SUGGESTION_VISIBLE_ROWS: f32 = 8.0;

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
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .inner_margin(Margin::symmetric(28, 14)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Anime Reco")
                            .size(19.0)
                            .family(theme::family(theme::FONT_BOLD))
                            .color(theme::TEXT),
                    );
                    ui.add_space(24.0);

                    let search = egui::Frame::new()
                        .fill(theme::INPUT_BG)
                        .stroke(Stroke::new(1.0, theme::BORDER))
                        .corner_radius(CornerRadius::same(theme::RADIUS_SM))
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search_text)
                                    .hint_text("Rechercher un anime...")
                                    .desired_width(340.0)
                                    .frame(egui::Frame::NONE),
                            )
                        });

                    if search.inner.changed() {
                        self.update_suggestions();
                    }
                    if !self.suggestions.is_empty() {
                        self.render_suggestions(ui.ctx(), search.response.rect, &search.inner);
                    }
                });
            });
    }

    /// Dropdown floating right under the search field, same width as it.
    fn render_suggestions(&mut self, ctx: &egui::Context, anchor: Rect, search: &egui::Response) {
        let width = anchor.width();
        let mut pick = None;

        let area = egui::Area::new(egui::Id::new("search_suggestions"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor.left_bottom() + egui::vec2(0.0, 4.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .shadow(theme::card_shadow(0.8))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        egui::ScrollArea::vertical()
                            .max_height(SUGGESTION_ROW_HEIGHT * SUGGESTION_VISIBLE_ROWS)
                            .show(ui, |ui| {
                                let last = self.suggestions.len().saturating_sub(1);
                                for (i, &idx) in self.suggestions.iter().enumerate() {
                                    let (rect, row) = ui.allocate_exact_size(
                                        egui::vec2(width, SUGGESTION_ROW_HEIGHT),
                                        egui::Sense::click(),
                                    );
                                    let painter = ui.painter_at(rect);
                                    if row.hovered() {
                                        painter.rect_filled(rect, 0.0, theme::ACCENT_SOFT_BG);
                                    }
                                    painter.text(
                                        rect.left_center() + egui::vec2(12.0, 0.0),
                                        Align2::LEFT_CENTER,
                                        self.store.all[idx].display_title(),
                                        FontId::new(14.0, FontFamily::Proportional),
                                        if row.hovered() { theme::ACCENT } else { theme::TEXT },
                                    );
                                    if i != last {
                                        painter.hline(
                                            rect.x_range(),
                                            rect.bottom() - 0.5,
                                            Stroke::new(1.0, theme::BORDER.gamma_multiply(0.6)),
                                        );
                                    }
                                    if row.clicked() {
                                        pick = Some(idx);
                                    }
                                }
                            });
                    });
            });

        let dismissed = ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || (area.response.clicked_elsewhere() && !search.hovered());

        if let Some(idx) = pick {
            self.select(idx);
        } else if dismissed {
            self.suggestions.clear();
        }
    }

    fn render_source_card(&self, ui: &mut egui::Ui, idx: usize) {
        let anime = &self.store.all[idx];
        egui::Frame::new()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(CornerRadius::same(theme::RADIUS_MD))
            .shadow(theme::card_shadow(0.0))
            .inner_margin(Margin::same(20))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    if !anime.cover.is_empty() {
                        ui.add(
                            egui::Image::new(anime.cover.as_str())
                                .fit_to_exact_size(SOURCE_COVER_SIZE)
                                .corner_radius(theme::RADIUS_SM),
                        );
                        ui.add_space(12.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(anime.display_title())
                                .size(22.0)
                                .family(theme::family(theme::FONT_SEMIBOLD))
                                .color(theme::TEXT),
                        );
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for genre in &anime.genres {
                                badge(ui, genre, theme::ACCENT_SOFT_BG, theme::ACCENT);
                            }
                        });
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(strip_html(&anime.description))
                                .size(13.5)
                                .color(theme::TEXT_MUTED),
                        );
                        if !anime.site_url.is_empty() {
                            ui.add_space(6.0);
                            ui.hyperlink_to("Voir sur AniList", &anime.site_url);
                        }
                    });
                });
            });
    }

    fn render_recommendations(&self, ui: &mut egui::Ui) {
        let ours_set: HashSet<usize> = self.ours.iter().copied().collect();
        let anilist_set: HashSet<usize> = self.anilist.iter().copied().collect();
        let common = ours_set.intersection(&anilist_set).count();

        ui.horizontal(|ui| {
            badge(ui, &format!("{common} en commun"), theme::BOTH_SOFT_BG, theme::BOTH);
            ui.label(
                RichText::new("entre notre algo et les recommandations AniList")
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.add_space(18.0);

        let (left, right, bottom) = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_GUTTER;
                ui.columns(2, |cols| {
                    self.render_column(&mut cols[0], "Notre algo", theme::ACCENT, &self.ours, &anilist_set);
                    self.render_column(
                        &mut cols[1],
                        "Recommandations AniList",
                        theme::ANILIST,
                        &self.anilist,
                        &ours_set,
                    );
                    let bottom = cols[0].min_rect().bottom().max(cols[1].min_rect().bottom());
                    (cols[0].min_rect(), cols[1].min_rect(), bottom)
                })
            })
            .inner;

        let divider_x = (left.right() + right.left()) / 2.0;
        ui.painter()
            .vline(divider_x, left.top()..=bottom, Stroke::new(1.0, theme::BORDER));
    }

    fn render_column(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        dot_color: Color32,
        ids: &[usize],
        other: &HashSet<usize>,
    ) {
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 4.0, dot_color);
            ui.label(
                RichText::new(title)
                    .size(16.0)
                    .family(theme::family(theme::FONT_SEMIBOLD))
                    .color(theme::TEXT),
            );
            ui.label(RichText::new(ids.len().to_string()).color(theme::TEXT_MUTED));
        });
        ui.add_space(12.0);

        if ids.is_empty() {
            ui.label(RichText::new("Aucune recommandation.").color(theme::TEXT_MUTED));
            return;
        }

        let card_width = ((ui.available_width() - CARD_GAP) / 2.0)
            .floor()
            .min(MAX_CARD_WIDTH);
        let cover_width = card_width - 2.0 * CARD_PADDING;
        let cover_size = egui::vec2(cover_width, (cover_width * COVER_RATIO).round());
        let card_size = egui::vec2(
            card_width,
            CARD_PADDING + cover_size.y + 8.0 + TITLE_HEIGHT + 6.0 + BADGE_HEIGHT + CARD_PADDING,
        );

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(CARD_GAP, CARD_GAP);
            for row in ids.chunks(2) {
                ui.horizontal(|ui| {
                    for &idx in row {
                        self.render_card(ui, idx, other.contains(&idx), card_size, cover_size);
                    }
                });
            }
        });
    }

    fn render_card(
        &self,
        ui: &mut egui::Ui,
        idx: usize,
        is_common: bool,
        card_size: egui::Vec2,
        cover_size: egui::Vec2,
    ) {
        let anime = &self.store.all[idx];
        let (rect, response) = ui.allocate_exact_size(card_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let hover = ui.ctx().animate_bool(response.id, response.hovered());
            let radius = CornerRadius::same(theme::RADIUS_MD);
            let border = if is_common {
                theme::BOTH
            } else {
                theme::BORDER.lerp_to_gamma(theme::ACCENT, hover)
            };

            let painter = ui.painter();
            painter.add(theme::card_shadow(hover).as_shape(rect, radius));
            painter.rect(
                rect,
                radius,
                theme::SURFACE,
                Stroke::new(if is_common { 1.5 } else { 1.0 }, border),
                StrokeKind::Inside,
            );

            // Draw inside the rect allocated above, without allocating a
            // second time in the row (that's what made cards overlap before).
            let inner = rect.shrink(CARD_PADDING);
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.shrink_clip_rect(inner);
            child.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);

            if !anime.cover.is_empty() {
                child.add(
                    egui::Image::new(anime.cover.as_str())
                        .fit_to_exact_size(cover_size)
                        .corner_radius(theme::RADIUS_SM),
                );
            }
            child.add_space(8.0);
            child.label(two_line_title(anime.display_title(), inner.width()));
            if is_common {
                child.add_space(6.0);
                badge(&mut child, "Commun", theme::BOTH_SOFT_BG, theme::BOTH);
            }
        }

        if response.clicked() && !anime.site_url.is_empty() {
            let _ = webbrowser::open(&anime.site_url);
        }
    }
}

impl eframe::App for AniRecoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_header(ui);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(28, 24)),
            )
            .show(ui, |ui| {
                let Some(selected_idx) = self.selected else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Cherche un anime ci-dessus pour voir ses recommandations.")
                                .size(15.0)
                                .color(theme::TEXT_MUTED),
                        );
                    });
                    return;
                };

                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.render_source_card(ui, selected_idx);
                        ui.add_space(24.0);
                        self.render_recommendations(ui);
                        ui.add_space(8.0);
                    });
            });
    }
}

fn badge(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(12.0)
                    .family(theme::family(theme::FONT_MEDIUM))
                    .color(fg),
            );
        });
}

/// Title wrapped to at most two lines, ellipsized past that, so every card
/// keeps the same height.
fn two_line_title(title: &str, max_width: f32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        title.to_owned(),
        TextFormat {
            font_id: FontId::new(13.0, theme::family(theme::FONT_MEDIUM)),
            color: theme::TEXT,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width,
        max_rows: 2,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    job
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

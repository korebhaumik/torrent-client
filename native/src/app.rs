mod details;
mod dialogs;

use crate::{
    config::{Paths, Settings},
    engine::{Command, Engine},
    model::*,
    search::{SearchClient, source_name},
};
use eframe::egui::*;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::design::*;

/// Seconds of transfer-rate history kept for the stat sparklines.
const HISTORY_LEN: usize = 60;

/// What the footer of a stat card shows.
enum Trend<'a> {
    /// Recent transfer rates, drawn as a sparkline.
    Rates(&'a [f64]),
    /// A fraction of a whole, drawn as a progress bar.
    Share(f32),
}

pub struct TorrentApp {
    engine: Engine,
    paths: Paths,
    settings: Settings,
    _lock: std::fs::File,
    configured_theme: Option<bool>,
    view: String,
    filter: String,
    query: String,
    add_open: bool,
    file_tab: bool,
    magnet_input: String,
    selected_file: Option<PathBuf>,
    adding: bool,
    add_error: Option<String>,
    add_focus: bool,
    selected: Option<String>,
    result_detail: Option<SearchResult>,
    deleting: Option<Torrent>,
    delete_files: bool,
    settings_open: bool,
    notice: Option<(String, bool, Instant)>,
    search_query: String,
    source: String,
    response: SearchResponse,
    search_due: Option<Instant>,
    searching: bool,
    search_client: SearchClient,
    runtime: tokio::runtime::Runtime,
    search_tx: mpsc::Sender<(u64, SearchResponse)>,
    search_rx: mpsc::Receiver<(u64, SearchResponse)>,
    search_id: u64,
    search_task: Option<tokio::task::JoinHandle<()>>,
    history: VecDeque<(f64, f64)>,
    history_sampled: Instant,
}

impl TorrentApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        paths: Paths,
        settings: Settings,
        lock: std::fs::File,
    ) -> anyhow::Result<Self> {
        cc.egui_ctx.set_fonts(fonts());
        let (search_tx, search_rx) = mpsc::channel();
        Ok(Self {
            engine: Engine::start(paths.clone(), settings.clone()),
            paths,
            settings,
            _lock: lock,
            configured_theme: None,
            view: "library".into(),
            filter: "all".into(),
            query: String::new(),
            add_open: false,
            file_tab: false,
            magnet_input: String::new(),
            selected_file: None,
            adding: false,
            add_error: None,
            add_focus: false,
            selected: None,
            result_detail: None,
            deleting: None,
            delete_files: false,
            settings_open: false,
            notice: None,
            search_query: String::new(),
            source: "all".into(),
            response: SearchResponse::default(),
            search_due: None,
            searching: false,
            search_client: SearchClient::new()?,
            runtime: tokio::runtime::Runtime::new()?,
            search_tx,
            search_rx,
            search_id: 0,
            search_task: None,
            history: VecDeque::with_capacity(HISTORY_LEN),
            history_sampled: Instant::now(),
        })
    }
    fn open_add(&mut self) {
        self.add_open = true;
        self.add_focus = true;
        self.add_error = None;
    }
    fn notice(&mut self, text: impl Into<String>, error: bool) {
        self.notice = Some((text.into(), error, Instant::now()));
    }
    fn persist_settings(&mut self) {
        if let Err(e) = self.paths.save_settings(&self.settings) {
            self.notice(e.to_string(), true);
        }
    }
    fn configure(&self, ctx: &Context, p: Palette) {
        let mut style = Style {
            visuals: if self.settings.light {
                Visuals::light()
            } else {
                Visuals::dark()
            },
            ..Default::default()
        };
        let v = &mut style.visuals;
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.override_text_color = Some(p.text);
        v.extreme_bg_color = p.field;
        v.text_edit_bg_color = Some(p.field);
        v.faint_bg_color = p.panel2;
        v.window_stroke = Stroke::new(1.0_f32, p.border);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(10);
        v.popup_shadow = Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if p.light { 28 } else { 80 }),
        };
        v.selection.bg_fill = BLUE.gamma_multiply(if p.light { 0.22 } else { 0.4 });
        v.selection.stroke = Stroke::new(1.0_f32, BLUE);
        v.text_cursor.stroke = Stroke::new(2.0_f32, BLUE);
        v.hyperlink_color = p.ink(BLUE);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
        v.widgets.inactive.bg_fill = p.panel2;
        v.widgets.inactive.weak_bg_fill = p.panel2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
        v.widgets.hovered.bg_fill = mix(p.panel2, BLUE, 0.06);
        v.widgets.hovered.weak_bg_fill = mix(p.panel2, BLUE, 0.06);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, mix(p.border, BLUE, 0.4));
        v.widgets.active.bg_fill = p.active;
        v.widgets.active.weak_bg_fill = p.active;
        v.widgets.active.bg_stroke = Stroke::new(1.0_f32, BLUE);
        for widget in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.noninteractive,
            &mut v.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(8);
            widget.expansion = 0.0;
        }
        style.spacing.item_spacing = vec2(10.0, 8.0);
        style.spacing.button_padding = vec2(12.0, 8.0);
        style.spacing.interact_size = vec2(32.0, 20.0);
        style.spacing.window_margin = Margin::same(20);
        style.spacing.icon_width = 16.0;
        style.spacing.icon_width_inner = 9.0;
        style.spacing.scroll = style::ScrollStyle::thin();
        style.spacing.scroll.bar_width = 6.0;
        style.animation_time = 0.13;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(12.5));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(11.0));
        ctx.set_style(style);
    }

    fn sidebar(&mut self, ctx: &Context, snapshot: &Snapshot, p: Palette) {
        SidePanel::left("sidebar")
            .exact_width(232.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(p.sidebar))
            .show(ctx, |ui| {
                let (brand, _) = ui.allocate_exact_size(vec2(232.0, 72.0), Sense::hover());
                brand_mark(
                    ui.painter(),
                    Rect::from_min_size(brand.min + vec2(22.0, 22.0), vec2(30.0, 30.0)),
                    p.light,
                );
                ui.painter().text(
                    brand.min + vec2(63.0, 38.0),
                    Align2::LEFT_CENTER,
                    "Torrent",
                    display_font(18.0),
                    p.text,
                );
                let version =
                    Rect::from_center_size(brand.min + vec2(196.0, 37.0), vec2(30.0, 18.0));
                ui.painter()
                    .rect_filled(version, 9, mix(p.sidebar, p.border, 0.7));
                ui.painter().text(
                    version.center(),
                    Align2::CENTER_CENTER,
                    "2.1",
                    FontId::proportional(10.0),
                    p.muted,
                );
                Frame::new()
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let section = |ui: &mut Ui, label: &str| {
                            Frame::new()
                                .inner_margin(Margin {
                                    left: 10,
                                    right: 10,
                                    top: 0,
                                    bottom: 6,
                                })
                                .show(ui, |ui| eyebrow(ui, label, p));
                        };
                        section(ui, "Workspace");
                        if nav(
                            ui,
                            "Library",
                            Some(snapshot.torrents.len()),
                            self.view == "library" && self.filter == "all",
                            Some(Glyph::Library),
                            p,
                        ) {
                            self.view = "library".into();
                            self.filter = "all".into();
                        }
                        if nav(
                            ui,
                            "Discover",
                            None,
                            self.view == "discover",
                            Some(Glyph::Compass),
                            p,
                        ) {
                            self.view = "discover".into();
                        }
                        ui.add_space(20.0);
                        section(ui, "Activity");
                        for (id, label, glyph) in [
                            ("downloading", "Downloading", Glyph::Down),
                            ("seeding", "Seeding", Glyph::Up),
                            ("paused", "Paused", Glyph::Pause),
                        ] {
                            let count = snapshot
                                .torrents
                                .iter()
                                .filter(|t| t.matches(id, ""))
                                .count();
                            let active = self.view == "library" && self.filter == id;
                            if nav(ui, label, Some(count), active, Some(glyph), p) {
                                self.view = "library".into();
                                self.filter = if active { "all" } else { id }.into();
                            }
                        }
                    });
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.allocate_ui_with_layout(
                        vec2(232.0, 170.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            Frame::new()
                                .inner_margin(Margin::symmetric(12, 14))
                                .show(ui, |ui| {
                                    ui.set_width(208.0);
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    if nav(ui, "Preferences", None, false, Some(Glyph::Settings), p)
                                    {
                                        self.settings_open = true;
                                    }
                                    self.transfer_summary(ui, snapshot, p);
                                });
                        },
                    );
                });
                let rect = ui.max_rect();
                ui.painter()
                    .vline(rect.right(), rect.y_range(), Stroke::new(1.0_f32, p.border));
            });
    }

    fn transfer_summary(&self, ui: &mut Ui, snapshot: &Snapshot, p: Palette) {
        Frame::new()
            .fill(p.panel)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(12)
            .inner_margin(Margin::symmetric(12, 10))
            .shadow(card_shadow(p))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                    let color = if snapshot.ready { GREEN } else { YELLOW };
                    ui.painter()
                        .circle_filled(r.center(), 4.5, color.gamma_multiply(0.25));
                    ui.painter().circle_filled(r.center(), 2.5, color);
                    ui.label(
                        RichText::new(if snapshot.ready {
                            "Engine ready"
                        } else {
                            "Starting engine…"
                        })
                        .size(11.5)
                        .color(p.muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} ratio",
                                ratio(snapshot.uploaded(), snapshot.downloaded())
                            ))
                            .size(11.0)
                            .color(p.faint),
                        );
                    });
                });
                for (glyph, color, label, value) in [
                    (Glyph::Down, BLUE, "Download", snapshot.down()),
                    (Glyph::Up, GREEN, "Upload", snapshot.up()),
                ] {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        icon(ui.painter(), r, glyph, p.ink(color));
                        ui.label(RichText::new(label).size(12.0).color(p.muted));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(speed(value)).size(12.0).color(p.text));
                        });
                    });
                }
            });
    }

    fn header(&mut self, ctx: &Context, snapshot: &Snapshot, p: Palette) {
        TopBottomPanel::top("header")
            .exact_height(76.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(32, 18)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let (title, subtitle) = if self.view == "discover" {
                            (
                                "Discover".to_owned(),
                                "Search public indexes for something new".to_owned(),
                            )
                        } else {
                            let title = match self.filter.as_str() {
                                "downloading" => "Downloading",
                                "seeding" => "Seeding",
                                "paused" => "Paused",
                                _ => "Library",
                            };
                            let total = snapshot.torrents.len();
                            (
                                title.to_owned(),
                                format!(
                                    "{total} {} · {} active",
                                    if total == 1 { "torrent" } else { "torrents" },
                                    snapshot.active()
                                ),
                            )
                        };
                        ui.label(display(title, 21.0).color(p.text));
                        ui.label(RichText::new(subtitle).size(11.5).color(p.muted));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary(ui, "+  Add torrent")
                            .on_hover_text(format!(
                                "Add a magnet link or torrent file · {}",
                                command_shortcut(ui.ctx(), Key::K)
                            ))
                            .clicked()
                        {
                            self.open_add();
                        }
                        if icon_button(
                            ui,
                            if self.settings.light {
                                Glyph::Sun
                            } else {
                                Glyph::Moon
                            },
                            if self.settings.light {
                                "Switch to dark theme"
                            } else {
                                "Switch to light theme"
                            },
                            p,
                        )
                        .clicked()
                        {
                            self.settings.light = !self.settings.light;
                            self.persist_settings();
                        }
                        if self.view != "discover" {
                            self.library_filter(ui, p);
                        }
                    });
                });
                let r = ui.max_rect();
                ui.painter().hline(
                    r.x_range().expand(32.0),
                    r.bottom() + 18.0,
                    Stroke::new(1.0_f32, mix(p.border, p.bg, 0.3)),
                );
            });
    }

    fn library_filter(&mut self, ui: &mut Ui, p: Palette) {
        let width = (ui.available_width() - 16.0).clamp(150.0, 300.0);
        let field = ui.add_sized(
            [width, 34.0],
            TextEdit::singleline(&mut self.query)
                .id(Id::new("library-filter"))
                .hint_text(RichText::new("Filter library").color(p.faint))
                .font(FontId::proportional(12.5))
                .margin(Margin {
                    left: 34,
                    right: 30,
                    top: 9,
                    bottom: 8,
                }),
        );
        icon(
            ui.painter(),
            Rect::from_min_size(field.rect.min + vec2(12.0, 10.0), vec2(14.0, 14.0)),
            Glyph::Search,
            p.faint,
        );
        if self.query.is_empty() {
            let hint = command_shortcut(ui.ctx(), Key::F);
            let r = Rect::from_center_size(
                field.rect.right_center() - vec2(22.0, 0.0),
                vec2(30.0, 18.0),
            );
            ui.painter()
                .rect_stroke(r, 5, Stroke::new(1.0_f32, p.border), StrokeKind::Inside);
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                hint,
                FontId::proportional(10.0),
                p.faint,
            );
        } else {
            let rect = Rect::from_center_size(
                field.rect.right_center() - vec2(16.0, 0.0),
                vec2(24.0, 24.0),
            );
            let clear = ui.interact(rect, Id::new("clear-library"), Sense::click());
            clear.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, true, "Clear library filter")
            });
            icon(
                ui.painter(),
                rect.shrink(7.0),
                Glyph::Close,
                if clear.hovered() { p.text } else { p.muted },
            );
            if clear.clicked() {
                self.query.clear();
                field.request_focus();
            }
        }
    }

    fn record_history(&mut self, s: &Snapshot) {
        if self.history_sampled.elapsed() < Duration::from_secs(1) && !self.history.is_empty() {
            return;
        }
        self.history_sampled = Instant::now();
        if self.history.len() == HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back((s.down(), s.up()));
    }

    fn stats(&self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let columns = if ui.available_width() < 700.0 { 2 } else { 4 };
        let ratio_value = if s.downloaded() == 0 {
            if s.uploaded() > 0 { 1.0 } else { 0.0 }
        } else {
            (s.uploaded() as f32 / s.downloaded() as f32).min(1.0)
        };
        let ratio_hint = if s.uploaded() == 0 {
            "No uploads yet"
        } else if s.uploaded() < s.downloaded() {
            "Building your ratio"
        } else {
            "Healthy sharing"
        };
        let active = if s.torrents.is_empty() {
            0.0
        } else {
            s.active() as f32 / s.torrents.len() as f32
        };
        let downs: Vec<f64> = self.history.iter().map(|h| h.0).collect();
        let ups: Vec<f64> = self.history.iter().map(|h| h.1).collect();
        let cards = [
            (
                "Download",
                speed(s.down()),
                format!("{} received", bytes(s.downloaded() as f64)),
                Glyph::Down,
                BLUE,
                Trend::Rates(&downs),
            ),
            (
                "Upload",
                speed(s.up()),
                format!("{} shared", bytes(s.uploaded() as f64)),
                Glyph::Up,
                GREEN,
                Trend::Rates(&ups),
            ),
            (
                "Active swarm",
                s.active().to_string(),
                format!(
                    "{} idle · {} total",
                    s.torrents.len().saturating_sub(s.active()),
                    s.torrents.len()
                ),
                Glyph::Peers,
                VIOLET,
                Trend::Share(active),
            ),
            (
                "Share ratio",
                ratio(s.uploaded(), s.downloaded()),
                ratio_hint.into(),
                Glyph::Refresh,
                YELLOW,
                Trend::Share(ratio_value),
            ),
        ];
        for chunk in cards.chunks(columns) {
            ui.columns(columns, |columns| {
                for (ui, (label, value, sub, g, color, trend)) in columns.iter_mut().zip(chunk) {
                    card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(*label).size(12.0).color(p.muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                icon_badge(ui, *g, *color, p, 26.0);
                            });
                        });
                        ui.add(Label::new(display(value, 24.0).color(p.text)).truncate());
                        ui.add(Label::new(RichText::new(sub).size(11.5).color(p.faint)).truncate());
                        ui.add_space(8.0);
                        let (r, _) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
                        match trend {
                            Trend::Rates(values) => sparkline(ui.painter(), r, values, *color),
                            Trend::Share(fraction) => {
                                let mut bar = ui.new_child(UiBuilder::new().max_rect(
                                    Rect::from_min_max(pos2(r.left(), r.bottom() - 5.0), r.max),
                                ));
                                progress(&mut bar, *fraction, *color, 5.0, p);
                            }
                        }
                    });
                }
            });
        }
        ui.add_space(26.0);
    }

    fn library(&mut self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let rows: Vec<_> = s
            .torrents
            .iter()
            .filter(|t| t.matches(&self.filter, &self.query))
            .collect();
        ui.horizontal(|ui| {
            let title = match self.filter.as_str() {
                "downloading" => "Downloading",
                "seeding" => "Seeding",
                "paused" => "Paused",
                _ => "All torrents",
            };
            ui.label(display(title, 16.0).color(p.text));
            badge(ui, &rows.len().to_string(), p.muted, p);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if (self.filter != "all" || !self.query.is_empty())
                    && ghost(ui, "Reset filters", Some(Glyph::Refresh), p).clicked()
                {
                    self.filter = "all".into();
                    self.query.clear();
                }
            });
        });
        ui.add_space(10.0);
        if rows.is_empty() {
            self.library_empty(ui, s, p);
            return;
        }
        let cols = ((ui.available_width() + 14.0) / 280.0)
            .floor()
            .clamp(1.0, 4.0) as usize;
        for chunk in rows.chunks(cols) {
            ui.columns(cols, |columns| {
                for (ui, t) in columns.iter_mut().zip(chunk) {
                    self.torrent_card(ui, t, p);
                }
            });
            ui.add_space(6.0);
        }
    }

    fn library_empty(&mut self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let empty = s.torrents.is_empty();
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 340.0), Sense::hover());
        ui.painter().add(card_shadow(p).as_shape(r, 16));
        ui.painter().rect(
            r,
            16,
            p.panel,
            Stroke::new(1.0_f32, p.border),
            StrokeKind::Inside,
        );
        // Soft concentric rings behind the illustration.
        let center = pos2(r.center().x, r.top() + 84.0);
        for (radius, alpha) in [(56.0, 0.05), (42.0, 0.08)] {
            ui.painter()
                .circle_filled(center, radius, BLUE.gamma_multiply(alpha));
        }
        let mut content = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    r.min + vec2(24.0, 58.0),
                    r.max - vec2(24.0, 24.0),
                ))
                .layout(Layout::top_down(Align::Center)),
        );
        if empty {
            let (mark, _) = content.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
            brand_mark(content.painter(), mark, p.light);
        } else {
            icon_badge(&mut content, Glyph::Search, BLUE, p, 52.0);
        }
        content.add_space(26.0);
        content.label(
            display(
                if !s.ready {
                    "Getting things ready"
                } else if empty {
                    "A little space for your next download"
                } else {
                    "No matching torrents"
                },
                19.0,
            )
            .color(p.text),
        );
        content.add_space(2.0);
        content.label(
            RichText::new(if empty {
                "Add a magnet link or a .torrent file to get started."
            } else {
                "Try another name or clear your activity filters."
            })
            .size(12.5)
            .color(p.muted),
        );
        content.add_space(14.0);
        content.horizontal(|ui| {
            let button_width = |text: &str, icon_width: f32| {
                ui.painter()
                    .layout_no_wrap(text.into(), FontId::proportional(12.5), p.text)
                    .size()
                    .x
                    + 26.0
                    + icon_width
            };
            let total = if empty {
                button_width("Add torrent", 21.0)
                    + ui.spacing().item_spacing.x
                    + button_width("Explore torrents", 21.0)
            } else {
                button_width("Reset filters", 0.0)
            };
            let pad = ((ui.available_width() - total) / 2.0).max(0.0);
            ui.add_space(pad);
            if primary(
                ui,
                if empty {
                    "+  Add torrent"
                } else {
                    "Reset filters"
                },
            )
            .clicked()
            {
                if empty {
                    self.open_add();
                } else {
                    self.filter = "all".into();
                    self.query.clear();
                }
            }
            if empty && secondary(ui, "Explore torrents", Some(Glyph::Compass), p).clicked() {
                self.view = "discover".into();
            }
        });
        content.add_space(12.0);
        let hint = format!(
            "{} to add a torrent  ·  {} to choose a file",
            command_shortcut(content.ctx(), Key::K),
            command_shortcut(content.ctx(), Key::O)
        );
        content.label(RichText::new(hint).size(11.0).color(p.faint));
    }

    fn torrent_card(&mut self, ui: &mut Ui, t: &Torrent, p: Palette) {
        let selected = self.selected.as_deref() == Some(&t.hash);
        let color = status_color(t);
        card(p)
            .stroke(Stroke::new(
                1.0_f32,
                if selected {
                    mix(p.border, BLUE, 0.7)
                } else {
                    p.border
                },
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.horizontal(|ui| {
                    status_pill(ui, t.status(), color, p);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            display(format!("{:.0}%", t.progress() * 100.0), 15.0).color(p.text),
                        );
                    });
                });
                ui.add_space(4.0);
                let name = clipped_text(ui, &t.name, 13.5, p.text, 2, 38.0);
                if ui
                    .interact(name.rect, Id::new(("select", &t.hash)), Sense::click())
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                {
                    self.selected = Some(t.hash.clone());
                }
                progress(ui, t.progress(), color, 6.0, p);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if t.size == 0 {
                            "Fetching metadata…".to_owned()
                        } else {
                            format!(
                                "{} of {}",
                                bytes(t.downloaded.min(t.size) as f64),
                                bytes(t.size as f64)
                            )
                        })
                        .size(11.5)
                        .color(p.muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if t.error.is_some() {
                                "Needs attention".into()
                            } else if t.done {
                                "Complete".into()
                            } else if t.paused {
                                "Paused".into()
                            } else {
                                format!("{} left", t.eta())
                            })
                            .size(11.5)
                            .color(if t.error.is_some() {
                                p.ink(RED)
                            } else {
                                p.muted
                            }),
                        );
                    });
                });
                ui.add_space(4.0);
                inset(p)
                    .inner_margin(Margin::symmetric(10, 7))
                    .corner_radius(8)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.columns(3, |cols| {
                            for (col, glyph, tint, value, tip) in [
                                (0, Glyph::Down, BLUE, speed(t.down_speed), "Download speed"),
                                (1, Glyph::Up, GREEN, speed(t.up_speed), "Upload speed"),
                                (
                                    2,
                                    Glyph::Peers,
                                    VIOLET,
                                    t.peers.to_string(),
                                    "Connected peers",
                                ),
                            ] {
                                cols[col]
                                    .horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 5.0;
                                        let (r, _) = ui
                                            .allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                        icon(ui.painter(), r, glyph, p.ink(tint));
                                        ui.add(
                                            Label::new(
                                                RichText::new(value).size(11.5).color(p.text),
                                            )
                                            .truncate(),
                                        );
                                    })
                                    .response
                                    .on_hover_text(tip);
                            }
                        });
                    });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if icon_button(
                        ui,
                        if t.paused || t.error.is_some() {
                            Glyph::Play
                        } else {
                            Glyph::Pause
                        },
                        if t.error.is_some() {
                            "Retry torrent"
                        } else if t.paused {
                            "Resume torrent"
                        } else {
                            "Pause torrent"
                        },
                        p,
                    )
                    .clicked()
                    {
                        self.toggle(t);
                    }
                    if icon_button(ui, Glyph::Folder, "Open download folder", p).clicked() {
                        self.open_path(&t.output);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Glyph::Trash, "Remove torrent", p).clicked() {
                            self.deleting = Some(t.clone());
                            self.delete_files = false;
                        }
                        if ghost(ui, "Details", None, p).clicked() {
                            self.selected = Some(t.hash.clone());
                        }
                    });
                });
            });
    }

    fn toggle(&self, t: &Torrent) {
        self.engine.send(if t.paused || t.error.is_some() {
            Command::Resume(t.hash.clone())
        } else {
            Command::Pause(t.hash.clone())
        });
    }
    fn open_path(&mut self, path: &std::path::Path) {
        if let Err(e) = open::that(path) {
            self.notice(format!("Cannot open {}: {e}", path.display()), true);
        }
    }
    fn schedule_search(&mut self) {
        self.search_id += 1;
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
        self.search_due = Some(Instant::now() + Duration::from_millis(400));
        self.searching = self.search_query.trim().len() >= 2;
        if !self.searching {
            self.response = SearchResponse::default();
        }
    }
    fn search_if_due(&mut self, ctx: &Context) {
        if self.search_due.is_none_or(|due| Instant::now() < due) {
            return;
        }
        self.search_due = None;
        if self.search_query.trim().len() < 2 {
            return;
        }
        let client = self.search_client.clone();
        let query = self.search_query.clone();
        let source = self.source.clone();
        let tx = self.search_tx.clone();
        let ctx = ctx.clone();
        let id = self.search_id;
        self.search_task = Some(self.runtime.spawn(async move {
            let response = client.search(&query, &source).await;
            let _ = tx.send((id, response));
            ctx.request_repaint();
        }));
    }
    fn discover(&mut self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let field = ui.add_sized(
            [ui.available_width(), 48.0],
            TextEdit::singleline(&mut self.search_query)
                .id(Id::new("discovery-search"))
                .hint_text(RichText::new("Search movies, shows, games, and more…").color(p.faint))
                .margin(Margin {
                    left: 44,
                    right: 40,
                    top: 15,
                    bottom: 13,
                })
                .font(FontId::proportional(14.0)),
        );
        icon(
            ui.painter(),
            Rect::from_min_size(field.rect.min + vec2(16.0, 16.0), vec2(16.0, 16.0)),
            Glyph::Search,
            if field.has_focus() { BLUE } else { p.faint },
        );
        if field.changed() {
            self.schedule_search();
        }
        if !self.search_query.is_empty() {
            let r = Rect::from_center_size(
                field.rect.right_center() - vec2(22.0, 0.0),
                vec2(28.0, 28.0),
            );
            let clear = ui.interact(r, Id::new("clear-search"), Sense::click());
            clear.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Clear search"));
            if clear.hovered() {
                ui.painter().rect_filled(r, 7, p.panel2);
            }
            icon(
                ui.painter(),
                r.shrink(8.0),
                Glyph::Close,
                if clear.hovered() { p.text } else { p.muted },
            );
            if clear.clicked() {
                self.search_query.clear();
                self.schedule_search();
                field.request_focus();
            }
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            track(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for source in ["all", "piratebay", "yts", "tv", "fitgirl"] {
                        if segment(
                            ui,
                            source_name(source),
                            self.source == source,
                            None,
                            p,
                            None,
                        ) {
                            self.source = if self.source == source { "all" } else { source }.into();
                            self.schedule_search();
                        }
                    }
                });
            });
            if !self.response.results.is_empty() && !self.searching {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let count = self.response.results.len();
                    ui.label(
                        RichText::new(format!(
                            "{count} {}",
                            if count == 1 { "result" } else { "results" }
                        ))
                        .size(12.0)
                        .color(p.muted),
                    );
                });
            }
        });
        ui.add_space(12.0);
        if self.searching {
            ui.horizontal(|ui| {
                ui.add(Spinner::new().size(14.0).color(BLUE));
                muted(ui, "Searching public indexes…", p);
            });
            ui.add_space(4.0);
            let cols = if ui.available_width() > 700.0 { 3 } else { 2 };
            let pulse = (ui.input(|i| i.time) * 2.4).sin() as f32 * 0.5 + 0.5;
            let shimmer = mix(p.panel2, p.border, pulse * 0.6);
            ui.columns(cols, |columns| {
                for ui in columns {
                    card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let (r, _) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
                        for (y, fraction, h) in [
                            (0.0, 0.28, 18.0),
                            (32.0, 0.92, 10.0),
                            (50.0, 0.64, 10.0),
                            (84.0, 0.4, 10.0),
                            (102.0, 1.0, 1.0),
                        ] {
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    r.min + vec2(0.0, y),
                                    vec2(r.width() * fraction, h),
                                ),
                                5,
                                shimmer,
                            );
                        }
                    });
                }
            });
            return;
        }
        if !self.response.warnings.is_empty() {
            Frame::new()
                .fill(p.tint(YELLOW, 0.06))
                .stroke(Stroke::new(1.0_f32, p.tint(YELLOW, 0.25)))
                .corner_radius(12)
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                        icon(ui.painter(), r, Glyph::Warning, p.ink(YELLOW));
                        ui.label(
                            RichText::new("Some indexes are unavailable")
                                .size(12.5)
                                .color(p.text),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ghost(ui, "Retry", Some(Glyph::Refresh), p).clicked() {
                                self.schedule_search();
                            }
                        });
                    });
                    for warning in &self.response.warnings {
                        ui.horizontal(|ui| {
                            ui.add_space(26.0);
                            ui.add(
                                Label::new(RichText::new(warning).size(11.5).color(p.muted)).wrap(),
                            );
                        });
                    }
                });
            ui.add_space(10.0);
        }
        if self.search_query.trim().len() < 2 || self.response.results.is_empty() {
            let idle = self.search_query.trim().len() < 2;
            card(p).corner_radius(16).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(120.0, 64.0), Sense::hover());
                    ui.painter()
                        .circle_filled(r.center(), 46.0, BLUE.gamma_multiply(0.05));
                    let mut tile = ui.new_child(
                        UiBuilder::new()
                            .max_rect(Rect::from_center_size(r.center(), vec2(48.0, 48.0))),
                    );
                    icon_badge(
                        &mut tile,
                        if idle { Glyph::Compass } else { Glyph::Search },
                        BLUE,
                        p,
                        48.0,
                    );
                    ui.add_space(12.0);
                    ui.label(
                        display(
                            if idle {
                                "Your next find starts here"
                            } else {
                                "No results this time"
                            },
                            18.0,
                        )
                        .color(p.text),
                    );
                    ui.add_space(2.0);
                    muted(
                        ui,
                        if idle {
                            "Type at least two characters, then choose an index or search them all."
                        } else {
                            "Try broader keywords or select a different index."
                        },
                        p,
                    );
                    ui.add_space(28.0);
                });
            });
            return;
        }
        let results = self.response.results.clone();
        let cols = ((ui.available_width() + 14.0) / 290.0)
            .floor()
            .clamp(1.0, 3.0) as usize;
        for chunk in results.chunks(cols) {
            ui.columns(cols, |columns| {
                for (ui, result) in columns.iter_mut().zip(chunk) {
                    let existing = s
                        .torrents
                        .iter()
                        .find(|t| t.hash.eq_ignore_ascii_case(&result.hash));
                    card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.horizontal(|ui| {
                            badge(
                                ui,
                                source_name(&result.source),
                                if result.source == "fitgirl" {
                                    VIOLET
                                } else {
                                    BLUE
                                },
                                p,
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add(
                                    Label::new(
                                        RichText::new(&result.category).size(11.0).color(p.faint),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                        ui.add_space(2.0);
                        let name = clipped_text(ui, &result.name, 13.5, p.text, 2, 38.0);
                        if ui
                            .interact(
                                name.rect,
                                Id::new(("result", &result.source, &result.hash)),
                                Sense::click(),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.result_detail = Some(result.clone());
                        }
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(bytes(result.size as f64))
                                    .size(11.5)
                                    .color(p.muted),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                for (count, color, tip) in [
                                    (result.leechers, RED, "Leechers"),
                                    (result.seeders, GREEN, "Seeders"),
                                ] {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 5.0;
                                        let tone = if count > 0 { color } else { p.faint };
                                        let (r, _) =
                                            ui.allocate_exact_size(vec2(6.0, 12.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 3.0, tone);
                                        ui.label(
                                            RichText::new(count.to_string())
                                                .size(11.5)
                                                .color(if count > 0 { p.text } else { p.faint }),
                                        );
                                    })
                                    .response
                                    .on_hover_text(tip);
                                }
                            });
                        });
                        ui.add_space(2.0);
                        divider(ui, p);
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            if ghost(ui, "Details", None, p).clicked() {
                                self.result_detail = Some(result.clone());
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if let Some(t) = existing {
                                    if ghost(ui, "In library", Some(Glyph::Check), p).clicked() {
                                        self.view = "library".into();
                                        self.filter = "all".into();
                                        self.selected = Some(t.hash.clone());
                                    }
                                } else if action_sized(
                                    ui,
                                    "Add",
                                    Some(Glyph::Plus),
                                    ButtonKind::Primary,
                                    p,
                                    None,
                                    30.0,
                                )
                                .clicked()
                                {
                                    self.engine.send(Command::AddMagnet(result.magnet.clone()));
                                }
                            });
                        });
                    });
                }
            });
            ui.add_space(6.0);
        }
    }
}

impl eframe::App for TorrentApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let p = Palette::new(self.settings.light);
        if self.configured_theme != Some(self.settings.light) {
            self.configure(ctx, p);
            self.configured_theme = Some(self.settings.light);
        }
        while let Ok(result) = self.engine.notices.try_recv() {
            match result {
                Ok(message) => {
                    if self.adding
                        && matches!(message.as_str(), "Torrent added" | "Torrent file added")
                    {
                        self.adding = false;
                        self.add_open = false;
                        self.magnet_input.clear();
                        self.selected_file = None;
                    }
                    self.notice(message, false);
                }
                Err(error) => {
                    if self.adding {
                        self.add_error = Some(error.clone());
                    }
                    self.adding = false;
                    self.notice(error, true);
                }
            }
        }
        while let Ok((id, response)) = self.search_rx.try_recv() {
            if id == self.search_id {
                self.response = response;
                self.searching = false;
            }
        }
        self.search_if_due(ctx);
        let snapshot = self.engine.snapshot();
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) && !self.adding {
            if self.add_open {
                self.add_open = false;
            } else {
                self.open_add();
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) {
            ctx.memory_mut(|memory| {
                memory.request_focus(Id::new(if self.view == "discover" {
                    "discovery-search"
                } else {
                    "library-filter"
                }))
            });
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Comma)) && !self.adding {
            self.settings_open = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) && !self.adding {
            self.open_add();
            self.file_tab = true;
            self.choose_torrent_file();
        }
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            if self.adding {
                continue;
            }
            if path.extension().is_some_and(|e| e == "torrent") {
                self.selected_file = Some(path);
                self.file_tab = true;
                self.open_add();
            } else {
                self.notice("Choose a .torrent file", true);
            }
        }
        self.record_history(&snapshot);
        self.sidebar(ctx, &snapshot, p);
        self.header(ctx, &snapshot, p);
        CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin {
                left: 32,
                right: 32,
                top: 22,
                bottom: 24,
            }))
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("content")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.stats(ui, &snapshot, p);
                        if self.view == "discover" {
                            self.discover(ui, &snapshot, p);
                        } else {
                            self.library(ui, &snapshot, p);
                        }
                    });
            });
        if let Some((text, error, started)) = self.notice.clone()
            && started.elapsed() < Duration::from_secs(if error { 14 } else { 4 })
        {
            Area::new(Id::new("notice"))
                .anchor(Align2::CENTER_BOTTOM, vec2(116.0, -24.0))
                .order(Order::Tooltip)
                .show(ctx, |ui| {
                    card(p)
                        .corner_radius(14)
                        .inner_margin(Margin {
                            left: 10,
                            right: 6,
                            top: 6,
                            bottom: 6,
                        })
                        .shadow(Shadow {
                            offset: [0, 10],
                            blur: 30,
                            spread: 0,
                            color: Color32::from_black_alpha(if p.light { 30 } else { 80 }),
                        })
                        .show(ui, |ui| {
                            ui.set_max_width(540.0);
                            ui.horizontal(|ui| {
                                ui.set_min_height(32.0);
                                icon_badge(
                                    ui,
                                    if error { Glyph::Warning } else { Glyph::Check },
                                    if error { RED } else { GREEN },
                                    p,
                                    26.0,
                                );
                                ui.add(
                                    Label::new(RichText::new(text).size(12.5).color(p.text)).wrap(),
                                );
                                if icon_button(ui, Glyph::Close, "Dismiss notification", p)
                                    .clicked()
                                {
                                    self.notice = None;
                                }
                            });
                        });
                });
        }
        if self.deleting.is_some() {
            self.delete_modal(ctx, p);
        } else if self.add_open {
            self.add_modal(ctx, p);
        } else if self.settings_open {
            self.preferences(ctx, p);
        } else {
            self.drawer(ctx, &snapshot, p);
        }
        ctx.request_repaint_after(Duration::from_millis(if self.searching {
            100
        } else {
            500
        }));
    }
    fn clear_color(&self, visuals: &Visuals) -> [f32; 4] {
        // Match the canvas so no dark seams show between panels in light mode.
        visuals.panel_fill.to_normalized_gamma_f32()
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
        self.engine.shutdown();
    }
}

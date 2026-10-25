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
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::design::*;

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
}

impl TorrentApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        paths: Paths,
        settings: Settings,
        lock: std::fs::File,
    ) -> anyhow::Result<Self> {
        let mut fonts = FontDefinitions::default();
        for (name, path, family) in [
            (
                "SF",
                "/System/Library/Fonts/SFNS.ttf",
                FontFamily::Proportional,
            ),
            (
                "SF Mono",
                "/System/Library/Fonts/SFNSMono.ttf",
                FontFamily::Monospace,
            ),
        ] {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.into(), FontData::from_owned(data).into());
                fonts
                    .families
                    .get_mut(&family)
                    .unwrap()
                    .insert(0, name.into());
            }
        }
        cc.egui_ctx.set_fonts(fonts);
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
        style.visuals.panel_fill = p.panel;
        style.visuals.window_fill = p.panel;
        style.visuals.override_text_color = Some(p.text);
        style.visuals.extreme_bg_color = p.bg;
        style.visuals.faint_bg_color = p.panel2;
        style.visuals.window_stroke = Stroke::new(1.0_f32, p.border);
        style.visuals.window_corner_radius = CornerRadius::same(12);
        style.visuals.selection.bg_fill = p.active;
        style.visuals.selection.stroke = Stroke::new(1.0_f32, BLUE);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
        style.visuals.widgets.inactive.bg_fill = p.panel2;
        style.visuals.widgets.inactive.weak_bg_fill = p.panel2;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
        style.visuals.widgets.hovered.bg_fill = mix(p.panel2, BLUE, 0.07);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, mix(p.border, BLUE, 0.45));
        style.visuals.widgets.active.bg_fill = p.active;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, BLUE);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.noninteractive,
        ] {
            widget.corner_radius = CornerRadius::same(6);
        }
        style.spacing.item_spacing = vec2(10.0, 8.0);
        style.spacing.button_padding = vec2(12.0, 9.0);
        style.spacing.interact_size = vec2(32.0, 20.0);
        style.spacing.window_margin = Margin::same(20);
        style.spacing.scroll = style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 5.0;
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
            .exact_width(240.0)
            .resizable(false)
            .frame(Frame::new().fill(p.panel))
            .show(ctx, |ui| {
                let (brand, _) = ui.allocate_exact_size(vec2(240.0, 64.0), Sense::hover());
                brand_mark(
                    ui.painter(),
                    Rect::from_min_size(brand.min + vec2(20.0, 15.0), vec2(34.0, 34.0)),
                );
                ui.painter().text(
                    brand.min + vec2(65.0, 25.0),
                    Align2::LEFT_CENTER,
                    "Torrent",
                    FontId::proportional(16.0),
                    p.text,
                );
                ui.painter().text(
                    brand.min + vec2(65.0, 43.0),
                    Align2::LEFT_CENTER,
                    "DESKTOP CLIENT",
                    FontId::monospace(8.5),
                    p.muted,
                );
                ui.painter().text(
                    brand.min + vec2(219.0, 31.0),
                    Align2::RIGHT_CENTER,
                    "2.1",
                    FontId::monospace(10.0),
                    p.muted,
                );
                ui.painter().hline(
                    brand.x_range(),
                    brand.bottom(),
                    Stroke::new(1.0_f32, p.border),
                );
                Frame::new()
                    .inner_margin(Margin::symmetric(14, 16))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 5.0;
                        Frame::new()
                            .inner_margin(Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                small(ui, "WORKSPACE", p);
                            });
                        if nav(
                            ui,
                            "Library",
                            Some(snapshot.torrents.len()),
                            self.view == "library",
                            Some(Glyph::Library),
                            p,
                        ) {
                            self.view = "library".into();
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
                        ui.add_space(19.0);
                        Frame::new()
                            .inner_margin(Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                small(ui, "ACTIVITY", p);
                            });
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
                        vec2(240.0, 166.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            Frame::new()
                                .inner_margin(Margin::symmetric(14, 14))
                                .show(ui, |ui| {
                                    ui.set_width(212.0);
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    if nav(ui, "Preferences", None, false, Some(Glyph::Settings), p)
                                    {
                                        self.settings_open = true;
                                    }
                                    ui.add_space(6.0);
                                    divider(ui, p);
                                    ui.add_space(3.0);
                                    value_row(ui, "Download", &speed(snapshot.down()), p);
                                    value_row(ui, "Upload", &speed(snapshot.up()), p);
                                    ui.horizontal(|ui| {
                                        let (r, _) =
                                            ui.allocate_exact_size(vec2(6.0, 6.0), Sense::hover());
                                        ui.painter().circle_filled(
                                            r.center(),
                                            3.0,
                                            if snapshot.ready { GREEN } else { YELLOW },
                                        );
                                        ui.label(
                                            RichText::new(if snapshot.ready {
                                                "Engine ready"
                                            } else {
                                                "Starting engine…"
                                            })
                                            .size(10.5)
                                            .color(p.muted),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.label(mono(
                                                    format!(
                                                        "{} ratio",
                                                        ratio(
                                                            snapshot.uploaded(),
                                                            snapshot.downloaded()
                                                        )
                                                    ),
                                                    10.0,
                                                    p.muted,
                                                ));
                                            },
                                        );
                                    });
                                });
                        },
                    );
                });
                let rect = ui.max_rect();
                ui.painter()
                    .vline(rect.right(), rect.y_range(), Stroke::new(1.0_f32, p.border));
            });
    }

    fn header(&mut self, ctx: &Context, p: Palette) {
        TopBottomPanel::top("header")
            .exact_height(64.0)
            .frame(
                Frame::new()
                    .fill(p.panel)
                    .inner_margin(Margin::symmetric(28, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 220.0).clamp(180.0, 420.0);
                    let field = ui.add_sized(
                        [width, 36.0],
                        TextEdit::singleline(&mut self.query)
                            .id(Id::new("library-filter"))
                            .hint_text("Filter your library…")
                            .font(FontId::proportional(12.5))
                            .margin(Margin {
                                left: 35,
                                right: 34,
                                top: 10,
                                bottom: 8,
                            }),
                    );
                    icon(
                        ui.painter(),
                        Rect::from_min_size(field.rect.min + vec2(12.0, 11.0), vec2(14.0, 14.0)),
                        Glyph::Search,
                        p.muted,
                    );
                    if !self.query.is_empty() {
                        let rect = Rect::from_center_size(
                            field.rect.right_center() - vec2(17.0, 0.0),
                            vec2(26.0, 26.0),
                        );
                        let clear = ui.interact(rect, Id::new("clear-library"), Sense::click());
                        clear.widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Button, true, "Clear library filter")
                        });
                        icon(ui.painter(), rect.shrink(7.0), Glyph::Close, p.muted);
                        if clear.clicked() {
                            self.query.clear();
                            field.request_focus();
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary(ui, "+  Add torrent")
                            .on_hover_text("Add a magnet link or torrent file · ⌘K")
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
                    });
                });
                let r = ui.max_rect();
                ui.painter().hline(
                    r.x_range(),
                    r.bottom() + 14.0,
                    Stroke::new(1.0_f32, p.border),
                );
            });
    }

    fn stats(&self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let columns = if ui.available_width() < 700.0 { 2 } else { 4 };
        let ratio_hint = if s.uploaded() == 0 {
            "No uploads yet"
        } else if s.uploaded() < s.downloaded() {
            "Building your ratio"
        } else {
            "Healthy sharing"
        };
        let cards = [
            (
                "DOWNLOAD",
                speed(s.down()),
                format!("{} received", bytes(s.downloaded() as f64)),
                Glyph::Down,
                BLUE,
            ),
            (
                "UPLOAD",
                speed(s.up()),
                format!("{} shared", bytes(s.uploaded() as f64)),
                Glyph::Up,
                GREEN,
            ),
            (
                "ACTIVE SWARM",
                s.active().to_string(),
                format!(
                    "{} idle · {} total",
                    s.torrents.len().saturating_sub(s.active()),
                    s.torrents.len()
                ),
                Glyph::Peers,
                p.muted,
            ),
            (
                "SHARE RATIO",
                ratio(s.uploaded(), s.downloaded()),
                ratio_hint.into(),
                Glyph::Refresh,
                p.muted,
            ),
        ];
        for chunk in cards.chunks(columns) {
            ui.columns(columns, |columns| {
                for (ui, (label, value, sub, g, color)) in columns.iter_mut().zip(chunk) {
                    card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            small(ui, *label, p);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                                icon(ui.painter(), r, *g, *color);
                            });
                        });
                        ui.add_space(5.0);
                        ui.label(mono(value, 20.0, p.text));
                        ui.label(RichText::new(sub).size(11.0).color(p.muted));
                    });
                }
            });
        }
        ui.add_space(23.0);
    }

    fn library(&mut self, ui: &mut Ui, s: &Snapshot, p: Palette) {
        let rows: Vec<_> = s
            .torrents
            .iter()
            .filter(|t| t.matches(&self.filter, &self.query))
            .collect();
        small(ui, "YOUR LIBRARY", p);
        ui.horizontal(|ui| {
            let title = match self.filter.as_str() {
                "downloading" => "Downloading",
                "seeding" => "Seeding",
                "paused" => "Paused",
                _ => "All torrents",
            };
            ui.label(RichText::new(title).size(21.0).strong());
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
        muted(ui, "Your downloads, organized in one place.", p);
        ui.add_space(14.0);
        if rows.is_empty() {
            let empty = s.torrents.is_empty();
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 322.0), Sense::hover());
            ui.painter().rect(
                r,
                10,
                p.panel,
                Stroke::new(1.0_f32, p.border),
                StrokeKind::Inside,
            );
            let mut content = ui.new_child(
                UiBuilder::new()
                    .max_rect(Rect::from_min_max(
                        r.min + vec2(24.0, 48.0),
                        r.max - vec2(24.0, 24.0),
                    ))
                    .layout(Layout::top_down(Align::Center)),
            );
            if empty {
                let (mark, _) = content.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
                brand_mark(content.painter(), mark);
            } else {
                icon_badge(&mut content, Glyph::Search, BLUE, p, 48.0);
            }
            content.add_space(12.0);
            content.label(
                RichText::new(if !s.ready {
                    "Getting things ready"
                } else if empty {
                    "A little space for your next download"
                } else {
                    "No matching torrents"
                })
                .size(18.0)
                .strong(),
            );
            content.label(
                RichText::new(if empty {
                    "Add a magnet link or a .torrent file to get started."
                } else {
                    "Try another name or clear your activity filters."
                })
                .size(12.5)
                .color(p.muted),
            );
            content.add_space(10.0);
            content.horizontal(|ui| {
                let button_width = |text: &str, icon_width: f32| {
                    ui.painter()
                        .layout_no_wrap(text.into(), FontId::proportional(12.5), p.text)
                        .size()
                        .x
                        + 24.0
                        + icon_width
                };
                let total = if empty {
                    button_width("Add torrent", 22.0)
                        + ui.spacing().item_spacing.x
                        + button_width("Explore torrents", 22.0)
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
            content.add_space(6.0);
            small(
                &mut content,
                "⌘K to add a torrent  ·  ⌘O to choose a file",
                p,
            );
            return;
        }
        let cols = ((ui.available_width() + 14.0) / 270.0)
            .floor()
            .clamp(1.0, 4.0) as usize;
        for chunk in rows.chunks(cols) {
            ui.columns(cols, |columns| {
                for (ui, t) in columns.iter_mut().zip(chunk) {
                    self.torrent_card(ui, t, p);
                }
            });
            ui.add_space(4.0);
        }
    }

    fn torrent_card(&mut self, ui: &mut Ui, t: &Torrent, p: Palette) {
        let selected = self.selected.as_deref() == Some(&t.hash);
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
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    icon(ui.painter(), r.shrink(2.0), Glyph::File, p.muted);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        badge(ui, t.status(), status_color(t), p);
                    });
                });
                ui.add_space(3.0);
                let name = clipped_text(ui, &t.name, 13.0, p.text, 2, 36.0);
                if ui
                    .interact(name.rect, Id::new(("select", &t.hash)), Sense::click())
                    .clicked()
                {
                    self.selected = Some(t.hash.clone());
                }
                ui.horizontal(|ui| {
                    ui.label(mono(format!("{:.0}%", t.progress() * 100.0), 12.0, p.text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if t.done {
                                "Complete".into()
                            } else if t.paused {
                                "Paused".into()
                            } else {
                                format!("{} remaining", t.eta())
                            })
                            .size(10.5)
                            .color(p.muted),
                        );
                    });
                });
                ui.add(
                    ProgressBar::new(t.progress())
                        .desired_height(7.0)
                        .fill(status_color(t))
                        .corner_radius(4),
                );
                ui.add_space(5.0);
                divider(ui, p);
                ui.add_space(3.0);
                ui.columns(3, |cols| {
                    for col in cols.iter_mut() {
                        col.spacing_mut().item_spacing.y = 3.0;
                    }
                    for (col, label, value) in [
                        (0, "DOWN", speed(t.down_speed)),
                        (1, "UP", speed(t.up_speed)),
                        (2, "PEERS", t.peers.to_string()),
                        (0, "ETA", t.eta()),
                        (1, "SIZE", bytes(t.size as f64)),
                        (2, "FILES", t.files.len().to_string()),
                    ] {
                        cols[col].label(mono(label, 9.0, p.muted));
                        cols[col].add(Label::new(mono(value, 11.0, p.text)).truncate());
                    }
                });
                ui.add_space(3.0);
                divider(ui, p);
                ui.horizontal(|ui| {
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
        small(ui, "EXPLORE", p);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Discover torrents").size(21.0).strong());
            if !self.response.results.is_empty() && !self.searching {
                badge(ui, &self.response.results.len().to_string(), p.muted, p);
            }
        });
        muted(
            ui,
            "Find something new across your favorite public indexes.",
            p,
        );
        ui.add_space(13.0);
        let field = ui.add_sized(
            [ui.available_width(), 46.0],
            TextEdit::singleline(&mut self.search_query)
                .id(Id::new("discovery-search"))
                .hint_text("Search movies, shows, games, and more…")
                .margin(Margin {
                    left: 40,
                    right: 38,
                    top: 14,
                    bottom: 12,
                })
                .font(FontId::proportional(13.0)),
        );
        icon(
            ui.painter(),
            Rect::from_min_size(field.rect.min + vec2(14.0, 15.0), vec2(16.0, 16.0)),
            Glyph::Search,
            p.muted,
        );
        if field.changed() {
            self.schedule_search();
        }
        if !self.search_query.is_empty() {
            let r = Rect::from_center_size(
                field.rect.right_center() - vec2(20.0, 0.0),
                vec2(28.0, 28.0),
            );
            let clear = ui.interact(r, Id::new("clear-search"), Sense::click());
            clear.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Clear search"));
            icon(ui.painter(), r.shrink(8.0), Glyph::Close, p.muted);
            if clear.clicked() {
                self.search_query.clear();
                self.schedule_search();
                field.request_focus();
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
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
        ui.add_space(10.0);
        if self.searching {
            ui.horizontal(|ui| {
                ui.spinner();
                muted(ui, "Searching public indexes…", p);
            });
            let cols = if ui.available_width() > 700.0 { 3 } else { 2 };
            ui.columns(cols, |columns| {
                for ui in columns {
                    card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let (r, _) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 116.0), Sense::hover());
                        for (y, fraction) in [(0.0, 0.32), (30.0, 0.9), (48.0, 0.65), (90.0, 0.45)]
                        {
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    r.min + vec2(0.0, y),
                                    vec2(r.width() * fraction, 8.0),
                                ),
                                3,
                                p.panel2,
                            );
                        }
                    });
                }
            });
            return;
        }
        if !self.response.warnings.is_empty() {
            card(p)
                .fill(mix(p.panel, YELLOW, 0.035))
                .stroke(Stroke::new(1.0_f32, mix(p.border, YELLOW, 0.18)))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                        icon(ui.painter(), r, Glyph::Warning, YELLOW);
                        ui.label(RichText::new("Some indexes are unavailable").size(12.5));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ghost(ui, "Retry", Some(Glyph::Refresh), p).clicked() {
                                self.schedule_search();
                            }
                        });
                    });
                    for warning in &self.response.warnings {
                        muted(ui, warning, p);
                    }
                });
            ui.add_space(6.0);
        }
        if self.search_query.trim().len() < 2 || self.response.results.is_empty() {
            card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    icon_badge(ui, Glyph::Compass, BLUE, p, 44.0);
                    ui.add_space(7.0);
                    ui.label(
                        RichText::new(if self.search_query.trim().len() < 2 {
                            "Your next find starts here"
                        } else {
                            "No results this time"
                        })
                        .size(16.0)
                        .strong(),
                    );
                    muted(
                        ui,
                        if self.search_query.trim().len() < 2 {
                            "Type at least two characters, then choose an index or search them all."
                        } else {
                            "Try broader keywords or select a different index."
                        },
                        p,
                    );
                    ui.add_space(20.0);
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
                        ui.horizontal(|ui| {
                            badge(
                                ui,
                                source_name(&result.source),
                                if result.source == "fitgirl" {
                                    BLUE
                                } else {
                                    p.muted
                                },
                                p,
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add(
                                    Label::new(
                                        RichText::new(&result.category).size(10.0).color(p.muted),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                        ui.add_space(2.0);
                        let name = clipped_text(ui, &result.name, 13.0, p.text, 2, 36.0);
                        if ui
                            .interact(
                                name.rect,
                                Id::new(("result", &result.source, &result.hash)),
                                Sense::click(),
                            )
                            .clicked()
                        {
                            self.result_detail = Some(result.clone());
                        }
                        divider(ui, p);
                        ui.horizontal(|ui| {
                            ui.label(mono(bytes(result.size as f64), 11.0, p.muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(mono(
                                    format!("↓ {}", result.leechers),
                                    11.0,
                                    if result.leechers > 0 { RED } else { p.muted },
                                ))
                                .on_hover_text("Leechers");
                                ui.label(mono(
                                    format!("↑ {}", result.seeders),
                                    11.0,
                                    if result.seeders > 0 { GREEN } else { p.muted },
                                ))
                                .on_hover_text("Seeders");
                            });
                        });
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
                                } else if primary(ui, "+  Add").clicked() {
                                    self.engine.send(Command::AddMagnet(result.magnet.clone()));
                                }
                            });
                        });
                    });
                }
            });
            ui.add_space(4.0);
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
        self.sidebar(ctx, &snapshot, p);
        self.header(ctx, p);
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(28, 26)),
            )
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
                .anchor(Align2::CENTER_BOTTOM, vec2(100.0, -20.0))
                .order(Order::Tooltip)
                .show(ctx, |ui| {
                    card(p)
                        .inner_margin(Margin::symmetric(14, 8))
                        .shadow(Shadow {
                            offset: [0, 4],
                            blur: 20,
                            spread: 0,
                            color: Color32::from_black_alpha(50),
                        })
                        .show(ui, |ui| {
                            ui.set_max_width(540.0);
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                icon(
                                    ui.painter(),
                                    r,
                                    if error { Glyph::Warning } else { Glyph::Check },
                                    if error { RED } else { GREEN },
                                );
                                ui.add(
                                    Label::new(RichText::new(text).size(12.0).color(p.text)).wrap(),
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
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
        self.engine.shutdown();
    }
}

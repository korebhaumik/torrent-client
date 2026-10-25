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

    fn add_modal(&mut self, ctx: &Context, p: Palette) {
        let mut submit = false;
        let modal = Modal::new(Id::new("add"))
            .backdrop_color(Color32::from_black_alpha(155))
            .frame(modal_frame(p))
            .show(ctx, |ui| {
                ui.set_width(480.0_f32.min(ctx.content_rect().width() - 80.0));
                if dialog_header(
                    ui,
                    "Add a torrent",
                    "Start with a magnet link or a torrent file.",
                    Glyph::Magnet,
                    BLUE,
                    p,
                    !self.adding,
                ) {
                    self.add_open = false;
                }
                ui.add_space(12.0);
                Frame::new()
                    .fill(p.bg)
                    .corner_radius(7)
                    .inner_margin(4)
                    .show(ui, |ui| {
                        let width = (ui.available_width() - 4.0) / 2.0;
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add_enabled_ui(!self.adding, |ui| {
                            ui.horizontal(|ui| {
                                if segment(
                                    ui,
                                    "Magnet link",
                                    !self.file_tab,
                                    Some(Glyph::Magnet),
                                    p,
                                    Some(width),
                                ) {
                                    self.file_tab = false;
                                    self.add_error = None;
                                    self.add_focus = true;
                                }
                                if segment(
                                    ui,
                                    "Torrent file",
                                    self.file_tab,
                                    Some(Glyph::File),
                                    p,
                                    Some(width),
                                ) {
                                    self.file_tab = true;
                                    self.add_error = None;
                                }
                            });
                        });
                    });
                ui.add_space(12.0);
                ui.add_enabled_ui(!self.adding, |ui| {
                    if self.file_tab {
                        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
                        let (zone, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::click());
                        response.widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Button, true, "Choose a torrent file")
                        });
                        ui.painter().rect(
                            zone,
                            8,
                            if hovering {
                                mix(p.bg, BLUE, 0.08)
                            } else {
                                p.bg
                            },
                            Stroke::new(
                                1.0_f32,
                                if hovering || response.hovered() {
                                    BLUE
                                } else {
                                    p.border
                                },
                            ),
                            StrokeKind::Inside,
                        );
                        let badge = Rect::from_center_size(
                            zone.center() - vec2(0.0, 35.0),
                            vec2(34.0, 34.0),
                        );
                        ui.painter().rect_filled(badge, 7, p.panel2);
                        icon(
                            ui.painter(),
                            badge.shrink(8.0),
                            if self.selected_file.is_some() {
                                Glyph::File
                            } else {
                                Glyph::Up
                            },
                            BLUE,
                        );
                        let name = self
                            .selected_file
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Drop your .torrent file here".into());
                        let name = ui.painter().layout(
                            name,
                            FontId::proportional(13.0),
                            p.text,
                            zone.width() - 32.0,
                        );
                        ui.painter().galley(
                            pos2(zone.center().x - name.size().x / 2.0, zone.center().y - 6.0),
                            name,
                            p.text,
                        );
                        let hint = self
                            .selected_file
                            .as_ref()
                            .and_then(|path| std::fs::metadata(path).ok())
                            .map(|m| format!("{} · Ready to add", bytes(m.len() as f64)))
                            .unwrap_or_else(|| "or click to browse · Up to 10 MB".into());
                        ui.painter().text(
                            zone.center() + vec2(0.0, 30.0),
                            Align2::CENTER_CENTER,
                            hint,
                            FontId::proportional(11.0),
                            p.muted,
                        );
                        if response.clicked() {
                            self.choose_torrent_file();
                        }
                        if self.selected_file.is_some() {
                            ui.horizontal(|ui| {
                                if ghost(ui, "Choose another file", Some(Glyph::Folder), p)
                                    .clicked()
                                {
                                    self.choose_torrent_file();
                                }
                                if ghost(ui, "Clear", Some(Glyph::Close), p).clicked() {
                                    self.selected_file = None;
                                    self.add_error = None;
                                }
                            });
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Magnet link or infohash").size(12.0));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ghost(ui, "Paste", Some(Glyph::Clipboard), p).clicked() {
                                    match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                                        Ok(text) => {
                                            self.magnet_input = text.trim().into();
                                            self.add_error = None;
                                        }
                                        Err(_) => self.add_error = Some(
                                            "Clipboard unavailable. Paste into the field with ⌘V."
                                                .into(),
                                        ),
                                    }
                                }
                            });
                        });
                        let field = ui.add_sized(
                            [ui.available_width(), 104.0],
                            TextEdit::multiline(&mut self.magnet_input)
                                .id(Id::new("magnet-input"))
                                .font(FontId::monospace(12.0))
                                .hint_text("magnet:?xt=urn:btih:…")
                                .margin(vec2(12.0, 12.0)),
                        );
                        if self.add_focus {
                            field.request_focus();
                            self.add_focus = false;
                        }
                        if field.changed() {
                            self.add_error = None;
                        }
                        muted(
                            ui,
                            "Paste a magnet URI or a 40-character BitTorrent infohash.",
                            p,
                        );
                    }
                });
                if let Some(error) = &self.add_error {
                    ui.add_space(4.0);
                    Frame::new()
                        .fill(mix(p.panel, RED, 0.07))
                        .stroke(Stroke::new(1.0_f32, mix(p.border, RED, 0.25)))
                        .corner_radius(7)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                                icon(ui.painter(), r, Glyph::Warning, RED);
                                ui.add(
                                    Label::new(RichText::new(error).size(12.0).color(RED)).wrap(),
                                );
                            });
                        });
                }
                ui.add_space(16.0);
                divider(ui, p);
                ui.add_space(6.0);
                let valid = if self.file_tab {
                    self.selected_file.is_some()
                } else {
                    !self.magnet_input.trim().is_empty()
                };
                ui.horizontal(|ui| {
                    if self.adding {
                        ui.spinner();
                        muted(ui, "Adding torrent…", p);
                    } else {
                        small(ui, "⌘↵ to add", p);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_enabled_ui(valid && !self.adding, |ui| {
                            if primary(
                                ui,
                                if self.adding {
                                    "Adding…"
                                } else {
                                    "+  Add torrent"
                                },
                            )
                            .clicked()
                            {
                                submit = true;
                            }
                        });
                        ui.add_enabled_ui(!self.adding, |ui| {
                            if secondary(ui, "Cancel", None, p).clicked() {
                                self.add_open = false;
                            }
                        });
                    });
                });
                if valid
                    && !self.adding
                    && ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                {
                    submit = true;
                }
            });
        if submit {
            if self.file_tab {
                if let Some(path) = &self.selected_file {
                    self.engine.send(Command::AddFile(path.clone()));
                    self.adding = true;
                    self.add_error = None;
                }
            } else {
                match crate::engine::parse_magnet(&self.magnet_input) {
                    Ok(_) => {
                        self.engine
                            .send(Command::AddMagnet(self.magnet_input.trim().into()));
                        self.adding = true;
                        self.add_error = None;
                    }
                    Err(_) => {
                        self.add_error = Some(
                            "Enter a valid magnet link or a 40-character hexadecimal infohash."
                                .into(),
                        )
                    }
                }
            }
        }
        if modal.should_close() && !self.adding {
            self.add_open = false;
            self.add_error = None;
        }
    }
    fn choose_torrent_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Torrent", &["torrent"])
            .pick_file()
        {
            self.selected_file = Some(path);
            self.add_error = None;
        }
    }

    fn delete_modal(&mut self, ctx: &Context, p: Palette) {
        let Some(t) = self.deleting.clone() else {
            return;
        };
        let modal = Modal::new(Id::new("delete"))
            .backdrop_color(Color32::from_black_alpha(155))
            .frame(modal_frame(p))
            .show(ctx, |ui| {
                ui.set_width(440.0);
                if dialog_header(
                    ui,
                    "Remove this torrent?",
                    "Choose what happens to its downloaded files.",
                    Glyph::Trash,
                    RED,
                    p,
                    true,
                ) {
                    self.deleting = None;
                }
                ui.add_space(12.0);
                card(p).fill(p.bg).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    clipped_text(ui, &t.name, 13.0, p.text, 2, 34.0);
                    ui.horizontal(|ui| {
                        small(ui, bytes(t.size as f64), p);
                        small(
                            ui,
                            format!(
                                "{} {}",
                                t.files.len(),
                                if t.files.len() == 1 { "file" } else { "files" }
                            ),
                            p,
                        );
                    });
                });
                ui.add_space(8.0);
                Frame::new()
                    .fill(if self.delete_files {
                        mix(p.panel, RED, 0.05)
                    } else {
                        p.panel2
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if self.delete_files {
                            mix(p.border, RED, 0.35)
                        } else {
                            p.border
                        },
                    ))
                    .corner_radius(8)
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.checkbox(
                            &mut self.delete_files,
                            RichText::new("Also delete downloaded files").size(12.5),
                        );
                        ui.add_space(2.0);
                        muted(
                            ui,
                            if self.delete_files {
                                "Files will be permanently deleted from disk."
                            } else {
                                "Your downloaded files will stay on this Mac."
                            },
                            p,
                        );
                    });
                ui.add_space(16.0);
                divider(ui, p);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if action(
                            ui,
                            if self.delete_files {
                                "Remove & delete files"
                            } else {
                                "Remove torrent"
                            },
                            Some(Glyph::Trash),
                            ButtonKind::Danger,
                            p,
                        )
                        .clicked()
                        {
                            self.engine
                                .send(Command::Remove(t.hash.clone(), self.delete_files));
                            self.deleting = None;
                            self.selected = None;
                        }
                        if secondary(ui, "Cancel", None, p).clicked() {
                            self.deleting = None;
                        }
                    });
                });
            });
        if modal.should_close() {
            self.deleting = None;
        }
    }

    fn drawer(&mut self, ctx: &Context, s: &Snapshot, p: Palette) {
        let torrent = self
            .selected
            .as_ref()
            .and_then(|hash| s.torrents.iter().find(|t| &t.hash == hash))
            .cloned();
        let result = self.result_detail.clone();
        if torrent.is_none() && result.is_none() {
            return;
        }
        let width = 490.0_f32.min(ctx.content_rect().width() - 70.0);
        let modal = Modal::new(Id::new("details"))
            .backdrop_color(Color32::from_black_alpha(140))
            .area(Modal::default_area(Id::new("details")).anchor(Align2::RIGHT_TOP, Vec2::ZERO))
            .frame(
                Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(width - 48.0);
                ui.set_height(ctx.content_rect().height() - 48.0);
                ui.horizontal(|ui| {
                    small(
                        ui,
                        if torrent.is_some() {
                            "TORRENT DETAILS"
                        } else {
                            "DISCOVERY DETAILS"
                        },
                        p,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Glyph::Close, "Close details", p).clicked() {
                            self.selected = None;
                            self.result_detail = None;
                        }
                    });
                });
                let title = torrent
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .or_else(|| result.as_ref().map(|r| r.name.as_str()))
                    .unwrap_or("");
                let mut title_job = text::LayoutJob::simple(
                    title.into(),
                    FontId::proportional(18.0),
                    p.text,
                    ui.available_width(),
                );
                title_job.wrap.max_rows = 3;
                title_job.wrap.break_anywhere = true;
                let title_galley = ui.fonts_mut(|fonts| fonts.layout_job(title_job));
                let (title_rect, title_response) = ui.allocate_exact_size(
                    vec2(ui.available_width(), title_galley.size().y),
                    Sense::hover(),
                );
                ui.painter().galley(title_rect.min, title_galley, p.text);
                title_response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, title));
                if let Some(t) = &torrent {
                    badge(ui, t.status(), status_color(t), p);
                } else if let Some(r) = &result {
                    badge(ui, source_name(&r.source), BLUE, p);
                }
                ui.add_space(12.0);
                divider(ui, p);
                let body_height =
                    (ctx.content_rect().bottom() - ui.cursor().top() - 100.0).max(120.0);
                ScrollArea::vertical()
                    .id_salt("drawer-scroll")
                    .max_height(body_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(10.0);
                        if let Some(t) = &torrent {
                            card(p).fill(p.bg).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                value_row(
                                    ui,
                                    &format!("{:.0}% completed", t.progress() * 100.0),
                                    &format!("ETA {}", t.eta()),
                                    p,
                                );
                                ui.add(
                                    ProgressBar::new(t.progress())
                                        .desired_height(7.0)
                                        .fill(status_color(t))
                                        .corner_radius(4),
                                );
                                ui.add_space(3.0);
                                value_row(ui, "Download", &speed(t.down_speed), p);
                                value_row(ui, "Upload", &speed(t.up_speed), p);
                            });
                            ui.add_space(10.0);
                            small(ui, "OVERVIEW", p);
                            card(p).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                value_row(ui, "Total size", &bytes(t.size as f64), p);
                                value_row(ui, "Downloaded", &bytes(t.downloaded as f64), p);
                                value_row(ui, "Uploaded", &bytes(t.uploaded as f64), p);
                                value_row(ui, "Share ratio", &ratio(t.uploaded, t.downloaded), p);
                                value_row(ui, "Connected peers", &t.peers.to_string(), p);
                                let added = chrono::DateTime::from_timestamp_millis(t.added_at)
                                    .map(|d| {
                                        d.with_timezone(&chrono::Local)
                                            .format("%d %b %Y, %H:%M")
                                            .to_string()
                                    })
                                    .unwrap_or_default();
                                value_row(ui, "Added", &added, p);
                            });
                            if let Some(error) = &t.error {
                                ui.add_space(5.0);
                                ui.colored_label(RED, error);
                            }
                            ui.add_space(10.0);
                            self.identifiers(ui, &t.hash, &t.magnet, p);
                            ui.add_space(10.0);
                            small(ui, format!("FILES · {}", t.files.len()), p);
                            if t.files.is_empty() {
                                muted(
                                    ui,
                                    "Files will appear when torrent metadata is available.",
                                    p,
                                );
                            }
                            for file in &t.files {
                                card(p).fill(p.bg).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    clipped_text(ui, &file.name, 12.5, p.text, 2, 32.0);
                                    value_row(
                                        ui,
                                        &bytes(file.size as f64),
                                        &format!("{} downloaded", bytes(file.completed as f64)),
                                        p,
                                    );
                                    let progress = if file.size == 0 {
                                        0.0
                                    } else {
                                        file.completed as f32 / file.size as f32
                                    };
                                    ui.add(
                                        ProgressBar::new(progress)
                                            .desired_height(4.0)
                                            .fill(BLUE)
                                            .corner_radius(2),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.add_enabled_ui(file.path.exists(), |ui| {
                                            if ghost(ui, "Open file", Some(Glyph::External), p)
                                                .clicked()
                                            {
                                                self.open_path(&file.path);
                                            }
                                        });
                                        if ghost(ui, "Show in Finder", Some(Glyph::Folder), p)
                                            .clicked()
                                        {
                                            let _ = std::process::Command::new("/usr/bin/open")
                                                .arg("-R")
                                                .arg(&file.path)
                                                .spawn();
                                        }
                                    });
                                });
                            }
                        } else if let Some(r) = &result {
                            card(p).fill(p.bg).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                value_row(ui, "Category", &r.category, p);
                                value_row(ui, "Size", &bytes(r.size as f64), p);
                                value_row(ui, "Seeders", &r.seeders.to_string(), p);
                                value_row(ui, "Leechers", &r.leechers.to_string(), p);
                            });
                            if !r.description.is_empty() {
                                ui.add_space(10.0);
                                small(ui, "ABOUT THIS TORRENT", p);
                                ui.add(
                                    Label::new(
                                        RichText::new(&r.description).size(12.5).color(p.muted),
                                    )
                                    .wrap(),
                                );
                            }
                            ui.add_space(10.0);
                            self.identifiers(ui, &r.hash, &r.magnet, p);
                            if !r.details_url.is_empty()
                                && ghost(ui, "View source page", Some(Glyph::External), p).clicked()
                                && (r.details_url.starts_with("https://")
                                    || r.details_url.starts_with("http://"))
                            {
                                let _ = open::that(&r.details_url);
                            }
                        }
                        ui.add_space(10.0);
                    });
                divider(ui, p);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if let Some(t) = &torrent {
                        if action(
                            ui,
                            if t.error.is_some() {
                                "Retry"
                            } else if t.paused {
                                "Resume torrent"
                            } else {
                                "Pause torrent"
                            },
                            Some(if t.paused || t.error.is_some() {
                                Glyph::Play
                            } else {
                                Glyph::Pause
                            }),
                            ButtonKind::Primary,
                            p,
                        )
                        .clicked()
                        {
                            self.toggle(t);
                        }
                        if secondary(ui, "Open folder", Some(Glyph::Folder), p).clicked() {
                            self.open_path(&t.output);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_button(ui, Glyph::Trash, "Remove torrent", p).clicked() {
                                self.deleting = Some(t.clone());
                                self.delete_files = false;
                            }
                        });
                    } else if let Some(r) = &result {
                        let existing = s
                            .torrents
                            .iter()
                            .any(|t| t.hash.eq_ignore_ascii_case(&r.hash));
                        if action_sized(
                            ui,
                            if existing {
                                "View in library"
                            } else {
                                "Add torrent"
                            },
                            Some(if existing {
                                Glyph::Library
                            } else {
                                Glyph::Plus
                            }),
                            ButtonKind::Primary,
                            p,
                            Some(ui.available_width()),
                            38.0,
                        )
                        .clicked()
                        {
                            if existing {
                                self.selected = Some(r.hash.clone());
                                self.result_detail = None;
                                self.view = "library".into();
                                self.filter = "all".into();
                            } else {
                                self.engine.send(Command::AddMagnet(r.magnet.clone()));
                            }
                        }
                    }
                });
            });
        if modal.should_close() {
            self.selected = None;
            self.result_detail = None;
        }
    }

    fn identifiers(&mut self, ui: &mut Ui, hash: &str, magnet: &str, p: Palette) {
        small(ui, "IDENTIFIERS", p);
        for (label, text) in [("Infohash", hash), ("Magnet link", magnet)] {
            card(p).fill(p.bg).inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                small(ui, label, p);
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 42.0).max(50.0);
                    ui.add_sized(
                        [width, 24.0],
                        Label::new(mono(text, 11.0, p.text))
                            .halign(Align::Min)
                            .truncate(),
                    )
                    .on_hover_text(text);
                    if icon_button(
                        ui,
                        Glyph::Copy,
                        &format!("Copy {}", label.to_lowercase()),
                        p,
                    )
                    .clicked()
                    {
                        ui.ctx().copy_text(text.into());
                        self.notice(format!("{label} copied to clipboard"), false);
                    }
                });
            });
        }
    }

    fn preferences(&mut self, ctx: &Context, p: Palette) {
        let modal = Modal::new(Id::new("preferences"))
            .backdrop_color(Color32::from_black_alpha(155))
            .frame(modal_frame(p))
            .show(ctx, |ui| {
                ui.set_width(500.0);
                if dialog_header(
                    ui,
                    "Preferences",
                    "Make Torrent feel at home on your Mac.",
                    Glyph::Settings,
                    BLUE,
                    p,
                    true,
                ) {
                    self.settings_open = false;
                }
                ScrollArea::vertical()
                    .id_salt("preferences-scroll")
                    .max_height((ctx.content_rect().height() - 216.0).max(240.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.add_space(16.0);
                        small(ui, "APPEARANCE", p);
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 10.0) / 2.0;
                            if segment(
                                ui,
                                "Dark",
                                !self.settings.light,
                                Some(Glyph::Moon),
                                p,
                                Some(width),
                            ) {
                                self.settings.light = false;
                                self.persist_settings();
                            }
                            if segment(
                                ui,
                                "Light",
                                self.settings.light,
                                Some(Glyph::Sun),
                                p,
                                Some(width),
                            ) {
                                self.settings.light = true;
                                self.persist_settings();
                            }
                        });
                        ui.add_space(12.0);
                        divider(ui, p);
                        ui.add_space(10.0);
                        ui.label(RichText::new("Download location").size(13.0).strong());
                        muted(ui, "Choose where new torrents save their files.", p);
                        card(p).fill(p.bg).inner_margin(12).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let width = (ui.available_width() - 112.0).max(120.0);
                                ui.add_sized(
                                    [width, 34.0],
                                    Label::new(mono(
                                        self.settings.download_dir.display().to_string(),
                                        11.0,
                                        p.text,
                                    ))
                                    .truncate(),
                                )
                                .on_hover_text(self.settings.download_dir.display().to_string());
                                if secondary(ui, "Choose…", Some(Glyph::Folder), p).clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .set_directory(&self.settings.download_dir)
                                        .pick_folder()
                                {
                                    self.engine.send(Command::SetFolder(path.clone()));
                                    self.settings.download_dir = path;
                                    self.persist_settings();
                                }
                            });
                        });
                        muted(
                            ui,
                            "Existing torrents keep their current download location.",
                            p,
                        );
                        ui.add_space(12.0);
                        divider(ui, p);
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Bring your library along")
                                .size(13.0)
                                .strong(),
                        );
                        muted(
                            ui,
                            "Import saved magnets from your previous WebTorrent session.",
                            p,
                        );
                        if secondary(ui, "Import session…", Some(Glyph::Library), p).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .set_title("Choose the old downloads/.state.json")
                                .add_filter("Session JSON", &["json"])
                                .pick_file()
                        {
                            self.engine.send(Command::Import(path));
                        }
                        ui.add_space(8.0);
                        CollapsingHeader::new(
                            RichText::new("Storage & shortcuts")
                                .size(11.5)
                                .color(p.muted),
                        )
                        .show(ui, |ui| {
                            small(ui, self.paths.data.display().to_string(), p);
                            muted(ui, "⌘K  Add torrent     ⌘O  Open file     ⌘F  Search", p);
                        });
                    });
                ui.add_space(16.0);
                divider(ui, p);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    small(ui, "TORRENT 2.1", p);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary(ui, "Done").clicked() {
                            self.settings_open = false;
                        }
                    });
                });
            });
        if modal.should_close() {
            self.settings_open = false;
        }
    }
}

impl eframe::App for TorrentApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let p = Palette::new(self.settings.light);
        self.configure(ctx, p);
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
        let snapshot = self
            .engine
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
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

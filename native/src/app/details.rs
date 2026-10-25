use super::TorrentApp;
use crate::search::source_name;
use crate::{design::*, engine::Command, model::*};
use eframe::egui::*;

impl TorrentApp {
    pub(super) fn drawer(&mut self, ctx: &Context, s: &Snapshot, p: Palette) {
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
                                        if ghost(
                                            ui,
                                            crate::platform::REVEAL_LABEL,
                                            Some(Glyph::Folder),
                                            p,
                                        )
                                        .clicked()
                                            && let Err(e) = crate::platform::reveal_file(&file.path)
                                        {
                                            self.notice(
                                                format!(
                                                    "Cannot reveal {}: {e}",
                                                    file.path.display()
                                                ),
                                                true,
                                            );
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
                    ui.allocate_ui_with_layout(
                        vec2(width, 32.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.set_min_size(vec2(width, 32.0));
                            ui.add(
                                Label::new(mono(text, 11.0, p.text))
                                    .halign(Align::Min)
                                    .truncate(),
                            )
                            .on_hover_text(text);
                        },
                    );
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
}

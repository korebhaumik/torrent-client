use super::TorrentApp;
use crate::{design::*, engine::Command, model::*};
use eframe::egui::*;

impl TorrentApp {
    pub(super) fn add_modal(&mut self, ctx: &Context, p: Palette) {
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
    pub(super) fn choose_torrent_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Torrent", &["torrent"])
            .pick_file()
        {
            self.selected_file = Some(path);
            self.add_error = None;
        }
    }

    pub(super) fn delete_modal(&mut self, ctx: &Context, p: Palette) {
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

    pub(super) fn preferences(&mut self, ctx: &Context, p: Palette) {
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

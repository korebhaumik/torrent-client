use eframe::egui::*;
use std::path::PathBuf;

pub(crate) fn command_shortcut(ctx: &Context, key: Key) -> String {
    if ctx.os().is_mac() && key == Key::Enter {
        format!("{}↵", ctx.format_modifiers(Modifiers::COMMAND))
    } else {
        ctx.format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key))
    }
}

/// Primary accent, used for actions, focus, and downloading.
pub(crate) const BLUE: Color32 = Color32::from_rgb(92, 104, 255);
/// Secondary accent for peers and the swarm.
pub(crate) const VIOLET: Color32 = Color32::from_rgb(124, 92, 255);
pub(crate) const GREEN: Color32 = Color32::from_rgb(38, 201, 148);
pub(crate) const YELLOW: Color32 = Color32::from_rgb(245, 180, 64);
pub(crate) const RED: Color32 = Color32::from_rgb(240, 98, 108);
const IDLE: Color32 = Color32::from_rgb(140, 144, 162);
pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Font family for titles and headline numbers. Always registered by
/// [`fonts`]; it falls back to the proportional fonts when no system face exists.
pub(crate) const DISPLAY: &str = "Display";

/// System fonts with bundled egui fonts kept as fallbacks for missing glyphs.
pub(crate) fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let windows_fonts = std::env::var_os("WINDIR")
        .map(|dir| PathBuf::from(dir).join("Fonts"))
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
    let candidates = [
        (
            "SF",
            PathBuf::from("/System/Library/Fonts/SFNS.ttf"),
            0,
            Some(FontFamily::Proportional),
        ),
        (
            "SF Mono",
            PathBuf::from("/System/Library/Fonts/SFNSMono.ttf"),
            0,
            Some(FontFamily::Monospace),
        ),
        // Face 2 of the collection is Avenir Next Demi Bold.
        (
            "Avenir Next Demi Bold",
            PathBuf::from("/System/Library/Fonts/Avenir Next.ttc"),
            2,
            None,
        ),
        (
            "Segoe UI",
            windows_fonts.join("segoeui.ttf"),
            0,
            Some(FontFamily::Proportional),
        ),
        (
            "Consolas",
            windows_fonts.join("consola.ttf"),
            0,
            Some(FontFamily::Monospace),
        ),
        (
            "Segoe UI Semibold",
            windows_fonts.join("seguisb.ttf"),
            0,
            None,
        ),
    ];
    let mut display = Vec::new();
    for (name, path, index, family) in candidates {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let mut font = FontData::from_owned(data);
        font.index = index;
        fonts.font_data.insert(name.into(), font.into());
        match family {
            Some(family) => fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.into()),
            None => display.push(name.to_owned()),
        }
    }
    display.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(FontFamily::Name(DISPLAY.into()), display);
    fonts
}
pub(crate) fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}
pub(crate) fn display(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(display_font(size))
}

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub light: bool,
    pub bg: Color32,
    pub sidebar: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub field: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub active: Color32,
}
impl Palette {
    pub fn new(light: bool) -> Self {
        if light {
            Self {
                light,
                bg: rgb(246, 246, 249),
                sidebar: rgb(239, 239, 244),
                panel: Color32::WHITE,
                panel2: rgb(242, 242, 247),
                field: Color32::WHITE,
                border: rgb(225, 226, 234),
                text: rgb(22, 23, 33),
                muted: rgb(100, 104, 122),
                faint: rgb(146, 149, 166),
                active: rgb(232, 233, 255),
            }
        } else {
            Self {
                light,
                bg: rgb(13, 13, 17),
                sidebar: rgb(17, 17, 22),
                panel: rgb(22, 22, 28),
                panel2: rgb(30, 31, 39),
                field: rgb(20, 20, 26),
                border: rgb(39, 40, 50),
                text: rgb(237, 237, 243),
                muted: rgb(142, 145, 162),
                faint: rgb(96, 99, 116),
                active: rgb(33, 35, 64),
            }
        }
    }
    /// A readable text tone of a status color on this theme's panels.
    pub fn ink(&self, color: Color32) -> Color32 {
        if self.light {
            mix(color, rgb(0, 0, 0), 0.28)
        } else {
            color
        }
    }
    pub fn tint(&self, color: Color32, amount: f32) -> Color32 {
        mix(
            self.panel,
            color,
            if self.light { amount } else { amount * 1.3 },
        )
    }
}
pub(crate) fn mono(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::monospace(size))
        .color(color)
}
/// Secondary metadata text.
pub(crate) fn small(ui: &mut Ui, text: impl Into<String>, p: Palette) {
    ui.label(RichText::new(text).size(11.0).color(p.muted));
}
/// Spaced uppercase section label.
pub(crate) fn eyebrow(ui: &mut Ui, text: &str, p: Palette) -> Response {
    let mut job = text::LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: FontId::proportional(10.5),
            color: p.faint,
            extra_letter_spacing: 0.9,
            ..Default::default()
        },
    );
    ui.label(job)
}
pub(crate) fn muted(ui: &mut Ui, text: impl Into<String>, p: Palette) {
    ui.label(RichText::new(text).size(12.0).color(p.muted));
}
pub(crate) fn card_shadow(p: Palette) -> Shadow {
    if p.light {
        Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: Color32::from_black_alpha(14),
        }
    } else {
        Shadow::NONE
    }
}
pub(crate) fn card(p: Palette) -> Frame {
    Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(12)
        .inner_margin(16)
        .shadow(card_shadow(p))
}
/// A recessed surface inside a card or dialog.
pub(crate) fn inset(p: Palette) -> Frame {
    Frame::new()
        .fill(if p.light { p.panel2 } else { p.bg })
        .stroke(Stroke::new(1.0_f32, mix(p.border, p.panel, 0.3)))
        .corner_radius(10)
        .inner_margin(14)
}
/// The track behind a row of [`segment`] controls.
pub(crate) fn track(p: Palette) -> Frame {
    Frame::new()
        .fill(if p.light { p.panel2 } else { p.bg })
        .stroke(Stroke::new(1.0_f32, mix(p.border, p.panel, 0.4)))
        .corner_radius(10)
        .inner_margin(3)
}
pub(crate) fn modal_frame(p: Palette) -> Frame {
    card(p).corner_radius(16).inner_margin(24).shadow(Shadow {
        offset: [0, 18],
        blur: 48,
        spread: 0,
        color: Color32::from_black_alpha(if p.light { 40 } else { 90 }),
    })
}
pub(crate) fn divider(ui: &mut Ui, p: Palette) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .hline(r.x_range(), r.center().y, Stroke::new(1.0_f32, p.border));
}
pub(crate) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let blend = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(
        blend(a.r(), b.r()),
        blend(a.g(), b.g()),
        blend(a.b(), b.b()),
        blend(a.a(), b.a()),
    )
}

#[derive(Clone, Copy)]
pub(crate) enum Glyph {
    Plus,
    Down,
    Up,
    Search,
    Library,
    Compass,
    Play,
    Pause,
    Folder,
    Trash,
    Close,
    Moon,
    Sun,
    Magnet,
    Settings,
    Copy,
    File,
    Check,
    Refresh,
    Warning,
    Peers,
    External,
    Clipboard,
}
pub(crate) fn icon(p: &Painter, r: Rect, g: Glyph, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let line = |points: &[(f32, f32)]| {
        p.add(Shape::line(
            points.iter().map(|&(x, y)| at(x, y)).collect(),
            stroke,
        ));
    };
    match g {
        Glyph::Plus => {
            line(&[(0.5, 0.18), (0.5, 0.82)]);
            line(&[(0.18, 0.5), (0.82, 0.5)]);
        }
        Glyph::Down | Glyph::Up => {
            let up = matches!(g, Glyph::Up);
            let a = if up { 0.76 } else { 0.15 };
            let b = if up { 0.15 } else { 0.76 };
            let c = if up { 0.4 } else { 0.5 };
            line(&[(0.5, a), (0.5, b)]);
            line(&[(0.24, c), (0.5, b), (0.76, c)]);
            line(&[(0.13, 0.72), (0.13, 0.92), (0.87, 0.92), (0.87, 0.72)]);
        }
        Glyph::Search => {
            p.circle_stroke(at(0.43, 0.43), r.width() * 0.29, stroke);
            line(&[(0.65, 0.65), (0.9, 0.9)]);
        }
        Glyph::Close => {
            line(&[(0.25, 0.25), (0.75, 0.75)]);
            line(&[(0.25, 0.75), (0.75, 0.25)]);
        }
        Glyph::Play => {
            p.add(Shape::closed_line(
                vec![at(0.3, 0.17), at(0.82, 0.5), at(0.3, 0.83)],
                stroke,
            ));
        }
        Glyph::Pause => {
            line(&[(0.32, 0.2), (0.32, 0.8)]);
            line(&[(0.68, 0.2), (0.68, 0.8)]);
        }
        Glyph::Folder => {
            p.add(Shape::closed_line(
                vec![
                    at(0.1, 0.28),
                    at(0.4, 0.28),
                    at(0.5, 0.4),
                    at(0.9, 0.4),
                    at(0.9, 0.84),
                    at(0.1, 0.84),
                ],
                stroke,
            ));
        }
        Glyph::Trash => {
            line(&[(0.18, 0.27), (0.82, 0.27)]);
            line(&[(0.4, 0.12), (0.6, 0.12)]);
            line(&[(0.28, 0.3), (0.32, 0.87), (0.68, 0.87), (0.72, 0.3)]);
            line(&[(0.44, 0.43), (0.44, 0.7)]);
            line(&[(0.57, 0.43), (0.57, 0.7)]);
        }
        Glyph::Library => {
            for x in [0.17, 0.42, 0.71] {
                p.rect_stroke(
                    Rect::from_min_max(at(x, 0.2), at(x + 0.12, 0.8)),
                    1,
                    stroke,
                    StrokeKind::Inside,
                );
            }
        }
        Glyph::Compass => {
            p.circle_stroke(r.center(), r.width() * 0.41, stroke);
            p.add(Shape::closed_line(
                vec![at(0.68, 0.29), at(0.57, 0.58), at(0.3, 0.7), at(0.42, 0.42)],
                stroke,
            ));
        }
        Glyph::Moon => {
            line(&[
                (0.67, 0.14),
                (0.42, 0.15),
                (0.23, 0.3),
                (0.17, 0.54),
                (0.27, 0.76),
                (0.5, 0.85),
                (0.73, 0.77),
                (0.86, 0.56),
                (0.65, 0.59),
                (0.48, 0.49),
                (0.44, 0.3),
                (0.54, 0.17),
                (0.67, 0.14),
            ]);
        }
        Glyph::Sun => {
            p.circle_stroke(r.center(), r.width() * 0.2, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [
                        r.center() + d * r.width() * 0.34,
                        r.center() + d * r.width() * 0.45,
                    ],
                    stroke,
                );
            }
        }
        Glyph::Magnet => {
            line(&[
                (0.2, 0.72),
                (0.2, 0.4),
                (0.28, 0.22),
                (0.5, 0.15),
                (0.72, 0.22),
                (0.8, 0.4),
                (0.8, 0.72),
            ]);
            line(&[
                (0.2, 0.56),
                (0.34, 0.56),
                (0.34, 0.4),
                (0.5, 0.31),
                (0.66, 0.4),
                (0.66, 0.72),
            ]);
            line(&[(0.2, 0.72), (0.34, 0.72)]);
            line(&[(0.66, 0.56), (0.8, 0.56)]);
        }
        Glyph::Settings => {
            let mut outline = Vec::with_capacity(32);
            for tooth in 0..8 {
                for (offset, radius) in [(0.0, 0.32), (0.2, 0.43), (0.6, 0.43), (0.8, 0.32)] {
                    let angle = (tooth as f32 + offset) * std::f32::consts::TAU / 8.0;
                    outline.push(r.center() + vec2(angle.cos(), angle.sin()) * r.width() * radius);
                }
            }
            p.add(Shape::closed_line(outline, stroke));
            p.circle_stroke(r.center(), r.width() * 0.13, stroke);
        }
        Glyph::Copy => {
            p.rect_stroke(
                Rect::from_min_max(at(0.32, 0.32), at(0.84, 0.86)),
                2,
                stroke,
                StrokeKind::Inside,
            );
            line(&[
                (0.2, 0.65),
                (0.16, 0.65),
                (0.16, 0.15),
                (0.65, 0.15),
                (0.65, 0.2),
            ]);
        }
        Glyph::File => {
            line(&[
                (0.62, 0.13),
                (0.21, 0.13),
                (0.21, 0.87),
                (0.79, 0.87),
                (0.79, 0.32),
                (0.62, 0.13),
                (0.62, 0.34),
                (0.79, 0.34),
            ]);
            line(&[(0.35, 0.55), (0.65, 0.55)]);
            line(&[(0.35, 0.7), (0.58, 0.7)]);
        }
        Glyph::Check => line(&[(0.18, 0.5), (0.4, 0.72), (0.83, 0.27)]),
        Glyph::Refresh => {
            line(&[
                (0.82, 0.42),
                (0.73, 0.22),
                (0.51, 0.14),
                (0.28, 0.22),
                (0.16, 0.44),
                (0.19, 0.68),
                (0.39, 0.83),
                (0.64, 0.83),
                (0.82, 0.66),
            ]);
            line(&[(0.82, 0.15), (0.82, 0.43), (0.55, 0.43)]);
        }
        Glyph::Warning => {
            p.add(Shape::closed_line(
                vec![at(0.5, 0.13), at(0.92, 0.85), at(0.08, 0.85)],
                stroke,
            ));
            line(&[(0.5, 0.36), (0.5, 0.58)]);
            p.circle_filled(at(0.5, 0.72), r.width() * 0.035, color);
        }
        Glyph::Peers => {
            p.circle_stroke(at(0.5, 0.3), r.width() * 0.13, stroke);
            line(&[
                (0.22, 0.82),
                (0.25, 0.65),
                (0.38, 0.55),
                (0.62, 0.55),
                (0.75, 0.65),
                (0.78, 0.82),
            ]);
            line(&[(0.17, 0.3), (0.12, 0.44), (0.2, 0.51)]);
            line(&[(0.83, 0.3), (0.88, 0.44), (0.8, 0.51)]);
        }
        Glyph::External => {
            line(&[
                (0.48, 0.2),
                (0.2, 0.2),
                (0.2, 0.82),
                (0.82, 0.82),
                (0.82, 0.55),
            ]);
            line(&[(0.47, 0.53), (0.87, 0.13)]);
            line(&[(0.58, 0.13), (0.87, 0.13), (0.87, 0.42)]);
        }
        Glyph::Clipboard => {
            p.rect_stroke(
                Rect::from_min_max(at(0.24, 0.21), at(0.8, 0.88)),
                2,
                stroke,
                StrokeKind::Inside,
            );
            p.rect(
                Rect::from_min_max(at(0.38, 0.12), at(0.66, 0.3)),
                2,
                color,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}
pub(crate) fn action(
    ui: &mut Ui,
    text: &str,
    glyph: Option<Glyph>,
    kind: ButtonKind,
    p: Palette,
) -> Response {
    action_sized(ui, text, glyph, kind, p, None, 34.0)
}
pub(crate) fn action_sized(
    ui: &mut Ui,
    text: &str,
    glyph: Option<Glyph>,
    kind: ButtonKind,
    p: Palette,
    width: Option<f32>,
    height: f32,
) -> Response {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), p.text);
    let icon_width = if glyph.is_some() { 21.0 } else { 0.0 };
    let natural = galley.size().x + 26.0 + icon_width;
    let (r, response) =
        ui.allocate_exact_size(vec2(width.unwrap_or(natural), height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), text));
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    let (base, over, stroke, text_color) = match kind {
        ButtonKind::Primary => (
            BLUE,
            mix(BLUE, Color32::WHITE, 0.12),
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
        ButtonKind::Secondary => (
            if p.light { p.panel } else { p.panel2 },
            if p.light {
                p.panel2
            } else {
                mix(p.panel2, Color32::WHITE, 0.04)
            },
            p.border,
            p.text,
        ),
        ButtonKind::Ghost => (
            Color32::TRANSPARENT,
            p.panel2,
            Color32::TRANSPARENT,
            p.muted,
        ),
        ButtonKind::Danger => (
            mix(RED, Color32::BLACK, 0.06),
            mix(RED, Color32::WHITE, 0.08),
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
    };
    let pressed = response.is_pointer_button_down_on();
    let enabled = ui.is_enabled();
    let fill = if pressed {
        mix(base, Color32::BLACK, 0.12)
    } else {
        mix(base, over, hover)
    };
    let solid = matches!(kind, ButtonKind::Primary | ButtonKind::Danger);
    if solid && enabled {
        ui.painter().add(
            Shadow {
                offset: [0, 2],
                blur: 8,
                spread: 0,
                color: base.gamma_multiply(if p.light { 0.25 } else { 0.18 }),
            }
            .as_shape(r, 8),
        );
    }
    ui.painter().rect(
        r,
        8,
        if enabled {
            fill
        } else if solid {
            p.tint(base, 0.14)
        } else {
            mix(p.panel, p.panel2, 0.6)
        },
        Stroke::new(1.0_f32, if enabled { stroke } else { p.border }),
        StrokeKind::Inside,
    );
    if solid && enabled {
        // A faint top highlight gives solid buttons some depth.
        ui.painter().hline(
            r.x_range().shrink(6.0),
            r.top() + 0.5,
            Stroke::new(1.0_f32, Color32::from_white_alpha(38)),
        );
    }
    let color = if enabled {
        if matches!(kind, ButtonKind::Ghost) {
            mix(text_color, p.text, hover)
        } else {
            text_color
        }
    } else if solid {
        mix(p.ink(base), p.panel, 0.3)
    } else {
        mix(p.muted, p.panel, 0.45)
    };
    let content_width = galley.size().x + icon_width;
    let x = r.center().x - content_width / 2.0;
    if let Some(g) = glyph {
        icon(
            ui.painter(),
            Rect::from_min_size(pos2(x, r.center().y - 7.0), vec2(14.0, 14.0)),
            g,
            color,
        );
    }
    ui.painter().text(
        pos2(x + icon_width, r.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    focus_ring(ui, &response, r, 8);
    response.on_hover_cursor(CursorIcon::PointingHand)
}
fn focus_ring(ui: &Ui, response: &Response, r: Rect, radius: u8) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            r.expand(2.0),
            radius + 2,
            Stroke::new(2.0_f32, BLUE.gamma_multiply(0.7)),
            StrokeKind::Outside,
        );
    }
}
pub(crate) fn primary(ui: &mut Ui, text: &str) -> Response {
    let p = Palette::new(!ui.visuals().dark_mode);
    let (text, g) = if let Some(text) = text.strip_prefix("+  ") {
        (text, Some(Glyph::Plus))
    } else {
        (text, None)
    };
    action(ui, text, g, ButtonKind::Primary, p)
}
pub(crate) fn secondary(ui: &mut Ui, text: &str, g: Option<Glyph>, p: Palette) -> Response {
    action(ui, text, g, ButtonKind::Secondary, p)
}
pub(crate) fn ghost(ui: &mut Ui, text: &str, g: Option<Glyph>, p: Palette) -> Response {
    action_sized(ui, text, g, ButtonKind::Ghost, p, None, 30.0)
}
pub(crate) fn icon_button(ui: &mut Ui, g: Glyph, tip: &str, p: Palette) -> Response {
    let (r, response) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), tip));
    let danger = matches!(g, Glyph::Trash);
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.1);
    if hover > 0.0 {
        let fill = if danger { p.tint(RED, 0.1) } else { p.panel2 };
        ui.painter()
            .rect_filled(r, 8, mix(Color32::TRANSPARENT, fill, hover));
    }
    focus_ring(ui, &response, r, 8);
    icon(
        ui.painter(),
        r.shrink(8.5),
        g,
        if !ui.is_enabled() {
            mix(p.muted, p.panel, 0.5)
        } else if danger && response.hovered() {
            p.ink(RED)
        } else {
            mix(p.muted, p.text, hover)
        },
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tip)
}
/// A rounded label, such as a count or a source name.
pub(crate) fn badge(ui: &mut Ui, label: &str, color: Color32, p: Palette) {
    let font = FontId::proportional(11.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.into(), font.clone(), color);
    let (r, _) = ui.allocate_exact_size(vec2(galley.size().x + 16.0, 20.0), Sense::hover());
    ui.painter().rect_filled(r, 10, p.tint(color, 0.1));
    ui.painter()
        .text(r.center(), Align2::CENTER_CENTER, label, font, p.ink(color));
}
/// A status label with a leading colored dot.
pub(crate) fn status_pill(ui: &mut Ui, label: &str, color: Color32, p: Palette) -> Response {
    let font = FontId::proportional(11.0);
    let mut chars = label.chars();
    let label: String = chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), font.clone(), color);
    let (r, response) = ui.allocate_exact_size(vec2(galley.size().x + 28.0, 22.0), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &label));
    ui.painter().rect_filled(r, 11, p.tint(color, 0.11));
    let dot = pos2(r.left() + 11.0, r.center().y);
    ui.painter()
        .circle_filled(dot, 6.0, color.gamma_multiply(0.22));
    ui.painter().circle_filled(dot, 3.0, color);
    ui.painter().text(
        pos2(r.left() + 19.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        p.ink(color),
    );
    response
}
/// A slim rounded progress bar.
pub(crate) fn progress(ui: &mut Ui, fraction: f32, color: Color32, height: f32, p: Palette) {
    let (r, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::ProgressIndicator,
            true,
            format!("{:.0}% complete", fraction * 100.0),
        )
    });
    let radius = (height / 2.0).round() as u8;
    ui.painter().rect_filled(
        r,
        radius,
        if p.light {
            mix(p.panel2, p.border, 0.5)
        } else {
            p.panel2
        },
    );
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let width = (r.width() * fraction).max(height);
        let fill = Rect::from_min_size(r.min, vec2(width, height));
        ui.painter().rect_filled(fill, radius, color);
        ui.painter().hline(
            fill.x_range().shrink(radius as f32),
            fill.top() + 0.5,
            Stroke::new(1.0_f32, Color32::from_white_alpha(40)),
        );
    }
}
/// A filled line chart of recent values, scaled to the largest value.
pub(crate) fn sparkline(painter: &Painter, r: Rect, values: &[f64], color: Color32) {
    if values.len() < 2 {
        return;
    }
    let max = values.iter().copied().fold(0.0_f64, f64::max).max(1.0);
    let step = r.width() / (values.len() - 1) as f32;
    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                r.left() + i as f32 * step,
                r.bottom() - (v / max) as f32 * r.height(),
            )
        })
        .collect();
    let mut mesh = Mesh::default();
    let top = color.gamma_multiply(0.22);
    let bottom = color.gamma_multiply(0.0);
    for pair in points.windows(2) {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(pair[0], top);
        mesh.colored_vertex(pair[1], top);
        mesh.colored_vertex(pos2(pair[1].x, r.bottom()), bottom);
        mesh.colored_vertex(pos2(pair[0].x, r.bottom()), bottom);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(mesh);
    painter.add(Shape::line(points, Stroke::new(1.5_f32, color)));
}
pub(crate) fn nav(
    ui: &mut Ui,
    label: &str,
    count: Option<usize>,
    active: bool,
    glyph: Option<Glyph>,
    p: Palette,
) -> bool {
    let (r, response) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::SelectableLabel, ui.is_enabled(), active, label)
    });
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.1);
    if active {
        ui.painter().rect_filled(r, 8, p.active);
    } else if hover > 0.0 {
        let fill = if p.light {
            mix(p.sidebar, p.border, 0.45)
        } else {
            p.panel2
        };
        ui.painter()
            .rect_filled(r, 8, mix(Color32::TRANSPARENT, fill, hover));
    }
    focus_ring(ui, &response, r, 8);
    if let Some(g) = glyph {
        icon(
            ui.painter(),
            Rect::from_center_size(pos2(r.left() + 20.0, r.center().y), vec2(15.0, 15.0)),
            g,
            if active {
                BLUE
            } else {
                mix(p.muted, p.text, hover * 0.6)
            },
        );
    }
    ui.painter().text(
        pos2(
            r.left() + if glyph.is_some() { 38.0 } else { 13.0 },
            r.center().y,
        ),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        if active {
            p.text
        } else {
            mix(p.muted, p.text, hover * 0.6)
        },
    );
    if let Some(count) = count {
        ui.painter().text(
            pos2(r.right() - 12.0, r.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.5),
            if active { p.ink(BLUE) } else { p.faint },
        );
    }
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}
/// One option of a segmented control; place a row of them inside [`track`].
pub(crate) fn segment(
    ui: &mut Ui,
    label: &str,
    active: bool,
    glyph: Option<Glyph>,
    p: Palette,
    width: Option<f32>,
) -> bool {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), p.text);
    let icon_width = if glyph.is_some() { 21.0 } else { 0.0 };
    let natural = galley.size().x + 24.0 + icon_width;
    let (r, response) =
        ui.allocate_exact_size(vec2(width.unwrap_or(natural), 30.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::SelectableLabel, ui.is_enabled(), active, label)
    });
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.1);
    if active {
        ui.painter().add(card_shadow(p).as_shape(r, 7));
        ui.painter().rect(
            r,
            7,
            if p.light { p.panel } else { p.panel2 },
            Stroke::new(
                1.0_f32,
                if p.light {
                    p.border
                } else {
                    mix(p.border, p.text, 0.06)
                },
            ),
            StrokeKind::Inside,
        );
    }
    focus_ring(ui, &response, r, 7);
    let color = if !ui.is_enabled() {
        mix(p.muted, p.panel, 0.45)
    } else if active {
        p.text
    } else {
        mix(p.muted, p.text, hover * 0.7)
    };
    let x = r.center().x - (galley.size().x + icon_width) / 2.0;
    if let Some(g) = glyph {
        icon(
            ui.painter(),
            Rect::from_min_size(pos2(x, r.center().y - 7.0), vec2(14.0, 14.0)),
            g,
            if active { BLUE } else { color },
        );
    }
    ui.painter().text(
        pos2(x + icon_width, r.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        color,
    );
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}
fn glyph_tile(painter: &Painter, r: Rect, g: Glyph, color: Color32, p: Palette) {
    let radius = (r.width() * 0.28).round() as u8;
    painter.rect(
        r,
        radius,
        p.tint(color, 0.12),
        Stroke::new(1.0_f32, p.tint(color, 0.2)),
        StrokeKind::Inside,
    );
    icon(painter, r.shrink(r.width() * 0.28), g, p.ink(color));
}
pub(crate) fn icon_badge(ui: &mut Ui, g: Glyph, color: Color32, p: Palette, size: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    glyph_tile(ui.painter(), r, g, color, p);
}
pub(crate) fn clipped_text(
    ui: &mut Ui,
    text: &str,
    size: f32,
    color: Color32,
    rows: usize,
    height: f32,
) -> Response {
    let (r, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let mut job =
        text::LayoutJob::simple(text.into(), FontId::proportional(size), color, r.width());
    job.wrap.max_rows = rows;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(r.min, galley, color);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), text));
    response.on_hover_text(text)
}
pub(crate) fn value_row(ui: &mut Ui, label: &str, value: &str, p: Palette) {
    ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.label(RichText::new(label).size(12.0).color(p.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(Label::new(RichText::new(value).size(12.0).color(p.text)).truncate())
                .on_hover_text(value);
        });
    });
}

/// A dashed rounded-rectangle outline, for drop zones.
pub(crate) fn dashed_outline(painter: &Painter, r: Rect, radius: f32, stroke: Stroke) {
    let mut points = Vec::with_capacity(41);
    for (corner, start) in [
        (pos2(r.right() - radius, r.top() + radius), -90.0_f32),
        (pos2(r.right() - radius, r.bottom() - radius), 0.0),
        (pos2(r.left() + radius, r.bottom() - radius), 90.0),
        (pos2(r.left() + radius, r.top() + radius), 180.0),
    ] {
        for step in 0..=9 {
            let angle = (start + step as f32 * 10.0).to_radians();
            points.push(corner + vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    points.push(points[0]);
    painter.extend(Shape::dashed_line(&points, stroke, 6.0, 4.0));
}

/// The app mark: the same artwork as `assets/brand.svg` and `make-icon.swift`.
/// The SVG's 64-unit canvas is cropped to 8..56 so the drop fills `r`. On light
/// themes the arrow cutout is white so it stands out against the light canvas.
pub(crate) fn brand_mark(p: &Painter, r: Rect, light: bool) {
    const SKY: Color32 = Color32::from_rgb(56, 189, 248);
    const INDIGO: Color32 = Color32::from_rgb(99, 102, 241);
    const PURPLE: Color32 = Color32::from_rgb(139, 92, 246);
    let at =
        |x: f32, y: f32| r.min + vec2((x - 8.0) / 48.0 * r.width(), (y - 8.0) / 48.0 * r.height());
    // Interpolates a gradient along `from -> to`, like an SVG linear gradient.
    let shade = |point: Pos2, from: Pos2, to: Pos2, stops: &[(f32, Color32)]| {
        let axis = to - from;
        let t = ((point - from).dot(axis) / axis.length_sq()).clamp(0.0, 1.0);
        stops
            .windows(2)
            .find(|w| t <= w[1].0)
            .map(|w| mix(w[0].1, w[1].1, (t - w[0].0) / (w[1].0 - w[0].0)))
            .unwrap_or(stops[stops.len() - 1].1)
    };
    let fan = |mesh: &mut Mesh, outline: &[Pos2], color: &dyn Fn(Pos2) -> Color32| {
        let center =
            outline.iter().fold(Vec2::ZERO, |sum, p| sum + p.to_vec2()) / outline.len() as f32;
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(center.to_pos2(), color(center.to_pos2()));
        for point in outline {
            mesh.colored_vertex(*point, color(*point));
        }
        let n = outline.len() as u32;
        for i in 0..n {
            mesh.add_triangle(base, base + 1 + i, base + 1 + (i + 1) % n);
        }
    };

    // Outer drop shield: four cubic segments, which together form a convex outline.
    let segments = [
        [(32.0, 10.0), (45.0, 10.0), (52.0, 20.0), (52.0, 34.0)],
        [(52.0, 34.0), (52.0, 46.0), (43.0, 54.0), (32.0, 54.0)],
        [(32.0, 54.0), (21.0, 54.0), (12.0, 46.0), (12.0, 34.0)],
        [(12.0, 34.0), (12.0, 20.0), (19.0, 10.0), (32.0, 10.0)],
    ];
    let mut outline = Vec::with_capacity(64);
    for [a, b, c, d] in segments {
        for step in 0..16 {
            let t = step as f32 / 16.0;
            let q = 1.0 - t;
            let x =
                a.0 * q * q * q + 3.0 * b.0 * q * q * t + 3.0 * c.0 * q * t * t + d.0 * t * t * t;
            let y =
                a.1 * q * q * q + 3.0 * b.1 * q * q * t + 3.0 * c.1 * q * t * t + d.1 * t * t * t;
            outline.push(at(x, y));
        }
    }
    let mut mesh = Mesh::default();
    let (from, to) = (at(12.0, 10.0), at(52.0, 54.0));
    fan(&mut mesh, &outline, &|point| {
        shade(point, from, to, &[(0.0, SKY), (0.5, INDIGO), (1.0, PURPLE)])
    });

    // Arrow cutout, then the inner highlight arrow at 90% opacity. Each arrow
    // is drawn as its convex head plus its rectangular stem.
    let arrow = |mesh: &mut Mesh,
                 head: [(f32, f32); 3],
                 stem: [(f32, f32); 2],
                 color: &dyn Fn(Pos2) -> Color32| {
        fan(mesh, &head.map(|(x, y)| at(x, y)), color);
        let [(x0, y0), (x1, y1)] = stem;
        fan(
            mesh,
            &[at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)],
            color,
        );
    };
    let cutout = if light {
        Color32::WHITE
    } else {
        Color32::from_rgb(14, 15, 20)
    };
    arrow(
        &mut mesh,
        [(32.0, 20.0), (44.0, 33.0), (20.0, 33.0)],
        [(27.0, 33.0), (37.0, 44.0)],
        &|_| cutout,
    );
    let (from, to) = (at(25.0, 40.0), at(39.0, 25.0));
    arrow(
        &mut mesh,
        [(32.0, 25.0), (39.0, 33.0), (25.0, 33.0)],
        [(30.0, 33.0), (34.0, 40.0)],
        &|point| {
            // Blend over the cutout so the result matches 90% opacity exactly.
            mix(
                cutout,
                shade(point, from, to, &[(0.0, INDIGO), (1.0, SKY)]),
                0.9,
            )
        },
    );
    p.add(mesh);
}

pub(crate) fn status_color(t: &crate::model::Torrent) -> Color32 {
    match t.status() {
        "downloading" => BLUE,
        "seeding" => GREEN,
        "metadata" | "checking" => YELLOW,
        "error" => RED,
        _ => IDLE,
    }
}

pub(crate) fn dialog_header(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    glyph: Glyph,
    color: Color32,
    p: Palette,
    close_enabled: bool,
) -> bool {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
    let tile = Rect::from_min_size(r.min + vec2(0.0, 2.0), vec2(42.0, 42.0));
    glyph_tile(ui.painter(), tile, glyph, color, p);
    let text_rect = Rect::from_min_max(r.min + vec2(56.0, 1.0), r.max - vec2(44.0, 0.0));
    let mut text = ui.new_child(
        UiBuilder::new()
            .max_rect(text_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    text.spacing_mut().item_spacing.y = 3.0;
    text.label(display(title, 19.0).color(p.text));
    text.add(Label::new(RichText::new(subtitle).size(12.0).color(p.muted)).truncate());
    let close_rect = Rect::from_min_size(pos2(r.right() - 32.0, r.top() + 2.0), vec2(32.0, 32.0));
    let mut close = ui.new_child(
        UiBuilder::new()
            .max_rect(close_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    close
        .add_enabled_ui(close_enabled, |ui| {
            icon_button(ui, Glyph::Close, "Close dialog", p).clicked()
        })
        .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_labels_match_mac_and_windows_keyboard_input() {
        use eframe::egui::os::OperatingSystem;

        for os in [OperatingSystem::Mac, OperatingSystem::Windows] {
            let ctx = Context::default();
            ctx.set_os(os);
            let modifiers = Modifiers {
                command: true,
                ctrl: os == OperatingSystem::Windows,
                mac_cmd: os == OperatingSystem::Mac,
                ..Default::default()
            };
            let _ = ctx.run(
                RawInput {
                    modifiers,
                    events: vec![Event::Key {
                        key: Key::K,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                    ..Default::default()
                },
                |ctx| {
                    let label = command_shortcut(ctx, Key::K);
                    if os == OperatingSystem::Windows {
                        assert_eq!(label, "Ctrl+K");
                        assert_eq!(command_shortcut(ctx, Key::Enter), "Ctrl+Enter");
                    } else {
                        assert!(label.contains('⌘') || label.contains("Cmd"));
                        assert!(!label.contains("Ctrl"));
                        assert!(command_shortcut(ctx, Key::Enter).ends_with('↵'));
                    }
                    assert!(ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)));
                },
            );
        }
    }

    #[test]
    fn modal_header_and_controls_fit_small_windows() {
        for size in [vec2(820.0, 548.0), vec2(1280.0, 788.0)] {
            let ctx = Context::default();
            ctx.set_fonts(fonts());
            let p = Palette::new(false);
            let mut bounds = Rect::NOTHING;
            for _ in 0..3 {
                let _ = ctx.run(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        let response = Modal::new(Id::new("layout-regression"))
                            .frame(modal_frame(p))
                            .show(ctx, |ui| {
                                ui.set_width(480.0);
                                dialog_header(
                                    ui,
                                    "Add a torrent",
                                    "Start with a magnet link or a torrent file.",
                                    Glyph::Magnet,
                                    BLUE,
                                    p,
                                    true,
                                );
                                ui.add_space(12.0);
                                let mut text = String::new();
                                ui.add_sized(
                                    [ui.available_width(), 104.0],
                                    TextEdit::multiline(&mut text),
                                );
                                ui.horizontal(|ui| {
                                    secondary(ui, "Cancel", None, p);
                                    primary(ui, "+  Add torrent");
                                });
                                ui.min_rect()
                            });
                        bounds = response.inner;
                    },
                );
            }
            assert!(
                bounds.height() < 300.0,
                "Header expanded the dialog: {bounds:?}"
            );
            assert!(
                bounds.top() >= 0.0 && bounds.bottom() <= size.y,
                "Dialog controls exceed the viewport: {bounds:?}"
            );
        }
    }

    #[test]
    fn display_family_always_has_fallback_fonts() {
        // Rendering a missing font family panics, so the display family must
        // exist and resolve even when no system face could be loaded.
        let fonts = fonts();
        let display = fonts
            .families
            .get(&FontFamily::Name(DISPLAY.into()))
            .expect("fonts() registers the display family");
        let proportional = &fonts.families[&FontFamily::Proportional];
        assert!(proportional.iter().all(|name| display.contains(name)));
        assert!(
            display
                .iter()
                .all(|name| fonts.font_data.contains_key(name))
        );
    }

    #[test]
    fn custom_button_clicks_and_disabled_state() {
        for enabled in [true, false] {
            let ctx = Context::default();
            let p = Palette::new(false);
            let mut rect = Rect::NOTHING;
            let mut clicked = false;
            let draw = |ctx: &Context, rect: &mut Rect, clicked: &mut bool| {
                CentralPanel::default().show(ctx, |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response =
                            action(ui, "Add torrent", Some(Glyph::Plus), ButtonKind::Primary, p);
                        *rect = response.rect;
                        *clicked = response.clicked();
                    });
                });
            };
            let raw = || RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(820.0, 548.0))),
                ..Default::default()
            };
            let _ = ctx.run(raw(), |ctx| draw(ctx, &mut rect, &mut clicked));
            let center = rect.center();
            for pressed in [true, false] {
                let mut input = raw();
                input.events = vec![
                    Event::PointerMoved(center),
                    Event::PointerButton {
                        pos: center,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ];
                let _ = ctx.run(input, |ctx| draw(ctx, &mut rect, &mut clicked));
            }
            assert_eq!(clicked, enabled, "Disabled controls must not submit");
            assert!(rect.height() >= 32.0);
        }
    }
}

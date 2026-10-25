use eframe::egui::*;

pub(crate) const BLUE: Color32 = Color32::from_rgb(76, 115, 236);
pub(crate) const GREEN: Color32 = Color32::from_rgb(52, 211, 153);
pub(crate) const YELLOW: Color32 = Color32::from_rgb(251, 191, 36);
pub(crate) const RED: Color32 = Color32::from_rgb(248, 113, 113);
pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub active: Color32,
}
impl Palette {
    pub fn new(light: bool) -> Self {
        if light {
            Self {
                bg: rgb(246, 247, 250),
                panel: Color32::WHITE,
                panel2: rgb(240, 242, 247),
                border: rgb(224, 228, 237),
                text: rgb(24, 29, 40),
                muted: rgb(107, 115, 132),
                active: rgb(231, 237, 255),
            }
        } else {
            Self {
                bg: rgb(10, 10, 13),
                panel: rgb(19, 19, 24),
                panel2: rgb(26, 27, 34),
                border: rgb(37, 39, 49),
                text: rgb(237, 239, 245),
                muted: rgb(143, 149, 166),
                active: rgb(31, 41, 69),
            }
        }
    }
}
pub(crate) fn mono(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::monospace(size))
        .color(color)
}
pub(crate) fn small(ui: &mut Ui, text: impl Into<String>, p: Palette) {
    ui.label(mono(text, 10.0, p.muted));
}
pub(crate) fn muted(ui: &mut Ui, text: impl Into<String>, p: Palette) {
    ui.label(RichText::new(text).size(12.0).color(p.muted));
}
pub(crate) fn card(p: Palette) -> Frame {
    Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(9)
        .inner_margin(16)
}
pub(crate) fn modal_frame(p: Palette) -> Frame {
    card(p).corner_radius(14).inner_margin(24).shadow(Shadow {
        offset: [0, 12],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(65),
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
            p.circle_stroke(r.center(), r.width() * 0.3, stroke);
            p.circle_stroke(r.center(), r.width() * 0.11, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [
                        r.center() + d * r.width() * 0.3,
                        r.center() + d * r.width() * 0.43,
                    ],
                    stroke,
                );
            }
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
    action_sized(ui, text, glyph, kind, p, None, 36.0)
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
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(12.5), p.text);
    let natural = galley.size().x + 24.0 + if glyph.is_some() { 22.0 } else { 0.0 };
    let (r, response) =
        ui.allocate_exact_size(vec2(width.unwrap_or(natural), height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), text));
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    let (base, over, stroke, text_color) = match kind {
        ButtonKind::Primary => (
            BLUE,
            rgb(90, 129, 250),
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
        ButtonKind::Secondary => (p.panel2, mix(p.panel2, BLUE, 0.09), p.border, p.text),
        ButtonKind::Ghost => (
            Color32::TRANSPARENT,
            p.panel2,
            Color32::TRANSPARENT,
            p.muted,
        ),
        ButtonKind::Danger => (
            mix(p.panel, RED, 0.09),
            mix(p.panel, RED, 0.17),
            mix(p.border, RED, 0.3),
            RED,
        ),
    };
    let pressed = response.is_pointer_button_down_on();
    let enabled = ui.is_enabled();
    let fill = if pressed {
        mix(base, Color32::BLACK, 0.14)
    } else {
        mix(base, over, hover)
    };
    ui.painter().rect(
        r,
        6,
        if enabled {
            fill
        } else {
            mix(p.panel, p.panel2, 0.6)
        },
        Stroke::new(1.0_f32, if enabled { stroke } else { p.border }),
        StrokeKind::Inside,
    );
    let color = if enabled {
        text_color
    } else {
        mix(p.muted, p.panel, 0.45)
    };
    let content_width = galley.size().x + if glyph.is_some() { 22.0 } else { 0.0 };
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
        pos2(x + if glyph.is_some() { 22.0 } else { 0.0 }, r.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(12.5),
        color,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            r.expand(2.0),
            8,
            Stroke::new(1.5_f32, BLUE),
            StrokeKind::Outside,
        );
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
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
    if response.hovered() || response.has_focus() {
        ui.painter().rect(
            r,
            6,
            if danger {
                mix(p.panel, RED, 0.1)
            } else {
                p.panel2
            },
            Stroke::new(1.0_f32, if response.has_focus() { BLUE } else { p.border }),
            StrokeKind::Inside,
        );
    }
    icon(
        ui.painter(),
        r.shrink(8.0),
        g,
        if !ui.is_enabled() {
            mix(p.muted, p.panel, 0.5)
        } else if danger && response.hovered() {
            RED
        } else if response.hovered() {
            p.text
        } else {
            p.muted
        },
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tip)
}
pub(crate) fn badge(ui: &mut Ui, label: &str, color: Color32, p: Palette) {
    let galley = ui
        .painter()
        .layout_no_wrap(label.into(), FontId::monospace(10.0), color);
    let (r, _) = ui.allocate_exact_size(vec2(galley.size().x + 16.0, 22.0), Sense::hover());
    ui.painter().rect(
        r,
        5,
        mix(p.panel, color, 0.08),
        Stroke::new(1.0_f32, mix(p.border, color, 0.15)),
        StrokeKind::Inside,
    );
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(10.0),
        color,
    );
}
pub(crate) fn nav(
    ui: &mut Ui,
    label: &str,
    count: Option<usize>,
    active: bool,
    glyph: Option<Glyph>,
    p: Palette,
) -> bool {
    let (r, response) = ui.allocate_exact_size(vec2(ui.available_width(), 37.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::SelectableLabel, ui.is_enabled(), active, label)
    });
    if active || response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(r, 6, if active { p.active } else { p.panel2 });
    }
    if active {
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(r.left(), r.top() + 10.0), vec2(2.0, 17.0)),
            2,
            BLUE,
        );
    }
    if let Some(g) = glyph {
        icon(
            ui.painter(),
            Rect::from_center_size(pos2(r.left() + 20.0, r.center().y), vec2(16.0, 16.0)),
            g,
            if active { BLUE } else { p.muted },
        );
    }
    ui.painter().text(
        pos2(
            r.left() + if glyph.is_some() { 37.0 } else { 13.0 },
            r.center().y,
        ),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        if active { p.text } else { p.muted },
    );
    if let Some(count) = count {
        let rect = Rect::from_center_size(pos2(r.right() - 20.0, r.center().y), vec2(25.0, 21.0));
        ui.painter().rect_filled(
            rect,
            4,
            if active {
                mix(p.active, BLUE, 0.12)
            } else {
                p.panel2
            },
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            count.to_string(),
            FontId::monospace(10.0),
            if active { BLUE } else { p.muted },
        );
    }
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}
pub(crate) fn segment(
    ui: &mut Ui,
    label: &str,
    active: bool,
    glyph: Option<Glyph>,
    p: Palette,
    width: Option<f32>,
) -> bool {
    let response = action_sized(
        ui,
        label,
        glyph,
        if active {
            ButtonKind::Secondary
        } else {
            ButtonKind::Ghost
        },
        p,
        width,
        34.0,
    );
    if active {
        ui.painter().hline(
            response.rect.x_range().shrink(10.0),
            response.rect.bottom() - 1.0,
            Stroke::new(2.0_f32, BLUE),
        );
    }
    response.clicked()
}
pub(crate) fn icon_badge(ui: &mut Ui, g: Glyph, color: Color32, p: Palette, size: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().rect(
        r,
        8,
        mix(p.panel, color, 0.09),
        Stroke::new(1.0_f32, mix(p.border, color, 0.12)),
        StrokeKind::Inside,
    );
    icon(ui.painter(), r.shrink(size * 0.27), g, color);
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
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(r.min, galley, color);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), text));
    response.on_hover_text(text)
}
pub(crate) fn value_row(ui: &mut Ui, label: &str, value: &str, p: Palette) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(p.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(Label::new(mono(value, 11.0, p.text)).truncate())
                .on_hover_text(value);
        });
    });
}

pub(crate) fn brand_mark(p: &Painter, r: Rect) {
    p.rect_filled(r, r.width() * 0.223, BLUE);
    let at = |x: f32, y: f32| {
        r.min
            + vec2(
                (x - 64.0) / 896.0 * r.width(),
                (960.0 - y) / 896.0 * r.height(),
            )
    };
    let a = at(535.0, 645.0);
    let c1 = at(710.0, 820.0);
    let c2 = at(890.0, 640.0);
    let b = at(715.0, 465.0);
    let mut points = vec![at(280.0, 390.0), a];
    for step in 1..=30 {
        let t = step as f32 / 30.0;
        let q = 1.0 - t;
        let point = a.to_vec2() * q.powi(3)
            + c1.to_vec2() * 3.0 * q * q * t
            + c2.to_vec2() * 3.0 * q * t * t
            + b.to_vec2() * t.powi(3);
        points.push(point.to_pos2());
    }
    points.push(at(460.0, 210.0));
    let width = r.width() * 64.0 / 896.0;
    p.add(Shape::line(
        points.clone(),
        Stroke::new(width, Color32::WHITE),
    ));
    p.circle_filled(points[0], width / 2.0, Color32::WHITE);
    p.circle_filled(*points.last().unwrap(), width / 2.0, Color32::WHITE);
    for (a, b) in [
        ((400.0, 510.0), (493.0, 417.0)),
        ((580.0, 330.0), (673.0, 423.0)),
    ] {
        p.line_segment(
            [at(a.0, a.1), at(b.0, b.1)],
            Stroke::new(r.width() * 24.0 / 896.0, Color32::WHITE),
        );
    }
}

pub(crate) fn status_color(t: &crate::model::Torrent) -> Color32 {
    match t.status() {
        "downloading" => BLUE,
        "seeding" => GREEN,
        "metadata" | "checking" => YELLOW,
        "error" => RED,
        _ => rgb(143, 149, 166),
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
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
    let tile = Rect::from_min_size(r.min + vec2(0.0, 2.0), vec2(40.0, 40.0));
    ui.painter().rect(
        tile,
        8,
        mix(p.panel, color, 0.09),
        Stroke::new(1.0_f32, mix(p.border, color, 0.12)),
        StrokeKind::Inside,
    );
    icon(ui.painter(), tile.shrink(11.0), glyph, color);
    let text_rect = Rect::from_min_max(r.min + vec2(54.0, 0.0), r.max - vec2(44.0, 0.0));
    let mut text = ui.new_child(
        UiBuilder::new()
            .max_rect(text_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    text.spacing_mut().item_spacing.y = 5.0;
    text.label(RichText::new(title).size(20.0).strong());
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
    fn modal_header_and_controls_fit_small_windows() {
        for size in [vec2(820.0, 548.0), vec2(1280.0, 788.0)] {
            let ctx = Context::default();
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

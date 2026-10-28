//! Hand-drawn widget kit, styled after macOS controls: flat fills, hairline
//! borders, one accent colour. Every control is painted with the painter.

use super::theme::{lerp_color, with_alpha, Theme};
use eframe::egui::{
    self, epaint::Shadow, Align2, Color32, CornerRadius, FontFamily, FontId, Id, Mesh, Painter, Pos2, Rect,
    Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use std::f32::consts::{PI, TAU};

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

pub fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

pub fn shadow(painter: &Painter, rect: Rect, radius: f32, strength: f32, theme: &Theme) {
    let a = if theme.dark { 0.30 } else { 0.06 } * strength;
    let s = Shadow { offset: [0, 2], blur: 12, spread: 0, color: with_alpha(Color32::BLACK, a) };
    painter.add(s.as_shape(rect, cr(radius)));
}

/// Vertical gradient rectangle (square corners), used for backgrounds.
pub fn vgradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

#[derive(Clone, Copy, PartialEq)]
pub enum BtnStyle {
    Primary,
    Danger,
    Ghost,
    Subtle,
}

pub fn button(ui: &mut Ui, rect: Rect, id: Id, label: &str, icon: Option<Icon>, style: BtnStyle, theme: &Theme) -> Response {
    button_ex(ui, rect, id, label, icon, style, theme, true)
}

#[allow(clippy::too_many_arguments)]
pub fn button_ex(ui: &mut Ui, rect: Rect, id: Id, label: &str, icon: Option<Icon>, style: BtnStyle, theme: &Theme, enabled: bool) -> Response {
    button_shaped(ui, rect, id, label, icon, style, theme, enabled, cr(6.0))
}

/// [`button_ex`] with custom corners (e.g. square top corners for a ribbon).
#[allow(clippy::too_many_arguments)]
pub fn button_shaped(ui: &mut Ui, rect: Rect, id: Id, label: &str, icon: Option<Icon>, style: BtnStyle, theme: &Theme, enabled: bool, radius: CornerRadius) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let resp = ui.interact(rect, id, sense);
    let ctx = ui.ctx();
    // pressed darkens, as on macOS; hover only for the cursor
    let down = ctx.animate_bool_with_time(id.with("p"), resp.is_pointer_button_down_on() && enabled, 0.05);
    let h = ctx.animate_bool_with_time(id.with("h"), resp.hovered() && enabled, 0.1);
    let painter = ui.painter();
    let r = rect;
    let press = |c: Color32| lerp_color(c, Color32::BLACK, 0.12 * down);
    let (fill, text) = match style {
        BtnStyle::Primary => (press(theme.accent), Color32::WHITE),
        BtnStyle::Danger => (press(theme.danger), Color32::WHITE),
        BtnStyle::Ghost => (with_alpha(theme.surface_hi, 0.7 * h + 0.3 * down), theme.text),
        BtnStyle::Subtle => (lerp_color(theme.surface, theme.surface_hi, 0.5 * down), theme.text),
    };
    let (fill, text) = if enabled { (fill, text) } else { (with_alpha(theme.surface_hi, 0.5), theme.text_faint) };
    painter.rect_filled(r, radius, fill);
    if style == BtnStyle::Subtle {
        painter.rect_stroke(r, radius, Stroke::new(1.0, theme.stroke), StrokeKind::Inside);
    }
    let icon_w = if icon.is_some() { 16.0 + if label.is_empty() { 0.0 } else { 6.0 } } else { 0.0 };
    // Fit long (translated) labels: shrink the font a little, then truncate.
    let room = (r.width() - 24.0 - icon_w).max(10.0);
    let weight = |size: f32| if matches!(style, BtnStyle::Primary | BtnStyle::Danger) { bold(size) } else { font(size) };
    let mut size = 13.0;
    let mut galley = painter.layout_no_wrap(label.to_string(), weight(size), text);
    while galley.size().x > room && size > 10.5 {
        size -= 0.5;
        galley = painter.layout_no_wrap(label.to_string(), weight(size), text);
    }
    if galley.size().x > room {
        let fitted = truncate(painter, label, &weight(size), room);
        galley = painter.layout_no_wrap(fitted, weight(size), text);
    }
    let total = galley.size().x + icon_w;
    let mut x = r.center().x - total / 2.0;
    if let Some(ic) = icon {
        draw_icon(painter, ic, Rect::from_center_size(Pos2::new(x + 8.0, r.center().y), Vec2::splat(16.0)), text);
        x += icon_w;
    }
    painter.galley(Pos2::new(x, r.center().y - galley.size().y / 2.0), galley, text);
    resp
}

/// Round icon-only button.
pub fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: Icon, theme: &Theme, enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let resp = ui.interact(rect, id, sense);
    let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered() && enabled, 0.12);
    let painter = ui.painter();
    let c = rect.center();
    let rad = rect.width().min(rect.height()) / 2.0;
    painter.rect_filled(rect, cr(6.0), with_alpha(theme.surface_hi, 0.8 * h));
    let col = if enabled { lerp_color(theme.text_dim, theme.text, h) } else { theme.text_faint };
    draw_icon(painter, icon, Rect::from_center_size(c, Vec2::splat(rad * 1.05)), col);
    resp
}

pub fn toggle(ui: &mut Ui, rect: Rect, id: Id, value: &mut bool, theme: &Theme) -> Response {
    let mut resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(id, *value, 0.16);
    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), Vec2::new(38.0, 22.0));
    let off = if theme.dark { theme.surface_hi } else { lerp_color(theme.surface_hi, theme.stroke, 0.5) };
    painter.rect_filled(track, cr(11.0), lerp_color(off, theme.accent, t));
    let x = track.left() + 11.0 + t * 16.0;
    let knob = Pos2::new(x, track.center().y);
    painter.circle_filled(knob + Vec2::new(0.0, 0.5), 9.5, with_alpha(Color32::BLACK, 0.12));
    painter.circle_filled(knob, 9.0, Color32::WHITE);
    resp
}

pub fn progress_bar(painter: &Painter, rect: Rect, frac: Option<f32>, theme: &Theme, time: f64, color: Option<Color32>) {
    let radius = rect.height() / 2.0;
    let track = if theme.dark { theme.surface_hi } else { lerp_color(theme.surface_hi, theme.stroke, 0.4) };
    painter.rect_filled(rect, cr(radius), track);
    let fill_c = color.unwrap_or(theme.accent);
    match frac {
        Some(f) => {
            let f = f.clamp(0.0, 1.0);
            if f > 0.0 {
                let w = (rect.width() * f).max(rect.height());
                painter.rect_filled(Rect::from_min_size(rect.min, Vec2::new(w, rect.height())), cr(radius), fill_c);
            }
        }
        None => {
            // indeterminate: a sliding segment
            let t = (time * 0.9).fract() as f32;
            let w = rect.width() * 0.3;
            let x = rect.left() - w + t * (rect.width() + w);
            let blob = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height())).intersect(rect);
            if blob.width() > 0.0 {
                painter.rect_filled(blob, cr(radius), fill_c);
            }
        }
    }
}

/// Circular progress ring.
pub fn ring(painter: &Painter, center: Pos2, radius: f32, width: f32, frac: f32, color: Color32, track: Color32) {
    painter.circle_stroke(center, radius, Stroke::new(width, track));
    let n = ((frac * 96.0).ceil() as usize).max(2);
    if frac > 0.0 {
        let pts: Vec<Pos2> =
            (0..=n).map(|i| center + Vec2::angled(-PI / 2.0 + TAU * frac * i as f32 / n as f32) * radius).collect();
        painter.add(Shape::line(pts, Stroke::new(width, color)));
    }
}

/// Frameless text input on a custom rounded field.
pub fn text_input(ui: &mut Ui, rect: Rect, id: Id, value: &mut String, hint: &str, theme: &Theme) -> Response {
    let focused = ui.memory(|m| m.has_focus(id));
    let t = ui.ctx().animate_bool_with_time(id.with("f"), focused, 0.15);
    {
        let painter = ui.painter();
        painter.rect_filled(rect, cr(6.0), theme.surface);
        painter.rect_stroke(rect, cr(6.0), Stroke::new(1.0, theme.stroke), StrokeKind::Inside);
        if t > 0.0 {
            painter.rect_stroke(rect, cr(6.0), Stroke::new(3.0, with_alpha(theme.accent, 0.4 * t)), StrokeKind::Outside);
        }
    }
    let inner = rect.shrink2(Vec2::new(12.0, 0.0));
    let edit = egui::TextEdit::singleline(value)
        .id(id)
        .frame(egui::Frame::NONE)
        .font(font(13.0))
        .text_color(theme.text)
        .hint_text(egui::RichText::new(hint).color(theme.text_faint).font(font(13.0)))
        .desired_width(inner.width())
        .vertical_align(egui::Align::Center);
    ui.put(inner, edit)
}

/// A selectable card (radio-like).
pub fn choice_card(ui: &mut Ui, rect: Rect, id: Id, selected: bool, title: &str, subtitle: &str, icon: Icon, theme: &Theme, enabled: bool) -> Response {
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered() && enabled, 0.12);
    let s = ui.ctx().animate_bool_with_time(id.with("s"), selected, 0.15);
    let painter = ui.painter();
    let fill = lerp_color(lerp_color(theme.surface, theme.surface_hi, 0.4 * h), lerp_color(theme.surface, theme.accent, 0.08), s);
    painter.rect_filled(rect, cr(8.0), fill);
    painter.rect_stroke(rect, cr(8.0), Stroke::new(1.0 + s, lerp_color(theme.stroke, theme.accent, s)), StrokeKind::Inside);
    let alpha = if enabled { 1.0 } else { 0.45 };
    let ic = Rect::from_center_size(Pos2::new(rect.left() + 26.0, rect.center().y), Vec2::splat(20.0));
    draw_icon(painter, icon, ic, with_alpha(lerp_color(theme.text_dim, theme.accent, s), alpha));
    // keep clear of the radio dot on the right (long translations)
    let room = rect.width() - 48.0 - 36.0;
    let title = truncate(painter, title, &bold(13.5), room);
    let subtitle = truncate(painter, subtitle, &font(11.5), room);
    painter.text(Pos2::new(rect.left() + 48.0, rect.center().y - 8.0), Align2::LEFT_CENTER, title, bold(13.5), with_alpha(theme.text, alpha));
    painter.text(Pos2::new(rect.left() + 48.0, rect.center().y + 9.0), Align2::LEFT_CENTER, subtitle, font(11.5), with_alpha(theme.text_dim, alpha));
    // radio dot
    let dot = Pos2::new(rect.right() - 20.0, rect.center().y);
    painter.circle_filled(dot, 7.5, lerp_color(theme.surface, theme.accent, s));
    painter.circle_stroke(dot, 7.5, Stroke::new(1.0, lerp_color(theme.stroke, theme.accent, s)));
    painter.circle_filled(dot, 3.0 * s, Color32::WHITE);
    resp
}

pub fn checkbox(ui: &mut Ui, rect: Rect, id: Id, value: &mut bool, label: &str, theme: &Theme) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *value = !*value;
    }
    let t = ui.ctx().animate_bool_with_time(id, *value, 0.12);
    let painter = ui.painter();
    let b = Rect::from_center_size(Pos2::new(rect.left() + 9.0, rect.center().y), Vec2::splat(16.0));
    painter.rect_filled(b, cr(4.0), lerp_color(theme.surface, theme.accent, t));
    painter.rect_stroke(b, cr(4.0), Stroke::new(1.0, lerp_color(theme.stroke, theme.accent, t)), StrokeKind::Inside);
    if t > 0.05 {
        draw_icon(painter, Icon::Check, b.shrink(3.5), with_alpha(Color32::WHITE, t));
    }
    painter.text(Pos2::new(rect.left() + 28.0, rect.center().y), Align2::LEFT_CENTER, label, font(13.0), theme.text);
    resp
}

/// Stepped slider with integer values.
pub fn stepper(ui: &mut Ui, rect: Rect, id: Id, value: &mut usize, min: usize, max: usize, theme: &Theme) -> Response {
    let mut resp = ui.interact(rect, id, Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 16.0, 4.0));
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let f = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let v = min + (f * (max - min) as f32).round() as usize;
            if v != *value {
                *value = v;
                resp.mark_changed();
            }
        }
    }
    let painter = ui.painter();
    painter.rect_filled(track, cr(2.0), theme.surface_hi);
    let steps = max - min;
    for i in 0..=steps {
        let x = track.left() + track.width() * i as f32 / steps as f32;
        painter.circle_filled(Pos2::new(x, track.center().y), 2.5, theme.stroke);
    }
    let f = (*value - min) as f32 / steps as f32;
    let target = track.left() + track.width() * f;
    let x = ui.ctx().animate_value_with_time(id.with("x"), target, 0.12);
    painter.rect_filled(Rect::from_min_max(track.left_top(), Pos2::new(x, track.bottom())), cr(2.0), theme.accent);
    painter.circle_filled(Pos2::new(x, track.center().y + 0.5), 9.5, with_alpha(Color32::BLACK, 0.12));
    painter.circle_filled(Pos2::new(x, track.center().y), 9.0, Color32::WHITE);
    painter.circle_stroke(Pos2::new(x, track.center().y), 9.0, Stroke::new(0.5, with_alpha(Color32::BLACK, 0.15)));
    resp
}

/// "PASSPHRASE OR PASSWORD" → "Passphrase or password". Older labels were
/// written in capitals; this keeps their translations while showing them as
/// ordinary labels.
pub fn sentence_case(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A form or group label: sentence case, the body text colour.
pub fn label(p: &Painter, pos: Pos2, text: &str, theme: &Theme) {
    p.text(pos, Align2::LEFT_TOP, sentence_case(text), bold(13.0), theme.text);
}

/// Text truncated with an ellipsis to fit `max_w`.
pub fn truncate(painter: &Painter, text: &str, font: &FontId, max_w: f32) -> String {
    let w = |s: &str| painter.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE).size().x;
    if w(text) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "\u{2026}";
        if w(&s) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "\u{2026}"
}

// ---------------------------------------------------------------------------
// Vector icons

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Back,
    Forward,
    Gear,
    Folder,
    File,
    Disk,
    External,
    Server,
    Cloud,
    Refresh,
    Close,
    Trash,
    Check,
    Warning,
    Acorn,
    Download,
    Eye,
    Shield,
    Bolt,
    /// Link leaving the app (↗).
    LinkOut,
    Plus,
    Pencil,
    Search,
    /// Two overlapping sheets: duplicates.
    Copies,
    /// Chevron pointing down.
    Expand,
}

pub fn draw_icon(p: &Painter, icon: Icon, r: Rect, c: Color32) {
    let s = r.width().min(r.height());
    let w = (s / 11.0).max(1.3);
    let st = Stroke::new(w, c);
    let ctr = r.center();
    let at = |x: f32, y: f32| Pos2::new(ctr.x + x * s / 2.0, ctr.y + y * s / 2.0);
    match icon {
        Icon::Back => {
            p.line(vec![at(0.25, -0.6), at(-0.35, 0.0), at(0.25, 0.6)], st);
        }
        Icon::Forward => {
            p.line(vec![at(-0.25, -0.6), at(0.35, 0.0), at(-0.25, 0.6)], st);
        }
        Icon::Close => {
            p.line_segment([at(-0.5, -0.5), at(0.5, 0.5)], st);
            p.line_segment([at(0.5, -0.5), at(-0.5, 0.5)], st);
        }
        Icon::Plus => {
            p.line_segment([at(0.0, -0.6), at(0.0, 0.6)], st);
            p.line_segment([at(-0.6, 0.0), at(0.6, 0.0)], st);
        }
        Icon::Pencil => {
            p.line(vec![at(-0.6, 0.6), at(-0.6, 0.25), at(0.3, -0.65), at(0.65, -0.3), at(-0.25, 0.6), at(-0.6, 0.6)], st);
            p.line_segment([at(0.08, -0.43), at(0.43, -0.08)], st);
        }
        Icon::Check => {
            p.line(vec![at(-0.6, 0.0), at(-0.15, 0.45), at(0.65, -0.45)], Stroke::new(w * 1.3, c));
        }
        Icon::Gear => {
            let n = 8;
            let mut pts = Vec::new();
            for i in 0..n * 2 {
                let a = i as f32 / (n * 2) as f32 * TAU;
                let rr = if i % 2 == 0 { 0.82 } else { 0.62 };
                for k in [-0.18f32, 0.18] {
                    pts.push(at((a + k * 0.5).cos() * rr, (a + k * 0.5).sin() * rr));
                }
            }
            p.add(Shape::closed_line(pts, st));
            p.circle_stroke(ctr, s * 0.16, st);
        }
        Icon::Folder => {
            let pts = vec![at(-0.8, -0.55), at(-0.25, -0.55), at(-0.1, -0.38), at(0.8, -0.38), at(0.8, 0.6), at(-0.8, 0.6)];
            p.add(Shape::closed_line(pts, st));
            p.line_segment([at(-0.8, -0.2), at(0.8, -0.2)], st);
        }
        Icon::File => {
            let pts = vec![at(-0.55, -0.8), at(0.2, -0.8), at(0.6, -0.4), at(0.6, 0.8), at(-0.55, 0.8)];
            p.add(Shape::closed_line(pts, st));
            p.line(vec![at(0.2, -0.8), at(0.2, -0.4), at(0.6, -0.4)], st);
        }
        Icon::Disk | Icon::External => {
            let body = Rect::from_min_max(at(-0.8, -0.5), at(0.8, 0.5));
            p.rect_stroke(body, cr(s * 0.12), st, StrokeKind::Middle);
            p.line_segment([at(-0.8, 0.1), at(0.8, 0.1)], st);
            p.circle_filled(at(0.5, 0.3), w * 0.9, c);
            if icon == Icon::External {
                p.circle_filled(at(0.25, 0.3), w * 0.9, c);
            }
        }
        Icon::Server => {
            for y in [-0.45f32, 0.35] {
                let rr = Rect::from_min_max(at(-0.75, y - 0.3), at(0.75, y + 0.3));
                p.rect_stroke(rr, cr(s * 0.08), st, StrokeKind::Middle);
                p.circle_filled(at(0.45, y), w * 0.9, c);
            }
        }
        Icon::Cloud => {
            p.circle_stroke(at(-0.3, 0.1), s * 0.25, st);
            p.circle_stroke(at(0.15, -0.12), s * 0.32, st);
            p.circle_stroke(at(0.5, 0.18), s * 0.2, st);
            p.line_segment([at(-0.55, 0.48), at(0.6, 0.48)], st);
        }
        Icon::Refresh => {
            // Lucide "rotate-cw" geometry on a 24-unit grid.
            let g = |x: f32, y: f32| Pos2::new(ctr.x + (x - 12.0) * s / 24.0, ctr.y + (y - 12.0) * s / 24.0);
            let stw = Stroke::new((s / 12.0).max(1.4), c);
            let mut pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let a = TAU * 0.875 * i as f32 / 40.0;
                    g(12.0 + 9.0 * a.cos(), 12.0 + 9.0 * a.sin())
                })
                .collect();
            pts.push(g(21.0, 8.0));
            p.line(pts, stw);
            p.line(vec![g(21.0, 3.0), g(21.0, 8.0), g(16.0, 8.0)], stw);
        }
        Icon::Trash => {
            p.line_segment([at(-0.75, -0.5), at(0.75, -0.5)], st);
            p.line(vec![at(-0.25, -0.5), at(-0.2, -0.75), at(0.2, -0.75), at(0.25, -0.5)], st);
            p.add(Shape::closed_line(vec![at(-0.55, -0.5), at(0.55, -0.5), at(0.45, 0.8), at(-0.45, 0.8)], st));
            p.line_segment([at(-0.15, -0.2), at(-0.12, 0.5)], st);
            p.line_segment([at(0.15, -0.2), at(0.12, 0.5)], st);
        }
        Icon::Warning => {
            p.add(Shape::closed_line(vec![at(0.0, -0.8), at(0.85, 0.7), at(-0.85, 0.7)], st));
            p.line_segment([at(0.0, -0.25), at(0.0, 0.25)], st);
            p.circle_filled(at(0.0, 0.48), w * 0.8, c);
        }
        Icon::Download => {
            p.line_segment([at(0.0, -0.8), at(0.0, 0.3)], st);
            p.line(vec![at(-0.4, -0.1), at(0.0, 0.3), at(0.4, -0.1)], st);
            p.line(vec![at(-0.75, 0.4), at(-0.75, 0.75), at(0.75, 0.75), at(0.75, 0.4)], st);
        }
        Icon::Eye => {
            let top: Vec<Pos2> = (0..=16).map(|i| {
                let t = i as f32 / 16.0;
                at(-0.85 + 1.7 * t, -(t * PI).sin() * 0.5)
            }).collect();
            let bot: Vec<Pos2> = (0..=16).map(|i| {
                let t = i as f32 / 16.0;
                at(-0.85 + 1.7 * t, (t * PI).sin() * 0.5)
            }).collect();
            p.line(top, st);
            p.line(bot, st);
            p.circle_filled(ctr, s * 0.14, c);
        }
        Icon::Shield => {
            p.add(Shape::closed_line(vec![at(0.0, -0.85), at(0.7, -0.55), at(0.6, 0.2), at(0.0, 0.85), at(-0.6, 0.2), at(-0.7, -0.55)], st));
        }
        Icon::LinkOut => {
            p.line_segment([at(-0.55, 0.55), at(0.6, -0.6)], st);
            p.line(vec![at(-0.15, -0.6), at(0.6, -0.6), at(0.6, 0.15)], st);
        }
        Icon::Bolt => {
            p.add(Shape::convex_polygon(vec![at(0.15, -0.9), at(-0.5, 0.1), at(0.0, 0.1), at(-0.15, 0.9), at(0.5, -0.1), at(0.0, -0.1)], c, Stroke::NONE));
        }
        Icon::Search => {
            p.circle_stroke(at(-0.15, -0.15), s * 0.3, st);
            p.line_segment([at(0.28, 0.28), at(0.72, 0.72)], Stroke::new(w * 1.2, c));
        }
        Icon::Copies => {
            p.rect_stroke(Rect::from_min_max(at(-0.75, -0.35), at(0.3, 0.8)), cr(s * 0.08), st, StrokeKind::Middle);
            p.line(vec![at(-0.35, -0.55), at(-0.35, -0.8), at(0.75, -0.8), at(0.75, 0.4), at(0.5, 0.4)], st);
        }
        Icon::Expand => {
            p.line(vec![at(-0.55, -0.25), at(0.0, 0.3), at(0.55, -0.25)], st);
        }
        Icon::Acorn => acorn(p, ctr, s, c, None),
    }
}

/// Our mascot-ish acorn. With `fill` it is drawn in colour.
pub fn acorn(p: &Painter, ctr: Pos2, s: f32, stroke_c: Color32, fill: Option<(Color32, Color32)>) {
    let at = |x: f32, y: f32| Pos2::new(ctr.x + x * s / 2.0, ctr.y + y * s / 2.0);
    // nut body
    let body: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = i as f32 / 20.0 * PI;
            at(a.cos() * 0.58, -0.12 + a.sin() * 0.88)
        })
        .collect();
    // cap
    let cap: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = PI + i as f32 / 20.0 * PI;
            at(a.cos() * 0.78, -0.1 + a.sin() * 0.55)
        })
        .collect();
    match fill {
        Some((nut, capc)) => {
            p.add(Shape::convex_polygon(body.clone(), nut, Stroke::NONE));
            p.add(Shape::convex_polygon(cap.clone(), capc, Stroke::NONE));
            // cap texture
            for i in 0..4 {
                let x = -0.5 + i as f32 * 0.33;
                p.line_segment([at(x, -0.18), at(x + 0.12, -0.45)], Stroke::new(s * 0.03, with_alpha(Color32::BLACK, 0.2)));
            }
            p.circle_filled(at(-0.2, 0.3), s * 0.07, with_alpha(Color32::WHITE, 0.25));
            p.line_segment([at(0.0, -0.62), at(0.12, -0.85)], Stroke::new(s * 0.07, capc));
        }
        None => {
            let st = Stroke::new((s / 11.0).max(1.3), stroke_c);
            p.line(body, st);
            let mut c2 = cap;
            c2.push(c2[0]);
            p.line(c2, st);
            p.line_segment([at(0.0, -0.62), at(0.12, -0.85)], st);
        }
    }
}

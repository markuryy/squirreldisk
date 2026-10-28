//! Drag ghost and toasts.

use super::app::App;
use super::theme::{with_alpha, Theme};
use super::widgets::{self, bold, cr, font};
use crate::sound::Sfx;
use crate::i18n::tr;
use crate::tree::fmt_size;
use eframe::egui::{self, Align2, Color32, Id, LayerId, Order, Pos2, Rect, Shape, Stroke, Vec2};
use std::time::Instant;

pub struct Drag {
    pub session: usize,
    pub node: u32,
    pub label: String,
    pub size: u64,
    pub color: Color32,
    pub pos: Pos2,
    pub vel: Vec2,
    pub over_bin: bool,
    pub started: Instant,
}

// ---------------------------------------------------------------------------

struct Toast {
    text: String,
    color: Color32,
    born: Instant,
    y: f32,
}

#[derive(Default)]
pub struct Toasts {
    list: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, text: String, color: Color32) {
        self.list.push(Toast { text, color, born: Instant::now(), y: -1.0 });
        if self.list.len() > 4 {
            self.list.remove(0);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn draw(&mut self, ctx: &egui::Context, theme: &Theme, dt: f32) {
        self.list.retain(|t| t.born.elapsed().as_secs_f32() < 5.0);
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("toasts")));
        let mut y = screen.bottom() - 32.0;
        let k = 1.0 - (-dt * 14.0).exp();
        for t in self.list.iter_mut().rev() {
            let _ = ();
            let age = t.born.elapsed().as_secs_f32();
            let appear = (age / 0.25).min(1.0);
            let fade = ((5.0 - age) / 0.4).clamp(0.0, 1.0);
            let a = appear * fade;
            let galley = painter.layout_no_wrap(t.text.clone(), font(13.0), theme.text);
            let w = galley.size().x + 44.0;
            let h = 36.0;
            let target = y - h;
            if t.y < 0.0 {
                t.y = target + 30.0;
            }
            t.y += (target - t.y) * k;
            let rect = Rect::from_min_size(Pos2::new(screen.right() - w - 24.0, t.y), Vec2::new(w, h));
            widgets::shadow(&painter, rect, 8.0, a, theme);
            painter.rect_filled(rect, cr(8.0), with_alpha(theme.surface_hi, a));
            painter.rect_stroke(rect, cr(8.0), Stroke::new(1.0, with_alpha(t.color, 0.6 * a)), egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(rect.left() + 18.0, rect.center().y), 4.5, with_alpha(t.color, a));
            painter.galley(Pos2::new(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0), galley, with_alpha(theme.text, a));
            y = t.y - 8.0;
        }
    }
}

// ---------------------------------------------------------------------------

impl App {
    /// Updates and draws the drag ghost; handles dropping into the collector.
    pub fn drag_overlay(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.drag.take() else { return };
        let (pointer, down, released) =
            ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.pointer.primary_released() || !i.pointer.primary_down()));
        let target = pointer.unwrap_or(d.pos) + Vec2::new(14.0, 10.0);
        // springy follow
        let dt = self.dt;
        let acc = (target - d.pos) * 900.0 - d.vel * 60.0;
        d.vel += acc * dt;
        d.pos += d.vel * dt;
        let bin = self.bin_rect.expand(24.0);
        d.over_bin = pointer.is_some_and(|p| bin.contains(p));
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);

        let theme = self.theme.clone();
        let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("drag")));
        let tilt = (d.vel.x / 3000.0).clamp(-0.06, 0.06);
        let appear = (d.started.elapsed().as_secs_f32() / 0.15).min(1.0);
        let name_font = bold(13.0);
        let label = widgets::truncate(&painter, &d.label, &name_font, 260.0);
        let g1 = painter.layout_no_wrap(label, name_font, theme.text);
        let g2 = painter.layout_no_wrap(fmt_size(d.size), font(11.5), theme.text_dim);
        let w = g1.size().x.max(g2.size().x) + 52.0;
        let h = 44.0;
        let scale = 0.8 + 0.2 * appear + if d.over_bin { 0.06 } else { 0.0 };
        let rect = Rect::from_min_size(d.pos, Vec2::new(w, h) * scale);
        widgets::shadow(&painter, rect.translate(Vec2::new(0.0, 6.0)), 14.0, appear, &theme);
        // fake tilt by skewing the card via a polygon
        let sk = tilt * rect.height();
        let pts = vec![
            rect.left_top() + Vec2::new(sk, 0.0),
            rect.right_top() + Vec2::new(sk, 0.0),
            rect.right_bottom() - Vec2::new(sk, 0.0),
            rect.left_bottom() - Vec2::new(sk, 0.0),
        ];
        let fill = if d.over_bin { theme::mix_accent(&theme) } else { theme.surface_hi };
        painter.add(Shape::convex_polygon(pts, with_alpha(fill, 0.97 * appear), Stroke::new(1.5, with_alpha(d.color, appear))));
        painter.circle_filled(Pos2::new(rect.left() + 22.0, rect.center().y), 9.0 * scale, d.color);
        painter.galley(Pos2::new(rect.left() + 40.0, rect.top() + 6.0), g1, theme.text);
        painter.galley(Pos2::new(rect.left() + 40.0, rect.top() + 24.0), g2, theme.text_dim);
        if d.over_bin {
            painter.text(rect.center_top() - Vec2::new(0.0, 8.0), Align2::CENTER_BOTTOM, tr("Drop to collect"), bold(12.0), theme.warn);
        }

        if released && !down {
            if d.over_bin {
                let ok = self.sessions.get_mut(d.session).map(|s| s.collect(d.node)).unwrap_or(false);
                if ok {
                    self.sfx(Sfx::Plop);
                    self.bin_bounce = 1.0;
                } else {
                    self.toasts.push(tr("Already collected (or not deletable)").into(), theme.warn);
                }
            }
            return; // drag ends
        }
        self.drag = Some(d);
    }
}

pub mod theme {
    use super::super::theme::{lerp_color, Theme};
    use eframe::egui::Color32;
    pub fn mix_accent(t: &Theme) -> Color32 {
        lerp_color(t.surface_hi, t.warn, 0.35)
    }
}

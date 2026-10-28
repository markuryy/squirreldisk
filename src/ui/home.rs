//! Home screen: disks, recent scans and other sources.

use super::app::{App, Modal, Screen, Target};
use super::theme::{lerp_color, with_alpha};
use super::widgets::{self, bold, cr, font, BtnStyle, Icon};
use crate::i18n::{tr, tr_status, trf};
use crate::tree::fmt_size;
use crate::update::State as UpState;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::sync::atomic::Ordering;

pub const TITLEBAR_INSET: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };

impl App {
    /// Header shared by all screens; returns the remaining rect below it.
    pub fn window_drag_area(&self, ui: &mut Ui, rect: Rect) {
        let resp = ui.interact(rect, Id::new("window-drag").with(rect.min.x as i32), Sense::click_and_drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if resp.double_clicked() {
            let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
        }
    }

    pub fn home_ui(&mut self, ui: &mut Ui, screen: Rect) {
        let theme = self.theme.clone();
        let t = self.time;
        self.window_drag_area(ui, Rect::from_min_size(screen.min, Vec2::new(screen.width(), 90.0 + TITLEBAR_INSET)));


        let w = (screen.width() - 64.0).min(820.0);
        let x0 = screen.center().x - w / 2.0;
        // The page scrolls when the window is shorter than the content.
        let max_scroll = (self.home_content_h - screen.height()).max(0.0);
        if self.modal.is_none() && ui.rect_contains_pointer(screen) {
            self.home_scroll -= ui.input(|i| i.smooth_scroll_delta.y);
        }
        self.home_scroll = self.home_scroll.clamp(0.0, max_scroll);
        let page_top = screen.top() - self.home_scroll;
        let mut y = page_top + TITLEBAR_INSET + 24.0;

        // --- header
        {
            let p = ui.painter();
            let _ = t;
            let logo = Rect::from_min_size(Pos2::new(x0, y), Vec2::splat(36.0));
            let tex = self.logo.get_or_insert_with(|| {
                let px = 184;
                let pm = crate::icon::render(px as u32, crate::icon::Frame::IDLE, false);
                let img = egui::ColorImage::from_rgba_unmultiplied([px, px], &crate::icon::rgba(&pm));
                ui.ctx().load_texture("app-icon", img, egui::TextureOptions::LINEAR)
            });
            p.image(tex.id(), logo, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            p.text(Pos2::new(x0 + 46.0, logo.center().y), Align2::LEFT_CENTER, "SquirrelDisk", widgets::display(22.0), theme.text);
        }
        let gear = Rect::from_center_size(Pos2::new(x0 + w - 14.0, y + 18.0), Vec2::splat(30.0));
        if widgets::icon_button(ui, gear, Id::new("settings"), Icon::Gear, &theme, true).clicked() {
            self.open_modal(Modal::Settings);
        }
        self.update_pill(ui, screen, gear.right());
        y += 60.0;

        // --- disks, then other scans (folders, servers, cloud), as one grouped list
        let volumes = self.volumes.clone();
        let others: Vec<usize> = (0..self.sessions.len()).filter(|i| !matches!(self.sessions[*i].target, Target::Volume(_))).collect();
        let rows = volumes.len() + others.len();
        let list_h = if rows == 0 { 56.0 } else { volumes.len() as f32 * 76.0 + others.len() as f32 * 64.0 };
        let list = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, list_h));
        ui.painter().rect_filled(list, cr(10.0), theme.surface);
        if !theme.dark {
            ui.painter().rect_stroke(list, cr(10.0), Stroke::new(1.0, with_alpha(theme.stroke, 0.6)), egui::StrokeKind::Inside);
        }
        if rows == 0 {
            ui.painter().text(list.center(), Align2::CENTER_CENTER, tr("No disks found"), font(13.0), theme.text_faint);
        }
        let mut k = 0;
        let separator = |ui: &Ui, y: f32| {
            ui.painter().hline(x0 + 60.0..=x0 + w, y, Stroke::new(1.0, with_alpha(theme.stroke, 0.7)));
        };
        for (i, v) in volumes.iter().enumerate() {
            if k > 0 {
                separator(ui, y);
            }
            self.volume_card(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 76.0)), i, v);
            y += 76.0;
            k += 1;
        }
        for i in others {
            if k > 0 {
                separator(ui, y);
            }
            self.session_card(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 64.0)), i);
            y += 64.0;
            k += 1;
        }
        if rows == 0 {
            y += list_h;
        }

        // --- other things to scan, and the duplicate finder: plain buttons
        y += 20.0;
        let dupes_label = match self.dupes.as_ref().map(|d| d.is_scanning()) {
            Some(true) => tr("Comparing…").to_string(),
            _ => format!("{}…", tr("Find duplicates")),
        };
        let actions = [
            (format!("{}…", tr("Scan a folder")), 0usize),
            (format!("{}…", tr("Remote server")), 1),
            (format!("{}…", tr("Cloud storage")), 2),
            (dupes_label, 3),
        ];
        let mut x = x0;
        for (label, action) in actions {
            let bw = ui.painter().layout_no_wrap(label.clone(), font(13.0), theme.text).size().x + 28.0;
            let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bw, 28.0));
            x += bw + 8.0;
            if !widgets::button(ui, r, Id::new(("home-action", action)), &label, None, BtnStyle::Subtle, &theme).clicked() {
                continue;
            }
            match action {
                0 => {
                    if let Some(p) = rfd::FileDialog::new().set_title(tr("Choose a folder to scan")).pick_folder() {
                        self.start_session(Target::Folder(p));
                    }
                }
                1 => {
                    let host = self.settings.ssh_history.first().cloned().unwrap_or_default();
                    self.open_modal(Modal::Ssh { host, path: "/".into() });
                }
                2 => self.open_modal(Modal::Rclone { path: String::new(), confirm_remove: None }),
                _ => self.open_dupes(),
            }
        }
        y += 28.0;
        self.home_content_h = y + 32.0 - page_top;
        if max_scroll > 0.0 {
            let track = Rect::from_min_max(Pos2::new(screen.right() - 9.0, screen.top() + TITLEBAR_INSET + 8.0), Pos2::new(screen.right() - 4.0, screen.bottom() - 8.0));
            let thumb_h = (track.height() * screen.height() / self.home_content_h).max(32.0);
            let t = self.home_scroll / max_scroll;
            let thumb = Rect::from_min_size(Pos2::new(track.left(), track.top() + (track.height() - thumb_h) * t), Vec2::new(track.width(), thumb_h));
            let resp = ui.interact(thumb.expand2(Vec2::new(4.0, 0.0)), Id::new("home-scroll"), Sense::drag());
            if resp.dragged() {
                self.home_scroll = (self.home_scroll + resp.drag_delta().y * max_scroll / (track.height() - thumb_h).max(1.0)).clamp(0.0, max_scroll);
            }
            let h = ui.ctx().animate_bool_with_time(Id::new("home-scroll-h"), resp.hovered() || resp.dragged(), 0.12);
            let p = ui.painter();
            p.rect_filled(track, cr(3.0), with_alpha(theme.stroke, 0.35));
            p.rect_filled(thumb, cr(3.0), lerp_color(with_alpha(theme.text_faint, 0.8), theme.text_dim, h));
        }
    }

    /// The update button, as a ribbon hanging from the top edge of the
    /// window: it slides down when it appears. `right` is its right edge.
    pub fn update_pill(&mut self, ui: &mut Ui, screen: Rect, right: f32) {
        let theme = self.theme.clone();
        let state = self.updater.state();
        let id = Id::new("update-pill");
        let (label, style, icon) = match &state {
            UpState::Available(r) => (trf("Update to {version}", &[("version", &r.version)]), BtnStyle::Primary, Icon::Download),
            UpState::Downloading => {
                let d = self.updater.downloaded.load(Ordering::Relaxed);
                let tot = self.updater.total.load(Ordering::Relaxed).max(1);
                (trf("Downloading {percent}%", &[("percent", &(d * 100 / tot))]), BtnStyle::Subtle, Icon::Download)
            }
            UpState::Ready(_) => (tr("Restart to update").to_string(), BtnStyle::Primary, Icon::Refresh),
            _ => {
                ui.ctx().animate_bool_with_time(id.with("in"), false, 0.0);
                return;
            }
        };
        let shown = super::sunburst::ease(ui.ctx().animate_bool_with_time(id.with("in"), true, 0.45));
        let (w, h) = (ui.painter().layout_no_wrap(label.clone(), bold(13.0), Color32::WHITE).size().x + 50.0, 30.0);
        let rect = Rect::from_min_size(Pos2::new(right - w, screen.top() - h * (1.0 - shown)), Vec2::new(w, h));
        let corners = egui::CornerRadius { nw: 0, ne: 0, sw: 9, se: 9 };
        if widgets::button_shaped(ui, rect, id, &label, Some(icon), style, &theme, true, corners).clicked() {
            match state {
                UpState::Available(r) => self.updater.install(r, ui.ctx().clone()),
                UpState::Ready(_) => crate::update::restart(),
                _ => {}
            }
        }
    }

    fn volume_card(&mut self, ui: &mut Ui, rect: Rect, i: usize, v: &crate::disks::Volume) {
        let theme = self.theme.clone();
        let id = Id::new(("vol", &v.mount));
        let resp = ui.interact(rect, id, Sense::click());
        let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.15);
        let session = self.sessions.iter().position(|s| matches!(&s.target, Target::Volume(x) if x.mount == v.mount));
        let r = rect;
        {
            let p = ui.painter();
            let _ = h;
            let tile = Rect::from_min_size(r.min + Vec2::new(14.0, 16.0), Vec2::splat(44.0));
            widgets::draw_icon(p, if v.removable { Icon::External } else { Icon::Disk }, tile.shrink(6.0), theme.text_dim);
            // texts
            p.text(Pos2::new(tile.right() + 14.0, r.top() + 26.0), Align2::LEFT_CENTER, &v.name, bold(15.0), theme.text);
            let kind = if v.is_boot { tr("startup disk") } else if v.removable { tr("removable") } else { tr("volume") };
            let sub = format!("{} · {} · {} · {}", fmt_size(v.total), kind, v.fs, v.mount);
            let sub = widgets::truncate(p, &sub, &font(12.0), r.width() - 480.0);
            p.text(Pos2::new(tile.right() + 14.0, r.top() + 48.0), Align2::LEFT_CENTER, sub, font(12.0), theme.text_dim);
        }

        // right side: bar + button(s)
        let btn = Rect::from_min_size(Pos2::new(r.right() - 118.0, r.center().y - 16.0), Vec2::new(104.0, 32.0));
        // leave room for the small refresh / cancel button that sits left of `btn`
        let bar = Rect::from_min_size(Pos2::new(btn.left() - 48.0 - 190.0, r.center().y - 10.0), Vec2::new(190.0, 8.0));
        let p = ui.painter().clone();
        match session.map(|s| &self.sessions[s]) {
            Some(s) if s.is_scanning() => {
                let frac = s.progress.fraction();
                widgets::progress_bar(&p, bar, frac, &theme, self.time, None);
                let files = s.progress.files.load(Ordering::Relaxed);
                let txt = match frac {
                    Some(f) => trf("scanning · {percent}% · {files} files", &[("percent", &format!("{:.0}", f * 100.0)), ("files", &crate::tree::fmt_count_compact(files))]),
                    None => trf("scanning · {files} files", &[("files", &crate::tree::fmt_count_compact(files))]),
                };
                p.text(Pos2::new(bar.left(), bar.bottom() + 12.0), Align2::LEFT_CENTER, txt, font(11.5), theme.text_dim);
                if widgets::button(ui, btn, id.with("view"), tr("Watch"), None, BtnStyle::Subtle, &theme).clicked() {
                    self.screen = Screen::Session(session.unwrap());
                }
                let x = Rect::from_center_size(Pos2::new(btn.left() - 22.0, btn.center().y), Vec2::splat(28.0));
                if widgets::icon_button(ui, x, id.with("cancel"), Icon::Close, &theme, true).clicked() {
                    self.cancel_session(session.unwrap());
                }
            }
            other => {
                let used = v.used_frac();
                // colour only when the disk is running out of room
                let (bar_c, text_c) = if used > 0.95 { (theme.danger, theme.danger) } else if used > 0.9 { (theme.warn, theme.warn) } else { (theme.text_faint, theme.text_dim) };
                widgets::progress_bar(&p, bar, Some(used), &theme, 0.25, Some(bar_c));
                p.text(Pos2::new(bar.right(), bar.bottom() + 12.0), Align2::RIGHT_CENTER, trf("{size} free", &[("size", &fmt_size(v.available))]), font(12.0), text_c);
                p.text(Pos2::new(bar.left(), bar.bottom() + 12.0), Align2::LEFT_CENTER, trf("{size} used", &[("size", &fmt_size(v.used()))]), font(11.5), theme.text_faint);
                if other.is_some() {
                    if widgets::button(ui, btn, id.with("view"), tr("View"), None, BtnStyle::Subtle, &theme).clicked() || resp.clicked() {
                        self.screen = Screen::Session(session.unwrap());
                        self.sfx(crate::sound::Sfx::Blip);
                    }
                    let rr = Rect::from_center_size(Pos2::new(btn.left() - 22.0, btn.center().y), Vec2::splat(28.0));
                    if widgets::icon_button(ui, rr, id.with("rescan"), Icon::Refresh, &theme, true).clicked() {
                        self.start_session(Target::Volume(v.clone()));
                    }
                } else if {
                    self.mark(format!("scan:{}", v.mount), btn);
                    widgets::button(ui, btn, id.with("scan"), tr("Scan"), None, BtnStyle::Primary, &theme).clicked() || resp.clicked()
                } {
                    self.start_session(Target::Volume(v.clone()));
                }
            }
        }
        let _ = i;
    }

    fn session_card(&mut self, ui: &mut Ui, rect: Rect, i: usize) {
        let theme = self.theme.clone();
        let id = Id::new(("sess", i));
        let resp = ui.interact(rect, id, Sense::click());
        let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.15);
        let s = &self.sessions[i];
        let (icon, kind) = match &s.target {
            Target::Folder(_) => (Icon::Folder, tr("folder")),
            Target::Ssh { .. } => (Icon::Server, tr("server")),
            Target::Rclone(_) => (Icon::Cloud, tr("cloud")),
            Target::Volume(_) => (Icon::Disk, tr("disk")),
        };
        let size = {
            let t = s.tree.read().unwrap();
            t.get(t.root).size
        };
        let status = if let Some(e) = s.progress.error.lock().unwrap().clone() {
            trf("failed: {error}", &[("error", &e)])
        } else if s.is_scanning() {
            let st = s.progress.status.lock().unwrap().clone();
            trf("{status}… {files} files", &[("status", &tr_status(&st)), ("files", &crate::tree::fmt_count_compact(s.progress.files.load(Ordering::Relaxed)))])
        } else {
            format!("{} · {}", kind, fmt_size(size))
        };
        let scanning = s.is_scanning();
        let title = s.title.clone();
        {
            let p = ui.painter();
            let r = rect;
            let _ = h;
            let tile = Rect::from_min_size(r.min + Vec2::new(14.0, 12.0), Vec2::splat(40.0));
            widgets::draw_icon(p, icon, tile.shrink(8.0), theme.text_dim);
            let t2 = widgets::truncate(p, &title, &bold(14.0), r.width() - 260.0);
            p.text(Pos2::new(tile.right() + 12.0, r.top() + 22.0), Align2::LEFT_CENTER, t2, bold(14.0), theme.text);
            let st = widgets::truncate(p, &status, &font(12.0), r.width() - 260.0);
            p.text(Pos2::new(tile.right() + 12.0, r.top() + 42.0), Align2::LEFT_CENTER, st, font(12.0), theme.text_dim);
        }
        let btn = Rect::from_min_size(Pos2::new(rect.right() - 118.0, rect.center().y - 16.0), Vec2::new(104.0, 32.0));
        if widgets::button(ui, btn, id.with("v"), if scanning { tr("Watch") } else { tr("View") }, None, BtnStyle::Subtle, &theme).clicked() || resp.clicked() {
            self.screen = Screen::Session(i);
        }
        let x = Rect::from_center_size(Pos2::new(btn.left() - 22.0, btn.center().y), Vec2::splat(28.0));
        if widgets::icon_button(ui, x, id.with("x"), Icon::Close, &theme, true).clicked() {
            self.cancel_session(i);
        }
    }
}


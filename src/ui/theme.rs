use eframe::egui::Color32;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    /// Window background (top / bottom of a vertical gradient).
    pub bg_top: Color32,
    pub bg_bottom: Color32,
    /// Cards, list hover, inputs.
    pub surface: Color32,
    pub surface_hi: Color32,
    pub stroke: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    pub danger: Color32,
    pub ok: Color32,
    pub warn: Color32,
    /// Chart colours, picked by angle. A single entry: the chart is one hue,
    /// since a colour per direction would mean nothing.
    pub wheel: &'static [[u8; 3]],
    /// Colour of aggregates ("small files", hidden space).
    pub neutral: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Follows the system appearance; resolved to "Light" or "Dark" each frame.
pub const AUTOMATIC: &str = "Automatic";

/// Choices offered in Settings, in order.
pub const CHOICES: [&str; 3] = [AUTOMATIC, "Light", "Dark"];

// Apple system colours (macOS light / dark variants).
pub fn all() -> Vec<Theme> {
    vec![
        Theme {
            name: "Light",
            dark: false,
            bg_top: c(0xf5, 0xf5, 0xf7),
            bg_bottom: c(0xf5, 0xf5, 0xf7),
            surface: c(0xff, 0xff, 0xff),
            surface_hi: c(0xe8, 0xe8, 0xed),
            stroke: c(0xd2, 0xd2, 0xd7),
            text: c(0x1d, 0x1d, 0x1f),
            text_dim: c(0x6e, 0x6e, 0x73),
            text_faint: c(0xa1, 0xa1, 0xa6),
            accent: c(0x00, 0x71, 0xe3),
            accent2: c(0x00, 0x71, 0xe3),
            danger: c(0xff, 0x3b, 0x30),
            ok: c(0x34, 0xc7, 0x59),
            warn: c(0xff, 0x95, 0x00),
            wheel: &[[0x00, 0x71, 0xe3]],
            neutral: c(0xc7, 0xc7, 0xcc),
        },
        Theme {
            name: "Dark",
            dark: true,
            bg_top: c(0x1c, 0x1c, 0x1e),
            bg_bottom: c(0x1c, 0x1c, 0x1e),
            surface: c(0x2c, 0x2c, 0x2e),
            surface_hi: c(0x3a, 0x3a, 0x3c),
            stroke: c(0x3d, 0x3d, 0x40),
            text: c(0xf5, 0xf5, 0xf7),
            text_dim: c(0xa1, 0xa1, 0xa6),
            text_faint: c(0x6e, 0x6e, 0x73),
            accent: c(0x0a, 0x84, 0xff),
            accent2: c(0x0a, 0x84, 0xff),
            danger: c(0xff, 0x45, 0x3a),
            ok: c(0x30, 0xd1, 0x58),
            warn: c(0xff, 0x9f, 0x0a),
            wheel: &[[0x0a, 0x84, 0xff]],
            neutral: c(0x48, 0x48, 0x4a),
        },
    ]
}

pub fn by_name(name: &str) -> Theme {
    let v = all();
    v.iter().find(|t| t.name == name).cloned().unwrap_or_else(|| v[0].clone())
}

/// The theme for a Settings choice. Unknown names (themes from older
/// versions) follow the system too.
pub fn resolve(choice: &str, system_dark: bool) -> Theme {
    match choice {
        "Light" | "Dark" => by_name(choice),
        _ => by_name(if system_dark { "Dark" } else { "Light" }),
    }
}

// ---------------------------------------------------------------------------
// colour helpers (HSL in linear-ish sRGB space, good enough for UI work)

pub fn to_hsl(c: Color32) -> (f32, f32, f32) {
    let r = c.r() as f32 / 255.0;
    let g = c.g() as f32 / 255.0;
    let b = c.b() as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

pub fn from_hsl(h: f32, s: f32, l: f32) -> Color32 {
    let hue = |p: f32, q: f32, mut t: f32| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    if s == 0.0 {
        let v = (l * 255.0) as u8;
        return Color32::from_rgb(v, v, v);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let r = hue(p, q, h + 1.0 / 3.0);
    let g = hue(p, q, h);
    let b = hue(p, q, h - 1.0 / 3.0);
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * c.a() as f32) as u8)
}

pub fn lighten(c: Color32, amount: f32) -> Color32 {
    let (h, s, l) = to_hsl(c);
    from_hsl(h, s, l + amount)
}

impl Theme {
    /// Colour of the wheel gradient at `t` in [0,1) (cyclic).
    pub fn wheel_at(&self, t: f32) -> Color32 {
        let n = self.wheel.len();
        let x = t.rem_euclid(1.0) * n as f32;
        let i = x.floor() as usize % n;
        let j = (i + 1) % n;
        let f = x.fract();
        let a = self.wheel[i];
        let b = self.wheel[j];
        lerp_color(c(a[0], a[1], a[2]), c(b[0], b[1], b[2]), f)
    }

    /// Aggregates ("smaller items", "smaller files"): a shade just darker than the background.
    pub fn aggregate_color(&self) -> Color32 {
        let bg = lerp_color(self.bg_top, self.bg_bottom, 0.5);
        lerp_color(bg, Color32::BLACK, if self.dark { 0.14 } else { 0.07 })
    }

    /// Final colour of a segment: the chart hue, fading toward the
    /// background ring by ring (files a step further), so depth is the only
    /// thing colour says. Neighbours are told apart by the gaps between them.
    pub fn segment_color(&self, mid: f32, depth: usize, is_file: bool) -> Color32 {
        let base = self.wheel_at(mid);
        let bg = self.bg_top;
        let d = depth.saturating_sub(1) as f32 + if is_file { 0.6 } else { 0.0 };
        let fade = if self.dark { 0.13 } else { 0.11 };
        lerp_color(base, bg, (d * fade).min(0.72))
    }
}

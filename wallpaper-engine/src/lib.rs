use std::f64::consts::PI;
#[link(wasm_import_module = "draw")]
extern "C" {
    fn color(r: f64, g: f64, b: f64, a: f64);
    fn rect(x: f64, y: f64, w: f64, h: f64);
    fn circle(x: f64, y: f64, r: f64);
    fn begin();
    fn point(x: f64, y: f64);
    fn fill();
    fn stroke(width: f64);
}
static mut SETTINGS: [f64; 3] = [0.0; 3];
#[no_mangle]
pub extern "C" fn set_prop(i: usize, v: f64) {
    if i < 3 {
        unsafe {
            SETTINGS[i] = v;
        }
    }
}
fn col(r: f64, g: f64, b: f64, a: f64) {
    unsafe { color(r, g, b, a) }
}
fn box_(x: f64, y: f64, w: f64, h: f64) {
    unsafe { rect(x, y, w, h) }
}
fn dot(x: f64, y: f64, r: f64) {
    unsafe { circle(x, y, r) }
}
fn start() {
    unsafe { begin() }
}
fn pt(x: f64, y: f64) {
    unsafe { point(x, y) }
}
fn end() {
    unsafe { fill() }
}
fn line(w: f64) {
    unsafe { stroke(w) }
}
fn hash(n: f64) -> f64 {
    let x = (n * 127.1 + 311.7).sin() * 43758.5453;
    x - x.floor()
}
fn bg(w: f64, h: f64, r: f64, g: f64, b: f64) {
    col(r, g, b, 1.);
    box_(0., 0., w, h)
}
fn smooth(x: f64) -> f64 {
    x.clamp(0., 1.).powi(2)
}
#[no_mangle]
pub extern "C" fn frame(scene: u32, w: f64, h: f64, t: f64, mx: f64, my: f64) {
    let p = unsafe { SETTINGS };
    if w <= 0. || h <= 0. {
        return;
    }
    let t = t;
    match scene {
        // A moonlit field with distant trees and drifting lights.
        0 => {
            bg(w, h, 12., 25., 23.);
            // Low-key atmosphere, with a diffused moon and distant stars.
            for band in 0..24 {
                let q = band as f64 / 24.;
                col(14. + q * 5., 28. + q * 8., 27. + q * 5., 1.);
                box_(0., h * q * 0.72, w, h * 0.031);
            }
            for i in 0..80 {
                let i = i as f64;
                let x = hash(i + 203.) * w;
                let y = hash(i + 903.) * h * 0.47;
                col(195., 208., 175., 0.1 + hash(i + 109.) * 0.18);
                dot(x, y, 0.35 + hash(i + 56.) * 0.6);
            }
            for j in (1..=18).rev() {
                col(137., 160., 121., 0.008);
                dot(w * 0.74, h * 0.23, h * (0.032 + j as f64 * 0.004));
            }
            col(177., 186., 143., 0.38);
            dot(w * 0.74, h * 0.23, h * 0.027);
            // Receding treeline gives the small lights a sense of place.
            for i in 0..60 {
                let x = i as f64 * w / 59.;
                let height = h * (0.035 + hash(i as f64 + 220.) * 0.11);
                col(11., 29., 24., 0.78);
                start();
                pt(x - w * 0.016, h * 0.69);
                pt(x, h * 0.69 - height);
                pt(x + w * 0.016, h * 0.69);
                end();
            }
            for layer in 0..3 {
                let k = layer as f64;
                col(15. + k * 4., 32. + k * 5., 26. + k * 4., 1.);
                start();
                pt(0., h);
                for j in 0..=80 {
                    let x = w * j as f64 / 80.;
                    let y = h * (0.68 + k * 0.10)
                        + (x * 0.004 + k * 3.).sin() * h * 0.025
                        + (x * 0.012 + k).sin() * h * 0.008;
                    pt(x, y)
                }
                pt(w, h);
                end()
            }
            for i in 0..p[0] as usize {
                let i = i as f64;
                let x = (hash(i + 3.) * w + (t * 0.9 + i).sin() * w * 0.012).rem_euclid(w);
                let y = hash(i + 81.) * h * 0.79 + (t * 0.45 + i * 4.).cos() * h * 0.018;
                let d = ((x - mx).powi(2) + (y - my).powi(2)).sqrt();
                let x = x + if d < 130. {
                    (x - mx) * 0.22 * (1. - d / 130.)
                } else {
                    0.
                };
                let a = 0.15 + 0.85 * smooth((1. + (t * 1.3 + hash(i + 9.) * 7.).sin()) * 0.5);
                let r = 1. + hash(i + 23.) * 1.5;
                if p[2] > 0.5 {
                    for j in (1..=4).rev() {
                        let s = j as f64;
                        col(226., 213., 118., a * 0.018);
                        dot(x, y, r + s * 3.)
                    }
                }
                col(255., 235., 157., a);
                dot(x, y, r)
            }
            // Sparse, swaying meadow silhouettes anchor the foreground.
            for i in 0..90 {
                let i = i as f64;
                let x = hash(i + 1200.) * w;
                let y = h * (0.91 + hash(i + 1250.) * 0.11);
                let blade = h * (0.018 + hash(i + 1300.) * 0.05);
                col(8., 22., 19., 0.38);
                start();
                pt(x, y);
                pt(
                    x + (t * 0.4 + i).sin() * blade * 0.13 + blade * 0.18,
                    y - blade,
                );
                line(1.);
            }
        }
        // Looking down at waves along a sandy shore.
        1 => {
            bg(
                w,
                h,
                if p[2] > 0.5 { 195. } else { 160. },
                if p[2] > 0.5 { 181. } else { 177. },
                if p[2] > 0.5 { 151. } else { 165. },
            );
            // Sand grain remains subdued beneath the desktop icons.
            for i in 0..260 {
                let i = i as f64;
                col(100., 99., 82., 0.10);
                dot(
                    hash(i + 51.) * w,
                    h * (0.68 + hash(i + 91.) * 0.32),
                    0.45 + hash(i + 101.) * 0.45,
                );
            }
            let cols = [
                (23., 71., 73.),
                (35., 93., 91.),
                (52., 119., 110.),
                (87., 148., 135.),
                (137., 178., 160.),
            ];
            for (k, c) in cols.iter().enumerate().rev() {
                let k = k as f64;
                let edge = |x: f64| {
                    h * (0.37 + k * 0.095)
                        + (x * 0.003 + k * 1.3 + t * 0.25).sin() * h * 0.032
                        + (x * 0.009 - k + t * 0.18).sin() * h * 0.009
                        + if mx >= 0. {
                            (-((x - mx) / 150.).powi(2)).exp() * (t * 1.4).sin() * 8.
                        } else {
                            0.
                        }
                };
                col(c.0, c.1, c.2, 1.);
                start();
                pt(0., 0.);
                pt(w, 0.);
                for j in (0..=100).rev() {
                    let x = w * j as f64 / 100.;
                    pt(x, edge(x))
                }
                end();
                if p[0] > 0.5 {
                    col(235., 239., 216., 0.48);
                    start();
                    for j in 0..=100 {
                        let x = w * j as f64 / 100.;
                        pt(x, edge(x))
                    }
                    line(1.5);
                    // Broken, offset foam traces soften the illustrated shore.
                    for seg in 0..24 {
                        let f = seg as f64;
                        let x = (hash(f + k * 31. + 502.) * w + t * (2. + k)).rem_euclid(w);
                        let length = 8. + hash(f + 31.) * 42.;
                        let inset = 5. + hash(f + 71.) * 18.;
                        col(239., 241., 218., 0.12 + hash(f + 12.) * 0.15);
                        start();
                        pt(x, edge(x) - inset);
                        pt((x + length).min(w), edge((x + length).min(w)) - inset);
                        line(0.8);
                    }
                }
            }
            // Tiny distant glints across the deeper water.
            for i in 0..65 {
                let i = i as f64;
                let x = hash(i + 600.) * w;
                let y = hash(i + 700.) * h * 0.3;
                let flicker = (0.5 + 0.5 * (t * 0.7 + i).sin()).powi(3);
                col(191., 218., 200., 0.08 * flicker);
                box_(x, y, 2. + hash(i + 800.) * 6., 0.8);
            }
        }
        // Fine textile-like threads, with a restrained copper accent.
        2 => {
            bg(w, h, 22., 26., 25.);
            // Very faint warp texture creates depth without competing with the weft.
            for j in 0..130 {
                let x = j as f64 * w / 129.;
                col(181., 192., 174., 0.035);
                start();
                pt(x, 0.);
                pt(x, h);
                line(0.65);
            }
            let n = p[0].max(1.) as usize;
            for i in 0..n {
                let i = i as f64;
                let y = (i + 0.5) * h / n as f64;
                if p[2] > 0.5 && (i as usize) % 7 == 0 {
                    col(197., 143., 107., 0.58)
                } else {
                    col(178., 191., 177., 0.33)
                }
                start();
                for j in 0..=90 {
                    let x = w * j as f64 / 90.;
                    let bend = (x * 0.004 + t * 0.18 + i * 0.06).sin() * h * 0.026
                        + (x * 0.0015 - t * 0.11 + i * 0.15).sin() * h * 0.018;
                    let d = if mx >= 0. {
                        (-((x - mx).powi(2) + (y - my).powi(2)) / 28000.).exp()
                            * 20.
                            * (t + i * 0.08).sin()
                    } else {
                        0.
                    };
                    pt(x, y + bend + d)
                }
                line(if i as usize % 7 == 0 { 1.2 } else { 0.7 });
            }
            // A few soft catches of light on the woven surface.
            for i in 0..45 {
                let i = i as f64;
                let x = hash(i + 2100.) * w;
                let y = hash(i + 2150.) * h;
                let shimmer = 0.5 + 0.5 * (t * 0.22 + i).sin();
                col(213., 205., 174., 0.06 * shimmer);
                box_(x, y, 2. + hash(i + 2200.) * 5., 0.8);
            }
        }
        // Twilight dunes, layered atmosphere, and wind-drawn sand.
        3 => {
            bg(w, h, 29., 32., 41.);
            // Soft dusk bands behind the horizon, laid down in broad translucent washes.
            for j in 0..48 {
                let y = j as f64 / 48.;
                col(133., 100., 91., 0.004 + y * 0.018);
                box_(0., y * h * 0.59, w, h * 0.59 / 48. + 1.);
            }
            let sx = w * 0.72;
            let sy = h * 0.26;
            let sr = h.min(w) * 0.064;
            for j in (1..=22).rev() {
                let q = j as f64 / 22.;
                col(228., 165., 103., 0.009 * (1. - q * 0.6));
                dot(sx, sy, sr * (1. + q * 1.65));
            }
            col(239., 192., 139., 0.92);
            dot(sx, sy, sr);
            let cs = [
                (76., 73., 77.),
                (111., 91., 86.),
                (145., 109., 94.),
                (180., 133., 106.),
                (211., 163., 125.),
            ];
            for (k, c) in cs.iter().enumerate() {
                let k = k as f64;
                let ridge = |x: f64| {
                    h * (0.46 + k * 0.115)
                        + (x * 0.002 + k * 1.7 + t * 0.018).sin() * h * 0.055
                        + (x * 0.005 + k).sin() * h * 0.015
                };
                col(c.0, c.1, c.2, 1.);
                start();
                pt(0., h);
                for j in 0..=100 {
                    let x = w * j as f64 / 100.;
                    pt(x, ridge(x));
                }
                pt(w, h);
                end();
                // Glancing light rides each crest; neighboring fine lines suggest sand.
                for band in 0..4 {
                    col(247., 208., 161., 0.12 / (band as f64 + 1.));
                    start();
                    for j in 0..=100 {
                        let x = w * j as f64 / 100.;
                        pt(
                            x,
                            ridge(x)
                                + band as f64 * (4. + k * 1.3)
                                + (x * 0.018 + k).sin() * band as f64 * 0.45,
                        );
                    }
                    line(if band == 0 { 1.1 } else { 0.6 });
                }
            }
            if p[0] > 0.5 {
                for i in 0..85 {
                    let i = i as f64;
                    let x = (hash(i + 1.) * w + t * (2. + hash(i + 90.) * 5.)).rem_euclid(w);
                    let y = hash(i + 71.) * h * 0.8 + h * 0.18 + (t * 0.25 + i).sin() * 3.;
                    col(250., 218., 170., 0.12 + hash(i + 32.) * 0.18);
                    start();
                    pt(x, y);
                    pt(x + 2. + hash(i + 20.) * 5., y);
                    line(0.6);
                }
            }
        }
        // Totality: an uneven, living corona against a fine starfield.
        4 => {
            bg(w, h, 8., 11., 19.);
            for i in 0..170 {
                let i = i as f64;
                let twinkle = 0.65 + 0.35 * (t * 0.35 + hash(i + 44.) * 8.).sin();
                col(211., 220., 225., (0.13 + hash(i + 9.) * 0.42) * twinkle);
                dot(
                    hash(i + 1.) * w,
                    hash(i + 300.) * h,
                    0.45 + hash(i + 19.) * 0.8,
                );
            }
            let cx = w * 0.5;
            let cy = h * 0.5;
            let r = h.min(w) * 0.18;
            // Many translucent disks make a warm halo with a long feathered falloff.
            for j in (1..=56).rev() {
                let q = j as f64 / 56.;
                col(
                    211.,
                    137. + q * 20.,
                    91. + q * 12.,
                    0.009 * (1. - q * 0.45) * p[0],
                );
                dot(cx, cy, r * (1. + q * 1.6));
            }
            // Fine curved magnetic streamers in place of regular radial spokes.
            for i in 0..240 {
                let i = i as f64;
                let a = i * PI * 2. / 240.;
                let field = (a * 5. + t * 0.13).sin() * 0.5 + (a * 11. - t * 0.08).sin() * 0.25;
                let length = 0.12 + 0.35 * hash(i + 73.) + 0.14 * field.abs();
                col(
                    244.,
                    178. + hash(i + 4.) * 35.,
                    129.,
                    (0.06 + hash(i + 6.) * 0.11) * p[0],
                );
                start();
                for j in 0..=7 {
                    let q = j as f64 / 7.;
                    let angle = a + field * q * q * 0.23;
                    let rad = r * (0.98 + q * length);
                    pt(cx + angle.cos() * rad, cy + angle.sin() * rad);
                }
                line(0.55 + hash(i + 21.) * 0.65);
            }
            col(7., 10., 16., 1.);
            dot(cx, cy, r);
            for j in 0..3 {
                col(250., 196., 148., 0.48 / (j as f64 + 1.));
                start();
                for i in 0..=180 {
                    let a = i as f64 * PI * 2. / 180.;
                    let rad = r + j as f64 * 0.75;
                    pt(cx + a.cos() * rad, cy + a.sin() * rad);
                }
                line(0.8);
            }
        }
        // Rain-streaked glass over a softened, distant city at night.
        5 => {
            bg(w, h, 14., 23., 31.);
            // Cool haze and an understated silhouette establish depth behind the glass.
            for j in 0..35 {
                let y = j as f64 / 35.;
                col(39., 69., 78., 0.012 + y * 0.01);
                box_(0., h * (0.36 + y * 0.64), w, h * 0.64 / 35. + 1.);
            }
            col(9., 17., 24., 0.48);
            start();
            pt(0., h);
            for i in 0..=40 {
                let x = w * i as f64 / 40.;
                let level = 0.61 + hash(i as f64 + 201.) * 0.21;
                pt(x, h * level);
                pt(x + w / 40., h * level);
            }
            pt(w, h);
            end();
            for i in 0..55 {
                let i = i as f64;
                let x = hash(i + 20.) * w;
                let y = h * (0.36 + hash(i + 39.) * 0.56);
                let r = 5. + hash(i + 2.) * 24.;
                let warm = hash(i + 55.) > 0.45;
                for j in (1..=9).rev() {
                    let q = j as f64 / 9.;
                    if warm {
                        col(239., 165., 104., 0.015)
                    } else {
                        col(123., 186., 191., 0.014)
                    }
                    dot(x, y, r * q);
                }
            }
            let count = p[0] as usize;
            for i in 0..count {
                let i = i as f64;
                let x = hash(i + 9.) * w;
                let speed = 12. + hash(i + 25.) * 35.;
                let y = (hash(i + 76.) * h + t * speed).rem_euclid(h + 120.) - 60.;
                let len = 10. + hash(i + 8.) * 40.;
                let bend = 1. + hash(i + 51.) * 3.;
                col(176., 206., 207., 0.10 + hash(i + 6.) * 0.16);
                start();
                pt(x, y);
                pt(x - bend * 0.35, y + len * 0.45);
                pt(x - bend, y + len);
                line(0.7 + hash(i + 2.) * 0.9);
                col(213., 230., 224., 0.26);
                dot(x - bend, y + len, 0.8 + hash(i + 87.) * 1.2);
            }
            // Static beads on the pane, with restrained reflected highlights.
            for i in 0..85 {
                let i = i as f64;
                let x = hash(i + 391.) * w;
                let y = hash(i + 490.) * h;
                let r = 0.8 + hash(i + 591.) * 2.;
                col(120., 165., 173., 0.12);
                dot(x, y, r + 0.8);
                col(221., 233., 221., 0.24);
                dot(x - r * 0.25, y - r * 0.3, r * 0.35);
            }
        }
        _ => (),
    }
}

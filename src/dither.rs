//! Ordered (Bayer) dithering core. No After Effects dependencies, so it can be
//! tested on its own.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Depth {
    U8,
    U16, // After Effects "16 bpc" = 0..32768
    F32,
}

impl Depth {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Depth::U8 => 4,
            Depth::U16 => 8,
            Depth::F32 => 16,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Matrix side: 2, 4, 8 or 16.
    pub matrix: usize,
    /// Cell size in buffer pixels (already scaled for preview resolution).
    pub cell_x: usize,
    pub cell_y: usize,
    /// Output levels per channel, >= 2.
    pub levels: u32,
    /// 1.0 = unchanged.
    pub brightness: f32,
    /// 1.0 = unchanged.
    pub contrast: f32,
    pub mono: bool,
    /// Mono foreground colour, 0..1 RGB.
    pub mono_color: [f32; 3],
    /// 0..1
    pub mono_opacity: f32,
    /// Mono background colour, 0..1 RGB.
    pub background: [f32; 3],
    pub transparent_bg: bool,
}

/// Bayer threshold matrix of side `n` (power of two), values 0..n*n-1, row-major.
pub fn bayer(n: usize) -> Vec<u32> {
    let mut m = vec![0u32];
    let mut size = 1;
    while size < n {
        let s2 = size * 2;
        let mut next = vec![0u32; s2 * s2];
        for y in 0..size {
            for x in 0..size {
                let v = 4 * m[y * size + x];
                next[y * s2 + x] = v;
                next[y * s2 + x + size] = v + 2;
                next[(y + size) * s2 + x] = v + 3;
                next[(y + size) * s2 + x + size] = v + 1;
            }
        }
        m = next;
        size = s2;
    }
    m
}

/// A view on a pixel buffer. Pixels are ARGB in After Effects channel order.
pub struct World<'a> {
    pub data: &'a [u8],
    pub stride: usize,
    pub width: usize,
    pub height: usize,
    /// Position of this buffer's (0,0) in layer coordinates.
    pub origin: (i32, i32),
}

pub struct WorldMut<'a> {
    pub data: &'a mut [u8],
    pub stride: usize,
    pub width: usize,
    pub height: usize,
    pub origin: (i32, i32),
}

#[inline]
fn read_px(data: &[u8], off: usize, depth: Depth) -> [f32; 4] {
    match depth {
        Depth::U8 => {
            let p = &data[off..off + 4];
            [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
        }
        Depth::U16 => {
            let c = |i: usize| u16::from_ne_bytes([data[off + 2 * i], data[off + 2 * i + 1]]) as f32 / 32768.0;
            [c(0), c(1), c(2), c(3)]
        }
        Depth::F32 => {
            let c = |i: usize| {
                let b = off + 4 * i;
                f32::from_ne_bytes([data[b], data[b + 1], data[b + 2], data[b + 3]])
            };
            [c(0), c(1), c(2), c(3)]
        }
    }
}

#[inline]
fn encode_px(px: [f32; 4], depth: Depth, out: &mut [u8; 16]) -> usize {
    match depth {
        Depth::U8 => {
            for i in 0..4 {
                out[i] = (px[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
            4
        }
        Depth::U16 => {
            for i in 0..4 {
                let v = (px[i].clamp(0.0, 1.0) * 32768.0 + 0.5) as u16;
                out[2 * i..2 * i + 2].copy_from_slice(&v.to_ne_bytes());
            }
            8
        }
        Depth::F32 => {
            for i in 0..4 {
                out[4 * i..4 * i + 4].copy_from_slice(&px[i].to_ne_bytes());
            }
            16
        }
    }
}

/// Quantise one value with ordered dither. `t` is the threshold in (0,1).
#[inline]
fn quantise(v: f32, levels: u32, t: f32) -> f32 {
    let steps = (levels - 1) as f32;
    let v = v.clamp(0.0, 1.0) * steps;
    ((v + t).floor().min(steps)) / steps
}

/// Dither `src` into `dst`. Cells are aligned to layer coordinates, so the
/// pattern does not swim when only part of the layer is rendered.
pub fn process(src: &World, dst: &mut WorldMut, depth: Depth, s: &Settings) {
    let n = s.matrix.max(1);
    let matrix = bayer(n);
    let nn = (n * n) as f32;
    let cx = s.cell_x.max(1) as i32;
    let cy = s.cell_y.max(1) as i32;
    let bpp = depth.bytes_per_pixel();
    let levels = s.levels.max(2);

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(32);

    // Cell rows covering the output, in layer coordinates.
    let y0 = dst.origin.1;
    let y1 = dst.origin.1 + dst.height as i32;
    let first_row = y0.div_euclid(cy);
    let last_row = (y1 - 1).div_euclid(cy);
    let cell_rows = (last_row - first_row + 1).max(0) as usize;
    if dst.width == 0 || dst.height == 0 || cell_rows == 0 {
        return;
    }

    // Split destination into bands of whole cell rows, one band per thread.
    let rows_per_band = cell_rows.div_ceil(threads).max(1);
    let mut bands: Vec<(i32, &mut [u8], usize)> = Vec::new(); // (first dst y, slice, line count)
    let mut rest: &mut [u8] = &mut dst.data[..];
    let mut consumed_lines = 0usize;
    let mut row = first_row;
    while row <= last_row {
        let band_end_row = (row + rows_per_band as i32 - 1).min(last_row);
        let band_y_end = ((band_end_row + 1) * cy).min(y1);
        let lines = (band_y_end - y0) as usize - consumed_lines;
        let r = std::mem::take(&mut rest);
        let take = if consumed_lines + lines == dst.height { r.len() } else { (lines * dst.stride).min(r.len()) };
        let (band, tail) = r.split_at_mut(take);
        bands.push((y0 + consumed_lines as i32, band, lines));
        rest = tail;
        consumed_lines += lines;
        row = band_end_row + 1;
    }

    let dst_w = dst.width;
    let dst_ox = dst.origin.0;
    let dst_stride = dst.stride;

    std::thread::scope(|scope| {
        for (band_y, band, lines) in bands {
            let matrix = &matrix;
            scope.spawn(move || {
                let band_y_end = band_y + lines as i32;
                let mut row = band_y.div_euclid(cy);
                let x_first = dst_ox.div_euclid(cx);
                let x_last = (dst_ox + dst_w as i32 - 1).div_euclid(cx);
                let mut enc = [0u8; 16];
                while row * cy < band_y_end {
                    let cell_y0 = row * cy;
                    let cell_y1 = cell_y0 + cy;
                    for col in x_first..=x_last {
                        let cell_x0 = col * cx;
                        let cell_x1 = cell_x0 + cx;

                        // --- average the source pixels in this cell (alpha weighted)
                        let sx0 = (cell_x0 - src.origin.0).max(0);
                        let sx1 = (cell_x1 - src.origin.0).min(src.width as i32);
                        let sy0 = (cell_y0 - src.origin.1).max(0);
                        let sy1 = (cell_y1 - src.origin.1).min(src.height as i32);
                        let (mut sa, mut sr, mut sg, mut sb, mut cnt) = (0f32, 0f32, 0f32, 0f32, 0f32);
                        if sx0 < sx1 && sy0 < sy1 {
                            for y in sy0..sy1 {
                                let line = y as usize * src.stride;
                                for x in sx0..sx1 {
                                    let p = read_px(src.data, line + x as usize * bpp, depth);
                                    let a = p[0].clamp(0.0, 1.0);
                                    sa += a;
                                    sr += p[1] * a;
                                    sg += p[2] * a;
                                    sb += p[3] * a;
                                    cnt += 1.0;
                                }
                            }
                        }
                        let out = if cnt == 0.0 || sa <= 0.0 {
                            [0.0; 4]
                        } else {
                            let alpha = sa / cnt;
                            let mut rgb = [sr / sa, sg / sa, sb / sa];
                            for c in rgb.iter_mut() {
                                *c = (*c * s.brightness - 0.5) * s.contrast + 0.5;
                            }
                            let mi = (row.rem_euclid(n as i32) as usize) * n + col.rem_euclid(n as i32) as usize;
                            let t = (matrix[mi] as f32 + 0.5) / nn;
                            if s.mono {
                                let lum = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
                                let q = quantise(lum, levels, t);
                                if s.transparent_bg {
                                    let fg = s.mono_color;
                                    [alpha * q * s.mono_opacity, fg[0], fg[1], fg[2]]
                                } else {
                                    let k = q * s.mono_opacity;
                                    let (fg, bg) = (s.mono_color, s.background);
                                    [
                                        alpha,
                                        bg[0] + (fg[0] - bg[0]) * k,
                                        bg[1] + (fg[1] - bg[1]) * k,
                                        bg[2] + (fg[2] - bg[2]) * k,
                                    ]
                                }
                            } else {
                                [
                                    alpha,
                                    quantise(rgb[0], levels, t),
                                    quantise(rgb[1], levels, t),
                                    quantise(rgb[2], levels, t),
                                ]
                            }
                        };
                        let len = encode_px(out, depth, &mut enc);

                        // --- fill the cell in the destination band
                        let dx0 = (cell_x0 - dst_ox).max(0) as usize;
                        let dx1 = ((cell_x1 - dst_ox).min(dst_w as i32)) as usize;
                        let dy0 = cell_y0.max(band_y);
                        let dy1 = cell_y1.min(band_y_end);
                        for y in dy0..dy1 {
                            let line = (y - band_y) as usize * dst_stride;
                            for x in dx0..dx1 {
                                let o = line + x * bpp;
                                band[o..o + len].copy_from_slice(&enc[..len]);
                            }
                        }
                    }
                    row += 1;
                }
            });
        }
    });
}


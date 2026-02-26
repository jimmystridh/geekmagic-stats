use std::f64::consts::PI;
use std::process::Command;

use ab_glyph::{FontRef, PxScale};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

const W: u32 = 240;
const H: u32 = 240;

const BG: Rgba<u8> = Rgba([12, 12, 16, 255]);
const TEXT_PRIMARY: Rgba<u8> = Rgba([240, 240, 245, 255]);
const TEXT_DIM: Rgba<u8> = Rgba([113, 113, 122, 255]);
const TEXT_MUTED: Rgba<u8> = Rgba([161, 161, 170, 255]);
const SEPARATOR: Rgba<u8> = Rgba([35, 35, 45, 255]);

const PIE_USED: Rgba<u8> = Rgba([99, 102, 241, 255]);
const PIE_USED_2: Rgba<u8> = Rgba([139, 92, 246, 255]);
const PIE_FREE: Rgba<u8> = Rgba([34, 197, 94, 255]);
const PIE_FREE_2: Rgba<u8> = Rgba([16, 185, 129, 255]);
const PIE_BG: Rgba<u8> = Rgba([30, 30, 40, 255]);

const FONT_BYTES: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

pub struct DiskInfo {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

pub fn get_disk_info() -> Result<DiskInfo> {
    let output = Command::new("diskutil")
        .args(["info", "/"])
        .output()
        .context("failed to run diskutil")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let total = extract_bytes(&stdout, "Container Total Space:")
        .or_else(|| extract_bytes(&stdout, "Disk Size:"))
        .context("could not find total space")?;

    let free =
        extract_bytes(&stdout, "Container Free Space:").context("could not find free space")?;

    Ok(DiskInfo {
        total_bytes: total,
        free_bytes: free,
        used_bytes: total.saturating_sub(free),
    })
}

fn extract_bytes(text: &str, label: &str) -> Option<u64> {
    for line in text.lines() {
        if line.contains(label) {
            if let Some(open) = line.find('(') {
                let after_paren = &line[open + 1..];
                let num_str: String = after_paren
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if !num_str.is_empty() {
                    return num_str.parse().ok();
                }
            }
        }
    }
    None
}

pub fn format_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1_000_000_000.0;
    if gb >= 1000.0 {
        format!("{:.1} TB", gb / 1000.0)
    } else if gb >= 100.0 {
        format!("{:.0} GB", gb)
    } else if gb >= 10.0 {
        format!("{:.1} GB", gb)
    } else {
        format!("{:.2} GB", gb)
    }
}

fn lerp_color(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    let t = t.clamp(0.0, 1.0);
    Rgba([
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        255,
    ])
}

fn approx_text_width(text: &str, scale: f32) -> i32 {
    let char_w = scale * 0.55;
    let mut w = 0.0f32;
    for ch in text.chars() {
        w += match ch {
            '.' | ':' | '!' | '|' | 'i' | 'l' | '1' => char_w * 0.55,
            'm' | 'w' | 'M' | 'W' => char_w * 1.25,
            ' ' => char_w * 0.6,
            '%' => char_w * 1.1,
            _ => char_w,
        };
    }
    w.ceil() as i32
}

fn draw_text_centered(
    img: &mut RgbaImage,
    color: Rgba<u8>,
    center_x: i32,
    y: i32,
    scale: f32,
    font: &FontRef,
    text: &str,
) {
    let w = approx_text_width(text, scale);
    draw_text_mut(
        img,
        color,
        center_x - w / 2,
        y,
        PxScale::from(scale),
        font,
        text,
    );
}

fn draw_text_right(
    img: &mut RgbaImage,
    color: Rgba<u8>,
    right_x: i32,
    y: i32,
    scale: f32,
    font: &FontRef,
    text: &str,
) {
    let w = approx_text_width(text, scale);
    draw_text_mut(img, color, right_x - w, y, PxScale::from(scale), font, text);
}

fn draw_rounded_rect(img: &mut RgbaImage, x: i32, y: i32, w: u32, h: u32, r: u32, color: Rgba<u8>) {
    for px in 0..w {
        for py in 0..h {
            if is_inside_rounded(px, py, w, h, r) {
                let abs_x = x as u32 + px;
                let abs_y = y as u32 + py;
                if abs_x < W && abs_y < H {
                    img.put_pixel(abs_x, abs_y, color);
                }
            }
        }
    }
}

fn is_inside_rounded(px: u32, py: u32, w: u32, h: u32, r: u32) -> bool {
    if r == 0 || w == 0 || h == 0 {
        return true;
    }
    let r = r.min(w / 2).min(h / 2);
    let corners = [
        (r, r),
        (w.saturating_sub(r + 1), r),
        (r, h.saturating_sub(r + 1)),
        (w.saturating_sub(r + 1), h.saturating_sub(r + 1)),
    ];
    for &(cx, cy) in &corners {
        let in_corner_x = if px <= cx {
            px < r
        } else {
            px > w.saturating_sub(r + 1)
        };
        let in_corner_y = if py <= cy {
            py < r
        } else {
            py > h.saturating_sub(r + 1)
        };
        if in_corner_x && in_corner_y {
            let dx = if px < cx { cx - px } else { px - cx };
            let dy = if py < cy { cy - py } else { py - cy };
            if dx * dx + dy * dy > r * r {
                return false;
            }
        }
    }
    true
}

pub fn render_disk(info: &DiskInfo) -> Result<RgbaImage> {
    let font = FontRef::try_from_slice(FONT_BYTES)?;
    let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES)?;
    let mut img = RgbaImage::from_pixel(W, H, BG);

    let mx = 16i32;
    let right_edge = W as i32 - mx;
    let content_w = (right_edge - mx) as u32;

    // Header
    let header_y = 10;
    draw_text_mut(
        &mut img,
        TEXT_PRIMARY,
        mx,
        header_y,
        PxScale::from(17.0),
        &font_bold,
        "Macintosh HD",
    );
    let total_text = format_size(info.total_bytes);
    draw_text_right(
        &mut img,
        TEXT_DIM,
        right_edge,
        header_y + 1,
        15.0,
        &font,
        &total_text,
    );

    draw_rounded_rect(&mut img, mx, 33, content_w, 1, 0, SEPARATOR);

    // Pie chart
    let pie_cx = 120.0f64;
    let pie_cy = 118.0f64;
    let pie_r_outer = 68.0f64;
    let pie_r_inner = 42.0f64;

    let used_frac = info.used_bytes as f64 / info.total_bytes as f64;
    let free_frac = 1.0 - used_frac;
    let used_angle = used_frac * 2.0 * PI;

    for py in 0..H {
        for px in 0..W {
            let dx = px as f64 - pie_cx;
            let dy = py as f64 - pie_cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist >= pie_r_inner && dist <= pie_r_outer {
                let angle = (dx.atan2(-dy) + 2.0 * PI) % (2.0 * PI);

                let edge_outer = (pie_r_outer - dist).clamp(0.0, 1.0) as f32;
                let edge_inner = (dist - pie_r_inner).clamp(0.0, 1.0) as f32;
                let aa = edge_outer.min(edge_inner);

                let base_color = if angle < used_angle {
                    let t = (angle / used_angle) as f32;
                    lerp_color(PIE_USED, PIE_USED_2, t)
                } else {
                    let t = ((angle - used_angle) / (2.0 * PI - used_angle)) as f32;
                    lerp_color(PIE_FREE, PIE_FREE_2, t)
                };

                let depth = ((dist - pie_r_inner) / (pie_r_outer - pie_r_inner)) as f32;
                let lit = lerp_color(
                    Rgba([
                        (base_color[0] as f32 * 0.8) as u8,
                        (base_color[1] as f32 * 0.8) as u8,
                        (base_color[2] as f32 * 0.8) as u8,
                        255,
                    ]),
                    base_color,
                    depth,
                );

                let blended = lerp_color(BG, lit, aa);
                img.put_pixel(px, py, blended);
            } else if dist < pie_r_inner && dist >= pie_r_inner - 1.0 {
                let aa = (pie_r_inner - dist).clamp(0.0, 1.0) as f32;
                let blended = lerp_color(BG, PIE_BG, aa * 0.3);
                img.put_pixel(px, py, blended);
            }
        }
    }

    // Center text: free percentage
    let free_pct = (free_frac * 100.0).round() as i32;
    let pct_text = format!("{free_pct}%");
    draw_text_centered(
        &mut img,
        TEXT_PRIMARY,
        pie_cx as i32,
        pie_cy as i32 - 16,
        30.0,
        &font_bold,
        &pct_text,
    );
    draw_text_centered(
        &mut img,
        TEXT_MUTED,
        pie_cx as i32,
        pie_cy as i32 + 12,
        13.0,
        &font,
        "free",
    );

    // Bottom area: legend with prominent GB values
    let legend_y = 192;
    let col1_x = mx + 10;
    let col2_x = 132;

    // Used
    draw_rounded_rect(&mut img, col1_x, legend_y + 4, 10, 10, 3, PIE_USED);
    draw_text_mut(
        &mut img,
        TEXT_MUTED,
        col1_x + 14,
        legend_y,
        PxScale::from(13.0),
        &font,
        "Used",
    );
    let used_text = format_size(info.used_bytes);
    draw_text_mut(
        &mut img,
        TEXT_PRIMARY,
        col1_x + 14,
        legend_y + 16,
        PxScale::from(22.0),
        &font_bold,
        &used_text,
    );

    // Free
    draw_rounded_rect(&mut img, col2_x, legend_y + 4, 10, 10, 3, PIE_FREE);
    draw_text_mut(
        &mut img,
        TEXT_MUTED,
        col2_x + 14,
        legend_y,
        PxScale::from(13.0),
        &font,
        "Free",
    );
    let free_text = format_size(info.free_bytes);
    draw_text_mut(
        &mut img,
        TEXT_PRIMARY,
        col2_x + 14,
        legend_y + 16,
        PxScale::from(22.0),
        &font_bold,
        &free_text,
    );

    Ok(img)
}

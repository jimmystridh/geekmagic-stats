use ab_glyph::{FontRef, PxScale};
use anyhow::Result;
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

use crate::stats::{ActiveData, UsageWindow};

const W: u32 = 240;
const H: u32 = 240;

const BG: Rgba<u8> = Rgba([12, 12, 16, 255]);
const PANEL_BG: Rgba<u8> = Rgba([22, 22, 30, 255]);
const TEXT_PRIMARY: Rgba<u8> = Rgba([240, 240, 245, 255]);
const TEXT_DIM: Rgba<u8> = Rgba([113, 113, 122, 255]);
const TEXT_MUTED: Rgba<u8> = Rgba([161, 161, 170, 255]);
const BAR_TRACK: Rgba<u8> = Rgba([40, 40, 50, 255]);
const BAR_FILL_LEFT: Rgba<u8> = Rgba([59, 130, 246, 255]);
const BAR_FILL_RIGHT: Rgba<u8> = Rgba([6, 182, 212, 255]);
const PACE_OK: Rgba<u8> = Rgba([34, 197, 94, 255]);
const PACE_WARN: Rgba<u8> = Rgba([249, 115, 22, 255]);
const WARN_FILL_LEFT: Rgba<u8> = Rgba([234, 179, 8, 255]);
const WARN_FILL_RIGHT: Rgba<u8> = Rgba([249, 115, 22, 255]);
const DANGER_FILL: Rgba<u8> = Rgba([239, 68, 68, 255]);
const SEPARATOR: Rgba<u8> = Rgba([35, 35, 45, 255]);

const FONT_BYTES: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

fn lerp_color(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    let t = t.clamp(0.0, 1.0);
    Rgba([
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        255,
    ])
}

fn bar_colors(usage_level: &str) -> (Rgba<u8>, Rgba<u8>) {
    match usage_level {
        "danger" | "over" => (DANGER_FILL, DANGER_FILL),
        "warn" => (WARN_FILL_LEFT, WARN_FILL_RIGHT),
        _ => (BAR_FILL_LEFT, BAR_FILL_RIGHT),
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

fn draw_gradient_bar(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    total_w: u32,
    h: u32,
    fill_frac: f32,
    left_color: Rgba<u8>,
    right_color: Rgba<u8>,
    corner_r: u32,
) {
    draw_rounded_rect(img, x, y, total_w, h, corner_r, BAR_TRACK);
    let fill_w = ((total_w as f32) * fill_frac.clamp(0.0, 1.0)) as u32;
    if fill_w == 0 {
        return;
    }
    for px in 0..fill_w {
        let t = if total_w > 1 {
            px as f32 / (total_w - 1) as f32
        } else {
            0.0
        };
        let color = lerp_color(left_color, right_color, t);
        let abs_x = x as u32 + px;
        for py in 0..h {
            let abs_y = y as u32 + py;
            if is_inside_rounded(px, py, fill_w, h, corner_r) && abs_x < W && abs_y < H {
                img.put_pixel(abs_x, abs_y, color);
            }
        }
    }
}

fn blend_over(base: Rgba<u8>, over: Rgba<u8>) -> Rgba<u8> {
    let a = over[3] as f32 / 255.0;
    Rgba([
        (base[0] as f32 * (1.0 - a) + over[0] as f32 * a) as u8,
        (base[1] as f32 * (1.0 - a) + over[1] as f32 * a) as u8,
        (base[2] as f32 * (1.0 - a) + over[2] as f32 * a) as u8,
        255,
    ])
}

fn draw_pace_marker(
    img: &mut RgbaImage,
    bar_x: i32,
    bar_y: i32,
    bar_w: u32,
    bar_h: u32,
    expected_pct: f64,
    ok: bool,
) {
    let marker_x = bar_x + (bar_w as f64 * expected_pct.clamp(0.0, 100.0) / 100.0) as i32;
    let color = if ok { PACE_OK } else { PACE_WARN };
    let glow = if ok {
        Rgba([34, 197, 94, 80])
    } else {
        Rgba([249, 115, 22, 80])
    };

    for dx in 0..2i32 {
        for dy in -3..(bar_h as i32 + 3) {
            let px = marker_x + dx;
            let py = bar_y + dy;
            if px >= 0 && px < W as i32 && py >= 0 && py < H as i32 {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
    for dx in [-1i32, 2] {
        for dy in -2..(bar_h as i32 + 2) {
            let px = marker_x + dx;
            let py = bar_y + dy;
            if px >= 0 && px < W as i32 && py >= 0 && py < H as i32 {
                let existing = *img.get_pixel(px as u32, py as u32);
                img.put_pixel(px as u32, py as u32, blend_over(existing, glow));
            }
        }
    }
}

fn draw_circle(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, color: Rgba<u8>) {
    for dx in -r..=r {
        for dy in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < W as i32 && py >= 0 && py < H as i32 {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

fn format_duration(minutes: f64) -> String {
    let total = minutes.max(0.0).round() as u64;
    let days = total / 1440;
    let hours = (total % 1440) / 60;
    let mins = total % 60;
    if days > 0 {
        if hours == 0 {
            return format!("{days}d");
        }
        return format!("{days}d {hours}h");
    }
    if hours == 0 {
        return format!("{mins}m");
    }
    if mins == 0 {
        return format!("{hours}h");
    }
    format!("{hours}h {mins}m")
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

fn format_updated_time(iso: &str) -> String {
    use chrono::{DateTime, Local};
    if let Ok(utc) = DateTime::parse_from_rfc3339(iso) {
        let local: DateTime<Local> = utc.with_timezone(&Local);
        return local.format("%H:%M").to_string();
    }
    // Fallback: try extracting time part
    if let Some(t_pos) = iso.find('T') {
        let time_part = &iso[t_pos + 1..];
        if time_part.len() >= 5 {
            return time_part[..5].to_string();
        }
    }
    "??:??".to_string()
}

struct BarSection {
    label: &'static str,
    window: UsageWindow,
}

pub fn render_bars(data: &ActiveData) -> Result<RgbaImage> {
    let font = FontRef::try_from_slice(FONT_BYTES)?;
    let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES)?;
    let mut img = RgbaImage::from_pixel(W, H, BG);

    let mut sections: Vec<BarSection> = Vec::new();
    if let Some(w) = &data.five_hour {
        sections.push(BarSection {
            label: "Session",
            window: w.clone(),
        });
    }
    if let Some(w) = &data.seven_day {
        sections.push(BarSection {
            label: "Weekly",
            window: w.clone(),
        });
    }

    if sections.is_empty() {
        draw_text_mut(
            &mut img,
            TEXT_DIM,
            60,
            110,
            PxScale::from(16.0),
            &font,
            "No usage data",
        );
        return Ok(img);
    }

    let mx = 16i32;
    let right_edge = (W as i32) - mx;
    let content_w = (right_edge - mx) as u32;

    // ── Header: "Claude Code" + updated time ──
    let header_y = 10;
    draw_text_mut(
        &mut img,
        TEXT_PRIMARY,
        mx,
        header_y,
        PxScale::from(17.0),
        &font_bold,
        "Claude Code",
    );

    // Updated timestamp (right-aligned, bigger)
    let updated_text = if let Some(ts) = &data.updated_at {
        format_updated_time(ts)
    } else {
        "—".to_string()
    };
    draw_text_right(
        &mut img,
        TEXT_DIM,
        right_edge,
        header_y + 1,
        15.0,
        &font,
        &updated_text,
    );

    // Separator
    draw_rounded_rect(&mut img, mx, 33, content_w, 1, 0, SEPARATOR);

    // ── Bar sections ──
    let section_h = 98i32;
    let gap = 1i32; // tighter gap between sections
    let start_y = 37; // moved up

    for (i, section) in sections.iter().enumerate() {
        let by = start_y + (i as i32) * (section_h + gap);
        let w = &section.window;
        let bar_x = mx + 8;
        let bar_w = content_w - 16;
        let inner_right = right_edge - 6;

        // Panel background
        draw_rounded_rect(
            &mut img,
            mx - 4,
            by - 2,
            content_w + 8,
            section_h as u32 + 4,
            10,
            PANEL_BG,
        );

        // Row 1: Label left, big percentage right
        let row1_y = by + 4;
        draw_text_mut(
            &mut img,
            TEXT_MUTED,
            bar_x,
            row1_y + 10,
            PxScale::from(14.0),
            &font_bold,
            section.label,
        );

        let pct_val = w.utilization.round() as i32;
        let pct_text = format!("{pct_val}%");
        draw_text_right(
            &mut img,
            TEXT_PRIMARY,
            inner_right,
            row1_y - 2,
            36.0,
            &font_bold,
            &pct_text,
        );

        // Row 2: Progress bar
        let bar_y = row1_y + 38;
        let bar_h = 14u32;
        let fill_frac = (w.utilization / 100.0) as f32;
        let (fill_l, fill_r) = bar_colors(&w.usage_level);
        draw_gradient_bar(
            &mut img, bar_x, bar_y, bar_w, bar_h, fill_frac, fill_l, fill_r, 7,
        );

        // Pace marker on bar
        if let Some(pace) = &w.pace {
            draw_pace_marker(
                &mut img,
                bar_x,
                bar_y,
                bar_w,
                bar_h,
                pace.expected_percent,
                pace.will_last_to_reset,
            );
        }

        // Row 3: "X% left" bigger + "Resets in ..."
        let row3_y = bar_y + bar_h as i32 + 6;
        let remaining = (100.0 - w.utilization).max(0.0);
        let left_text = format!("{}% left", remaining.round() as i32);
        draw_text_mut(
            &mut img,
            TEXT_PRIMARY,
            bar_x,
            row3_y,
            PxScale::from(15.0),
            &font_bold,
            &left_text,
        );

        if let Some(mins) = w.resets_in_minutes {
            let reset_text = format!("resets {}", format_duration(mins));
            draw_text_right(
                &mut img,
                TEXT_DIM,
                inner_right,
                row3_y + 1,
                15.0,
                &font,
                &reset_text,
            );
        }

        // Row 4: Pace info
        if let Some(pace) = &w.pace {
            let pace_y = row3_y + 18;
            let abs_delta = pace.delta_percent.abs().round() as i32;
            let (pace_text, pace_color) = if abs_delta <= 2 {
                ("On pace".to_string(), PACE_OK)
            } else if pace.delta_percent < 0.0 {
                (format!("{abs_delta}% reserve"), PACE_OK)
            } else {
                (format!("{abs_delta}% deficit"), PACE_WARN)
            };

            // Colored dot + text (bigger green/orange text)
            draw_circle(&mut img, bar_x + 4, pace_y + 6, 3, pace_color);
            draw_text_mut(
                &mut img,
                pace_color,
                bar_x + 12,
                pace_y,
                PxScale::from(13.0),
                &font,
                &pace_text,
            );

            // Right side: ETA
            let right_text = if pace.will_last_to_reset {
                "Lasts to reset".to_string()
            } else if let Some(eta) = pace.eta_minutes {
                format!("Out in {}", format_duration(eta))
            } else {
                String::new()
            };
            if !right_text.is_empty() {
                draw_text_right(
                    &mut img,
                    pace_color,
                    inner_right,
                    pace_y,
                    12.0,
                    &font,
                    &right_text,
                );
            }
        }
    }

    Ok(img)
}

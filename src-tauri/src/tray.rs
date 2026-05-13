// 托盘实时指标：
//
// 设计：多个并列托盘图标，每个图标本身把数字渲染上去（不依赖 tooltip）。
//   · loft-main —— LOGO + 底部 CPU 进度条 + 菜单
//   · loft-cpu  —— "27" 透明背景 + 彩字 + 柔投影
//   · loft-temp —— "58" 同上（拿不到温度时整个图标隐藏）
//   · loft-geo  —— "CN" 同上（拿不到 IP 时整个图标隐藏）
//
// 文字渲染：
//   - 运行时加载 Windows 自带的 Segoe UI Black（seguibl.ttf）
//   - 通过 ab_glyph 抗锯齿光栅化，避免 5x7 像素字的复古/笨重感
//   - 1px 黑色柔投影，深/浅任务栏底色都清晰
//
// 公网 IP / 国家：
//   - 走 Windows 10/11 自带的 curl.exe，轮询 4 个端点
//   - PowerShell Invoke-RestMethod 在某些机器上 TLS/代理/BOM 会静默失败

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

static BASE: OnceLock<BaseIcon> = OnceLock::new();
static GEO: OnceLock<Mutex<GeoCache>> = OnceLock::new();

pub struct BaseIcon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    pub ip: String,
    pub country: String,
    pub city: Option<String>,
}

#[derive(Default)]
struct GeoCache {
    info: Option<GeoInfo>,
    fetched_at: Option<Instant>,
    last_error: Option<String>,
}

// ============================================================
// 基础 LOGO（懒解码）
// ============================================================
pub fn base_icon() -> &'static BaseIcon {
    BASE.get_or_init(|| {
        let img = image::load_from_memory(ICON_PNG)
            .expect("[tray] failed to decode base icon PNG");
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        BaseIcon {
            width,
            height,
            rgba: rgba.into_raw(),
        }
    })
}

/// 给基础图标底部叠一条 CPU 进度条
pub fn render_icon_with_cpu(cpu_pct: f32) -> Vec<u8> {
    let base = base_icon();
    let mut buf = base.rgba.clone();
    let w = base.width as usize;
    let h = base.height as usize;
    let bar_height = ((h as f32) * 0.18).round() as usize;
    let bar_top = h.saturating_sub(bar_height);
    let pct = cpu_pct.clamp(0.0, 100.0);
    let fill_width = ((pct / 100.0) * (w as f32)).round() as usize;
    let (fr, fg, fb) = cpu_color(pct);
    for y in bar_top..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            if x < fill_width {
                buf[idx] = fr;
                buf[idx + 1] = fg;
                buf[idx + 2] = fb;
                buf[idx + 3] = 255;
            } else {
                buf[idx] = 0x1a;
                buf[idx + 1] = 0x1a;
                buf[idx + 2] = 0x1a;
                buf[idx + 3] = 200;
            }
        }
    }
    buf
}

fn cpu_color(pct: f32) -> (u8, u8, u8) {
    if pct < 30.0 {
        (0x10, 0xb9, 0x81) // emerald (主图标底部进度条仍用饱和色)
    } else if pct < 70.0 {
        (0xfb, 0xbf, 0x24) // amber
    } else {
        (0xef, 0x44, 0x44) // red
    }
}

/// 文字图标用的"亮一档"颜色 —— 在深色任务栏上更醒目
fn cpu_text_color(pct: f32) -> (u8, u8, u8) {
    if pct < 30.0 {
        (0x34, 0xd3, 0x99) // emerald-400
    } else if pct < 70.0 {
        (0xfc, 0xd3, 0x4d) // amber-300
    } else {
        (0xf8, 0x71, 0x71) // red-400
    }
}

fn temp_text_color(t: f32) -> (u8, u8, u8) {
    if t < 50.0 {
        (0x34, 0xd3, 0x99) // emerald-400
    } else if t < 75.0 {
        (0xfb, 0x92, 0x3c) // orange-400
    } else {
        (0xf8, 0x71, 0x71) // red-400
    }
}

// ============================================================
// 5x7 像素字体
// ============================================================
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;

fn glyph(c: char) -> Option<[u8; GLYPH_H]> {
    Some(match c.to_ascii_uppercase() {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10011, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        ' ' => [0; GLYPH_H],
        _ => return None,
    })
}

// ============================================================
// 系统字体加载（Windows 自带的 Segoe UI Black）
// ============================================================
static FONT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

fn load_system_font_bytes() -> &'static [u8] {
    FONT_BYTES.get_or_init(|| {
        // 候选优先级：粗 → 中粗 → 半粗 → 常规 → Arial
        // 现代 Windows 都自带这些路径下的字体
        const CANDIDATES: &[&str] = &[
            "C:\\Windows\\Fonts\\seguibl.ttf",  // Segoe UI Black
            "C:\\Windows\\Fonts\\segoeuib.ttf", // Segoe UI Bold
            "C:\\Windows\\Fonts\\seguisb.ttf",  // Segoe UI Semibold
            "C:\\Windows\\Fonts\\arialbd.ttf",  // Arial Bold
            "C:\\Windows\\Fonts\\segoeui.ttf",  // Segoe UI Regular
        ];
        for path in CANDIDATES {
            if let Ok(data) = std::fs::read(path) {
                return data;
            }
        }
        Vec::new()
    })
    .as_slice()
}

fn system_font() -> Option<FontRef<'static>> {
    let bytes = load_system_font_bytes();
    if bytes.is_empty() {
        return None;
    }
    FontRef::try_from_slice(bytes).ok()
}

/// 玻璃卡片风格文字图标 —— 暗色半透明底 + 白字 + 底部彩色状态条
pub fn render_text_icon(text: &str, size: u32, accent: (u8, u8, u8)) -> Vec<u8> {
    let w = size as usize;
    let h = size as usize;
    let mut buf = vec![0u8; w * h * 4];
    let s = size as i32;

    let radius = ((size as f32) * 0.24).round() as i32;
    let dark_glass = (15u8, 23u8, 42u8); // slate-900

    // 1) 暗色玻璃底（半透明，能看到任务栏底色）
    fill_rounded_rect_aa(&mut buf, w, h, 0, 0, s, s, radius, dark_glass, 215);

    // 2) 顶部高光（玻璃边缘反光感）
    add_top_highlight(&mut buf, w, h, s, radius);

    // 3) 外圈细描边（白色低透明，定义边缘）
    draw_rounded_border(&mut buf, w, h, 0, 0, s, s, radius, (255, 255, 255), 70);

    // 4) 底部 accent 状态条（指标颜色，2px 高度）
    draw_bottom_accent(&mut buf, w, h, s, radius, accent);

    // 5) 白色粗字，居中
    let chars: Vec<char> = text.chars().take(3).collect();
    if !chars.is_empty() {
        if let Some(font) = system_font() {
            render_text_ttf(&mut buf, w, h, &font, &chars, (255, 255, 255));
        } else {
            render_text_pixel(&mut buf, w, h, &chars, (255, 255, 255));
        }
    }

    buf
}

// ============================================================
// 2D 绘图基础原语：4x 超采样抗锯齿圆角矩形 + alpha 混合
// ============================================================
fn blend_pixel(buf: &mut [u8], w: usize, x: usize, y: usize, color: (u8, u8, u8), alpha: u8) {
    if alpha == 0 {
        return;
    }
    let idx = (y * w + x) * 4;
    let src_a = alpha as u32;
    let inv = 255u32 - src_a;
    let dst_a = buf[idx + 3] as u32;
    buf[idx] = ((color.0 as u32 * src_a + buf[idx] as u32 * inv) / 255) as u8;
    buf[idx + 1] = ((color.1 as u32 * src_a + buf[idx + 1] as u32 * inv) / 255) as u8;
    buf[idx + 2] = ((color.2 as u32 * src_a + buf[idx + 2] as u32 * inv) / 255) as u8;
    buf[idx + 3] = (dst_a + (src_a * (255 - dst_a)) / 255) as u8;
}

fn inside_rounded_f(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if x < x0 || x >= x1 || y < y0 || y >= y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
}

fn fill_rounded_rect_aa(
    buf: &mut [u8],
    w: usize,
    h: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    r: i32,
    color: (u8, u8, u8),
    alpha: u8,
) {
    for py in y0..y1 {
        if py < 0 || py >= h as i32 {
            continue;
        }
        for px in x0..x1 {
            if px < 0 || px >= w as i32 {
                continue;
            }
            // 4x4 超采样
            let mut covered = 0u32;
            for dy in 0..4 {
                for dx in 0..4 {
                    let sx = px as f32 + (dx as f32 + 0.5) / 4.0;
                    let sy = py as f32 + (dy as f32 + 0.5) / 4.0;
                    if inside_rounded_f(sx, sy, x0 as f32, y0 as f32, x1 as f32, y1 as f32, r as f32) {
                        covered += 1;
                    }
                }
            }
            if covered == 0 {
                continue;
            }
            let cov = covered as f32 / 16.0;
            let a = (alpha as f32 * cov).round() as u8;
            blend_pixel(buf, w, px as usize, py as usize, color, a);
        }
    }
}

fn draw_rounded_border(
    buf: &mut [u8],
    w: usize,
    h: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    r: i32,
    color: (u8, u8, u8),
    alpha: u8,
) {
    // 在外圈 1px 范围内根据是否同时贴合外/内圈来判定描边像素
    let r_outer = r as f32;
    let r_inner = (r - 1).max(0) as f32;
    for py in y0..y1 {
        if py < 0 || py >= h as i32 {
            continue;
        }
        for px in x0..x1 {
            if px < 0 || px >= w as i32 {
                continue;
            }
            let cx = px as f32 + 0.5;
            let cy = py as f32 + 0.5;
            let in_outer = inside_rounded_f(cx, cy, x0 as f32, y0 as f32, x1 as f32, y1 as f32, r_outer);
            if !in_outer {
                continue;
            }
            let in_inner = inside_rounded_f(
                cx,
                cy,
                (x0 + 1) as f32,
                (y0 + 1) as f32,
                (x1 - 1) as f32,
                (y1 - 1) as f32,
                r_inner,
            );
            if in_inner {
                continue;
            }
            blend_pixel(buf, w, px as usize, py as usize, color, alpha);
        }
    }
}

/// 顶部 30% 高度处加一道由白到透明的渐变高光，模拟玻璃边缘反光
fn add_top_highlight(buf: &mut [u8], w: usize, h: usize, s: i32, r: i32) {
    let highlight_h = (s as f32 * 0.32) as i32;
    for py in 0..highlight_h {
        if py >= h as i32 {
            break;
        }
        // 从顶部往下衰减：top = alpha 60, bottom = 0
        let t = py as f32 / highlight_h as f32;
        let a = ((1.0 - t) * 55.0) as u8;
        for px in 0..s {
            if px < 0 || px >= w as i32 {
                continue;
            }
            let cx = px as f32 + 0.5;
            let cy = py as f32 + 0.5;
            if !inside_rounded_f(cx, cy, 0.5, 0.5, s as f32 - 0.5, s as f32 - 0.5, r as f32) {
                continue;
            }
            blend_pixel(buf, w, px as usize, py as usize, (255, 255, 255), a);
        }
    }
}

/// 底部 2px 状态条（指标颜色）
fn draw_bottom_accent(
    buf: &mut [u8],
    w: usize,
    h: usize,
    s: i32,
    r: i32,
    color: (u8, u8, u8),
) {
    let bar_h = (s as f32 * 0.10).round().max(2.0) as i32;
    let bar_top = s - bar_h - 2; // 距底边 2px 余量
    let inset = (s as f32 * 0.18) as i32;
    for py in bar_top..(bar_top + bar_h) {
        if py < 0 || py >= h as i32 {
            continue;
        }
        for px in inset..(s - inset) {
            if px < 0 || px >= w as i32 {
                continue;
            }
            // 让状态条也跟随圆角形状
            let cx = px as f32 + 0.5;
            let cy = py as f32 + 0.5;
            if !inside_rounded_f(cx, cy, 0.0, 0.0, s as f32, s as f32, r as f32) {
                continue;
            }
            blend_pixel(buf, w, px as usize, py as usize, color, 255);
        }
    }
}

fn render_text_ttf(
    buf: &mut [u8],
    w: usize,
    h: usize,
    font: &FontRef<'static>,
    chars: &[char],
    color: (u8, u8, u8),
) {
    // 给底部 accent 条留 ~20% 空间，文字在上方 75% 区域内尽量大
    let max_w = (w as f32) * 0.78;
    let max_h = (h as f32) * 0.62;
    let mut lo: f32 = 6.0;
    let mut hi: f32 = h as f32 * 1.4;
    let mut best_scale = lo;
    for _ in 0..14 {
        let mid = (lo + hi) / 2.0;
        let sized = font.as_scaled(PxScale::from(mid));
        let advance: f32 = chars
            .iter()
            .map(|c| sized.h_advance(sized.scaled_glyph(*c).id))
            .sum();
        let cap_h = sized.ascent() - sized.descent();
        if advance <= max_w && cap_h <= max_h {
            best_scale = mid;
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let sized = font.as_scaled(PxScale::from(best_scale));
    let advance: f32 = chars
        .iter()
        .map(|c| sized.h_advance(sized.scaled_glyph(*c).id))
        .sum();
    let start_x = ((w as f32) - advance) / 2.0;
    // 把文字垂直居中放进"顶部 80%"区域 —— 给底部 accent 条让位
    let upper_h = h as f32 * 0.82;
    let baseline = (upper_h + sized.ascent() * 0.78) / 2.0;

    // 白字直接画 —— 暗色玻璃底反差天然够大，不需要投影
    draw_text_pass(buf, w, h, &sized, chars, start_x, baseline, color, 1.0);
}

fn draw_text_pass(
    buf: &mut [u8],
    w: usize,
    h: usize,
    sized: &ab_glyph::PxScaleFont<&FontRef<'static>>,
    chars: &[char],
    start_x: f32,
    baseline: f32,
    color: (u8, u8, u8),
    alpha_mul: f32,
) {
    let mut cur_x = start_x;
    for c in chars {
        let mut g = sized.scaled_glyph(*c);
        g.position = ab_glyph::point(cur_x, baseline);
        cur_x += sized.h_advance(g.id);
        if let Some(outlined) = sized.outline_glyph(g) {
            let bb = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bb.min.x as i32 + gx as i32;
                let py = bb.min.y as i32 + gy as i32;
                if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 {
                    return;
                }
                let idx = ((py as usize) * w + (px as usize)) * 4;
                let a = (coverage * alpha_mul * 255.0).round().clamp(0.0, 255.0) as u8;
                if a == 0 {
                    return;
                }
                // 直接以源覆盖目标的 alpha 混合
                let src_a = a as u32;
                let inv = 255u32 - src_a;
                let dst_a = buf[idx + 3] as u32;
                buf[idx] = ((color.0 as u32 * src_a + buf[idx] as u32 * inv) / 255) as u8;
                buf[idx + 1] = ((color.1 as u32 * src_a + buf[idx + 1] as u32 * inv) / 255) as u8;
                buf[idx + 2] = ((color.2 as u32 * src_a + buf[idx + 2] as u32 * inv) / 255) as u8;
                buf[idx + 3] = (dst_a + (src_a * (255 - dst_a)) / 255) as u8;
            });
        }
    }
}

// ----- Fallback：极端情况下系统字体读不到才用 -----
fn render_text_pixel(buf: &mut [u8], w: usize, h: usize, chars: &[char], color: (u8, u8, u8)) {
    let n = chars.len();
    let gap = 1usize;
    let avail_w = w.saturating_sub(2);
    let avail_h = h.saturating_sub(2);
    let per_char = (avail_w.saturating_sub(gap * n.saturating_sub(1)) / n) / GLYPH_W;
    let scale = per_char.min(avail_h / GLYPH_H).max(1);
    let text_w = scale * GLYPH_W * n + scale * gap * n.saturating_sub(1);
    let text_h = scale * GLYPH_H;
    let sx = (w.saturating_sub(text_w)) / 2;
    let sy = (h.saturating_sub(text_h)) / 2;
    for (i, c) in chars.iter().enumerate() {
        let Some(g) = glyph(*c) else { continue };
        let cx = sx + i * (scale * GLYPH_W + scale * gap);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - col)) & 1 == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = cx + col * scale + dx;
                        let py = sy + row * scale + dy;
                        if px >= w || py >= h {
                            continue;
                        }
                        let idx = (py * w + px) * 4;
                        buf[idx] = color.0;
                        buf[idx + 1] = color.1;
                        buf[idx + 2] = color.2;
                        buf[idx + 3] = 255;
                    }
                }
            }
        }
    }
}

// ============================================================
// CPU 温度（sysinfo Components 兜底）
// ============================================================
pub fn read_cpu_temp() -> Option<f32> {
    let components = sysinfo::Components::new_with_refreshed_list();
    let mut best: Option<f32> = None;
    for c in &components {
        let label = c.label().to_lowercase();
        let t = c.temperature();
        if !t.is_finite() || t <= 0.0 || t >= 150.0 {
            continue;
        }
        if label.contains("cpu")
            || label.contains("package")
            || label.contains("core")
            || label.contains("tctl")
            || label.contains("tdie")
        {
            return Some(t);
        }
        best = Some(match best {
            None => t,
            Some(b) => b.max(t),
        });
    }
    best
}

// ============================================================
// IP / 国家（curl.exe + 多端点回退）
// ============================================================
fn geo_slot() -> &'static Mutex<GeoCache> {
    GEO.get_or_init(|| Mutex::new(GeoCache::default()))
}

pub fn current_geo() -> Option<GeoInfo> {
    geo_slot().lock().info.clone()
}

#[allow(dead_code)]
pub fn last_geo_error() -> Option<String> {
    geo_slot().lock().last_error.clone()
}

/// 5 分钟内已成功获取过则跳过；首次失败 30s 后再试，避免循环阻塞。
pub fn maybe_refresh_geo() {
    {
        let cache = geo_slot().lock();
        if let Some(t) = cache.fetched_at {
            let elapsed = t.elapsed();
            if cache.info.is_some() && elapsed < Duration::from_secs(300) {
                return;
            }
            if cache.info.is_none() && elapsed < Duration::from_secs(30) {
                return;
            }
        }
    }
    let (info, err) = fetch_geo();
    let mut cache = geo_slot().lock();
    cache.fetched_at = Some(Instant::now());
    if let Some(i) = info {
        cache.info = Some(i);
        cache.last_error = None;
    } else {
        cache.last_error = Some(err.unwrap_or_else(|| "未知错误".to_string()));
    }
}

#[cfg(windows)]
fn run_curl(url: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let out = Command::new("curl.exe")
        .args(["-sS", "--max-time", "5", "--connect-timeout", "3", "-L", url])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl 启动失败: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(format!(
            "curl 失败 ({}): {}",
            out.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(not(windows))]
fn run_curl(_url: &str) -> Result<String, String> {
    Err("non-windows".to_string())
}

fn parse_geo(text: &str) -> Option<GeoInfo> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    // ip-api.com 用 status 字段
    if let Some(status) = v.get("status").and_then(|x| x.as_str()) {
        if status != "success" {
            return None;
        }
    }
    let ip = v
        .get("ip")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("query").and_then(|x| x.as_str()))?
        .to_string();
    let country = v
        .get("country")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("country_code").and_then(|x| x.as_str()))
        .or_else(|| v.get("countryCode").and_then(|x| x.as_str()))?
        .to_string();
    let city = v
        .get("city")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    Some(GeoInfo {
        ip,
        country: country.to_uppercase(),
        city,
    })
}

fn fetch_geo() -> (Option<GeoInfo>, Option<String>) {
    const ENDPOINTS: &[&str] = &[
        "https://api.country.is/",
        "https://ipinfo.io/json",
        "https://ipapi.co/json/",
        "https://ip-api.com/json/?fields=status,countryCode,city,query",
    ];
    let mut last_err: Option<String> = None;
    for url in ENDPOINTS {
        match run_curl(url) {
            Ok(text) => {
                if let Some(geo) = parse_geo(&text) {
                    return (Some(geo), None);
                } else {
                    last_err = Some(format!("{}: 响应无法解析", url));
                }
            }
            Err(e) => {
                last_err = Some(format!("{}: {}", url, e));
            }
        }
    }
    (None, last_err)
}

// ============================================================
// 主图标的 hover tooltip（多行，仅含可用信息）
// ============================================================
pub fn format_main_tooltip(
    cpu_pct: f32,
    mem_pct: f32,
    temp: Option<f32>,
    geo: Option<&GeoInfo>,
) -> String {
    let mut lines = vec![
        "凌台 · Loft".to_string(),
        format!("CPU {:>4.0}%  内存 {:>4.0}%", cpu_pct, mem_pct),
    ];
    if let Some(t) = temp {
        lines.push(format!("温度 {:.0} °C", t));
    }
    if let Some(g) = geo {
        let mut s = format!("{} · {}", g.country, g.ip);
        if let Some(city) = &g.city {
            s.push_str(&format!(" · {}", city));
        }
        lines.push(s);
    }
    lines.join("\n")
}

// 单数据托盘图标的 tooltip 辅助
pub fn cpu_text(pct: f32) -> String {
    format!("{:.0}", pct.clamp(0.0, 99.0))
}

pub fn temp_text(t: f32) -> String {
    format!("{:.0}", t.clamp(0.0, 99.0))
}

// 文字图标用的"亮档"颜色 —— 透明背景上更醒目
pub fn cpu_bg(pct: f32) -> (u8, u8, u8) {
    cpu_text_color(pct)
}

pub fn temp_bg(t: f32) -> (u8, u8, u8) {
    temp_text_color(t)
}

pub fn geo_bg() -> (u8, u8, u8) {
    (0x2d, 0xd4, 0xbf) // teal-400 —— 亮一档
}

#[cfg(test)]
mod preview {
    use super::*;

    #[test]
    fn save_preview_icons() {
        let cases = [
            ("preview_cpu_low.png", "27", cpu_bg(20.0)),
            ("preview_cpu_mid.png", "58", cpu_bg(50.0)),
            ("preview_cpu_high.png", "92", cpu_bg(92.0)),
            ("preview_temp_mid.png", "58", temp_bg(58.0)),
            ("preview_temp_hot.png", "85", temp_bg(85.0)),
            ("preview_geo_cn.png", "CN", geo_bg()),
            ("preview_geo_us.png", "US", geo_bg()),
        ];
        for (name, text, color) in cases {
            let buf = render_text_icon(text, 64, color);
            let img = image::RgbaImage::from_raw(64, 64, buf).expect("RgbaImage");
            let path = std::path::Path::new("target").join(name);
            std::fs::create_dir_all("target").ok();
            img.save(&path).expect("save png");
            println!("wrote {:?}", path);
        }
    }
}

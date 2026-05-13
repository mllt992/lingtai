// 托盘实时指标：
//
// 设计：多个并列托盘图标，每个图标本身把数字渲染上去（不依赖 tooltip）。
//   · loft-main —— LOGO + 底部 CPU 进度条 + 菜单（点击显示主窗口、退出）
//   · loft-cpu  —— "23" 文字图标，emerald 背景
//   · loft-temp —— "58" 文字图标，amber 背景（拿不到温度时隐藏）
//   · loft-geo  —— "CN" 文字图标，teal 背景（拿不到 IP 时隐藏）
//
// 实现细节：
//   - 基础 LOGO 启动时解码一次，缓存在 OnceLock
//   - 文字渲染走手写 5x7 像素字体（0-9 / A-Z / % / ° / -），无新依赖
//   - 公网 IP 改用 curl.exe（Win10+ 自带），轮询 4 个端点，更稳
//   - 温度走 sysinfo::Components（家用机常拿不到，正常现象）

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
        (0x10, 0xb9, 0x81) // emerald
    } else if pct < 70.0 {
        (0xfb, 0xbf, 0x24) // amber
    } else {
        (0xef, 0x44, 0x44) // red
    }
}

fn temp_color(t: f32) -> (u8, u8, u8) {
    if t < 50.0 {
        (0x10, 0xb9, 0x81) // emerald
    } else if t < 75.0 {
        (0xf9, 0x73, 0x16) // orange
    } else {
        (0xef, 0x44, 0x44) // red
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

/// 渲染一个文字托盘图标。
/// - size: 图标边长（像素）
/// - text: 最多 3 个字符，自动居中
/// - bg: 背景圆角方块颜色 (RGB)
/// 返回 RGBA buffer。
pub fn render_text_icon(text: &str, size: u32, bg: (u8, u8, u8)) -> Vec<u8> {
    let w = size as usize;
    let h = size as usize;
    let mut buf = vec![0u8; w * h * 4];

    // 1) 圆角方块背景
    let margin = ((size as f32) * 0.04).round() as i32;
    let bg_radius = ((size as f32) * 0.22).round() as i32;
    let x0 = margin;
    let y0 = margin;
    let x1 = (size as i32) - margin;
    let y1 = (size as i32) - margin;
    for y in 0..(h as i32) {
        for x in 0..(w as i32) {
            if !inside_rounded(x, y, x0, y0, x1, y1, bg_radius) {
                continue;
            }
            let idx = ((y as usize) * w + (x as usize)) * 4;
            buf[idx] = bg.0;
            buf[idx + 1] = bg.1;
            buf[idx + 2] = bg.2;
            buf[idx + 3] = 255;
        }
    }

    // 2) 计算文字布局：把 text 中能识别的字符画出来
    let chars: Vec<char> = text.chars().take(3).collect();
    if chars.is_empty() {
        return buf;
    }
    let n = chars.len();
    // 字符之间留 1*scale px 间距
    let text_inner_margin = ((size as f32) * 0.10).round() as usize;
    let avail_w = w.saturating_sub(text_inner_margin * 2);
    let avail_h = h.saturating_sub(text_inner_margin * 2);
    // 每字最大可用宽度
    let per_char_w = if n == 1 {
        avail_w
    } else {
        avail_w.saturating_sub(n - 1) / n
    };
    let scale_w = per_char_w / GLYPH_W;
    let scale_h = avail_h / GLYPH_H;
    let scale = scale_w.min(scale_h).max(1);
    let text_w = scale * GLYPH_W * n + scale.saturating_sub(0).max(1) * (n.saturating_sub(1));
    let text_h = scale * GLYPH_H;
    let start_x = (w.saturating_sub(text_w)) / 2;
    let start_y = (h.saturating_sub(text_h)) / 2;

    // 3) 绘制每个字符（白色，仅在背景之上）
    for (i, c) in chars.iter().enumerate() {
        let g = match glyph(*c) {
            Some(g) => g,
            None => continue,
        };
        let cx = start_x + i * (scale * GLYPH_W + scale.max(1));
        for (row, bits) in g.iter().enumerate() {
            for col in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - col)) & 1 == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = cx + col * scale + dx;
                        let py = start_y + row * scale + dy;
                        if px >= w || py >= h {
                            continue;
                        }
                        let idx = (py * w + px) * 4;
                        if buf[idx + 3] == 0 {
                            continue;
                        }
                        buf[idx] = 255;
                        buf[idx + 1] = 255;
                        buf[idx + 2] = 255;
                    }
                }
            }
        }
    }

    buf
}

fn inside_rounded(x: i32, y: i32, x0: i32, y0: i32, x1: i32, y1: i32, r: i32) -> bool {
    if x < x0 || x >= x1 || y < y0 || y >= y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r - 1);
    let cy = y.clamp(y0 + r, y1 - r - 1);
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
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

pub fn cpu_bg(pct: f32) -> (u8, u8, u8) {
    cpu_color(pct)
}

pub fn temp_bg(t: f32) -> (u8, u8, u8) {
    temp_color(t)
}

pub fn geo_bg() -> (u8, u8, u8) {
    (0x0d, 0x94, 0x88) // teal-600
}

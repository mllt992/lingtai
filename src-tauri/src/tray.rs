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

/// 渲染一个文字托盘图标（透明背景）。
/// - size: 图标边长（像素）
/// - text: 最多 3 个字符，自动居中并按可用空间缩放
/// - color: 文字主色（RGB）；外圈自动加 1px 深色描边保证浅色/深色任务栏都能看清
/// 返回 RGBA buffer。
pub fn render_text_icon(text: &str, size: u32, color: (u8, u8, u8)) -> Vec<u8> {
    let w = size as usize;
    let h = size as usize;
    let mut buf = vec![0u8; w * h * 4]; // 完全透明

    let chars: Vec<char> = text.chars().take(3).collect();
    if chars.is_empty() {
        return buf;
    }
    let n = chars.len();
    let gap_units = 1usize; // 字符间留 1 个 scale 的间隙

    // 留 2px 给描边，避免文字贴边被切
    let avail_w = w.saturating_sub(2);
    let avail_h = h.saturating_sub(2);
    let total_gaps = gap_units * n.saturating_sub(1);
    let per_char_glyphs = (avail_w.saturating_sub(total_gaps) / n) / GLYPH_W;
    let scale_h = avail_h / GLYPH_H;
    let scale = per_char_glyphs.min(scale_h).max(1);

    let text_w = scale * GLYPH_W * n + scale * gap_units * n.saturating_sub(1);
    let text_h = scale * GLYPH_H;
    let start_x = (w.saturating_sub(text_w)) / 2;
    let start_y = (h.saturating_sub(text_h)) / 2;

    // Pass 1：8 邻居方向的深色描边（在任何颜色任务栏上都能凸显文字边缘）
    let outline = (15u8, 15u8, 15u8);
    for &(dx, dy) in &[
        (-1i32, -1i32),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        draw_chars(
            &mut buf,
            w,
            h,
            &chars,
            start_x as i32 + dx,
            start_y as i32 + dy,
            scale,
            gap_units,
            outline,
        );
    }

    // Pass 2：主色文字盖在描边之上
    draw_chars(
        &mut buf,
        w,
        h,
        &chars,
        start_x as i32,
        start_y as i32,
        scale,
        gap_units,
        color,
    );

    buf
}

fn draw_chars(
    buf: &mut [u8],
    w: usize,
    h: usize,
    chars: &[char],
    start_x: i32,
    start_y: i32,
    scale: usize,
    gap_units: usize,
    color: (u8, u8, u8),
) {
    for (i, c) in chars.iter().enumerate() {
        let Some(g) = glyph(*c) else { continue };
        let cx = start_x + (i * (scale * GLYPH_W + scale * gap_units)) as i32;
        for (row, bits) in g.iter().enumerate() {
            for col in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - col)) & 1 == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = cx + (col * scale + dx) as i32;
                        let py = start_y + (row * scale + dy) as i32;
                        if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 {
                            continue;
                        }
                        let idx = ((py as usize) * w + (px as usize)) * 4;
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

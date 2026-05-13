// 托盘 + HUD 指标支持模块
//
// 设计变更：放弃在托盘里挤"CPU/温度/IP"小图标 —— 16x16 注定看不清。
// 改为：
//   · 托盘只保留 LOGO（底部 CPU 进度条）+ 操作菜单
//   · 实时数据通过 emit("metrics") 推给独立的 HUD 小窗口，在屏幕上用真正
//     的 HTML/CSS 渲染（无尺寸约束）
//
// 这个模块负责：
//   - 基础 LOGO 的 RGBA 数据（启动时解码一次）
//   - 给 LOGO 叠 CPU 进度条
//   - 读 CPU 温度（sysinfo Components 兜底）
//   - 拉公网 IP / 国家（curl.exe 多端点回退，缓存 5 分钟）
//   - 主图标 hover tooltip 文本
//   - MetricsPayload 结构（emit 给 HUD）

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

/// 推给 HUD 窗口的实时指标
#[derive(Debug, Clone, Serialize)]
pub struct MetricsPayload {
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub temp: Option<f32>,
    pub country: Option<String>,
    pub ip: Option<String>,
    pub city: Option<String>,
    /// 当 country 为 "CN" 时为 true；country 不明时为 false
    pub is_china: bool,
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
        (0x10, 0xb9, 0x81)
    } else if pct < 70.0 {
        (0xfb, 0xbf, 0x24)
    } else {
        (0xef, 0x44, 0x44)
    }
}

// ============================================================
// CPU 温度
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
// IP / 国家（curl.exe + 多端点回退，5min cache，失败 30s 重试）
// ============================================================
fn geo_slot() -> &'static Mutex<GeoCache> {
    GEO.get_or_init(|| Mutex::new(GeoCache::default()))
}

pub fn current_geo() -> Option<GeoInfo> {
    geo_slot().lock().info.clone()
}

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
// 主图标 hover tooltip
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
        if g.country == "CN" {
            // 在中国：tooltip 里只显示城市，不显示 VPN 标签
            if let Some(city) = &g.city {
                lines.push(format!("CN · {}", city));
            }
        } else {
            // 不在中国：标 VPN
            let mut s = format!("VPN {} · {}", g.country, g.ip);
            if let Some(city) = &g.city {
                s.push_str(&format!(" · {}", city));
            }
            lines.push(s);
        }
    }
    lines.join("\n")
}

// 构造 MetricsPayload
pub fn build_metrics(
    cpu_pct: f32,
    mem_pct: f32,
    temp: Option<f32>,
    geo: Option<&GeoInfo>,
) -> MetricsPayload {
    MetricsPayload {
        cpu_pct,
        mem_pct,
        temp,
        country: geo.map(|g| g.country.clone()),
        ip: geo.map(|g| g.ip.clone()),
        city: geo.and_then(|g| g.city.clone()),
        is_china: geo.map(|g| g.country == "CN").unwrap_or(false),
    }
}

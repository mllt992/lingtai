// 托盘实时指标：CPU 占用条叠加到图标 + 多行 tooltip（CPU / 内存 / 温度 / IP + 国家）
//
// - 基础 LOGO 在程序启动时解码一次，存放在 OnceLock 里
// - 每次刷新克隆基础像素，在底部叠一条彩色进度条（emerald→amber→red 三档）
// - 温度走 sysinfo::Components（Windows 上多数家用机拿不到，会显示 N/A，这是 OS 限制）
// - 公网 IP / 国家通过 PowerShell Invoke-RestMethod 调 ipapi.co / ip-api.com，
//   失败则换备用源；结果缓存 5 分钟

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
}

// ============================================================
// Base icon (lazy decode)
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

/// 给基础图标底部叠一条 CPU 占用进度条，返回新的 RGBA buffer。
pub fn render_icon_with_cpu(cpu_pct: f32) -> Vec<u8> {
    let base = base_icon();
    let mut buf = base.rgba.clone();
    let w = base.width as usize;
    let h = base.height as usize;

    // 进度条占图标高度 18%，底部贴边
    let bar_height = ((h as f32) * 0.18).round() as usize;
    let bar_top = h.saturating_sub(bar_height);
    let pct = cpu_pct.clamp(0.0, 100.0);
    let fill_width = ((pct / 100.0) * (w as f32)).round() as usize;

    // 颜色按档位变化
    let (fr, fg, fb) = if pct < 30.0 {
        (0x10u8, 0xb9u8, 0x81u8) // emerald-500
    } else if pct < 70.0 {
        (0xfbu8, 0xbfu8, 0x24u8) // amber-400
    } else {
        (0xefu8, 0x44u8, 0x44u8) // red-500
    };

    // 进度条背景（半透明深色），上面覆盖填充色
    for y in bar_top..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            if x < fill_width {
                buf[idx]     = fr;
                buf[idx + 1] = fg;
                buf[idx + 2] = fb;
                buf[idx + 3] = 255;
            } else {
                buf[idx]     = 0x1a;
                buf[idx + 1] = 0x1a;
                buf[idx + 2] = 0x1a;
                buf[idx + 3] = 200;
            }
        }
    }
    buf
}

// ============================================================
// Temperature (best-effort via sysinfo Components)
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
        // 优先用 "cpu" / "core" / "package" 等明确 CPU 相关传感器
        if label.contains("cpu")
            || label.contains("package")
            || label.contains("core")
            || label.contains("tctl")
            || label.contains("tdie")
        {
            return Some(t);
        }
        // 兜底：取所有合理读数的最大值
        best = Some(match best {
            None => t,
            Some(b) => b.max(t),
        });
    }
    best
}

// ============================================================
// IP / Country / City (cached, refresh every 5min)
// ============================================================
fn geo_slot() -> &'static Mutex<GeoCache> {
    GEO.get_or_init(|| Mutex::new(GeoCache::default()))
}

pub fn current_geo() -> Option<GeoInfo> {
    geo_slot().lock().info.clone()
}

/// 若上次成功获取距今 < 5min，直接跳过。
pub fn maybe_refresh_geo() {
    {
        let cache = geo_slot().lock();
        if let Some(t) = cache.fetched_at {
            if t.elapsed() < Duration::from_secs(300) && cache.info.is_some() {
                return;
            }
        }
    }
    if let Some(info) = fetch_geo() {
        let mut cache = geo_slot().lock();
        cache.info = Some(info);
        cache.fetched_at = Some(Instant::now());
    } else {
        // 失败也写入时间戳，避免每次循环都重试
        let mut cache = geo_slot().lock();
        cache.fetched_at = Some(Instant::now());
    }
}

fn fetch_geo() -> Option<GeoInfo> {
    fetch_via_ipapi_co().or_else(fetch_via_ip_api_com)
}

#[cfg(windows)]
fn run_powershell_json(script: &str) -> Option<serde_json::Value> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }
    serde_json::from_str(&text).ok()
}

#[cfg(not(windows))]
fn run_powershell_json(_script: &str) -> Option<serde_json::Value> {
    None
}

fn fetch_via_ipapi_co() -> Option<GeoInfo> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
try {
  $r = Invoke-RestMethod -Uri 'https://ipapi.co/json/' -TimeoutSec 4
  if ($r.ip) {
    $obj = [ordered]@{ ip = $r.ip; country = $r.country_code; city = $r.city }
    $obj | ConvertTo-Json -Compress
  }
} catch { }
"#;
    let v = run_powershell_json(script)?;
    Some(GeoInfo {
        ip: v.get("ip")?.as_str()?.to_string(),
        country: v.get("country")?.as_str()?.to_string(),
        city: v.get("city").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from),
    })
}

fn fetch_via_ip_api_com() -> Option<GeoInfo> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
try {
  $r = Invoke-RestMethod -Uri 'https://ip-api.com/json/?fields=status,countryCode,city,query' -TimeoutSec 4
  if ($r.status -eq 'success') {
    $obj = [ordered]@{ ip = $r.query; country = $r.countryCode; city = $r.city }
    $obj | ConvertTo-Json -Compress
  }
} catch { }
"#;
    let v = run_powershell_json(script)?;
    Some(GeoInfo {
        ip: v.get("ip")?.as_str()?.to_string(),
        country: v.get("country")?.as_str()?.to_string(),
        city: v.get("city").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from),
    })
}

// ============================================================
// Tooltip formatting
// ============================================================
pub fn format_tooltip(
    cpu_pct: f32,
    mem_pct: f32,
    temp: Option<f32>,
    geo: Option<&GeoInfo>,
) -> String {
    let cpu_line = format!("CPU {:>4.0}%  内存 {:>4.0}%", cpu_pct, mem_pct);
    let temp_line = match temp {
        Some(t) => format!("温度 {:.0} °C", t),
        None => "温度 N/A".to_string(),
    };
    let geo_line = match geo {
        Some(g) => {
            let mut s = format!("{} · {}", g.country, g.ip);
            if let Some(city) = &g.city {
                s.push_str(&format!(" · {}", city));
            }
            s
        }
        None => "IP 加载中…".to_string(),
    };
    format!("凌台 · Loft\n{}\n{}\n{}", cpu_line, temp_line, geo_line)
}

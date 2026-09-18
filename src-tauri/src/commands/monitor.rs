use crate::state::AppState;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, Disks};
use tauri::State;

const CPU_META_TTL: Duration = Duration::from_secs(30);
const GPU_STATIC_TTL: Duration = Duration::from_secs(60);

#[derive(Serialize, Clone)]
pub struct CpuSnapshot {
    pub total: f32,
    pub per_core: Vec<f32>,
    pub brand: String,
    pub frequency_mhz: u64,
    pub cores: usize,
}

#[derive(Serialize, Clone)]
pub struct MemSnapshot {
    pub used: u64,
    pub total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

#[derive(Serialize, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub utilization: Option<f32>,
    pub mem_used: Option<u64>,
    pub mem_total: Option<u64>,
    pub vendor: String,
}

#[derive(Serialize, Clone)]
pub struct SystemSnapshot {
    pub cpu: CpuSnapshot,
    pub mem: MemSnapshot,
    pub gpus: Vec<GpuInfo>,
    pub uptime_secs: u64,
}

#[derive(Serialize, Clone)]
pub struct DriveInfo {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub available: u64,
    pub file_system: String,
    pub kind: String,
}

#[derive(Clone)]
struct CpuMeta {
    brand: String,
    frequency_mhz: u64,
    cores: usize,
    collected_at: Instant,
}

struct GpuStaticCache {
    list: Vec<GpuInfo>,
    collected_at: Option<Instant>,
}

static CPU_META: Lazy<Mutex<Option<CpuMeta>>> = Lazy::new(|| Mutex::new(None));
static GPU_STATIC: Lazy<Mutex<GpuStaticCache>> = Lazy::new(|| {
    Mutex::new(GpuStaticCache {
        list: Vec::new(),
        collected_at: None,
    })
});

#[cfg(feature = "gpu-nvml")]
static NVML: Lazy<Mutex<Option<nvml_wrapper::Nvml>>> = Lazy::new(|| Mutex::new(None));

#[tauri::command]
pub fn get_system_snapshot(state: State<'_, AppState>) -> Result<SystemSnapshot, String> {
    let cpu_meta = resolve_cpu_meta(&state);
    let (total, per_core, mem) = {
        let mut sys = state.sys.lock();
        // 轮询路径只刷 usage/memory，避免 everything() 的额外开销
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let total = sys.global_cpu_usage();
        let mem = MemSnapshot {
            used: sys.used_memory(),
            total: sys.total_memory(),
            swap_used: sys.used_swap(),
            swap_total: sys.total_swap(),
        };
        (total, per_core, mem)
    };

    // 锁外采集 GPU，避免托盘线程被长操作堵住
    let gpus = collect_gpus();

    Ok(SystemSnapshot {
        cpu: CpuSnapshot {
            total,
            per_core,
            brand: cpu_meta.brand,
            frequency_mhz: cpu_meta.frequency_mhz,
            cores: cpu_meta.cores,
        },
        mem,
        gpus,
        uptime_secs: sysinfo::System::uptime(),
    })
}

fn resolve_cpu_meta(state: &AppState) -> CpuMeta {
    {
        let guard = CPU_META.lock();
        if let Some(meta) = guard.as_ref() {
            if meta.collected_at.elapsed() < CPU_META_TTL {
                return meta.clone();
            }
        }
    }

    let (brand, frequency_mhz, cores) = {
        let mut sys = state.sys.lock();
        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_frequency());
        let first = sys.cpus().first();
        let brand = first.map(|c| c.brand().trim().to_string()).unwrap_or_default();
        let frequency_mhz = first.map(|c| c.frequency()).unwrap_or(0);
        let cores = sys.cpus().len();
        (brand, frequency_mhz, cores)
    };

    let meta = CpuMeta {
        brand,
        frequency_mhz,
        cores,
        collected_at: Instant::now(),
    };
    *CPU_META.lock() = Some(meta.clone());
    meta
}

fn get_static_gpus_cached() -> Vec<GpuInfo> {
    {
        let cache = GPU_STATIC.lock();
        if let Some(at) = cache.collected_at {
            if at.elapsed() < GPU_STATIC_TTL {
                return cache.list.clone();
            }
        }
    }

    let fresh = collect_static_gpus();
    let mut cache = GPU_STATIC.lock();
    cache.list = fresh.clone();
    cache.collected_at = Some(Instant::now());
    fresh
}

#[cfg(feature = "gpu-nvml")]
fn collect_gpus() -> Vec<GpuInfo> {
    let mut guard = NVML.lock();
    if guard.is_none() {
        *guard = nvml_wrapper::Nvml::init().ok();
    }
    let Some(nvml) = guard.as_ref() else {
        return get_static_gpus_cached();
    };

    let count = nvml.device_count().unwrap_or(0);
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        if let Ok(dev) = nvml.device_by_index(i) {
            let name = dev.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
            let util = dev.utilization_rates().ok().map(|u| u.gpu as f32);
            let mem = dev.memory_info().ok();
            out.push(GpuInfo {
                name,
                utilization: util,
                mem_used: mem.as_ref().map(|m| m.used),
                mem_total: mem.as_ref().map(|m| m.total),
                vendor: "NVIDIA".to_string(),
            });
        }
    }
    if out.is_empty() {
        return get_static_gpus_cached();
    }
    out
}

#[cfg(not(feature = "gpu-nvml"))]
fn collect_gpus() -> Vec<GpuInfo> {
    get_static_gpus_cached()
}

/// 静态 GPU 信息（名称）。不 spawn 任何外部进程。
#[cfg(windows)]
fn collect_static_gpus() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};

    fn wchar_to_string(buf: &[u16]) -> String {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len]).trim().to_string()
    }

    let mut out = Vec::new();
    let mut index = 0u32;
    loop {
        let mut dd = DISPLAY_DEVICEW::default();
        dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let ok = unsafe { EnumDisplayDevicesW(None, index, &mut dd, 0) };
        if !ok.as_bool() {
            break;
        }
        let name = wchar_to_string(&dd.DeviceString);
        index += 1;
        if name.is_empty() {
            continue;
        }
        out.push(GpuInfo {
            name,
            utilization: None,
            mem_used: None,
            mem_total: None,
            vendor: "Unknown".to_string(),
        });
    }
    out
}

#[cfg(not(windows))]
fn collect_static_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

#[tauri::command]
pub fn list_drives() -> Result<Vec<DriveInfo>, String> {
    let disks = Disks::new_with_refreshed_list();
    let out = disks
        .iter()
        .map(|d| DriveInfo {
            name: d.name().to_string_lossy().to_string(),
            mount: d.mount_point().to_string_lossy().to_string(),
            total: d.total_space(),
            available: d.available_space(),
            file_system: d.file_system().to_string_lossy().to_string(),
            kind: format!("{:?}", d.kind()),
        })
        .collect();
    Ok(out)
}

use crate::state::AppState;
use serde::Serialize;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind};
use tauri::State;

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

#[tauri::command]
pub fn get_system_snapshot(state: State<'_, AppState>) -> Result<SystemSnapshot, String> {
    let mut sys = state.sys.lock();
    sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    sys.refresh_memory_specifics(MemoryRefreshKind::everything());

    let per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let frequency_mhz = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
    let cores = sys.cpus().len();

    let cpu = CpuSnapshot {
        total: sys.global_cpu_usage(),
        per_core,
        brand,
        frequency_mhz,
        cores,
    };

    let mem = MemSnapshot {
        used: sys.used_memory(),
        total: sys.total_memory(),
        swap_used: sys.used_swap(),
        swap_total: sys.total_swap(),
    };

    let gpus = collect_gpus();

    Ok(SystemSnapshot {
        cpu,
        mem,
        gpus,
        uptime_secs: sysinfo::System::uptime(),
    })
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

#[cfg(feature = "gpu-nvml")]
fn collect_gpus() -> Vec<GpuInfo> {
    use nvml_wrapper::Nvml;
    let nvml = match Nvml::init() {
        Ok(n) => n,
        Err(_) => return Vec::new(),
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
    out
}

#[cfg(all(not(feature = "gpu-nvml"), windows))]
fn collect_gpus() -> Vec<GpuInfo> {
    // Best-effort fallback: query WMI Win32_VideoController via PowerShell
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = "Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM | ConvertTo-Json -Compress";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(out) = output else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    // Result may be single object or array; normalize to array
    let val: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr: Vec<serde_json::Value> = match val {
        serde_json::Value::Array(a) => a,
        other => vec![other],
    };
    arr.into_iter()
        .filter_map(|v| {
            let name = v.get("Name")?.as_str()?.to_string();
            let mem_total = v
                .get("AdapterRAM")
                .and_then(|x| x.as_u64());
            Some(GpuInfo {
                name,
                utilization: None,
                mem_used: None,
                mem_total,
                vendor: "Unknown".to_string(),
            })
        })
        .collect()
}

#[cfg(all(not(feature = "gpu-nvml"), not(windows)))]
fn collect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

use parking_lot::Mutex;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

pub struct AppState {
    pub sys: Mutex<System>,
}

impl AppState {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(sysinfo::MemoryRefreshKind::everything())
                .with_processes(sysinfo::ProcessRefreshKind::new()),
        );
        sys.refresh_all();
        Self { sys: Mutex::new(sys) }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

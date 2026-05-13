use crate::state::AppState;
use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};
use tauri::State;

#[derive(Serialize, Clone)]
pub struct PortEntry {
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    pub is_system: bool,
}

const SYSTEM_PROCESS_NAMES: &[&str] = &[
    "System",
    "Idle",
    "svchost.exe",
    "services.exe",
    "lsass.exe",
    "csrss.exe",
    "smss.exe",
    "winlogon.exe",
    "wininit.exe",
    "spoolsv.exe",
    "Registry",
    "Memory Compression",
    "RuntimeBroker.exe",
    "dwm.exe",
];

fn is_system_process(name: &str, pid: u32) -> bool {
    if pid <= 4 {
        return true;
    }
    SYSTEM_PROCESS_NAMES
        .iter()
        .any(|sys_name| name.eq_ignore_ascii_case(sys_name))
}

#[tauri::command]
pub fn list_user_ports(
    state: State<'_, AppState>,
    include_system: Option<bool>,
) -> Result<Vec<PortEntry>, String> {
    let include_sys = include_system.unwrap_or(false);

    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let family_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = get_sockets_info(family_flags, proto_flags)
        .map_err(|e| format!("无法获取端口列表: {e}"))?;

    let mut sys = state.sys.lock();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always),
    );

    let mut out: Vec<PortEntry> = Vec::with_capacity(sockets.len());
    for s in sockets {
        let (protocol, local_addr, local_port, remote_addr, remote_port, state_str) =
            match &s.protocol_socket_info {
                ProtocolSocketInfo::Tcp(t) => {
                    // Only show listening sockets to keep list focused on "occupied" ports
                    if t.state != TcpState::Listen {
                        continue;
                    }
                    (
                        "TCP".to_string(),
                        t.local_addr.to_string(),
                        t.local_port,
                        Some(t.remote_addr.to_string()),
                        Some(t.remote_port),
                        format!("{:?}", t.state),
                    )
                }
                ProtocolSocketInfo::Udp(u) => (
                    "UDP".to_string(),
                    u.local_addr.to_string(),
                    u.local_port,
                    None,
                    None,
                    "Listening".to_string(),
                ),
            };

        let pid = s.associated_pids.first().copied();
        let (process_name, process_path, is_sys) = match pid {
            Some(p) if p > 0 => {
                if let Some(proc) = sys.process(Pid::from_u32(p)) {
                    let name = proc.name().to_string_lossy().to_string();
                    let path = proc.exe().map(|x| x.to_string_lossy().to_string());
                    let sys_flag = is_system_process(&name, p);
                    (Some(name), path, sys_flag)
                } else {
                    (None, None, false)
                }
            }
            _ => (None, None, false),
        };

        if !include_sys && is_sys {
            continue;
        }

        out.push(PortEntry {
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state: state_str,
            pid,
            process_name,
            process_path,
            is_system: is_sys,
        });
    }

    out.sort_by(|a, b| {
        a.protocol
            .cmp(&b.protocol)
            .then(a.local_port.cmp(&b.local_port))
    });
    Ok(out)
}

#[tauri::command]
pub fn kill_process(state: State<'_, AppState>, pid: u32) -> Result<bool, String> {
    let mut sys = state.sys.lock();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let Some(proc) = sys.process(Pid::from_u32(pid)) else {
        return Err(format!("未找到进程 PID={pid}"));
    };
    if is_system_process(&proc.name().to_string_lossy(), pid) {
        return Err("禁止终止系统进程".into());
    }
    Ok(proc.kill())
}

// 启动诊断 & 崩溃日志
//
// release 构建用 windows_subsystem = "windows" → 没有控制台 → eprintln 看不到。
// 这里提供两条诊断路径：
//   1) AttachConsole(ATTACH_PARENT_PROCESS)：从 PowerShell 跑 exe 时 stderr 会接到那个终端
//   2) 写文件日志：每条 log_step 都写一行到 %APPDATA%\com.loft.app\boot.log
//      panic 触发时把完整 panic 信息 + backtrace 写进去
// 用户下次再闪退，看这个文件就知道死在哪一步。

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub fn log_file_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|p| {
        PathBuf::from(p).join("com.loft.app").join("boot.log")
    })
}

fn append_line(line: &str) {
    if let Some(p) = log_file_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(f, "[{}] {}", ts, line);
        }
    }
}

/// 启动各阶段打点，方便定位卡在哪
pub fn log_step(msg: &str) {
    eprintln!("[Loft] {}", msg);
    append_line(msg);
}

/// 安装 panic 钩子：吞掉 lnk crate 的已知问题，其他 panic 写入崩溃日志
pub fn install_crash_logger() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // 已知：lnk 0.5.1 解析某些 .lnk 时硬 unwrap None，我们已经 catch_unwind 兜住，
        // 这里直接静默掉它的 panic 噪声
        if let Some(loc) = info.location() {
            if loc.file().contains("lnk-0.5") {
                return;
            }
        }

        // 写崩溃日志
        if let Some(p) = log_file_path() {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
                let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(f, "\n[{}] === PANIC ===", ts);
                let _ = writeln!(f, "{}", info);
                if let Some(loc) = info.location() {
                    let _ = writeln!(
                        f,
                        "  at {}:{}:{}",
                        loc.file(),
                        loc.line(),
                        loc.column()
                    );
                }
                let bt = std::backtrace::Backtrace::force_capture();
                let _ = writeln!(f, "Backtrace:\n{}", bt);
            }
        }

        // 同时让原来的 hook 也跑一遍（dev 模式下打到 stderr）
        prev(info);
    }));
}

/// Windows：如果从已有控制台启动，把 stderr/stdout 接到那个控制台
#[cfg(windows)]
pub fn attach_console() {
    unsafe {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
pub fn attach_console() {}

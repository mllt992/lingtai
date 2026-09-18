use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const MAX_NAME_CHARS: usize = 255;
const MAX_VALUE_CHARS: usize = 32767;

#[derive(Serialize, Clone)]
pub struct EnvVarRow {
    pub name: String,
    pub scope: String,
    pub kind: String,
}

#[derive(Deserialize)]
struct ElevatePayload {
    op: String,
    scope: String,
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Serialize)]
struct ElevateResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn elevate_payload_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--elevate-env-write") {
        Some(PathBuf::from(args.get(2).cloned().unwrap_or_default()))
    } else {
        None
    }
}

pub fn run_elevate_helper(payload_path: &Path) -> Result<(), String> {
    let result_file = result_path(payload_path);
    let outcome = (|| {
        if payload_path.as_os_str().is_empty() {
            return Err("缺少提权载荷路径".to_string());
        }
        if !is_allowed_payload_path(payload_path) {
            return Err("提权载荷路径不合法".to_string());
        }
        let text = fs::read_to_string(payload_path)
            .map_err(|e| format!("读取提权载荷失败: {e}"))?;
        let payload: ElevatePayload = serde_json::from_str(&text)
            .map_err(|e| format!("解析提权载荷失败: {e}"))?;
        if payload.scope != "machine" {
            return Err("提权通道只允许修改系统级环境变量".to_string());
        }
        match payload.op.as_str() {
            "upsert" => write_scope("machine", &payload.name, &payload.value),
            "delete" => delete_scope("machine", &payload.name),
            other => Err(format!("不支持的提权操作: {other}")),
        }
    })();

    let body = match &outcome {
        Ok(()) => ElevateResult {
            ok: true,
            error: None,
        },
        Err(e) => ElevateResult {
            ok: false,
            error: Some(e.clone()),
        },
    };
    if let Ok(text) = serde_json::to_string(&body) {
        let _ = fs::write(&result_file, text);
    }
    outcome
}

fn result_path(payload: &Path) -> PathBuf {
    payload.with_extension("result.json")
}

fn is_allowed_payload_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !name.starts_with("loft-env-elevate-") || !name.ends_with(".json") {
        return false;
    }
    match fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.len() <= 64 * 1024,
        Err(_) => false,
    }
}

pub fn validate_env_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("变量名不能为空".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!("变量名不能超过 {MAX_NAME_CHARS} 个字符"));
    }
    if name.contains('=') || name.contains('\0') || name.contains('\n') || name.contains('\r') {
        return Err("变量名不能包含 = 或换行".into());
    }
    Ok(())
}

fn validate_value(value: &str) -> Result<(), String> {
    if value.chars().count() > MAX_VALUE_CHARS {
        return Err(format!("变量值不能超过 {MAX_VALUE_CHARS} 个字符"));
    }
    if value.contains('\0') {
        return Err("变量值不能包含空字符".into());
    }
    Ok(())
}

#[cfg(test)]
pub fn split_path_value(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
pub fn join_path_value(entries: &[String]) -> String {
    entries
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(";")
}

fn env_name_eq(a: &str, b: &str) -> bool {
    #[cfg(windows)]
    {
        a.eq_ignore_ascii_case(b)
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

fn looks_expand(value: &str) -> bool {
    let first = value.find('%');
    match first {
        Some(i) => value[i + 1..].contains('%'),
        None => false,
    }
}

fn scope_rank(scope: &str) -> u8 {
    match scope {
        "user" => 0,
        "machine" => 1,
        "process" => 2,
        _ => 9,
    }
}

#[tauri::command]
pub fn list_env_vars(scopes: Vec<String>) -> Result<Vec<EnvVarRow>, String> {
    let mut rows = Vec::new();
    for scope in &scopes {
        match scope.as_str() {
            "user" | "machine" => {
                #[cfg(windows)]
                {
                    rows.extend(list_registry_scope(scope)?);
                }
                #[cfg(not(windows))]
                {
                    let _ = scope;
                }
            }
            "process" => rows.extend(list_process_scope()),
            other => return Err(format!("未知范围: {other}")),
        }
    }
    rows.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then_with(|| scope_rank(&a.scope).cmp(&scope_rank(&b.scope)))
    });
    Ok(rows)
}

#[tauri::command]
pub fn get_env_value(name: String, scope: String) -> Result<String, String> {
    validate_env_name(&name)?;
    match scope.as_str() {
        "user" | "machine" => {
            #[cfg(windows)]
            {
                read_registry_value(&scope, &name)
            }
            #[cfg(not(windows))]
            {
                Err("当前系统不支持读取用户/系统环境变量".into())
            }
        }
        "process" => read_process_value(&name),
        other => Err(format!("未知范围: {other}")),
    }
}

#[tauri::command]
pub fn upsert_env_var(name: String, scope: String, value: String) -> Result<(), String> {
    write_scope(&scope, &name, &value)
}

#[tauri::command]
pub fn delete_env_var(name: String, scope: String) -> Result<(), String> {
    delete_scope(&scope, &name)
}

fn write_scope(scope: &str, name: &str, value: &str) -> Result<(), String> {
    validate_env_name(name)?;
    validate_value(value)?;
    let name = name.trim();
    match scope {
        "user" => {
            #[cfg(windows)]
            {
                write_registry_value("user", name, value)?;
                broadcast_environment();
                Ok(())
            }
            #[cfg(not(windows))]
            {
                Err("当前系统不支持修改用户/系统环境变量".into())
            }
        }
        "machine" => {
            #[cfg(windows)]
            {
                if is_process_elevated() {
                    write_registry_value("machine", name, value)?;
                    broadcast_environment();
                    Ok(())
                } else {
                    elevate_machine("upsert", name, Some(value))
                }
            }
            #[cfg(not(windows))]
            {
                Err("当前系统不支持修改用户/系统环境变量".into())
            }
        }
        "process" => set_process_env(name, Some(value)),
        other => Err(format!("未知范围: {other}")),
    }
}

fn delete_scope(scope: &str, name: &str) -> Result<(), String> {
    validate_env_name(name)?;
    let name = name.trim();
    match scope {
        "user" => {
            #[cfg(windows)]
            {
                delete_registry_value("user", name)?;
                broadcast_environment();
                Ok(())
            }
            #[cfg(not(windows))]
            {
                Err("当前系统不支持修改用户/系统环境变量".into())
            }
        }
        "machine" => {
            #[cfg(windows)]
            {
                if is_process_elevated() {
                    delete_registry_value("machine", name)?;
                    broadcast_environment();
                    Ok(())
                } else {
                    elevate_machine("delete", name, None)
                }
            }
            #[cfg(not(windows))]
            {
                Err("当前系统不支持修改用户/系统环境变量".into())
            }
        }
        "process" => set_process_env(name, None),
        other => Err(format!("未知范围: {other}")),
    }
}

fn list_process_scope() -> Vec<EnvVarRow> {
    std::env::vars_os()
        .filter_map(|(k, _)| {
            let name = k.to_string_lossy();
            if name.is_empty() || name.starts_with('=') {
                return None;
            }
            Some(EnvVarRow {
                name: name.into_owned(),
                scope: "process".into(),
                kind: "process".into(),
            })
        })
        .collect()
}

fn read_process_value(name: &str) -> Result<String, String> {
    for (k, v) in std::env::vars_os() {
        let kn = k.to_string_lossy();
        if env_name_eq(&kn, name) {
            return Ok(v.to_string_lossy().into_owned());
        }
    }
    Err(format!("变量不存在: {name}"))
}

fn set_process_env(name: &str, value: Option<&str>) -> Result<(), String> {
    #[cfg(windows)]
    {
        set_process_env_windows(name, value)
    }
    #[cfg(not(windows))]
    {
        match value {
            Some(v) => std::env::set_var(name, v),
            None => std::env::remove_var(name),
        }
        Ok(())
    }
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取配置目录失败: {e}"))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    Ok(dir)
}

#[tauri::command]
pub fn load_env_notes(app: AppHandle) -> Result<Value, String> {
    let path = config_dir(&app)?.join("env-notes.json");
    if !path.exists() {
        return Ok(serde_json::json!({ "version": 1, "notes": {} }));
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取备注失败: {e}"))?;
    if text.trim().is_empty() {
        return Ok(serde_json::json!({ "version": 1, "notes": {} }));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析备注失败: {e}"))
}

#[tauri::command]
pub fn save_env_notes(app: AppHandle, notes: Value) -> Result<(), String> {
    let path = config_dir(&app)?.join("env-notes.json");
    let text = serde_json::to_string_pretty(&notes).map_err(|e| format!("序列化备注失败: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入备注失败: {e}"))
}

#[cfg(windows)]
fn list_registry_scope(scope: &str) -> Result<Vec<EnvVarRow>, String> {
    Ok(read_registry_all(scope)?
        .into_iter()
        .map(|(name, _, kind)| EnvVarRow {
            name,
            scope: scope.to_string(),
            kind,
        })
        .collect())
}

#[cfg(windows)]
fn read_registry_value(scope: &str, name: &str) -> Result<String, String> {
    let entries = read_registry_all(scope)?;
    entries
        .into_iter()
        .find(|(n, _, _)| env_name_eq(n, name))
        .map(|(_, v, _)| v)
        .ok_or_else(|| format!("变量不存在: {name}"))
}

#[cfg(windows)]
fn read_registry_all(scope: &str) -> Result<Vec<(String, String, String)>, String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumValueW, RegOpenKeyExW, HKEY, KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };

    let (root, subkey) = registry_target(scope)?;
    unsafe {
        let mut key = HKEY::default();
        let err = RegOpenKeyExW(root, subkey, 0, KEY_READ, &mut key);
        if err != ERROR_SUCCESS {
            return Err(format!("打开{scope}环境变量失败: {}", err.0));
        }
        let mut out = Vec::new();
        let mut index = 0u32;
        let mut name_buf = vec![0u16; 256];
        let mut data = vec![0u8; 4096];
        loop {
            let mut name_len = name_buf.len() as u32;
            let mut ty = 0u32;
            let mut data_len = data.len() as u32;
            name_buf.fill(0);
            let err = RegEnumValueW(
                key,
                index,
                windows::core::PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                Some(&mut ty as *mut u32),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            );
            if err == windows::Win32::Foundation::ERROR_NO_MORE_ITEMS {
                break;
            }
            if err == windows::Win32::Foundation::ERROR_MORE_DATA {
                if (name_len as usize) + 1 > name_buf.len() {
                    name_buf.resize((name_len as usize) + 8, 0);
                }
                if (data_len as usize) > data.len() {
                    data.resize((data_len as usize) + 8, 0);
                }
                continue;
            }
            if err != ERROR_SUCCESS {
                let _ = RegCloseKey(key);
                return Err(format!("枚举{scope}环境变量失败: {}", err.0));
            }
            if ty == REG_SZ.0 || ty == REG_EXPAND_SZ.0 {
                let end = (name_len as usize).min(name_buf.len());
                let n = String::from_utf16_lossy(&name_buf[..end]);
                let v = from_reg_bytes(&data[..data_len as usize]);
                if !n.is_empty() {
                    out.push((
                        n,
                        v,
                        if ty == REG_EXPAND_SZ.0 {
                            "expand_sz".into()
                        } else {
                            "sz".into()
                        },
                    ));
                }
            }
            index += 1;
        }
        let _ = RegCloseKey(key);
        Ok(out)
    }
}

#[cfg(windows)]
fn registry_target(
    scope: &str,
) -> Result<
    (
        windows::Win32::System::Registry::HKEY,
        windows::core::PCWSTR,
    ),
    String,
> {
    use windows::core::w;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    match scope {
        "user" => Ok((HKEY_CURRENT_USER, w!("Environment"))),
        "machine" => Ok((
            HKEY_LOCAL_MACHINE,
            w!("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"),
        )),
        _ => Err(format!("未知范围: {scope}")),
    }
}

#[cfg(windows)]
fn write_registry_value(scope: &str, name: &str, value: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ,
    };

    let (root, subkey) = registry_target(scope)?;
    let ty = if looks_expand(value) {
        REG_EXPAND_SZ
    } else {
        REG_SZ
    };
    let data = utf16_bytes(value);
    let name_hs = HSTRING::from(name);
    unsafe {
        let mut key = HKEY::default();
        let err = RegOpenKeyExW(root, subkey, 0, KEY_SET_VALUE, &mut key);
        if err != ERROR_SUCCESS {
            return Err(format!("打开{scope}环境变量写入失败: {}", err.0));
        }
        let err = RegSetValueExW(key, &name_hs, 0, ty, Some(&data));
        let _ = RegCloseKey(key);
        if err != ERROR_SUCCESS {
            return Err(format!("写入 {name} 失败: {}", err.0));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn delete_registry_value(scope: &str, name: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, KEY_SET_VALUE,
    };

    let (root, subkey) = registry_target(scope)?;
    let name_hs = HSTRING::from(name);
    unsafe {
        let mut key = HKEY::default();
        let err = RegOpenKeyExW(root, subkey, 0, KEY_SET_VALUE, &mut key);
        if err != ERROR_SUCCESS {
            return Err(format!("打开{scope}环境变量失败: {}", err.0));
        }
        let err = RegDeleteValueW(key, &name_hs);
        let _ = RegCloseKey(key);
        if err != ERROR_SUCCESS && err != ERROR_FILE_NOT_FOUND {
            return Err(format!("删除 {name} 失败: {}", err.0));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn from_reg_bytes(data: &[u8]) -> String {
    if data.len() < 2 {
        return String::new();
    }
    let mut units: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

#[cfg(windows)]
fn utf16_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect()
}

#[cfg(windows)]
fn broadcast_environment() {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    unsafe {
        let mut result = 0usize;
        let env_name = w!("Environment");
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(env_name.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            Some(&mut result),
        );
    }
}

#[cfg(windows)]
fn is_process_elevated() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

#[cfg(windows)]
fn set_process_env_windows(name: &str, value: Option<&str>) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Environment::SetEnvironmentVariableW;

    let name_hs = HSTRING::from(name);
    let result = unsafe {
        match value {
            Some(v) => {
                let value_hs = HSTRING::from(v);
                SetEnvironmentVariableW(&name_hs, &value_hs)
            }
            None => SetEnvironmentVariableW(&name_hs, None),
        }
    };
    result.map_err(|e| format!("修改进程环境变量失败: {e}"))
}

#[cfg(windows)]
fn elevate_machine(op: &str, name: &str, value: Option<&str>) -> Result<(), String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::WaitForSingleObject;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    crate::diag::log_step(&format!("env elevate: op={op} name={name} scope=machine"));

    let payload = serde_json::json!({
        "op": op,
        "scope": "machine",
        "name": name,
        "value": value.unwrap_or(""),
    });
    let temp = std::env::temp_dir().join(format!(
        "loft-env-elevate-{}-{}.json",
        std::process::id(),
        chrono::Local::now().timestamp_millis()
    ));
    fs::write(&temp, payload.to_string()).map_err(|e| format!("写入提权载荷失败: {e}"))?;

    let exe = std::env::current_exe().map_err(|e| format!("无法定位自身: {e}"))?;
    let params = format!("--elevate-env-write \"{}\"", temp.display());
    let exe_hs = windows::core::HSTRING::from(exe.to_string_lossy().as_ref());
    let params_hs = windows::core::HSTRING::from(params.as_str());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(exe_hs.as_ptr()),
        lpParameters: PCWSTR(params_hs.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    let launched = unsafe { ShellExecuteExW(&mut info) };
    match launched {
        Ok(()) => {}
        Err(e) => {
            let _ = fs::remove_file(&temp);
            if is_uac_cancelled(&e) {
                return Err("已取消提权".into());
            }
            return Err(format!("无法拉起提权进程: {e}"));
        }
    }

    let wait_ms = 60_000u32;
    unsafe {
        if !info.hProcess.is_invalid() {
            let wait = WaitForSingleObject(info.hProcess, wait_ms);
            let _ = CloseHandle(info.hProcess);
            if wait == WAIT_TIMEOUT {
                let _ = fs::remove_file(&temp);
                return Err("提权进程超时".into());
            }
        }
    }

    let result_file = result_path(&temp);
    let text = fs::read_to_string(&result_file);
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&result_file);
    match text {
        Ok(t) => {
            let parsed: Value =
                serde_json::from_str(&t).map_err(|e| format!("解析提权结果失败: {e}"))?;
            if parsed.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                broadcast_environment();
                Ok(())
            } else {
                let msg = parsed
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("提权写入失败");
                Err(msg.to_string())
            }
        }
        Err(_) => Err("提权进程未返回结果".into()),
    }
}

#[cfg(windows)]
fn is_uac_cancelled(e: &windows::core::Error) -> bool {
    let code = e.code().0 as u32;
    (code & 0xFFFF) == 1223
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_names() {
        assert!(validate_env_name("").is_err());
        assert!(validate_env_name("FOO=BAR").is_err());
        assert!(validate_env_name("OK").is_ok());
    }

    #[test]
    fn path_roundtrip() {
        let v = "C:\\Windows;C:\\Tools;; ;D:\\bin";
        let parts = split_path_value(v);
        assert_eq!(parts, vec!["C:\\Windows", "C:\\Tools", "D:\\bin"]);
        assert_eq!(join_path_value(&parts), "C:\\Windows;C:\\Tools;D:\\bin");
    }

    #[test]
    fn expand_detection() {
        assert!(looks_expand("%SystemRoot%\\system32"));
        assert!(!looks_expand("C:\\Windows"));
        assert!(!looks_expand("100%"));
    }

    #[test]
    fn payload_path_must_be_named_json() {
        let dir = std::env::temp_dir();
        let good = dir.join("loft-env-elevate-unit.json");
        fs::write(&good, "{}").unwrap();
        assert!(is_allowed_payload_path(&good));
        let _ = fs::remove_file(&good);
        assert!(!is_allowed_payload_path(Path::new("loft-env-elevate-x.json")));
        assert!(!is_allowed_payload_path(&dir.join("other.json")));
    }
}

// 从 Windows 可执行文件 / 快捷方式抽取关联图标，返回 base64-encoded PNG（data URL）。
// 实现走 PowerShell + System.Drawing.Icon —— 零额外 crate 依赖、覆盖最广、兼容
// 32位/64位 PE、.lnk、.url、关联文件等所有情形。
//
// 性能：单次抽取约 150–250 ms，前端按需调用并缓存到 item.iconData 后即不再调用。

#[cfg(windows)]
#[tauri::command]
pub fn extract_icon(path: String) -> Result<Option<String>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 防御性 path 长度与单引号转义（PowerShell 单引号字符串只需双写单引号）
    if path.is_empty() {
        return Ok(None);
    }
    let safe = path.replace('\'', "''");

    // 脚本逻辑：
    // - 若是 .lnk：尝试通过 WScript.Shell COM 读 TargetPath，
    //   再对目标 exe 调用 ExtractAssociatedIcon；失败则退回对 .lnk 本身抽取
    // - 若是其他文件：直接 ExtractAssociatedIcon
    // - 抽到的 Icon -> Bitmap -> MemoryStream(PNG) -> Base64
    let script = format!(
        r#"
$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Drawing | Out-Null
try {{
    $p = '{safe}'
    $icon = $null
    if ([System.IO.Path]::GetExtension($p).ToLower() -eq '.lnk') {{
        $shell = New-Object -ComObject WScript.Shell
        $link = $shell.CreateShortcut($p)
        $t = $link.TargetPath
        if ($t -and (Test-Path -LiteralPath $t)) {{
            $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($t)
        }}
    }}
    if (-not $icon) {{
        if (Test-Path -LiteralPath $p) {{
            $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($p)
        }}
    }}
    if ($icon) {{
        $bmp = $icon.ToBitmap()
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        [Convert]::ToBase64String($ms.ToArray())
        $bmp.Dispose()
        $icon.Dispose()
        $ms.Dispose()
    }}
}} catch {{ }}
"#,
        safe = safe
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("PowerShell 启动失败: {e}"))?;

    if !output.status.success() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    if s.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!("data:image/png;base64,{}", s)))
}

#[cfg(not(windows))]
#[tauri::command]
pub fn extract_icon(_path: String) -> Result<Option<String>, String> {
    Ok(None)
}

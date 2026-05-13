@echo off
REM 凌台 Loft 快速构建 EXE 的批处理包装
REM 用法：
REM   scripts\build-exe.bat              -- 便携 EXE
REM   scripts\build-exe.bat -Installer   -- 完整安装包
REM   scripts\build-exe.bat -Open        -- 完成后打开输出目录

setlocal
set "PWSH=pwsh"
where %PWSH% >nul 2>&1 || set "PWSH=powershell"

%PWSH% -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-exe.ps1" %*
exit /b %ERRORLEVEL%

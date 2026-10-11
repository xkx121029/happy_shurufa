# Happy 输入法安装脚本
# 用法（管理员 PowerShell）：
#   .\scripts\install.ps1 [-Release]
# 说明：编译 → 拷贝 DLL → regsvr32 注册（DllRegisterServer 内部自动 HKLM/HKCU 回退）。

param(
    [switch]$Release  # 使用 release 构建产物
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Profile = if ($Release) { "release" } else { "debug" }

# 1. 编译
Write-Host "==> cargo build（$Profile）..." -ForegroundColor Cyan
Push-Location $Root
if ($Release) { cargo build --release } else { cargo build }
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "编译失败" }
Pop-Location

# 2. 定位 DLL
$Dll = Join-Path $Root "target\$Profile\happy_ime.dll"
if (-not (Test-Path $Dll)) { throw "未找到 $Dll" }

# 3. 安装目录
$InstallDir = Join-Path $env:ProgramFiles "happy_shurufa"
try {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $Target = Join-Path $InstallDir "happy_ime.dll"
    Copy-Item $Dll $Target -Force
} catch {
    # 无管理员权限时回退到用户目录
    Write-Host "Program Files 写入失败，回退到用户目录" -ForegroundColor Yellow
    $InstallDir = Join-Path $env:APPDATA "happy_shurufa"
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $Target = Join-Path $InstallDir "happy_ime.dll"
    Copy-Item $Dll $Target -Force
}
Write-Host "==> 已安装到 $Target"

# 4. 注册 COM + TSF（DllRegisterServer 自带 HKLM→HKCU 回退）
Write-Host "==> regsvr32 注册..." -ForegroundColor Cyan
Start-Process regsvr32 -ArgumentList "/s", "`"$Target`"" -Wait
Write-Host "安装完成。按 Win+Space 切换到「Happy 输入法」。"

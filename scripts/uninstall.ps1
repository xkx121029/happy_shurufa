# Happy 输入法卸载脚本
# 用法：.\scripts\uninstall.ps1

$ErrorActionPreference = "Continue"

$Candidates = @(
    (Join-Path $env:ProgramFiles "happy_shurufa\happy_ime.dll"),
    (Join-Path $env:APPDATA "happy_shurufa\happy_ime.dll")
)

$Found = $false
foreach ($Dll in $Candidates) {
    if (Test-Path $Dll) {
        $Found = $true
        Write-Host "==> 反注册 $Dll"
        Start-Process regsvr32 -ArgumentList "/s", "/u", "`"$Dll`"" -Wait
        Remove-Item $Dll -Force -ErrorAction SilentlyContinue
        Write-Host "==> 已删除 $Dll"
    }
}
if (-not $Found) {
    Write-Host "未找到已安装的 happy_ime.dll"
}

# 清理配置与用户数据目录（保留与否由用户决定，默认保留）
Write-Host "用户数据目录（%APPDATA%\happy_shurufa）已保留，如需彻底清除请手动删除。"
Write-Host "卸载完成。若输入法列表中仍显示，请注销或重启后自然消失。"

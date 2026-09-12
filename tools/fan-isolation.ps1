# Reversible diagnostic: stop ONLY the two MSI lighting services, then restore.
$ErrorActionPreference = 'Stop'
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Management\Microsoft.PowerShell.Management.psd1"
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1"
Import-Module "$PSHOME\Modules\CimCmdlets\CimCmdlets.psd1"
$running = @(Get-Service -Name 'LightKeeperService','Mystic_Light_Service' | Where-Object Status -eq Running)
$led = @(Get-CimInstance Win32_Process -Filter "Name='LEDKeeper2.exe'" | Where-Object ExecutablePath -eq 'C:\Program Files (x86)\MSI\MSI Center\Mystic Light\LEDKeeper2.exe')
$stopped = @()
try {
    foreach ($service in $running) { Stop-Service -Name $service.Name; $stopped += $service.Name }
    foreach ($process in $led) { Stop-Process -Id $process.ProcessId -ErrorAction SilentlyContinue }
    & 'C:\code\control-windows-devices\src-tauri\target\debug\examples\fan_visible_test.exe' > (Join-Path $PSScriptRoot 'fan-isolation-result.txt')
} catch {
    $_ | Out-File (Join-Path $PSScriptRoot 'fan-isolation-result.txt') -Append
} finally {
    foreach ($name in $stopped) { Start-Service -Name $name -ErrorAction Continue }
    if ($led.Count -gt 0 -and -not (Get-Process LEDKeeper2 -ErrorAction SilentlyContinue)) {
        Start-Process -FilePath $led[0].ExecutablePath -WindowStyle Hidden
    }
    Get-Service -Name 'LightKeeperService','Mystic_Light_Service' | Select-Object Name,Status | Out-File (Join-Path $PSScriptRoot 'fan-isolation-result.txt') -Append
}

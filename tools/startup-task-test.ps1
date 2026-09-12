$ErrorActionPreference = 'Stop'
$root = 'C:\code\control-windows-devices'
$resultPath = Join-Path $root 'tools\startup-task-result.json'
$exe = Join-Path $root 'src-tauri\target\release\rgb-switch.exe'
$script = Get-Content -LiteralPath (Join-Path $root 'src-tauri\src\startup_task.ps1') -Raw
$env:RGB_SWITCH_STARTUP_EXE = $exe
$powershell = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$env:PSModulePath = $null
$checks = [ordered]@{}
try {
    foreach ($action in @('disable', 'enable')) {
        $env:RGB_SWITCH_STARTUP_ACTION = $action
        $output = & $powershell -NoProfile -NonInteractive -Command $script
        if ($LASTEXITCODE -ne 0) { throw ($output -join "`n") }
        $data = ($output -join "`n") | ConvertFrom-Json
        if ($data.enabled -ne ($action -eq 'enable')) { throw "Unexpected $action result" }
        $checks[$action] = $true
    }
    $scheduler = New-Object -ComObject Schedule.Service
    $scheduler.Connect()
    $name = 'RGB Switch - ' + [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $task = $scheduler.GetFolder('\').GetTask($name)
    if ($task.Definition.RegistrationInfo.Description -ne 'RGB Switch per-user tray startup v1') { throw 'Unexpected task owner' }
    $instance = $task.Run($null)
    $checks['scheduledLaunch'] = $null -ne $instance
    $checks['ok'] = $true
} catch {
    $checks['ok'] = $false
    $checks['error'] = $_.Exception.Message
} finally {
    # Leave the user's requested automatic startup enabled even if testing failed.
    $env:RGB_SWITCH_STARTUP_ACTION = 'enable'
    $restored = & $powershell -NoProfile -NonInteractive -Command $script
    $checks['enabledAfterTest'] = $LASTEXITCODE -eq 0
    $checks | ConvertTo-Json | Set-Content -LiteralPath $resultPath
}

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $sid = $identity.User.Value
    $taskName = "RGB Switch - $sid"
    $marker = 'RGB Switch per-user tray startup v1'
    $scheduler = New-Object -ComObject 'Schedule.Service'
    $scheduler.Connect()
    $folder = $scheduler.GetFolder('\')
    $existing = $null
    try { $existing = $folder.GetTask($taskName) } catch {
        if ($_.Exception.HResult -ne -2147024894) { throw }
    }
    if ($null -ne $existing -and $existing.Definition.RegistrationInfo.Description -ne $marker) {
        throw 'A task with this name is not managed by RGB Switch.'
    }
    if ($env:RGB_SWITCH_STARTUP_ACTION -eq 'disable') {
        if ($null -ne $existing) { $folder.DeleteTask($taskName, 0) }
        Write-Output '{"enabled":false}'
        exit 0
    }
    if ($env:RGB_SWITCH_STARTUP_ACTION -ne 'enable') { throw 'Unknown action.' }
    $exe = $env:RGB_SWITCH_STARTUP_EXE
    if (-not [System.IO.File]::Exists($exe) -or [System.IO.Path]::GetExtension($exe) -ne '.exe') { throw 'App executable is missing.' }
    $definition = $scheduler.NewTask(0)
    $definition.RegistrationInfo.Description = $marker
    $definition.RegistrationInfo.Author = $identity.Name
    $definition.Principal.UserId = $sid
    $definition.Principal.LogonType = 3 # InteractiveToken: this logged-on user only
    $definition.Principal.RunLevel = 1 # HighestAvailable: required by the embedded RAM driver
    $definition.Settings.Enabled = $true
    $definition.Settings.StartWhenAvailable = $true
    $definition.Settings.DisallowStartIfOnBatteries = $false
    $definition.Settings.StopIfGoingOnBatteries = $false
    $definition.Settings.ExecutionTimeLimit = 'PT0S'
    $definition.Settings.MultipleInstances = 2 # IgnoreNew
    $trigger = $definition.Triggers.Create(9) # Logon
    $trigger.UserId = $sid
    $trigger.Delay = 'PT15S'
    $action = $definition.Actions.Create(0) # Exec, no shell command string
    $action.Path = $exe
    $action.Arguments = '--background'
    $action.WorkingDirectory = [System.IO.Path]::GetDirectoryName($exe)
    $task = $folder.RegisterTaskDefinition($taskName, $definition, 6, $sid, $null, 3, $null)
    $check = $task.Definition
    # Scheduler normalizes the SID to an account name on this Windows version.
    function Resolve-TaskSid([string] $value) {
        if ($value -match '^S-1-') { return ([System.Security.Principal.SecurityIdentifier]::new($value)).Value }
        return ([System.Security.Principal.NTAccount]::new($value)).Translate([System.Security.Principal.SecurityIdentifier]).Value
    }
    if (-not $task.Enabled -or $check.Actions.Item(1).Path -ne $exe -or $check.Actions.Item(1).Arguments -ne '--background' -or $check.Principal.RunLevel -ne 1 -or $check.Principal.LogonType -ne 3 -or (Resolve-TaskSid $check.Principal.UserId) -ne $sid -or (Resolve-TaskSid $check.Triggers.Item(1).UserId) -ne $sid -or $check.Triggers.Item(1).Delay -ne 'PT15S') {
        throw 'Startup task readback mismatch.'
    }
    Write-Output '{"enabled":true}'
} catch {
    @{ error = $_.Exception.Message } | ConvertTo-Json -Compress
    exit 1
}

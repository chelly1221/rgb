# Fixed read-only checks for this PC. Do not depend on the parent's PSModulePath.
$ErrorActionPreference = 'Stop'
$stage = 'cim'
try {
    Import-Module "$PSHOME\Modules\CimCmdlets\CimCmdlets.psd1" -ErrorAction Stop
    $stage = 'board'
    $board = Get-CimInstance Win32_BaseBoard
    if ($board.Product -ne 'MAG B860M MORTAR WIFI (MS-7E40)') { throw 'mismatch' }
    $stage = 'memory'
    $memory = @(Get-CimInstance Win32_PhysicalMemory)
    if ($memory.Count -ne 2 -or @($memory | Where-Object { $_.PartNumber.Trim() -ne 'CMH128GX5M2B6400C42' }).Count -ne 0) { throw 'mismatch' }
    $stage = 'smbus'
    $buses = @(Get-CimInstance Win32_PnPEntity | Where-Object DeviceID -Like 'PCI\VEN_8086&DEV_7F23*')
    if ($buses.Count -ne 1) { throw 'ambiguous' }
    $ports = @(Get-CimAssociatedInstance -InputObject $buses[0] -ResultClassName Win32_PortResource)
    if (-not ($ports | Where-Object { $_.StartingAddress -eq 16384 -and $_.EndingAddress -eq 16415 })) { throw 'resource' }
    [Console]::Out.WriteLine('verified')
    exit 0
} catch {
    # ASCII machine-readable stage codes avoid console-encoding and localization issues.
    [Console]::Out.WriteLine("failed:$stage")
    exit 1
}

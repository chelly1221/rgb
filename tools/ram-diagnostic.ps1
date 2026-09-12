$ErrorActionPreference = 'Stop'
$board = Get-CimInstance Win32_BaseBoard
if ($board.Product -ne 'MAG B860M MORTAR WIFI (MS-7E40)') { throw 'Unexpected motherboard' }
$ram = @(Get-CimInstance Win32_PhysicalMemory)
if ($ram.Count -ne 2 -or @($ram | Where-Object {$_.PartNumber.Trim() -ne 'CMH128GX5M2B6400C42'}).Count -ne 0) { throw 'Unexpected RAM kit' }
$bus = Get-CimInstance Win32_PnPEntity | Where-Object DeviceID -Like 'PCI\VEN_8086&DEV_7F23*'
$ports = @(Get-CimAssociatedInstance -InputObject $bus -ResultClassName Win32_PortResource)
if (-not ($ports | Where-Object {$_.StartingAddress -eq 0x4000 -and $_.EndingAddress -eq 0x401f})) { throw 'Unexpected SMBus I/O resource' }
$diagnosticPython = 'C:\Users\chell\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
& $diagnosticPython (Join-Path $PSScriptRoot 'ram-diagnostic.py') (Join-Path $PSScriptRoot 'ram-diagnostic-result.json')

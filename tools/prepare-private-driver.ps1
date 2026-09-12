# Own-PC build input only. Copies the exact installed, signed Corsair binaries unchanged.
$ErrorActionPreference = 'Stop'
$source = 'C:\Program Files\Corsair\Corsair iCUE5 Software'
$destination = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\private-driver'))
$files = @{
    'CorsairLLAccessLib64.dll' = 'DE834E260B50BCB00063618004EE664DC7117BEA90E03C1FA04C2F619DAAC29E'
    'CorsairLLAccess64.sys' = '020ACBF028C67C0936798AD7BA8436D0D58603912F5019A5E56898C2211CF056'
}
foreach ($name in $files.Keys) {
    $file = Join-Path $source $name
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $actual = [BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($file))).Replace('-', '') } finally { $sha.Dispose() }
    if ($actual -ne $files[$name]) { throw "Unverified driver version: $name" }
    if ((Get-AuthenticodeSignature -LiteralPath $file).Status -ne 'Valid') { throw "Invalid signature: $name" }
}
if (-not (Test-Path -LiteralPath (Join-Path $source 'EULA.html'))) { throw 'Missing original EULA.html' }
[IO.Directory]::CreateDirectory($destination) | Out-Null
foreach ($name in @($files.Keys) + @('EULA.html')) {
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination (Join-Path $destination $name)
}
Write-Output 'Verified personal-build driver inputs are ready. These files are excluded from version control.'

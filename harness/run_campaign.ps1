# Reconstruction of the recorded launch protocol for future reproductions.
# This is not the original historical launch script. Run outside sync activity.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$OutputRoot,
    [string[]]$Panels = @('core','density','laws','scale','roads','layout'),
    [string]$DataDirectory = (Join-Path $PSScriptRoot 'data'),
    [string]$Executable = (Join-Path $PSScriptRoot 'portable/target/release/campaign.exe'),
    [switch]$ValidateOnly
)
$ErrorActionPreference = 'Stop'
$taskOutput = [IO.Path]::GetFullPath($OutputRoot)
$taskBinary = (Resolve-Path -LiteralPath $Executable).Path
foreach ($panel in $Panels) {
    if ($panel -notin @('smoke','core','density','laws','scale','roads','layout')) {
        throw "Unknown panel: $panel"
    }
}
if (Test-Path -LiteralPath $taskOutput) { throw "Use a fresh output root: $taskOutput" }
if ($Panels -contains 'roads') {
    foreach ($region in @('NY','BAY','COL')) {
        if (-not (Test-Path -LiteralPath (Join-Path $DataDirectory "USA-road-d.$region.gr"))) {
            throw "Missing decompressed road input: $region"
        }
    }
}
if ($ValidateOnly) { Write-Output 'Arguments validated; no files created or timings run.'; return }
New-Item -ItemType Directory -Path $taskOutput | Out-Null
$taskCpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$taskMetadata = [ordered]@{
    started = (Get-Date).ToString('o')
    os = (Get-CimInstance Win32_OperatingSystem).Caption
    cpu = $taskCpu.Name
    l3_kib = $taskCpu.L3CacheSize
    ram = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    rustc = ((& rustc -Vv) -join "`n")
    binary = (Get-FileHash -LiteralPath $taskBinary -Algorithm SHA256).Hash
    protocol = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'REVISION_PROTOCOL.md') -Algorithm SHA256).Hash
    source = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'src/revision.rs') -Algorithm SHA256).Hash
    harness = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'src/bin/campaign.rs') -Algorithm SHA256).Hash
    executable = $taskBinary
    priority = 'Normal'
    affinity = 'OS default'
    parallelism = 1
    driver = 'Documented reconstruction; independently built executable must match current source.'
}
$taskMetadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskOutput 'environment.json') -Encoding utf8
foreach ($panel in $Panels) {
    $taskPanelOutput = Join-Path $taskOutput $panel
    New-Item -ItemType Directory -Path $taskPanelOutput | Out-Null
    (Get-Date).ToString('o') | Set-Content -LiteralPath (Join-Path $taskPanelOutput 'started.txt')
    $taskArguments = @($panel, $taskPanelOutput)
    if ($panel -eq 'roads') { $taskArguments += $DataDirectory }
    $taskArguments | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskPanelOutput 'arguments.json')
    & $taskBinary @taskArguments 1> (Join-Path $taskPanelOutput 'stdout.txt') 2> (Join-Path $taskPanelOutput 'stderr.txt')
    if ($LASTEXITCODE -ne 0) { throw "Panel $panel exited $LASTEXITCODE; completion marker withheld." }
    (Get-Date).ToString('o') | Set-Content -LiteralPath (Join-Path $taskPanelOutput 'completed.txt')
}

[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$ServerBinary,

  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory = "dist/tspserver",

  [string]$PagesDirectory = "pages",

  [string]$PublicDirectory = "public",

  [string]$ConfigFile = "tsp.config.json",

  [string]$WorkerBinary = ""
)

$serverPath = [System.IO.Path]::GetFullPath($ServerBinary)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$configPath = [System.IO.Path]::GetFullPath($ConfigFile)
$agentsPath = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\docs\AGENTS.md"))
$serverName = if ($serverPath.EndsWith(".exe", [System.StringComparison]::OrdinalIgnoreCase)) { "tspserver.exe" } else { "tspserver" }

if (!(Test-Path -LiteralPath $serverPath -PathType Leaf)) {
  throw "server binary not found: $serverPath"
}
if (!(Test-Path -LiteralPath $configPath -PathType Leaf)) {
  throw "config file not found: $configPath"
}
if (!(Test-Path -LiteralPath $agentsPath -PathType Leaf)) {
  throw "user guide not found: $agentsPath"
}
if ($WorkerBinary -and !(Test-Path -LiteralPath ([System.IO.Path]::GetFullPath($WorkerBinary)) -PathType Leaf)) {
  throw "worker binary not found: $WorkerBinary"
}
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$serverTarget = Join-Path $outputPath $serverName
if ($serverPath -ne $serverTarget) {
  Copy-Item -LiteralPath $serverPath -Destination $serverTarget -Force
}
if (Test-Path -LiteralPath $PagesDirectory -PathType Container) {
  Copy-Item -LiteralPath $PagesDirectory -Destination (Join-Path $outputPath "pages") -Recurse -Force
}
if (Test-Path -LiteralPath (Join-Path $outputPath "routes")) {
  Remove-Item -LiteralPath (Join-Path $outputPath "routes") -Recurse -Force
}
if (Test-Path -LiteralPath $PublicDirectory -PathType Container) {
  Copy-Item -LiteralPath $PublicDirectory -Destination (Join-Path $outputPath "public") -Recurse -Force
}
$configTarget = Join-Path $outputPath "tsp.config.json"
if ($configPath -ne [System.IO.Path]::GetFullPath($configTarget)) {
  Copy-Item -LiteralPath $configPath -Destination $configTarget -Force
}
Copy-Item -LiteralPath $agentsPath -Destination (Join-Path $outputPath "AGENTS.md") -Force
if ($WorkerBinary) {
  Copy-Item -LiteralPath ([System.IO.Path]::GetFullPath($WorkerBinary)) -Destination (Join-Path $outputPath "tsp-worker.exe") -Force
}

# Native process-worker distribution contract: the package contains the host
# plus its TSP-owned worker executable, never a separate JavaScript runtime.
# Pre-existing legacy runtime files are removed so re-packaging stays clean.
$staleBun = Join-Path $outputPath "bun.exe"
if (Test-Path -LiteralPath $staleBun -PathType Leaf) {
  Write-Warning "removing stale standalone bun.exe from $outputPath (embedded-worker ships a single binary)"
  Remove-Item -LiteralPath $staleBun -Force
}
$staleBunNoExt = Join-Path $outputPath "bun"
if (Test-Path -LiteralPath $staleBunNoExt -PathType Leaf) {
  Write-Warning "removing stale standalone bun from $outputPath (embedded-worker ships a single binary)"
  Remove-Item -LiteralPath $staleBunNoExt -Force
}

$staleManifest = Join-Path $outputPath "tspserver-runtime.json"
if (Test-Path -LiteralPath $staleManifest -PathType Leaf) {
  Write-Warning "removing retired runtime manifest from $outputPath"
  Remove-Item -LiteralPath $staleManifest -Force
}

Write-Host "Packaged single-file TSP runtime at $outputPath"

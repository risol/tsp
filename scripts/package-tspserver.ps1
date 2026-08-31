[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$ServerBinary,

  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory = "dist/tspserver",

  [string]$PagesDirectory = "pages",

  [string]$PublicDirectory = "public",

  [string]$ConfigFile = "tsp.config.json"
)

$serverPath = [System.IO.Path]::GetFullPath($ServerBinary)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$configPath = [System.IO.Path]::GetFullPath($ConfigFile)
$serverName = if ($serverPath.EndsWith(".exe", [System.StringComparison]::OrdinalIgnoreCase)) { "tspserver.exe" } else { "tspserver" }

if (!(Test-Path -LiteralPath $serverPath -PathType Leaf)) {
  throw "server binary not found: $serverPath"
}
if (!(Test-Path -LiteralPath $configPath -PathType Leaf)) {
  throw "config file not found: $configPath"
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

# embedded-worker distribution contract: the packaged directory must NOT
# ship a standalone `bun.exe`. The master self-spawns the
# same `tspserver.exe`; shipping a separate Bun would be a
# regression of the single-binary contract. Pre-existing
# standalone files are removed so a re-packaging against a
# stale dist/tspserver stays clean.
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

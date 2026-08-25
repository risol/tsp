[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$ServerBinary,

  [Parameter(Mandatory = $true)]
  [string]$BunBinary,

  [string]$OutputDirectory = "dist/tsp-v2",

  [string]$RoutesDirectory = "routes",

  [string]$PublicDirectory = "public"
)

$serverPath = [System.IO.Path]::GetFullPath($ServerBinary)
$bunPath = [System.IO.Path]::GetFullPath($BunBinary)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)

if (!(Test-Path -LiteralPath $serverPath -PathType Leaf)) {
  throw "server binary not found: $serverPath"
}
if (!(Test-Path -LiteralPath $bunPath -PathType Leaf)) {
  throw "Bun runtime binary not found: $bunPath"
}

New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
Copy-Item -LiteralPath $serverPath -Destination (Join-Path $outputPath "tspserver_v2.exe") -Force
Copy-Item -LiteralPath $bunPath -Destination (Join-Path $outputPath "bun.exe") -Force
if (Test-Path -LiteralPath $RoutesDirectory -PathType Container) {
  Copy-Item -LiteralPath $RoutesDirectory -Destination (Join-Path $outputPath "routes") -Recurse -Force
}
if (Test-Path -LiteralPath $PublicDirectory -PathType Container) {
  Copy-Item -LiteralPath $PublicDirectory -Destination (Join-Path $outputPath "public") -Recurse -Force
}

$manifest = [ordered]@{
  runtime = "tsp-v2"
  server = "tspserver_v2.exe"
  bun = "bun.exe"
  routes = "routes"
  public = "public"
  resolver = "bundled-runtime"
}
$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $outputPath "tsp-v2-runtime.json") -Encoding UTF8

Write-Host "Packaged tspserver_v2 and bundled Bun runtime at $outputPath"

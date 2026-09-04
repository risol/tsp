[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)] [string]$ServerBinary,
  [Parameter(Mandatory = $true)] [string]$WorkerBinary,
  [Parameter(Mandatory = $true)] [string]$OutputDirectory,
  [Parameter(Mandatory = $true)] [string]$RoutesDirectory,
  [Parameter(Mandatory = $true)] [string]$PublicDirectory,
  [Parameter(Mandatory = $true)] [string]$ConfigFile
)

$serverPath = [System.IO.Path]::GetFullPath($ServerBinary)
$workerPath = [System.IO.Path]::GetFullPath($WorkerBinary)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$routesPath = [System.IO.Path]::GetFullPath($RoutesDirectory)
$publicPath = [System.IO.Path]::GetFullPath($PublicDirectory)
$configPath = [System.IO.Path]::GetFullPath($ConfigFile)
$agentsPath = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\docs\AGENTS.md"))

foreach ($file in @($serverPath, $workerPath, $configPath, $agentsPath)) {
  if (!(Test-Path -LiteralPath $file -PathType Leaf)) { throw "required file not found: $file" }
}
foreach ($directory in @($routesPath, $publicPath)) {
  if (!(Test-Path -LiteralPath $directory -PathType Container)) { throw "required directory not found: $directory" }
}
foreach ($file in @((Join-Path $routesPath "manifest.json"), (Join-Path $routesPath "bundle.js"))) {
  if (!(Test-Path -LiteralPath $file -PathType Leaf)) { throw "compiled route artifact is missing: $file" }
}

if (Test-Path -LiteralPath $outputPath) { Remove-Item -LiteralPath $outputPath -Recurse -Force }
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
Copy-Item -LiteralPath $serverPath -Destination (Join-Path $outputPath "tspserver.exe") -Force
Copy-Item -LiteralPath $workerPath -Destination (Join-Path $outputPath "tsp-worker.exe") -Force
Copy-Item -LiteralPath $routesPath -Destination (Join-Path $outputPath "routes") -Recurse -Force
Copy-Item -LiteralPath $publicPath -Destination (Join-Path $outputPath "public") -Recurse -Force
Copy-Item -LiteralPath $configPath -Destination (Join-Path $outputPath "tsp.config.json") -Force
Copy-Item -LiteralPath $agentsPath -Destination (Join-Path $outputPath "AGENTS.md") -Force

Write-Host "Packaged native TSP server at $outputPath"

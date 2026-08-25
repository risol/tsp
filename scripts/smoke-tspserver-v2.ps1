[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)] [string]$ServerBinary,
  [Parameter(Mandatory = $true)] [string]$WorkerBinary,
  [string]$RoutesDirectory = "tests/v2_smoke/routes",
  [int]$Port = 9137,
  [switch]$SkipHotReload
)

$server = [System.IO.Path]::GetFullPath($ServerBinary)
$worker = [System.IO.Path]::GetFullPath($WorkerBinary)
$sourceRoutes = [System.IO.Path]::GetFullPath($RoutesDirectory)
if (!(Test-Path -LiteralPath $server -PathType Leaf)) { throw "server binary not found: $server" }
if (!(Test-Path -LiteralPath $worker -PathType Leaf)) { throw "worker binary not found: $worker" }
if (!(Test-Path -LiteralPath $sourceRoutes -PathType Container)) { throw "routes directory not found: $sourceRoutes" }

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tsp-v2-smoke-" + [guid]::NewGuid().ToString("N"))
$routes = Join-Path $tempRoot "routes"
New-Item -ItemType Directory -Path $routes -Force | Out-Null
Get-ChildItem -LiteralPath $sourceRoutes -Force | Copy-Item -Destination $routes -Recurse -Force

$info = [System.Diagnostics.ProcessStartInfo]::new()
$info.FileName = $server
$info.WorkingDirectory = (Split-Path -Parent $server)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$info.Environment["TSP_PORT"] = "$Port"
$info.Environment["TSP_ROUTES_DIR"] = $routes
$info.Environment["TSP_EMBEDDED_WORKER"] = "1"
$info.Environment["TSP_WORKER_BIN"] = $worker
$info.Environment["TSP_WORKER_COUNT"] = "2"
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $info

try {
  $null = $process.Start()
  $uri = "http://127.0.0.1:$Port/"
  $ready = $false
  for ($attempt = 0; $attempt -lt 150; $attempt++) {
    try {
      $probe = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 1
      if ($probe.StatusCode -eq 200) { $ready = $true; break }
    } catch { Start-Sleep -Milliseconds 100 }
  }
  if (!$ready) {
    $stderr = $process.StandardError.ReadToEnd()
    throw "v2 server did not become ready. $stderr"
  }

  for ($index = 0; $index -lt 5; $index++) {
    $response = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 30
    if ($response.StatusCode -ne 200 -or $response.Content -notmatch "Hello from TSP v2") {
      throw "embedded worker response was invalid: $($response.StatusCode) $($response.Content)"
    }
  }

  $metrics = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/__tsp/metrics" -TimeoutSec 30
  if ($metrics.StatusCode -ne 200 -or $metrics.Content -notmatch "tsp_requests_total") {
    throw "metrics endpoint did not return Prometheus output"
  }

  if (!$SkipHotReload) {
    $routeFile = Join-Path $routes "index.tsp"
    $source = Get-Content -Raw -LiteralPath $routeFile
    Set-Content -LiteralPath $routeFile -Value ($source.Replace("Hello from TSP v2", "Hello after reload")) -NoNewline
    $reloaded = $false
    for ($attempt = 0; $attempt -lt 150; $attempt++) {
      try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 2
        if ($response.Content -match "Hello after reload") { $reloaded = $true; break }
      } catch { }
      Start-Sleep -Milliseconds 100
    }
    if (!$reloaded) { throw "v2 hot reload did not publish the changed route" }
  }

  Write-Output "TSP v2 embedded-worker smoke test passed"
} finally {
  if (!$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
  $process.Dispose()
  if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}

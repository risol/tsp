[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)] [string]$ServerBinary,
  [string]$PagesDirectory = "tests/smoke/pages",
  [int]$Port = 9137,
  [switch]$SkipHotReload
)

$server = [System.IO.Path]::GetFullPath($ServerBinary)
$sourcePages = [System.IO.Path]::GetFullPath($PagesDirectory)
if (!(Test-Path -LiteralPath $server -PathType Leaf)) { throw "server binary not found: $server" }
if (!(Test-Path -LiteralPath $sourcePages -PathType Container)) { throw "pages directory not found: $sourcePages" }

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tspserver-smoke-" + [guid]::NewGuid().ToString("N"))
$pages = Join-Path $tempRoot "pages"
$stdoutLog = Join-Path $tempRoot "tspserver.stdout.log"
$stderrLog = Join-Path $tempRoot "tspserver.stderr.log"
New-Item -ItemType Directory -Path $pages -Force | Out-Null
Get-ChildItem -LiteralPath $sourcePages -Force | Copy-Item -Destination $pages -Recurse -Force

$info = [System.Diagnostics.ProcessStartInfo]::new()
$info.FileName = $env:ComSpec
$info.Arguments = "/d /s /c `"`"$server`" 1>`"$stdoutLog`" 2>`"$stderrLog`"`""
$info.WorkingDirectory = (Split-Path -Parent $server)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $false
$info.RedirectStandardError = $false
$info.Environment["TSP_PORT"] = "$Port"
$info.Environment["TSP_ROUTES_DIR"] = $pages
$info.Environment["TSP_EMBEDDED_WORKER"] = "1"
$info.Environment["TSP_WORKER_COUNT"] = "2"
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $info

function Get-LogTail {
  param([string]$Path)
  if (!(Test-Path -LiteralPath $Path -PathType Leaf)) { return @("<not created>") }
  $lines = @(Get-Content -LiteralPath $Path)
  if ($lines.Count -eq 0) { return @("<empty>") }
  $start = [Math]::Max(0, $lines.Count - 200)
  return $lines[$start..($lines.Count - 1)]
}

function Get-StartupDiagnostics {
  $stderr = Get-LogTail $stderrLog
  $stdout = Get-LogTail $stdoutLog
  $markers = @($stderr | Where-Object {
      $_ -like "TSP worker startup:*" -or $_ -like "TSP worker VM init:*"
    })
  return @(
    "Worker startup markers:"
    $(if ($markers.Count -gt 0) { $markers } else { "<none captured>" })
    "Last stderr lines:"
    $stderr
    "Last stdout lines:"
    $stdout
  ) -join [Environment]::NewLine
}

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
    if (!$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
    $diagnostics = Get-StartupDiagnostics
    throw "server did not become ready.`n$diagnostics"
  }

  for ($index = 0; $index -lt 5; $index++) {
    $response = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 30
    if ($response.StatusCode -ne 200 -or $response.Content -notmatch "Hello from TSP") {
      throw "embedded worker response was invalid: $($response.StatusCode) $($response.Content)"
    }
  }

  $metrics = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/__tsp/metrics" -TimeoutSec 30
  if ($metrics.StatusCode -ne 200 -or $metrics.Content -notmatch "tsp_requests_total") {
    throw "metrics endpoint did not return Prometheus output"
  }

  if (!$SkipHotReload) {
    $routeFile = Join-Path $pages "index.tsp"
    $source = Get-Content -Raw -LiteralPath $routeFile
    Set-Content -LiteralPath $routeFile -Value ($source.Replace("Hello from TSP", "Hello after reload")) -NoNewline
    $reloaded = $false
    for ($attempt = 0; $attempt -lt 150; $attempt++) {
      try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 2
        if ($response.Content -match "Hello after reload") { $reloaded = $true; break }
      } catch { }
      Start-Sleep -Milliseconds 100
    }
    if (!$reloaded) { throw "hot reload did not publish the changed route" }
  }

  Write-Output "TSP embedded-worker smoke test passed"
} finally {
  if (!$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
  $process.Dispose()
  # Windows may report the inherited worker log handle as busy for a brief
  # interval after Kill(entireProcessTree: true) returns. Retry the exact temp
  # directory instead of leaving CI artifacts or emitting a false cleanup error.
  for ($attempt = 0; $attempt -lt 50 -and (Test-Path -LiteralPath $tempRoot); $attempt++) {
    try {
      Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction Stop
    } catch {
      if ($attempt -eq 49) { throw }
      Start-Sleep -Milliseconds 100
    }
  }
}

[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)] [string]$ServerBinary,
  [string]$PagesDirectory = "pages",
  [int]$Port = 9140,
  [int]$Requests = 50,
  [string]$OutputJson = ""
)

$server = [System.IO.Path]::GetFullPath($ServerBinary)
$pages = [System.IO.Path]::GetFullPath($PagesDirectory)
if (!(Test-Path -LiteralPath $server -PathType Leaf)) { throw "server binary not found: $server" }
if (!(Test-Path -LiteralPath $pages -PathType Container)) { throw "pages directory not found: $pages" }
if ($Requests -lt 1) { throw "Requests must be positive" }

$info = [System.Diagnostics.ProcessStartInfo]::new()
$info.FileName = $server
$info.WorkingDirectory = (Get-Location).Path
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.Environment["TSP_PORT"] = "$Port"
$info.Environment["TSP_ROUTES_DIR"] = $pages
$info.Environment["TSP_EMBEDDED_WORKER"] = "1"
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $info
$null = $process.Start()

function Get-Quantile([double[]]$Values, [double]$Quantile) {
  $position = [Math]::Min($Values.Count - 1, [Math]::Max(0, [Math]::Ceiling($Values.Count * $Quantile) - 1))
  return $Values[$position]
}

try {
  $uri = "http://127.0.0.1:$Port/"
  $ready = $false
  for ($attempt = 0; $attempt -lt 100; $attempt++) {
    try { $null = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 1; $ready = $true; break }
    catch { Start-Sleep -Milliseconds 100 }
  }
  if (!$ready) { throw "server did not become ready" }

  $coldWatch = [System.Diagnostics.Stopwatch]::StartNew()
  $null = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 30
  $coldWatch.Stop()
  $samples = [System.Collections.Generic.List[double]]::new()
  for ($index = 0; $index -lt $Requests; $index++) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $response = Invoke-WebRequest -UseBasicParsing -Uri $uri -TimeoutSec 30
    $watch.Stop()
    if ($response.StatusCode -ne 200) { throw "request returned $($response.StatusCode)" }
    $samples.Add($watch.Elapsed.TotalMilliseconds)
  }
  $ordered = [double[]]@($samples | Sort-Object)
  $result = [ordered]@{
    timestamp = [DateTime]::UtcNow.ToString("o")
    requests = $Requests
    cold_ms = [Math]::Round($coldWatch.Elapsed.TotalMilliseconds, 3)
    p50_ms = [Math]::Round((Get-Quantile $ordered 0.50), 3)
    p95_ms = [Math]::Round((Get-Quantile $ordered 0.95), 3)
    p99_ms = [Math]::Round((Get-Quantile $ordered 0.99), 3)
    min_ms = [Math]::Round($ordered[0], 3)
    max_ms = [Math]::Round($ordered[$ordered.Count - 1], 3)
  }
  $json = $result | ConvertTo-Json
  if ($OutputJson) { $json | Set-Content -LiteralPath ([System.IO.Path]::GetFullPath($OutputJson)) -Encoding UTF8 }
  $json
} finally {
  if (!$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
  $process.Dispose()
}

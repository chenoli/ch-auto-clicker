param(
    [Parameter(Mandatory = $true)][string]$Csv,
    [double]$WarmupMin = 30
)

$ErrorActionPreference = "Stop"

$limits = [ordered]@{
    private_mb      = 5
    webview_mb      = 20
    handles         = 50
    threads         = 2
    gdi             = 10
    user            = 10
    webview_handles = 100
}

$rows = Import-Csv $Csv
$alive = $rows | Where-Object { $_.root_pid -ne "" }
$missing = @($rows | Where-Object { $_.root_pid -eq "" }).Count
$restarts = @($rows | Where-Object { $_.restarted -eq "True" }).Count
$samples = @($alive | Where-Object { [double]$_.elapsed_min -ge $WarmupMin })

$span = if ($rows.Count) { [double]$rows[-1].elapsed_min } else { 0 }
Write-Host ("{0} samples over {1:N1} h; {2} after a {3} min warm-up" -f $rows.Count, ($span / 60), $samples.Count, $WarmupMin)
if ($missing -gt 0) { Write-Host "FAIL: process absent in $missing samples" -ForegroundColor Red }
if ($restarts -gt 0) { Write-Host "FAIL: process restarted $restarts time(s)" -ForegroundColor Red }
if ($samples.Count -lt 10) {
    Write-Host "Not enough samples after the warm-up to judge growth (need 10)." -ForegroundColor Yellow
    exit 2
}

function Get-Slope($Points, [string]$Metric) {
    $xs = $Points | ForEach-Object { [double]$_.elapsed_min / 60 }
    $ys = $Points | ForEach-Object { [double]$_.$Metric }
    $n = $xs.Count
    $mx = ($xs | Measure-Object -Average).Average
    $my = ($ys | Measure-Object -Average).Average
    $num = 0.0
    $den = 0.0
    for ($i = 0; $i -lt $n; $i++) {
        $num += ($xs[$i] - $mx) * ($ys[$i] - $my)
        $den += ($xs[$i] - $mx) * ($xs[$i] - $mx)
    }
    if ($den -eq 0) { 0 } else { $num / $den }
}

$suspect = $false
$report = foreach ($metric in $limits.Keys) {
    $values = $samples | ForEach-Object { [double]$_.$metric } | Where-Object { $_ -ge 0 }
    if (-not $values) { continue }
    $slope = Get-Slope ($samples | Where-Object { [double]$_.$metric -ge 0 }) $metric
    $verdict = if ($slope -gt $limits[$metric]) { $suspect = $true; "SUSPECT" } else { "ok" }
    [pscustomobject]@{
        metric       = $metric
        first        = $values[0]
        last         = $values[-1]
        min          = ($values | Measure-Object -Minimum).Minimum
        max          = ($values | Measure-Object -Maximum).Maximum
        "per_hour"   = [math]::Round($slope, 2)
        "limit_hour" = $limits[$metric]
        verdict      = $verdict
    }
}
$report | Format-Table -AutoSize

if ($missing -gt 0 -or $restarts -gt 0) { exit 1 }
if ($suspect) {
    Write-Host "Growth above the limit: rerun that scenario longer before calling it a leak." -ForegroundColor Yellow
    exit 1
}
Write-Host "No sustained growth." -ForegroundColor Green

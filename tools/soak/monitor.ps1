param(
    [string]$ProcessName = "ch-clicker",
    [int]$IntervalSec = 60,
    [string]$Out = (Join-Path $PSScriptRoot ("soak-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".csv"))
)

$ErrorActionPreference = "Stop"

Add-Type -Namespace Soak -Name Gui -MemberDefinition @'
[DllImport("user32.dll")] public static extern uint GetGuiResources(System.IntPtr hProcess, uint uiFlags);
'@

function Get-Descendants([int]$RootId, $All) {
    $result = @()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootId)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($child in $All | Where-Object { $_.ParentProcessId -eq $parent }) {
            $result += $child
            $queue.Enqueue([int]$child.ProcessId)
        }
    }
    $result
}

function Get-GuiCount([System.Diagnostics.Process]$Process, [uint32]$Kind) {
    try { [int][Soak.Gui]::GetGuiResources($Process.Handle, $Kind) } catch { -1 }
}

function Format-Number([double]$Value) {
    [math]::Round($Value, 1).ToString([System.Globalization.CultureInfo]::InvariantCulture)
}

$started = Get-Date
$lastRoot = $null
Write-Host "Sampling $ProcessName every $IntervalSec s into $Out (Ctrl+C to stop)"

while ($true) {
    $now = Get-Date
    $root = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Sort-Object StartTime | Select-Object -First 1
    $row = [ordered]@{
        timestamp      = $now.ToString("s")
        elapsed_min    = Format-Number ($now - $started).TotalMinutes
        root_pid       = ""
        restarted      = $false
        private_mb     = ""
        working_set_mb = ""
        handles        = ""
        threads        = ""
        gdi            = ""
        user           = ""
        webview_count  = ""
        webview_mb     = ""
        webview_handles = ""
    }
    if ($root) {
        $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name
        $children = Get-Descendants $root.Id $all
        $webviews = foreach ($c in $children) { Get-Process -Id $c.ProcessId -ErrorAction SilentlyContinue }
        $row.root_pid = $root.Id
        $row.restarted = ($null -ne $lastRoot -and $lastRoot -ne $root.Id)
        $row.private_mb = Format-Number ($root.PrivateMemorySize64 / 1MB)
        $row.working_set_mb = Format-Number ($root.WorkingSet64 / 1MB)
        $row.handles = $root.HandleCount
        $row.threads = $root.Threads.Count
        $row.gdi = Get-GuiCount $root 0
        $row.user = Get-GuiCount $root 1
        $row.webview_count = @($webviews).Count
        $row.webview_mb = Format-Number ((($webviews | Measure-Object PrivateMemorySize64 -Sum).Sum) / 1MB)
        $row.webview_handles = ($webviews | Measure-Object HandleCount -Sum).Sum
        $lastRoot = $root.Id
    }
    [pscustomobject]$row | Export-Csv -Path $Out -Append -NoTypeInformation -Encoding utf8
    $state = if ($root) { "pid $($row.root_pid) private $($row.private_mb) MB, webview $($row.webview_mb) MB, handles $($row.handles), gdi $($row.gdi), user $($row.user)" } else { "PROCESS NOT RUNNING" }
    Write-Host ("[{0}] {1}" -f $now.ToString("HH:mm:ss"), $state)
    Start-Sleep -Seconds $IntervalSec
}

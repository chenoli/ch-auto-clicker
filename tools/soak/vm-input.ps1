param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("shot", "move", "click", "drag", "key", "type", "windows", "focus", "cursor")]
    [string]$Command,
    [int]$X,
    [int]$Y,
    [int]$ToX,
    [int]$ToY,
    [string]$Hold,
    [int]$Steps = 20,
    [ValidateSet("left", "right", "middle", "x1", "x2")][string]$Button = "left",
    [switch]$Double,
    [string]$Keys,
    [string]$Text,
    [string]$Title,
    [string]$Path,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$model = (Get-CimInstance Win32_ComputerSystem).Model
if (-not $Force -and $Command -notin @("shot", "windows", "cursor") -and $model -notmatch "Virtual Machine|VMware|VirtualBox") {
    Write-Host "Refusing to inject input: this does not look like a VM (model '$model')." -ForegroundColor Red
    exit 1
}

Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -Namespace Soak -Name Input -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public System.IntPtr dwExtraInfo; }
[StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public System.IntPtr dwExtraInfo; }
[StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
[StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public UNION u; }
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
public delegate bool EnumProc(System.IntPtr hWnd, System.IntPtr lParam);
[DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, System.IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int cmd);
'@
[void][Soak.Input]::SetProcessDPIAware()

$size = [System.Runtime.InteropServices.Marshal]::SizeOf([type][Soak.Input+INPUT])

function Send-Inputs([Soak.Input+INPUT[]]$Inputs) {
    $sent = [Soak.Input]::SendInput([uint32]$Inputs.Count, $Inputs, $size)
    if ($sent -ne $Inputs.Count) { throw "SendInput accepted $sent of $($Inputs.Count) events (blocked?)" }
}

function New-Mouse([uint32]$Flags, [uint32]$Data = 0) {
    $input = New-Object Soak.Input+INPUT
    $input.type = 0
    $input.u.mi.dwFlags = $Flags
    $input.u.mi.mouseData = $Data
    $input
}

function New-Key([uint16]$Vk, [bool]$Up) {
    $input = New-Object Soak.Input+INPUT
    $input.type = 1
    $input.u.ki.wVk = $Vk
    if ($Up) { $input.u.ki.dwFlags = 2 }
    $input
}

$named = @{
    ctrl = 0x11; alt = 0x12; shift = 0x10; win = 0x5B; enter = 0x0D; esc = 0x1B; escape = 0x1B; tab = 0x09
    space = 0x20; backspace = 0x08; delete = 0x2E; del = 0x2E; home = 0x24; end = 0x23; pageup = 0x21; pagedown = 0x22
    left = 0x25; up = 0x26; right = 0x27; down = 0x28
}

function Get-Vk([string]$Name) {
    $n = $Name.ToLowerInvariant()
    if ($named.ContainsKey($n)) { return [uint16]$named[$n] }
    if ($n -match '^f([1-9]|1[0-9]|2[0-4])$') { return [uint16](0x70 + [int]$Matches[1] - 1) }
    if ($n -match '^[a-z0-9]$') { return [uint16][int][char]$n.ToUpperInvariant() }
    throw "Unknown key '$Name'"
}

switch ($Command) {
    "shot" {
        if (-not $Path) { $Path = Join-Path "C:\soak\shots" ("shot-" + (Get-Date -Format "HHmmss") + ".png") }
        New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
        $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
        $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $graphics.Dispose()
        $bitmap.Dispose()
        Write-Output $Path
    }
    "cursor" {
        $p = New-Object Soak.Input+POINT
        [void][Soak.Input]::GetCursorPos([ref]$p)
        Write-Output "$($p.X),$($p.Y)"
    }
    "move" {
        [void][Soak.Input]::SetCursorPos($X, $Y)
    }
    "click" {
        [void][Soak.Input]::SetCursorPos($X, $Y)
        $flags = @{ left = @(0x2, 0x4, 0); right = @(0x8, 0x10, 0); middle = @(0x20, 0x40, 0); x1 = @(0x80, 0x100, 1); x2 = @(0x80, 0x100, 2) }[$Button]
        $times = if ($Double) { 2 } else { 1 }
        $events = @()
        for ($i = 0; $i -lt $times; $i++) {
            $events += New-Mouse $flags[0] $flags[2]
            $events += New-Mouse $flags[1] $flags[2]
        }
        Send-Inputs $events
    }
    "drag" {
        $held = @()
        if ($Hold) { $held = @($Hold -split '\+' | ForEach-Object { Get-Vk $_.Trim() }) }
        $flags = @{ left = @(0x2, 0x4); right = @(0x8, 0x10); middle = @(0x20, 0x40) }[$Button]
        if (-not $flags) { throw "drag supports left, right and middle" }
        if ($held.Count) { Send-Inputs @($held | ForEach-Object { New-Key $_ $false }) }
        try {
            [void][Soak.Input]::SetCursorPos($X, $Y)
            Start-Sleep -Milliseconds 80
            Send-Inputs @(New-Mouse $flags[0])
            for ($i = 1; $i -le $Steps; $i++) {
                [void][Soak.Input]::SetCursorPos([int]($X + ($ToX - $X) * $i / $Steps), [int]($Y + ($ToY - $Y) * $i / $Steps))
                Start-Sleep -Milliseconds 15
            }
            Start-Sleep -Milliseconds 80
            Send-Inputs @(New-Mouse $flags[1])
        } finally {
            if ($held.Count) { Send-Inputs @($held | ForEach-Object { New-Key $_ $true }) }
        }
    }
    "key" {
        $parts = $Keys -split '\+' | ForEach-Object { Get-Vk $_.Trim() }
        $events = @()
        foreach ($vk in $parts) { $events += New-Key $vk $false }
        [array]::Reverse($parts)
        foreach ($vk in $parts) { $events += New-Key $vk $true }
        Send-Inputs $events
    }
    "type" {
        [System.Windows.Forms.SendKeys]::SendWait(($Text -replace '([+^%~(){}\[\]])', '{$1}'))
    }
    "windows" {
        $list = New-Object System.Collections.Generic.List[string]
        $callback = [Soak.Input+EnumProc]{
            param($h, $l)
            if ([Soak.Input]::IsWindowVisible($h)) {
                $sb = New-Object System.Text.StringBuilder 256
                [void][Soak.Input]::GetWindowText($h, $sb, 256)
                if ($sb.Length -gt 0) {
                    $r = New-Object Soak.Input+RECT
                    [void][Soak.Input]::GetWindowRect($h, [ref]$r)
                    $procId = [uint32]0
                    [void][Soak.Input]::GetWindowThreadProcessId($h, [ref]$procId)
                    $name = (Get-Process -Id $procId -ErrorAction SilentlyContinue).ProcessName
                    $list.Add(("{0} | {1} | ({2},{3})-({4},{5})" -f $name, $sb, $r.Left, $r.Top, $r.Right, $r.Bottom))
                }
            }
            $true
        }
        [void][Soak.Input]::EnumWindows($callback, [IntPtr]::Zero)
        $list
    }
    "focus" {
        $process = Get-Process | Where-Object { $_.MainWindowTitle -like "*$Title*" } | Select-Object -First 1
        if (-not $process) { throw "No window with a title like '$Title'" }
        [void][Soak.Input]::ShowWindow($process.MainWindowHandle, 9)
        [void][Soak.Input]::SetForegroundWindow($process.MainWindowHandle)
        Write-Output "$($process.ProcessName) | $($process.MainWindowTitle)"
    }
}

param(
    [int]$Cycles = 200,
    [int]$RunSec = 20,
    [int]$IdleSec = 5,
    [int]$CountdownSec = 3,
    [string]$StartStop = "F6",
    [string]$PauseResume = "F7",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$model = (Get-CimInstance Win32_ComputerSystem).Model
if (-not $Force -and $model -notmatch "Virtual Machine|VMware|VirtualBox") {
    Write-Host "Refusing to inject keys: this does not look like a VM (model '$model')." -ForegroundColor Red
    exit 1
}

Add-Type -Namespace Soak -Name Keys -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public System.IntPtr dwExtraInfo; }
[StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public System.IntPtr dwExtraInfo; }
[StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
[StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public UNION u; }
[DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
public static uint Tap(ushort vk) {
    var inputs = new INPUT[2];
    inputs[0].type = 1; inputs[0].u.ki.wVk = vk;
    inputs[1].type = 1; inputs[1].u.ki.wVk = vk; inputs[1].u.ki.dwFlags = 2;
    return SendInput(2, inputs, Marshal.SizeOf(typeof(INPUT)));
}
'@

function Get-Vk([string]$Key) {
    if ($Key -notmatch '^F([1-9]|1[0-2])$') { throw "Only F1-F12 are supported, got '$Key'" }
    [uint16](0x70 + [int]$Matches[1] - 1)
}

function Send-Key([string]$Key) {
    if ([Soak.Keys]::Tap((Get-Vk $Key)) -ne 2) { Write-Host "SendInput rejected $Key" -ForegroundColor Red }
}

$started = Get-Date
for ($i = 1; $i -le $Cycles; $i++) {
    Send-Key $StartStop
    Start-Sleep -Seconds ($CountdownSec + [int]($RunSec / 2))
    if ($i % 3 -eq 0) {
        Send-Key $PauseResume
        Start-Sleep -Seconds 2
        Send-Key $PauseResume
    }
    Start-Sleep -Seconds ([int]($RunSec / 2))
    Send-Key $StartStop
    Write-Host ("[{0}] cycle {1}/{2}" -f (Get-Date).ToString("HH:mm:ss"), $i, $Cycles)
    Start-Sleep -Seconds $IdleSec
}
Write-Host ("Done: {0} cycles in {1:N1} min" -f $Cycles, ((Get-Date) - $started).TotalMinutes)

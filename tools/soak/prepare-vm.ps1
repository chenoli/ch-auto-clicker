param(
    [string]$Root = "C:\soak",
    [int]$PauseDays = 35,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$model = (Get-CimInstance Win32_ComputerSystem).Model
if (-not $Force -and $model -notmatch "Virtual Machine|VMware|VirtualBox") {
    Write-Host "Refusing to change power and lock settings: this does not look like a VM (model '$model')." -ForegroundColor Red
    exit 1
}
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host "Run this from PowerShell as administrator." -ForegroundColor Red
    exit 1
}

function Step([string]$Label, [scriptblock]$Action) {
    try {
        & $Action | Out-Null
        Write-Host "ok   $Label" -ForegroundColor Green
    } catch {
        Write-Host "FAIL $Label : $($_.Exception.Message)" -ForegroundColor Red
    }
}

Step "screen, sleep and hibernate never turn off" {
    powercfg /change monitor-timeout-ac 0
    powercfg /change standby-timeout-ac 0
    powercfg /change hibernate-timeout-ac 0
}
Step "no sign-in required after the display turns off" {
    powercfg /setacvalueindex SCHEME_CURRENT SUB_NONE CONSOLELOCK 0
    powercfg /setactive SCHEME_CURRENT
}
Step "screen saver off" {
    Set-ItemProperty "HKCU:\Control Panel\Desktop" -Name ScreenSaveActive -Value "0"
    Remove-ItemProperty "HKCU:\Control Panel\Desktop" -Name SCRNSAVE.EXE -ErrorAction SilentlyContinue
}
Step "Windows Update paused for $PauseDays days" {
    $key = "HKLM:\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings"
    New-Item $key -Force | Out-Null
    $start = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $end = (Get-Date).AddDays($PauseDays).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    Set-ItemProperty $key -Name PauseUpdatesExpiryTime -Value $end
    Set-ItemProperty $key -Name PauseFeatureUpdatesStartTime -Value $start
    Set-ItemProperty $key -Name PauseFeatureUpdatesEndTime -Value $end
    Set-ItemProperty $key -Name PauseQualityUpdatesStartTime -Value $start
    Set-ItemProperty $key -Name PauseQualityUpdatesEndTime -Value $end
}
Step "PowerShell scripts allowed for this user" {
    Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force
}
Step "folders $Root and $Root\results" {
    New-Item -ItemType Directory -Force -Path $Root, (Join-Path $Root "results")
}
Step "downloaded files unblocked in $Root" {
    Get-ChildItem $Root -Recurse -File | Unblock-File
}

Write-Host ""
Write-Host "Check by hand: Settings > Windows Update shows 'Updates paused', and Settings > Accounts > Sign-in options > 'If you've been away' is 'Never'."
Write-Host "Then shut the VM down and take the 'limpa' checkpoint in Hyper-V Manager."

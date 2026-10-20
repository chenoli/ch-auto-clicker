param(
    [string]$Source = $PSScriptRoot,
    [string]$Root = "C:\soak",
    [switch]$SkipVSCode,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$model = (Get-CimInstance Win32_ComputerSystem).Model
if (-not $Force -and $model -notmatch "Virtual Machine|VMware|VirtualBox") {
    Write-Host "Refusing to run: this does not look like a VM (model '$model')." -ForegroundColor Red
    exit 1
}
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host "Run this as administrator (setup-vm.cmd elevates by itself)." -ForegroundColor Red
    exit 1
}

$results = [ordered]@{}

function Step([string]$Label, [scriptblock]$Action) {
    Write-Host "==> $Label" -ForegroundColor Cyan
    try {
        & $Action
        $results[$Label] = "ok"
    } catch {
        $results[$Label] = "FAIL: $($_.Exception.Message)"
        Write-Host "    $($results[$Label])" -ForegroundColor Red
    }
}

function Update-SessionPath {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")
}

function Test-Winget {
    [bool](Get-Command winget -ErrorAction SilentlyContinue)
}

function Install-WithWinget([string]$Id, [string[]]$Extra = @()) {
    if (-not (Test-Winget)) { return $false }
    $args = @("install", "--id", $Id, "-e", "--silent", "--accept-package-agreements", "--accept-source-agreements", "--disable-interactivity") + $Extra
    & winget @args | Out-Host
    $LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq -1978335189
}

function Invoke-Installer([string]$Url, [string]$FileName, [string[]]$Arguments) {
    $path = Join-Path $env:TEMP $FileName
    Invoke-WebRequest -Uri $Url -OutFile $path -UseBasicParsing
    $process = Start-Process -FilePath $path -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$FileName exited with $($process.ExitCode)" }
}

Step "copy the package to $Root" {
    New-Item -ItemType Directory -Force -Path $Root, (Join-Path $Root "results"), (Join-Path $Root "shots") | Out-Null
    $vm = Join-Path $Source "vm"
    if (-not (Test-Path $vm)) { throw "folder 'vm' not found next to this script ($Source)" }
    Copy-Item (Join-Path $vm "*") $Root -Recurse -Force
    Get-ChildItem $Source -File -Filter *.md | Copy-Item -Destination $Root -Force
    Get-ChildItem $Root -Recurse -File | Unblock-File
}

Step "prepare the VM (power, lock, updates)" {
    & (Join-Path $Root "prepare-vm.ps1") -Root $Root
}

Step "install Git for Windows" {
    if (Test-Path "C:\Program Files\Git\bin\bash.exe") { Write-Host "    already installed"; return }
    if (-not (Install-WithWinget "Git.Git" @("--scope", "machine"))) {
        $release = Invoke-RestMethod "https://api.github.com/repos/git-for-windows/git/releases/latest" -UseBasicParsing
        $asset = $release.assets | Where-Object { $_.name -match '^Git-[\d.]+-64-bit\.exe$' } | Select-Object -First 1
        if (-not $asset) { throw "no 64-bit Git installer in the latest release" }
        Invoke-Installer $asset.browser_download_url $asset.name @("/VERYSILENT", "/NORESTART", "/NOCANCEL", "/SP-", "/SUPPRESSMSGBOXES")
    }
    if (-not (Test-Path "C:\Program Files\Git\bin\bash.exe")) { throw "bash.exe not found after the install" }
}

if (-not $SkipVSCode) {
    Step "install VS Code" {
        if (Test-Path "C:\Program Files\Microsoft VS Code\Code.exe") { Write-Host "    already installed"; return }
        if (-not (Install-WithWinget "Microsoft.VisualStudioCode" @("--scope", "machine"))) {
            Invoke-Installer "https://update.code.visualstudio.com/latest/win32-x64/stable" "vscode-setup.exe" @("/VERYSILENT", "/NORESTART", "/MERGETASKS=!runcode,addtopath")
        }
    }
}

Step "install Claude Code" {
    Update-SessionPath
    [Environment]::SetEnvironmentVariable("CLAUDE_CODE_GIT_BASH_PATH", "C:\Program Files\Git\bin\bash.exe", "User")
    $env:CLAUDE_CODE_GIT_BASH_PATH = "C:\Program Files\Git\bin\bash.exe"
    $exe = Join-Path $env:USERPROFILE ".local\bin\claude.exe"
    if (-not (Test-Path $exe)) {
        $installer = Invoke-RestMethod "https://claude.ai/install.ps1" -UseBasicParsing
        & ([scriptblock]::Create($installer))
    }
    if (-not (Test-Path $exe)) { throw "claude.exe not found in $exe after the install" }
    $bin = Split-Path $exe
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($userPath -split ";") -notcontains $bin) {
        [Environment]::SetEnvironmentVariable("Path", ($userPath.TrimEnd(";") + ";" + $bin), "User")
    }
}

Step "Claude Code settings for $Root" {
    $dir = Join-Path $Root ".claude"
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $settings = [ordered]@{
        permissions = [ordered]@{
            allow = @("Bash", "PowerShell", "Read", "Edit", "Write", "Glob", "Grep")
        }
    }
    $settings | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $dir "settings.json") -Encoding ascii
}

Step "shortcut 'Claude CH Soak' on the desktop" {
    $launcher = Join-Path $Root "claude.cmd"
    Set-Content $launcher -Encoding ascii -Value @(
        "@echo off",
        "cd /d $Root",
        "set CLAUDE_CODE_GIT_BASH_PATH=C:\Program Files\Git\bin\bash.exe",
        "`"%USERPROFILE%\.local\bin\claude.exe`""
    )
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath("Desktop")) "Claude CH Soak.lnk"))
    $link.TargetPath = $launcher
    $link.WorkingDirectory = $Root
    $link.Save()
}

Write-Host ""
Write-Host "Summary" -ForegroundColor Cyan
foreach ($key in $results.Keys) {
    $color = if ($results[$key] -eq "ok") { "Green" } else { "Red" }
    Write-Host ("  {0,-45} {1}" -f $key, $results[$key]) -ForegroundColor $color
}
Write-Host ""
Write-Host "Next: open 'Claude CH Soak' on the desktop, sign in, and say: leia o CLAUDE.md e comece."
Write-Host "Before that, shut down once and take the 'limpa' checkpoint in Hyper-V Manager if you want a clean restore point."
if ($results.Values | Where-Object { $_ -ne "ok" }) { exit 1 }

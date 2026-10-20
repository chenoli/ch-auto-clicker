param(
    [Parameter(Mandatory = $true)][string]$IsoPath,
    [string]$Name = "ch-soak",
    [int]$MemoryGB = 8,
    [int]$Cpu = 4,
    [int]$DiskGB = 64
)

$ErrorActionPreference = "Stop"

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host "Run this from PowerShell as administrator." -ForegroundColor Red
    exit 1
}
if (-not (Get-Command New-VM -ErrorAction SilentlyContinue)) {
    Write-Host "Hyper-V is not enabled. Run: Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All, then restart." -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $IsoPath)) {
    Write-Host "ISO not found: $IsoPath" -ForegroundColor Red
    exit 1
}
if (Get-VM -Name $Name -ErrorAction SilentlyContinue) {
    Write-Host "A VM named '$Name' already exists." -ForegroundColor Red
    exit 1
}

$switch = Get-VMSwitch -Name "Default Switch" -ErrorAction SilentlyContinue
$disk = Join-Path (Get-VMHost).VirtualHardDiskPath "$Name.vhdx"

$vm = if ($switch) {
    New-VM -Name $Name -Generation 2 -MemoryStartupBytes ($MemoryGB * 1GB) -NewVHDPath $disk -NewVHDSizeBytes ($DiskGB * 1GB) -SwitchName $switch.Name
} else {
    New-VM -Name $Name -Generation 2 -MemoryStartupBytes ($MemoryGB * 1GB) -NewVHDPath $disk -NewVHDSizeBytes ($DiskGB * 1GB)
}
Set-VMMemory -VM $vm -DynamicMemoryEnabled $false
Set-VMProcessor -VM $vm -Count $Cpu
Set-VMKeyProtector -VM $vm -NewLocalKeyProtector
Enable-VMTPM -VM $vm
$dvd = Add-VMDvdDrive -VM $vm -Path $IsoPath -Passthru
Set-VMFirmware -VM $vm -FirstBootDevice $dvd
Set-VM -VM $vm -AutomaticCheckpointsEnabled $false -CheckpointType Standard -AutomaticStopAction ShutDown

Write-Host "VM '$Name' created: $MemoryGB GB, $Cpu vCPU, $DiskGB GB disk, TPM on, booting from the ISO." -ForegroundColor Green
if (-not $switch) { Write-Host "No 'Default Switch' found: the VM has no network adapter connected." -ForegroundColor Yellow }
Write-Host "Next: vmconnect localhost $Name, then Start, and press a key when it asks to boot from DVD."

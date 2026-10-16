param([string]$Exe = ".\PrtSc.exe", [int]$Seconds = 10, [string]$Out = ".\idle-measurement.json")
$ErrorActionPreference = "Stop"
if ($Seconds -lt 2) { throw "Use at least 2 seconds" }
$path = (Resolve-Path $Exe).Path
$existing = Get-Process -Name "PrtSc" -ErrorAction SilentlyContinue
if ($existing) { throw "Quit the existing PrtSc instance via its tray menu before this isolated measurement." }
$p = Start-Process -FilePath $path -PassThru
Start-Sleep -Seconds 3
$p.Refresh()
$before = $p.TotalProcessorTime.TotalSeconds
$start = [Diagnostics.Stopwatch]::StartNew()
Start-Sleep -Seconds $Seconds
$p.Refresh()
$elapsed = $start.Elapsed.TotalSeconds
$cpuSeconds = $p.TotalProcessorTime.TotalSeconds - $before
$result = [ordered]@{
  utc = [DateTime]::UtcNow.ToString("o")
  file_bytes = (Get-Item $path).Length
  sha256 = (Get-FileHash $path -Algorithm SHA256).Hash
  scope = "Idle after launch, no capture/clipboard/OCR; native Windows measurement"
  sample_seconds = $elapsed
  working_set_bytes = $p.WorkingSet64
  private_bytes = $p.PrivateMemorySize64
  cpu_percent_one_logical_core = 100.0 * $cpuSeconds / $elapsed
  cpu_percent_entire_machine = 100.0 * $cpuSeconds / $elapsed / [Environment]::ProcessorCount
  capture_to_overlay_ms = $null
  selection_to_clipboard_4k_ms = $null
  os = [Environment]::OSVersion.VersionString
}
$result | ConvertTo-Json | Set-Content -Path $Out -Encoding UTF8
$result | ConvertTo-Json
Write-Host "The application remains in the tray. Exit it from its menu when finished."

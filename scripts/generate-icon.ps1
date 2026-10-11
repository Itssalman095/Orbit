# Generates a 1024x1024 app icon and all platform icon sizes required by Tauri.
# Requires PowerShell with System.Drawing (present on Windows 10/11).

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

$cx = $size / 2
$cy = $size / 2

# Outer glass disc
$outer = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(235, 18, 22, 33))
$g.FillEllipse($outer, 96, 96, $size - 192, $size - 192)

# Ring border
$ring = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(90, 255, 255, 255), 10)
$g.DrawEllipse($ring, 120, 120, $size - 240, $size - 240)

# Six radial dots (the radial menu motif)
$colors = @(
    [System.Drawing.Color]::FromArgb(255, 79, 156, 255),
    [System.Drawing.Color]::FromArgb(255, 124, 107, 255),
    [System.Drawing.Color]::FromArgb(255, 62, 207, 142),
    [System.Drawing.Color]::FromArgb(255, 255, 180, 79),
    [System.Drawing.Color]::FromArgb(255, 255, 95, 109),
    [System.Drawing.Color]::FromArgb(255, 96, 181, 255)
)
$radius = 265
$dotR = 68
for ($i = 0; $i -lt 6; $i++) {
    $angle = -[Math]::PI / 2 + (2 * [Math]::PI * $i / 6)
    $x = $cx + $radius * [Math]::Cos($angle)
    $y = $cy + $radius * [Math]::Sin($angle)
    $brush = New-Object System.Drawing.SolidBrush($colors[$i])
    $g.FillEllipse($brush, [float]($x - $dotR), [float]($y - $dotR), [float](2 * $dotR), [float](2 * $dotR))
}

# Center dot
$center = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 232, 236, 244))
$g.FillEllipse($center, [float]($cx - 84), [float]($cy - 84), 168, 168)

$g.Dispose()

$outDir = Join-Path $PSScriptRoot "..\src-tauri\app-icon.png"
$bmp.Save($outDir, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "icon written to $outDir"

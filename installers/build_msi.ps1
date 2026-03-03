param(
    [string]$Type = "rust" # "rust" or "selfhosted"
)

$wixDir = "C:\Program Files (x86)\WiX Toolset v3.11\bin"
$candle = Join-Path $wixDir "candle.exe"
$light = Join-Path $wixDir "light.exe"

$wxsFile = if ($Type -eq "rust") { "wix\pulse_rust.wxs" } else { "wix\pulse_selfhosted.wxs" }
$objFile = if ($Type -eq "rust") { "pulse_rust.wixobj" } else { "pulse_selfhosted.wixobj" }
$msiFile = if ($Type -eq "rust") { "pulse_rust.msi" } else { "pulse_selfhosted.msi" }

Write-Host "Building MSI for $Type compiler..."

# Compile
& $candle $wxsFile -o $objFile
if ($LASTEXITCODE -ne 0) {
    Write-Error "Candle failed"
    exit $LASTEXITCODE
}

# Link
& $light -ext WixUIExtension $objFile -o $msiFile
if ($LASTEXITCODE -ne 0) {
    Write-Error "Light failed"
    exit $LASTEXITCODE
}

Write-Host "Successfully built $msiFile"
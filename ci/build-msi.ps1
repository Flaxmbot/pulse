param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("rust", "selfhost")]
    [string]$Track,
    [string]$Version = "0.1.2",
    [string]$OutputDir = "dist\windows"
)

$ErrorActionPreference = "Stop"

if ($PSVersionTable.Platform -and $PSVersionTable.Platform -ne "Win32NT") {
    throw "MSI build is supported on Windows only."
}

function Require-Command([string]$name) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        throw "Required command '$name' not found in PATH."
    }
}

function Xml-Escape([string]$value) {
    return [System.Security.SecurityElement]::Escape($value)
}

function New-PathGuid([string]$trackName) {
    if ($trackName -eq "selfhost") {
        return "{6DDE72A0-BD7D-460A-90A3-5A6E5E7AB9D1}"
    }
    return "{9C1E5E9A-13A9-4E57-8E2A-C1D74BA64D5A}"
}

function Get-UpgradeCode([string]$trackName) {
    if ($trackName -eq "selfhost") {
        return "{9A31C9B5-95F6-48E6-A5BF-A17E4C2D0A21}"
    }
    return "{D0C9B433-85EC-4A9C-9B52-5E39E34A4E71}"
}

function Get-DirectoryIdForRelativePath([string]$relPath) {
    $normalized = $relPath.Replace("/", "\")
    if ($normalized.StartsWith("bin\")) {
        return "BinDir"
    }
    if ($normalized.StartsWith("lib\")) {
        return "LibDir"
    }
    if ($normalized.StartsWith("share\pulse\self-hosted\")) {
        return "SelfHostedDir"
    }
    if ($normalized.StartsWith("share\pulse\")) {
        return "PulseShareDir"
    }
    return "INSTALLFOLDER"
}

Require-Command cargo
Require-Command dotnet

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

# Keep MSI builds non-interactive and workspace-local.
$DotnetHome = Join-Path $Root ".dotnet"
if (-not (Test-Path $DotnetHome)) {
    New-Item -ItemType Directory -Path $DotnetHome -Force | Out-Null
}
$env:DOTNET_CLI_HOME = $DotnetHome
$env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE = "1"
$env:DOTNET_NOLOGO = "1"
$env:NPM_CONFIG_FUND = "false"
$env:NPM_CONFIG_AUDIT = "false"

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

Write-Host "[STEP] Building release binaries..."
cargo build --release -p pulse_cli -p pulse_lsp -p pulse_aot_runtime

$Target = Join-Path $Root "target\release"
$Stage = Join-Path $Root ("dist\staging\windows-" + $Track)
if (Test-Path $Stage) {
    Remove-Item -Recurse -Force $Stage
}
$StageBin = Join-Path $Stage "bin"
$StageLib = Join-Path $Stage "lib"
$StageShare = Join-Path $Stage "share\pulse"
New-Item -ItemType Directory -Path $StageBin, $StageLib, $StageShare -Force | Out-Null

Copy-Item (Join-Path $Target "pulse_cli.exe") (Join-Path $StageBin "pulse_cli.exe") -Force
if (Test-Path (Join-Path $Target "pulse-lsp.exe")) {
    Copy-Item (Join-Path $Target "pulse-lsp.exe") (Join-Path $StageBin "pulse-lsp.exe") -Force
}
foreach ($lib in @("pulse_aot_runtime.dll", "pulse_aot_runtime.dll.lib", "pulse_aot_runtime.lib")) {
    $src = Join-Path $Target $lib
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $StageLib $lib) -Force
    }
}

Copy-Item -Recurse -Force (Join-Path $Root "self-hosted") (Join-Path $StageShare "self-hosted")

$defaultTrack = if ($Track -eq "selfhost") { "selfhost" } else { "rust" }
$pulseCmd = @"
@echo off
set PULSE_COMPILER_TRACK=$defaultTrack
set PULSE_HOME=%~dp0..
"%~dp0pulse_cli.exe" %*
"@
$pulseRust = @"
@echo off
set PULSE_COMPILER_TRACK=rust
set PULSE_HOME=%~dp0..
"%~dp0pulse_cli.exe" %*
"@
$pulseSelfhost = @"
@echo off
set PULSE_COMPILER_TRACK=selfhost
set PULSE_HOME=%~dp0..
set PULSE_SELFHOST_ENTRY=%~dp0..\share\pulse\self-hosted\compiler.pulse
"%~dp0pulse_cli.exe" %*
"@

Set-Content -Path (Join-Path $StageBin "pulse.cmd") -Value $pulseCmd -Encoding ASCII
Set-Content -Path (Join-Path $StageBin "pulse-rust.cmd") -Value $pulseRust -Encoding ASCII
Set-Content -Path (Join-Path $StageBin "pulse-selfhost.cmd") -Value $pulseSelfhost -Encoding ASCII

if ((Test-Path (Join-Path $Root "vscode-pulse\package.json")) -and (Get-Command npm -ErrorAction SilentlyContinue)) {
    Push-Location (Join-Path $Root "vscode-pulse")
    npm install --no-audit --no-fund
    npm run compile
    npm run package
    $vsix = Get-ChildItem -Filter *.vsix | Sort-Object LastWriteTime | Select-Object -Last 1
    if ($vsix) {
        Copy-Item $vsix.FullName (Join-Path $StageShare "pulse-language.vsix") -Force
    }
    Pop-Location
}

$allFiles = Get-ChildItem -Path $Stage -Recurse -File | Sort-Object FullName
if (-not $allFiles) {
    throw "No files staged for MSI packaging."
}

$fileLines = New-Object System.Collections.Generic.List[string]
foreach ($file in $allFiles) {
    $relative = $file.FullName.Substring($Stage.Length).TrimStart('\')
    $dirId = Get-DirectoryIdForRelativePath $relative
    $srcEscaped = Xml-Escape($file.FullName)
    $fileLines.Add("    <File Source=""$srcEscaped"" Directory=""$dirId"" />")
}

$productName = if ($Track -eq "selfhost") { "Pulse Selfhost Compiler" } else { "Pulse Rust Compiler" }
$upgradeCode = Get-UpgradeCode $Track
$pathComponentGuid = New-PathGuid $Track

$WixDir = Join-Path $Root ("dist\staging\wix-" + $Track)
if (Test-Path $WixDir) {
    Remove-Item -Recurse -Force $WixDir
}
New-Item -ItemType Directory -Path $WixDir -Force | Out-Null

$projectName = "pulse-" + $Track
$wixprojPath = Join-Path $WixDir ($projectName + ".wixproj")
$wxsPath = Join-Path $WixDir "Package.wxs"
$nugetConfigPath = Join-Path $WixDir "NuGet.config"
$msiPath = Join-Path $OutputDir ("pulse-" + $Track + "-" + $Version + ".msi")

$wixproj = @"
<Project Sdk="WixToolset.Sdk/6.0.0">
  <PropertyGroup>
    <InstallerPlatform>x64</InstallerPlatform>
    <OutputName>$projectName</OutputName>
    <BaseOutputPath>_wix\</BaseOutputPath>
    <SuppressValidation>true</SuppressValidation>
  </PropertyGroup>
</Project>
"@

$nugetConfig = @"
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" protocolVersion="3" />
  </packageSources>
</configuration>
"@

$filesXml = $fileLines -join "`r`n"
$wxs = @"
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Id="Pulse.$Track" Name="$productName" Manufacturer="Pulse" Version="$Version" UpgradeCode="$upgradeCode">
    <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
    <MediaTemplate EmbedCab="yes" />
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLFOLDER" Name="Pulse">
        <Directory Id="BinDir" Name="bin" />
        <Directory Id="LibDir" Name="lib" />
        <Directory Id="ShareDir" Name="share">
          <Directory Id="PulseShareDir" Name="pulse">
            <Directory Id="SelfHostedDir" Name="self-hosted" />
          </Directory>
        </Directory>
      </Directory>
    </StandardDirectory>
$filesXml
    <Component Id="cmpPath" Directory="INSTALLFOLDER" Guid="$pathComponentGuid">
      <RegistryValue Root="HKLM" Key="Software\Pulse" Name="InstallDir" Type="string" Value="[INSTALLFOLDER]" KeyPath="yes" />
      <Environment Name="PATH" Action="set" Part="last" System="yes" Value="[BinDir]" />
    </Component>
  </Package>
</Wix>
"@

Set-Content -Path $wixprojPath -Value $wixproj -Encoding UTF8
Set-Content -Path $nugetConfigPath -Value $nugetConfig -Encoding UTF8
Set-Content -Path $wxsPath -Value $wxs -Encoding UTF8

Write-Host "[STEP] Building MSI via dotnet/WiX SDK..."
Push-Location $WixDir
dotnet restore $wixprojPath --configfile $nugetConfigPath
dotnet build $wixprojPath --configfile $nugetConfigPath -c Release
Pop-Location

$builtMsi = Get-ChildItem -Path $WixDir -Recurse -Filter ($projectName + ".msi") | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $builtMsi) {
    throw "MSI build succeeded but no MSI artifact was found under $WixDir"
}

Copy-Item $builtMsi.FullName $msiPath -Force
Write-Host "[OK] MSI created at $msiPath"


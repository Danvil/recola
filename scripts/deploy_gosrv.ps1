# Set strict mode
Set-StrictMode -Version Latest

# Parameters
$TargetDir = "target\release"
$DeployDir = "deploy\main\gosrv"
$AssetSourceDir = "assets"

# Ensure deploy directory exists
Write-Host "Creating deployment directory..."
New-Item -ItemType Directory -Force -Path $DeployDir | Out-Null

# Copy gosrv
$exePath = Join-Path $TargetDir "gosrv.exe"
if (-Not (Test-Path $exePath)) {
    Write-Error "Executable not found: $exePath. Did you run 'cargo build --release'?"
    exit 1
}
Copy-Item -Force $exePath $DeployDir
Write-Host "Copied gosrv to $DeployDir"

# Copy asset files (flattened into deploy folder)
Write-Host "Copying asset files directly into deploy folder..."
Copy-Item -Recurse -Force "$AssetSourceDir\bin\*" $DeployDir
Write-Host "Assets copied to $DeployDir"

# Create steam_appid.txt (optional for dev)
#$SteamAppId = "480"  # Replace with your actual App ID for production
#$AppIdPath = Join-Path $DeployDir "steam_appid.txt"
#Set-Content -Path $AppIdPath -Value $SteamAppId -Encoding ASCII
#Write-Host "Created steam_appid.txt with AppID $SteamAppId"

Write-Host "`n✅ Deployment folder is ready at: $DeployDir"

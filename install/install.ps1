$REPO = "faisalfakih/cambridge-psudocode-inter"
$BINARY = "cps-windows-x86_64.exe"
$INSTALL_DIR = "$env:LOCALAPPDATA\cps"

Write-Host "🔍 Fetching latest release..."
$VERSION = (Invoke-RestMethod "https://api.github.com/repos/$REPO/releases/latest").tag_name

Write-Host "📥 Downloading cps $VERSION..."
$URL = "https://github.com/$REPO/releases/download/$VERSION/$BINARY"

New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
Invoke-WebRequest -Uri $URL -OutFile "$INSTALL_DIR\cps.exe"

# Add to PATH if not already there
$PATH = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($PATH -notlike "*$INSTALL_DIR*") {
    [Environment]::SetEnvironmentVariable("PATH", "$PATH;$INSTALL_DIR", "User")
    Write-Host "✅ Added $INSTALL_DIR to PATH"
}

try {
    Invoke-RestMethod -Uri "https://api.counterapi.dev/v2/faisal-fakihs-team-3104/pseudocode-interpreter/up" `
        -Headers @{ Authorization = "Bearer ut_mcAnzaVRSEhLNrkq4vCKzvTxlGh6Be03xc4okb4p" } `
        -Method Get | Out-Null
} catch {}

Write-Host ""
Write-Host "✅ Installed cps $VERSION successfully!"
Write-Host "   Restart your terminal then run: cps yourfile.cps"

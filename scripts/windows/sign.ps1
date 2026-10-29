# Signs one file with Azure Artifact Signing. The release CI jobs hand this
# to Tauri as its Windows signCommand, so it runs for knobset.exe, the NSIS
# uninstaller and the installer. azure/login has already signed in through
# GitHub OIDC, and the client's AzureCliCredential picks that up; there's no
# signing secret anywhere.
#
# KNOBSET_SIGN_DLIB is the client's Azure.CodeSigning.Dlib.dll and
# KNOBSET_SIGN_METADATA its metadata.json (endpoint, account, profile).
param([Parameter(Mandatory)][string]$Path)
$ErrorActionPreference = 'Stop'

$dlib = $env:KNOBSET_SIGN_DLIB
$metadata = $env:KNOBSET_SIGN_METADATA
if (-not $dlib -or -not $metadata) {
    throw 'KNOBSET_SIGN_DLIB and KNOBSET_SIGN_METADATA must be set'
}

# The signing client ships for x64 (and x86) only, so this uses the x64
# signtool everywhere; Windows on ARM runs it under emulation.
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" |
    Where-Object { $_.Directory.Parent.Name -match '^\d+(\.\d+){3}$' } |
    Sort-Object { [version]$_.Directory.Parent.Name } |
    Select-Object -Last 1
if (-not $signtool) { throw 'no x64 signtool.exe under Windows Kits\10\bin' }

& $signtool.FullName sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib $dlib /dmdf $metadata $Path
if ($LASTEXITCODE) { throw "signtool exited $LASTEXITCODE signing $Path" }

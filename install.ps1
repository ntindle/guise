# guise installer (Windows).
#
#   irm https://raw.githubusercontent.com/siddhjagani/guise/main/install.ps1 | iex
#
# Works two ways:
#   * piped from irm (no clone)       -> downloads a prebuilt binary from the
#                                         latest GitHub release
#   * run inside a cloned checkout     -> builds from source with cargo
#
# Env overrides:
#   $env:GUISE_VERSION = "v1.2.3"      install a specific release tag (default: latest)

$ErrorActionPreference = "Stop"

$Repo = "siddhjagani/guise"
$BinName = "guise.exe"
$Asset = "guise-windows-x86_64.zip"
$InstallDir = Join-Path $env:LOCALAPPDATA "guise\bin"

# $MyInvocation.MyCommand.Path is unset when piped from irm; fall back to
# prebuilt download in that case.
$RepoDir = ""
if ($MyInvocation.MyCommand.Path) {
    $RepoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

function Install-FromSource {
    Write-Host "Building $BinName from source (release)..."
    $manifest = Join-Path $RepoDir "Cargo.toml"
    cargo build --release --manifest-path $manifest
    Copy-Item -Force (Join-Path $RepoDir "target\release\$BinName") `
        (Join-Path $InstallDir $BinName)
}

function Install-Prebuilt {
    $ver = if ($env:GUISE_VERSION) { $env:GUISE_VERSION } else { "latest" }
    if ($ver -eq "latest") {
        $url = "https://github.com/$Repo/releases/latest/download/$Asset"
    } else {
        $url = "https://github.com/$Repo/releases/download/$ver/$Asset"
    }
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        Write-Host "Downloading $BinName ($ver)..."
        try {
            Invoke-WebRequest -Uri $url -OutFile (Join-Path $tmp $Asset)
        } catch {
            Write-Error ("error: could not download $url`n" +
                "       (no release yet? install from source: " +
                "git clone https://github.com/$Repo && cd guise && ./install.ps1)")
            exit 1
        }
        Expand-Archive -Path (Join-Path $tmp $Asset) -DestinationPath $tmp -Force
        Copy-Item -Force (Join-Path $tmp $BinName) (Join-Path $InstallDir $BinName)
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

if (($RepoDir -ne "") -and (Test-Path (Join-Path $RepoDir "Cargo.toml")) -and
    (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Install-FromSource
} else {
    Install-Prebuilt
}

Write-Host "Installed $(Join-Path $InstallDir $BinName)"
if (($env:PATH -split ";") -notcontains $InstallDir) {
    Write-Host "Note: $InstallDir is not on your PATH. Add it with:"
    Write-Host "  `$p = [Environment]::GetEnvironmentVariable('Path', 'User')"
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"`$p;$InstallDir`", 'User')"
    Write-Host "(restart your terminal afterwards)"
}
Write-Host "Run 'guise doctor' to verify your setup, then 'guise add <name>'."

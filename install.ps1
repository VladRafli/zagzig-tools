# Installs zagzig-tui (the zagzig-tools terminal app) on Windows.
#
#   irm https://raw.githubusercontent.com/VladRafli/zagzig-tools/main/install.ps1 | iex
#
# Optional environment variables (set them before running):
#   ZAGZIG_VERSION      version to install, like 0.14.0 or v0.14.0 (default: the latest release)
#   ZAGZIG_INSTALL_DIR  where to put the program (default: %LOCALAPPDATA%\Programs\zagzig-tui)
#   ZAGZIG_NO_PATH      set to 1 to leave your PATH alone
#
# It downloads the release zip from GitHub, checks its SHA-256 against the
# release's checksums.txt, and copies zagzig-tui.exe into place. No administrator
# rights are needed.
& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'VladRafli/zagzig-tools'
    $asset = 'zagzig-tui-x86_64-pc-windows-msvc.zip'
    $installDir = if ($env:ZAGZIG_INSTALL_DIR) { $env:ZAGZIG_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\zagzig-tui' }

    if ($env:ZAGZIG_VERSION) {
        $version = $env:ZAGZIG_VERSION.TrimStart('v')
        $base = "https://github.com/$repo/releases/download/v$version"
        Write-Host "Installing zagzig-tui v$version"
    } else {
        $base = "https://github.com/$repo/releases/latest/download"
        Write-Host 'Installing the latest zagzig-tui'
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("zagzig-tui-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $zip = Join-Path $tmp $asset
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $zip
        } catch {
            throw "Couldn't download $base/$asset (does that version exist?): $($_.Exception.Message)"
        }

        # The checksum is an integrity check (did the download complete), so a
        # missing checksums.txt only produces a warning.
        try {
            $sums = (Invoke-WebRequest -UseBasicParsing -Uri "$base/checksums.txt").Content
            if ($sums -is [byte[]]) { $sums = [Text.Encoding]::UTF8.GetString($sums) }
            $line = ($sums -split "`r?`n") | Where-Object { $_ -match ("\s" + [regex]::Escape($asset) + "$") } | Select-Object -First 1
            if (-not $line) {
                Write-Warning "checksums.txt has no entry for $asset, skipping the check."
            } else {
                $expected = ($line -split '\s+')[0].ToLower()
                $actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLower()
                if ($expected -ne $actual) {
                    throw "Checksum mismatch for $asset (expected $expected, got $actual). Nothing was installed."
                }
                Write-Host 'Checksum OK.'
            }
        } catch [System.Net.WebException] {
            Write-Warning "Couldn't fetch checksums.txt, skipping the check."
        }

        Expand-Archive -LiteralPath $zip -DestinationPath (Join-Path $tmp 'out') -Force
        $exe = Join-Path $tmp 'out\zagzig-tui.exe'
        if (-not (Test-Path $exe)) { throw "The archive doesn't contain zagzig-tui.exe." }

        if (Get-Process -Name 'zagzig-tui' -ErrorAction SilentlyContinue) {
            throw 'zagzig-tui is running. Close it and run this again.'
        }
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
        Copy-Item -LiteralPath $exe -Destination (Join-Path $installDir 'zagzig-tui.exe') -Force
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host "Installed to $(Join-Path $installDir 'zagzig-tui.exe')"

    $onPath = ($env:Path -split ';') -contains $installDir
    if ($env:ZAGZIG_NO_PATH -eq '1') {
        if (-not $onPath) { Write-Host "Run it with: $(Join-Path $installDir 'zagzig-tui.exe')" }
    } elseif (-not $onPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not (($userPath -split ';') -contains $installDir)) {
            $newPath = if ($userPath) { $userPath.TrimEnd(';') + ';' + $installDir } else { $installDir }
            [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        }
        $env:Path = $env:Path.TrimEnd(';') + ';' + $installDir
        Write-Host 'Added it to your user PATH. Open a new terminal to use it anywhere. Run it with: zagzig-tui'
    } else {
        Write-Host 'Run it with: zagzig-tui'
    }
}

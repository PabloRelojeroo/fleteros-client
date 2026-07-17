param(
    [string] $launcherName,
    [string] $tag,
    [string] $deployRoot = "X:\var\www\html\launchers",
    [string] $repo
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI (gh) no esta instalado o no esta en el PATH. Instalalo desde https://cli.github.com/"
}

$confPath = Join-Path $PSScriptRoot '..\src-tauri\tauri.conf.json'
if (-not (Test-Path $confPath)) {
    throw "No se encontro tauri.conf.json en $confPath"
}
$conf = Get-Content -Raw -Path $confPath | ConvertFrom-Json

if (-not $launcherName) {
    $slug = $conf.productName.ToLowerInvariant() -replace '[^a-z0-9]+', '-'
    $launcherName = $slug.Trim('-')
}
$launcherName = $launcherName.Trim()
if (-not $launcherName) {
    throw "No se pudo determinar launcherName (productName vacio en tauri.conf.json). Pasalo con -launcherName."
}

$ghArgs = @()
if ($repo) { $ghArgs += @('--repo', $repo) }

if (-not $tag) {
    $tag = gh release list --limit 1 --json tagName -q '.[0].tagName' @ghArgs
    if (-not $tag) {
        throw "No se encontro ningun release en GitHub. Pasa -tag explicitamente o pushea un tag para disparar el workflow."
    }
}

Write-Output "Descargando assets del release '$tag'..."
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) "launcher-deploy-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

try {
    gh release download $tag --dir $tempDir --clobber @ghArgs
    if ($LASTEXITCODE -ne 0) {
        throw "gh release download fallo para el tag '$tag'"
    }

    $targetDir = Join-Path $deployRoot $launcherName
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

    $version = $tag.TrimStart('v')
    $platforms = [ordered]@{}

    function Deploy-Asset {
        param($file, [string] $platformKey)
        $sig = Get-ChildItem -Path $tempDir -Filter "$($file.Name).sig" -File | Select-Object -First 1
        if (-not $sig) {
            Write-Warning "No se encontro .sig para $($file.Name), se omite $platformKey"
            return $null
        }
        Copy-Item -Path $file.FullName -Destination (Join-Path $targetDir $file.Name) -Force
        Copy-Item -Path $sig.FullName -Destination (Join-Path $targetDir $sig.Name) -Force
        return [ordered]@{
            signature = [System.IO.File]::ReadAllText($sig.FullName)
            url       = "https://pablorelojero.online/launchers/$launcherName/$([System.Uri]::EscapeDataString($file.Name))"
        }
    }

    $winFile = Get-ChildItem -Path $tempDir -Filter "*-setup.exe" -File | Select-Object -First 1
    if ($winFile) {
        $entry = Deploy-Asset -file $winFile -platformKey "windows-x86_64"
        if ($entry) { $platforms["windows-x86_64"] = $entry }
    }

    $linuxFile = Get-ChildItem -Path $tempDir -Filter "*.AppImage" -File | Select-Object -First 1
    if ($linuxFile) {
        $entry = Deploy-Asset -file $linuxFile -platformKey "linux-x86_64"
        if ($entry) { $platforms["linux-x86_64"] = $entry }
    }

    $macFile = Get-ChildItem -Path $tempDir -Filter "*.app.tar.gz" -File | Select-Object -First 1
    if ($macFile) {
        $entry = Deploy-Asset -file $macFile -platformKey "darwin"
        if ($entry) {
            $platforms["darwin-x86_64"] = $entry
            $platforms["darwin-aarch64"] = $entry
        }
    }

    if ($platforms.Count -eq 0) {
        throw "No se encontro ningun instalador reconocible entre los assets descargados en $tempDir"
    }

    $latest = [ordered]@{
        version   = $version
        notes     = "Release $version"
        pub_date  = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
        platforms = $platforms
    }

    $latestJson = $latest | ConvertTo-Json -Depth 6
    $latestPath = Join-Path $targetDir 'latest.json'
    [System.IO.File]::WriteAllText($latestPath, $latestJson, (New-Object System.Text.UTF8Encoding $false))

    Write-Output "Deployado '$launcherName' version $version a $targetDir"
    Write-Output "Plataformas incluidas: $($platforms.Keys -join ', ')"
    Write-Output "latest.json en $latestPath"
}
finally {
    Remove-Item -Recurse -Force -Path $tempDir -ErrorAction SilentlyContinue
}

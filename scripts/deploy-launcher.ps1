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

    $latestOrigenPath = Join-Path $tempDir 'latest.json'
    if (-not (Test-Path $latestOrigenPath)) {
        throw "El release '$tag' no trae latest.json (lo genera Tauri con createUpdaterArtifacts). No se puede deployar sin la version/firmas reales del build."
    }

    # Se usa el latest.json que genera Tauri como unica fuente de verdad para
    # version/firmas/plataformas: sale del build real, no del nombre del tag.
    # Si el tag no coincide con la version bumpeada en tauri.conf.json (ej. te
    # olvidaste de correr bump-version antes de taggear), ESTE archivo va a
    # tener la version correcta igual, y eso es lo que hay que confiar.
    $latestOrigen = Get-Content -Raw -Path $latestOrigenPath | ConvertFrom-Json

    if ($latestOrigen.version -ne $tag.TrimStart('v')) {
        Write-Warning "El tag es '$tag' pero el build real es version '$($latestOrigen.version)' (tauri.conf.json no se bumpeo antes de taggear). Se va a deployar la version real: $($latestOrigen.version)."
    }

    $targetDir = Join-Path $deployRoot $launcherName
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

    $platforms = [ordered]@{}
    $archivosCopiados = @{}

    foreach ($prop in $latestOrigen.platforms.PSObject.Properties) {
        $plataforma = $prop.Name
        $entradaOrigen = $prop.Value
        $nombreArchivo = [System.Uri]::UnescapeDataString(([System.Uri]$entradaOrigen.url).Segments[-1])

        $archivoLocal = Get-ChildItem -Path $tempDir -Filter $nombreArchivo -File | Select-Object -First 1
        if (-not $archivoLocal) {
            Write-Warning "No se encontro '$nombreArchivo' entre los assets descargados, se omite la plataforma '$plataforma'"
            continue
        }

        if (-not $archivosCopiados.ContainsKey($nombreArchivo)) {
            Copy-Item -Path $archivoLocal.FullName -Destination (Join-Path $targetDir $nombreArchivo) -Force
            $archivosCopiados[$nombreArchivo] = $true
        }

        $platforms[$plataforma] = [ordered]@{
            signature = $entradaOrigen.signature
            url       = "https://pablorelojero.online/launchers/$launcherName/$([System.Uri]::EscapeDataString($nombreArchivo))"
        }
    }

    if ($platforms.Count -eq 0) {
        throw "No se pudo resolver ninguna plataforma del latest.json contra los assets descargados en $tempDir"
    }

    $latest = [ordered]@{
        version   = $latestOrigen.version
        notes     = "Release $($latestOrigen.version)"
        pub_date  = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
        platforms = $platforms
    }

    $latestJson = $latest | ConvertTo-Json -Depth 6
    $latestPath = Join-Path $targetDir 'latest.json'
    [System.IO.File]::WriteAllText($latestPath, $latestJson, (New-Object System.Text.UTF8Encoding $false))

    Write-Output "Deployado '$launcherName' version $($latestOrigen.version) a $targetDir"
    Write-Output "Plataformas incluidas: $($platforms.Keys -join ', ')"
    Write-Output "latest.json en $latestPath"
}
finally {
    Remove-Item -Recurse -Force -Path $tempDir -ErrorAction SilentlyContinue
}

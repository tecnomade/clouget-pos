# =============================================================
# Clouget POS - Script de Release
# Uso:
#   powershell -ExecutionPolicy Bypass -File scripts\release.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Beta
#
# Flags:
#   -Beta  Publica al canal beta (los testers la reciben, clientes estables no)
# =============================================================

param(
    [switch]$Beta
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $ProjectRoot

# --- Leer version actual ---
$PackageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
$Version = $PackageJson.version
$Canal = if ($Beta) { "BETA" } else { "STABLE" }
Write-Host "=== Clouget POS Release v$Version - Canal: $Canal ===" -ForegroundColor Cyan

# --- Rutas ---
$BundleDir = "src-tauri\target\release\bundle\nsis"
$ExeName = "Clouget Punto de Venta_${Version}_x64-setup.exe"
$ZipName = "Clouget Punto de Venta_${Version}_x64-setup.nsis.zip"
$SigName = "$ZipName.sig"
$SignTool = "sign-tool\target\release\sign-tool.exe"
$KeyPath = "$env:USERPROFILE\.tauri\clouget-pos.key"
$GhRepo = "tecnomade/clouget-pos"

# --- Paso 1: Verificar herramientas ---
Write-Host "`n[1/6] Verificando herramientas..." -ForegroundColor Yellow
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: GitHub CLI (gh) no esta instalado. Instalar: winget install GitHub.cli" -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $SignTool)) {
    Write-Host "ERROR: sign-tool no encontrado. Compilar: cd sign-tool && cargo build --release" -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $KeyPath)) {
    Write-Host "ERROR: Clave de firma no encontrada en $KeyPath" -ForegroundColor Red
    exit 1
}
Write-Host "OK" -ForegroundColor Green

# --- Paso 2: Build de produccion ---
Write-Host "`n[2/6] Compilando app..." -ForegroundColor Yellow
npm run tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Build fallo" -ForegroundColor Red
    exit 1
}

$ExePath = Join-Path $BundleDir $ExeName
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: No se encontro el instalador en $ExePath" -ForegroundColor Red
    exit 1
}
Write-Host "OK - Instalador: $ExeName" -ForegroundColor Green

# --- Paso 3: Crear zip para updater ---
Write-Host "`n[3/6] Creando artefacto de update (.nsis.zip)..." -ForegroundColor Yellow
$ZipPath = Join-Path $BundleDir $ZipName
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
python -c "import zipfile,sys; z=zipfile.ZipFile(sys.argv[1],'w',zipfile.ZIP_STORED); z.write(sys.argv[2],sys.argv[3]); z.close()" "$ZipPath" "$ExePath" "$ExeName"
Write-Host "OK - $ZipName ($('{0:N1}' -f ((Get-Item $ZipPath).Length / 1MB)) MB)" -ForegroundColor Green

# --- Paso 4: Firmar ---
Write-Host "`n[4/6] Firmando artefacto..." -ForegroundColor Yellow
& $SignTool $KeyPath $ZipPath
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Firma fallo" -ForegroundColor Red
    exit 1
}
$SigPath = Join-Path $BundleDir $SigName
if (-not (Test-Path $SigPath)) {
    Write-Host "ERROR: Archivo de firma no encontrado" -ForegroundColor Red
    exit 1
}
Write-Host "OK - Firmado" -ForegroundColor Green

# --- Paso 5: Generar latest.json ---
Write-Host "`n[5/6] Generando latest.json..." -ForegroundColor Yellow

$SigBytes = [System.IO.File]::ReadAllBytes($SigPath)
$Signature = [System.Convert]::ToBase64String($SigBytes)
$ZipUrlName = $ZipName -replace ' ', '.'
$PubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$LatestJson = @{
    version = $Version
    notes = "Clouget Punto de Venta v$Version$(if ($Beta) { ' (BETA)' })"
    pub_date = $PubDate
    platforms = @{
        "windows-x86_64" = @{
            signature = $Signature
            url = "https://github.com/$GhRepo/releases/download/v$Version/$ZipUrlName"
        }
    }
} | ConvertTo-Json -Depth 4

$LatestJsonPath = Join-Path $BundleDir "latest.json"
[System.IO.File]::WriteAllText($LatestJsonPath, $LatestJson, (New-Object System.Text.UTF8Encoding $false))
Write-Host "OK - latest.json generado (sin BOM)" -ForegroundColor Green

# --- Paso 6: Crear GitHub Release ---
Write-Host "`n[6/6] Creando GitHub Release v$Version..." -ForegroundColor Yellow

$ReleaseFiles = @(
    (Join-Path $BundleDir $ExeName),
    (Join-Path $BundleDir $ZipName),
    (Join-Path $BundleDir $SigName),
    $LatestJsonPath
)

# Verificar que todos los archivos existen
foreach ($f in $ReleaseFiles) {
    if (-not (Test-Path $f)) {
        Write-Host "ERROR: No existe $f" -ForegroundColor Red
        exit 1
    }
}

# El release SIEMPRE se crea como v$Version (prerelease si beta)
# Los archivos siempre suben a ese release
$TitleSuffix = if ($Beta) { " (BETA)" } else { "" }
$PrereleaseFlag = if ($Beta) { "--prerelease" } else { "" }

# Crear el release
if ($Beta) {
    gh release create "v$Version" `
        --repo $GhRepo `
        --title "Clouget POS v$Version$TitleSuffix" `
        --notes "Clouget Punto de Venta v$Version (Version BETA - solo testers)" `
        --prerelease `
        $ReleaseFiles[0] $ReleaseFiles[1] $ReleaseFiles[2] $ReleaseFiles[3]
} else {
    gh release create "v$Version" `
        --repo $GhRepo `
        --title "Clouget POS v$Version" `
        --notes "Clouget Punto de Venta v$Version" `
        $ReleaseFiles[0] $ReleaseFiles[1] $ReleaseFiles[2] $ReleaseFiles[3]
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: No se pudo crear el release en GitHub" -ForegroundColor Red
    Write-Host "Verifica: gh auth status" -ForegroundColor Yellow
    exit 1
}

if ($Beta) {
    # CANAL BETA: publicar latest.json en el tag fijo 'beta-channel' (movible)
    # Los clientes con canal=beta consultan ese tag especifico
    Write-Host "`n[Extra] Publicando manifest en canal BETA..." -ForegroundColor Yellow

    # Borrar release anterior de beta-channel si existe (para poder reemplazarlo)
    gh release view "beta-channel" --repo $GhRepo *> $null
    if ($LASTEXITCODE -eq 0) {
        gh release delete "beta-channel" --repo $GhRepo --cleanup-tag --yes
    }

    # Crear un release "movible" que contenga solo el latest.json apuntando a esta beta
    gh release create "beta-channel" `
        --repo $GhRepo `
        --title "Canal Beta (ultima beta disponible)" `
        --notes "Este release contiene el manifest latest.json del canal BETA. No instalar directamente." `
        --prerelease `
        $LatestJsonPath
    # Tambien el exe e instalador para debug
    gh release upload "beta-channel" $ReleaseFiles[1] $ReleaseFiles[2] --repo $GhRepo --clobber

    Write-Host "OK - Canal beta actualizado. URL del manifest:" -ForegroundColor Green
    Write-Host "  https://github.com/$GhRepo/releases/download/beta-channel/latest.json" -ForegroundColor White
} else {
    # CANAL STABLE: subir copia con nombre fijo para landing page
    $FixedExe = Join-Path $BundleDir "Clouget-POS-setup.exe"
    Copy-Item $ExePath $FixedExe -Force
    gh release upload "v$Version" $FixedExe --repo $GhRepo --clobber
    Write-Host "OK - Subido Clouget-POS-setup.exe (enlace fijo para landing)" -ForegroundColor Green
}

Write-Host "`n=== Release v$Version publicado exitosamente en canal $Canal ===" -ForegroundColor Green
Write-Host "URL: https://github.com/$GhRepo/releases/tag/v$Version" -ForegroundColor Cyan
if ($Beta) {
    Write-Host "Canal BETA manifest: https://github.com/$GhRepo/releases/download/beta-channel/latest.json" -ForegroundColor Yellow
    Write-Host "`nSOLO los clientes con 'Canal: beta' recibiran esta actualizacion." -ForegroundColor Yellow
}
Write-Host "`nArchivos subidos:" -ForegroundColor White
Write-Host "  - $ExeName" -ForegroundColor White
Write-Host "  - $ZipName" -ForegroundColor White
Write-Host "  - $SigName" -ForegroundColor White
Write-Host "  - latest.json" -ForegroundColor White
if (-not $Beta) {
    Write-Host "  - Clouget-POS-setup.exe (enlace fijo para landing page)" -ForegroundColor White
}

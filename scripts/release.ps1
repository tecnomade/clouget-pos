# =============================================================
# Clouget POS - Script de Release
# Uso: powershell -ExecutionPolicy Bypass -File scripts\release.ps1
# =============================================================

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $ProjectRoot

# --- Leer version actual ---
$PackageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
$Version = $PackageJson.version
Write-Host "=== Clouget POS Release v$Version ===" -ForegroundColor Cyan

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
Compress-Archive -Path $ExePath -DestinationPath $ZipPath -CompressionLevel Optimal
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
$Signature = Get-Content $SigPath -Raw
$ZipUrlName = $ZipName -replace ' ', '.'
$PubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$LatestJson = @{
    version = $Version
    notes = "Clouget Punto de Venta v$Version"
    pub_date = $PubDate
    platforms = @{
        "windows-x86_64" = @{
            signature = $Signature.Trim()
            url = "https://github.com/$GhRepo/releases/download/v$Version/$ZipUrlName"
        }
    }
} | ConvertTo-Json -Depth 4

$LatestJsonPath = Join-Path $BundleDir "latest.json"
Set-Content -Path $LatestJsonPath -Value $LatestJson -Encoding UTF8
Write-Host "OK - latest.json generado" -ForegroundColor Green

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

gh release create "v$Version" `
    --repo $GhRepo `
    --title "Clouget POS v$Version" `
    --notes "Clouget Punto de Venta v$Version" `
    $ReleaseFiles[0] $ReleaseFiles[1] $ReleaseFiles[2] $ReleaseFiles[3]

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: No se pudo crear el release en GitHub" -ForegroundColor Red
    Write-Host "Verifica: gh auth status" -ForegroundColor Yellow
    exit 1
}

Write-Host "`n=== Release v$Version publicado exitosamente ===" -ForegroundColor Green
Write-Host "URL: https://github.com/$GhRepo/releases/tag/v$Version" -ForegroundColor Cyan
Write-Host "`nArchivos subidos:" -ForegroundColor White
Write-Host "  - $ExeName (instalador para nuevos clientes)" -ForegroundColor White
Write-Host "  - $ZipName (artefacto de auto-update)" -ForegroundColor White
Write-Host "  - $SigName (firma criptografica)" -ForegroundColor White
Write-Host "  - latest.json (manifiesto de update)" -ForegroundColor White

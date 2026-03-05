Write-Host "🔍 Verificando dependencias de desarrollo para macmon v4..."

# 1. Verificar Rust / Cargo
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "❌ Error: 'cargo' no está instalado. Instala Rust (https://rustup.rs/)."
    exit 1
}
Write-Host "✅ cargo detectado: $(cargo --version)"

# 2. Verificar Node.js / NPM
if (!(Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Error "❌ Error: 'npm' no está instalado. Instala Node.js."
    exit 1
}
Write-Host "✅ npm detectado: $(npm --version)"

# 3. Verificar WebView2 (requerido para Tauri en Windows)
$webview2 = Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if ($null -eq $webview2) {
    Write-Warning "⚠️ Advertencia: WebView2 Runtime no parece estar instalado. Tauri lo requiere en Windows."
    Write-Warning "💡 Instálalo desde: https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
} else {
    Write-Host "✅ WebView2 Runtime detectado."
}

# 4. Instalando dependencias NPM
Write-Host "`n📦 Instalando dependencias de NPM (si aplica)..."
if (Test-Path "apps\desktop") {
    Set-Location apps\desktop
    if (Test-Path "package.json") {
        npm install
    } else {
        Write-Warning "⚠️ No se encontró package.json en apps/desktop, omitiendo npm install."
    }
    Set-Location ..\..
}

Write-Host "🚀 ¡Entorno configurado correctamente! Usa 'make dev' para iniciar."

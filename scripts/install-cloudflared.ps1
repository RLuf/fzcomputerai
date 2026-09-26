<#
.SYNOPSIS
    Script auxiliar para download e instalacao do Cloudflare Tunnel (cloudflared).
    Utilizado pelo FzComputerAI para exposicao segura do servidor MCP na internet.

.DESCRIPTION
    Este script verifica se o cloudflared ja esta presente no sistema. Caso contrario,
    efetua o download do executavel oficial da Cloudflare e o posiciona no diretorio
    padrao de binarios do FzComputerAI (%LOCALAPPDATA%\FzComputerAI\bin).
#>
[CmdletBinding()]
param(
    [string]$TargetDir = "$env:LOCALAPPDATA\FzComputerAI\bin"
)

$ErrorActionPreference = 'Stop'

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   FzComputerAI - Instalador Auxiliar do Cloudflare Tunnel" -ForegroundColor Yellow
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Verifica se ja existe no PATH e se e funcional (ignora symlink quebrado)
$existing = Get-Command cloudflared.exe -ErrorAction SilentlyContinue
if ($existing) {
    try {
        $ver = & $existing.Source --version 2>$null
        if ($ver -match "cloudflared") {
            Write-Host "[OK] cloudflared detectado e funcional no PATH: $($existing.Source)" -ForegroundColor Green
            Write-Host "   $ver" -ForegroundColor Gray
            Write-Host ""
            Write-Host "Nenhuma acao necessaria." -ForegroundColor Cyan
            exit 0
        }
    } catch {
        # Continua para o download se o executavel falhar
    }
    Write-Host "[AVISO] O arquivo no PATH ($($existing.Source)) esta inacessivel ou quebrado. Prosseguindo com download..." -ForegroundColor Yellow
}

# 2. Cria diretorio de destino se necessario
if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

$destExe = Join-Path $TargetDir "cloudflared.exe"

if (Test-Path $destExe) {
    Write-Host "[OK] cloudflared ja presente em: $destExe" -ForegroundColor Green
    & $destExe --version
    exit 0
}

Write-Host "Baixando o binario oficial do cloudflared (x64 Windows)..." -ForegroundColor Yellow

# Tenta via winget primeiro (valida integridade)
$hasWinget = Get-Command winget.exe -ErrorAction SilentlyContinue
$installedViaWinget = $false

if ($hasWinget) {
    Write-Host "Tentando instalacao via winget..." -ForegroundColor Gray
    try {
        & winget.exe install --exact --id Cloudflare.cloudflared --source winget --installer-type portable --accept-source-agreements --accept-package-agreements --disable-interactivity --location $TargetDir 2>&1 | Out-Null
        if (Test-Path $destExe) {
            $installedViaWinget = $true
        }
    } catch {
        Write-Host "winget retornou aviso, tentando download direto..." -ForegroundColor Gray
    }
}

# Fallback: download direto do GitHub oficial da Cloudflare
if (-not $installedViaWinget -and -not (Test-Path $destExe)) {
    $downloadUrl = "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe"
    Write-Host "Baixando de: $downloadUrl" -ForegroundColor Gray
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
        Invoke-WebRequest -Uri $downloadUrl -OutFile $destExe -UseBasicParsing
    } catch {
        Write-Host "[FALHA] Nao foi possivel baixar o cloudflared: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "Voce tambem pode baixar manualmente em: https://github.com/cloudflare/cloudflared/releases" -ForegroundColor Yellow
        exit 1
    }
}

if (Test-Path $destExe) {
    Write-Host "[OK] cloudflared instalado com sucesso em: $destExe" -ForegroundColor Green
    & $destExe --version
    Write-Host ""
    Write-Host "A GUI do FzComputerAI detectara o executavel automaticamente." -ForegroundColor Cyan
} else {
    Write-Host "[FALHA] O arquivo nao foi encontrado apos o download." -ForegroundColor Red
    exit 1
}

# ==============================================================================
# Script de Instalação do FzComputerAI / CUA Driver (Computer Vision via MCP)
# Suporte Nativo: Windows PowerShell 5.1+ / PowerShell Core 7+
# Licença: Creative Commons Attribution 4.0 International (CC BY 4.0)
# Desenvolvido por: Roger Luft (Webstorage Tecnologia & Imóvel Site)
# Patrocinadores: www.webstorage.com.br | www.imovelsite.com.br
# ==============================================================================

[CmdletBinding()]
param(
    [switch]$Advanced,
    [switch]$AutoStart = $true,
    [switch]$NoAutoStart,
    [switch]$NoPathUpdate,
    [string]$InstallDir = "",
    [string]$HttpPort = "8000"
)

if ($NoAutoStart) { $AutoStart = $false }

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Header {
    Write-Host ""
    Write-Host "======================================================================" -ForegroundColor Cyan
    Write-Host "   FzComputerAI — Servidor de Computer Vision via MCP (Windows)" -ForegroundColor Yellow
    Write-Host "   Webstorage Tecnologia (www.webstorage.com.br)" -ForegroundColor Green
    Write-Host "   Imóvel Site (www.imovelsite.com.br)" -ForegroundColor Green
    Write-Host "   Autor: Roger Luft <roger@webstorage.com.br>" -ForegroundColor Green
    Write-Host "======================================================================" -ForegroundColor Cyan
    Write-Host ""
}

function Write-Success([string]$msg) {
    Write-Host "[SUCCESS] $msg" -ForegroundColor Green
}

function Write-Info([string]$msg) {
    Write-Host "[INFO] $msg" -ForegroundColor Cyan
}

function Write-Warn([string]$msg) {
    Write-Host "[AVISO] $msg" -ForegroundColor Yellow
}

function Write-Err([string]$msg) {
    Write-Host "[ERRO] $msg" -ForegroundColor Red
}

Write-Header

$ScriptRoot = $PSScriptRoot
if (-not $ScriptRoot) { $ScriptRoot = Get-Location }

Write-Info "Iniciando verificação de dependências e ambiente..."

# Configurar variável para transporte HTTP TCP na porta informada (ex: 8000)
if ($HttpPort) {
    Write-Info "Configurando variável CUA_DRIVER_RS_MCP_HTTP_PORT=$HttpPort para suporte TCP/IP..."
    [Environment]::SetEnvironmentVariable("CUA_DRIVER_RS_MCP_HTTP_PORT", $HttpPort, "User")
    $env:CUA_DRIVER_RS_MCP_HTTP_PORT = $HttpPort
    Write-Success "Transporte HTTP/TCP habilitado na porta $HttpPort."
}

# 1. Verificar se Rust/Cargo está instalado
$hasCargo = $false
try {
    $cargoVersion = & cargo --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        $hasCargo = $true
        Write-Success "Rust/Cargo detectado: $cargoVersion"
    }
} catch {
    $hasCargo = $false
}

# 2. Verificar se cua-driver já está instalado no PATH
$hasCuaDriver = $false
try {
    $cuaVersion = & cua-driver --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        $hasCuaDriver = $true
        Write-Success "cua-driver detectado no PATH: $cuaVersion"
    }
} catch {
    $hasCuaDriver = $false
}

# Modos de instalação
if ($Advanced) {
    Write-Info "Modo de Instalação: AVANÇADO / CUSTOMIZADO"
} else {
    Write-Info "Modo de Instalação: PADRÃO (Automático)"
}

$BinPath = ""

# Se Cargo estiver disponível e o projeto local contiver a crate cua-driver
$rustWorkspacePath = Join-Path $ScriptRoot "cua\libs\cua-driver\rust\Cargo.toml"

if (-not (Test-Path $rustWorkspacePath)) {
    $rustWorkspacePath = Join-Path $ScriptRoot "libs\cua-driver\rust\Cargo.toml"
}

if ($hasCargo -and (Test-Path $rustWorkspacePath)) {
    Write-Info "Compilando o motor cua-driver em modo Release via Cargo..."
    Push-Location (Split-Path -Parent $rustWorkspacePath)
    try {
        & cargo build --release --package cua-driver
        if ($LASTEXITCODE -eq 0) {
            $BinPath = Join-Path (Split-Path -Parent $rustWorkspacePath) "target\release\cua-driver.exe"
            Write-Success "Compilação concluída com sucesso: $BinPath"
        } else {
            Write-Warn "Falha na compilação local via Cargo. Buscando instalações alternativas..."
        }
    } finally {
        Pop-Location
    }
}

if (-not $BinPath -or -not (Test-Path $BinPath)) {
    if ($hasCuaDriver) {
        $BinPath = (Get-Command cua-driver).Source
        Write-Info "Utilizando binário do PATH: $BinPath"
    } else {
        # Executa o instalador remoto do Cua
        Write-Info "Baixando e instalando o binário oficial cua-driver via PowerShell..."
        $installUrl = "https://cua.ai/driver/install.ps1"
        try {
            Invoke-Expression (Invoke-RestMethod -Uri $installUrl)
            Write-Success "Instalação oficial concluída!"
            $BinPath = Join-Path $env:LOCALAPPDATA "Programs\Cua\cua-driver\bin\cua-driver.exe"
        } catch {
            Write-Err "Não foi possível baixar ou compilar o cua-driver. Verifique sua conexão ou instale o Rust."
            exit 1
        }
    }
}

# Adicionar ao PATH do Usuário se necessário
if (-not $NoPathUpdate -and $BinPath) {
    $binDir = Split-Path -Parent $BinPath
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$binDir*") {
        Write-Info "Adicionando $binDir ao PATH do Usuário..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$binDir", "User")
        $env:Path = "$env:Path;$binDir"
        Write-Success "PATH atualizado com sucesso."
    }
}

# Gerar arquivo de configuração MCP local (.mcp.json)
$mcpJsonPath = Join-Path $ScriptRoot ".mcp.json"
Write-Info "Configurando servidor MCP local em $mcpJsonPath..."
$mcpConfig = @{
    "mcpServers" = @{
        "fz-computer-vision" = @{
            "command" = "cua-driver"
            "args" = @("mcp")
            "env" = @{
                "RUST_LOG" = "info"
                "CUA_DRIVER_RS_MCP_HTTP_PORT" = $HttpPort
            }
        }
    }
}
$mcpConfig | ConvertTo-Json -Depth 5 | Set-Content -Path $mcpJsonPath -Encoding UTF8
Write-Success "Arquivo .mcp.json criado/atualizado com sucesso."

# Registrar Autostart se solicitado
if ($AutoStart -and $BinPath -and (Test-Path $BinPath)) {
    Write-Info "Verificando/Registrando serviço de inicialização automática..."
    try {
        & "$BinPath" stop 2>$null
        & "$BinPath" autostart enable 2>$null
        & "$BinPath" autostart kick 2>$null
        Write-Success "Autostart ativado e reiniciado com porta TCP $HttpPort."
    } catch {
        Write-Warn "Não foi possível ativar o autostart automaticamente."
    }
}

# Diagnóstico de saúde
Write-Info "Executando verificação de saúde do sistema (cua-driver doctor)..."
if (Get-Command cua-driver -ErrorAction SilentlyContinue) {
    & cua-driver doctor
} elseif (Test-Path $BinPath) {
    & "$BinPath" doctor
}

Write-Host ""
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   Instalação concluída com sucesso!" -ForegroundColor Green
Write-Host "   Servidor MCP em Stdio: cua-driver mcp" -ForegroundColor Yellow
Write-Host "   Servidor MCP em TCP/IP: http://127.0.0.1:$HttpPort/mcp" -ForegroundColor Yellow
Write-Host "   Patrocinadores: www.webstorage.com.br | www.imovelsite.com.br" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host ""

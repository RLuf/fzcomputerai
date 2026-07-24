# ==============================================================================
# Script de Instalação do FzComputerAI (Computer Vision via MCP & GUI Rust)
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
    Write-Host "   FzComputerAI -- Servidor de Computer Vision via MCP (Windows)" -ForegroundColor Yellow
    Write-Host "   Webstorage Tecnologia (www.webstorage.com.br)" -ForegroundColor Green
    Write-Host "   Imovel Site (www.imovelsite.com.br)" -ForegroundColor Green
    Write-Host "   Autor: Roger Luft roger@webstorage.com.br" -ForegroundColor Green
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

function Set-FzCodeSigning([string]$exePath) {
    if ($exePath -and (Test-Path $exePath)) {
        Write-Info "Aplicando autoassinatura de codigo (Authenticode) em: $exePath"
        try {
            $cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Where-Object { $_.Subject -like "*FzComputerAI*" } | Select-Object -First 1
            if (-not $cert) {
                Write-Info "Gerando novo Certificado Digital de Codigo (CN=FzComputerAI)..."
                $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=FzComputerAI (Webstorage Tecnologia)" -CertStoreLocation "Cert:\CurrentUser\My"
                $rootStore = New-Object System.Security.Cryptography.X509Certificates.X509Store("Root", "CurrentUser")
                $rootStore.Open("ReadWrite")
                $rootStore.Add($cert)
                $rootStore.Close()
                Write-Success "Certificado gerado e instalado na Raiz Confiavel (Cert:\CurrentUser\Root)."
            } else {
                Write-Info "Utilizando certificado existente: $($cert.Thumbprint)"
            }
            Set-AuthenticodeSignature -FilePath $exePath -Certificate $cert -ErrorAction SilentlyContinue | Out-Null
            Write-Success "Binario assinado digitalmente com sucesso!"
        } catch {
            Write-Warn "Nao foi possivel aplicar a autoassinatura de codigo: $_"
        }
    }
}

Write-Header

$ScriptRoot = $PSScriptRoot
if (-not $ScriptRoot) { $ScriptRoot = Get-Location }

Write-Info "Iniciando verificacao de dependencias e ambiente..."

if ($HttpPort) {
    Write-Info "Configurando variavel CUA_DRIVER_RS_MCP_HTTP_PORT=$HttpPort para suporte TCP/IP..."
    [Environment]::SetEnvironmentVariable("CUA_DRIVER_RS_MCP_HTTP_PORT", $HttpPort, "User")
    $env:CUA_DRIVER_RS_MCP_HTTP_PORT = $HttpPort
    Write-Success "Transporte HTTP/TCP habilitado na porta $HttpPort."
}

# 1. Verificar se Rust/Cargo esta instalado
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

# 2. Verificar se cua-driver ja esta instalado no PATH
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

$BinPath = ""

# 2.1 Compilar GUI FzComputerAI em Rust (se presente)
$guiCargoPath = Join-Path $ScriptRoot "fzcomputerai\Cargo.toml"
if ($hasCargo -and (Test-Path $guiCargoPath)) {
    Write-Info "Compilando a interface grafica FzComputerAI (GUI Rust)..."
    Push-Location (Split-Path -Parent $guiCargoPath)
    try {
        & cargo build --release
        if ($LASTEXITCODE -eq 0) {
            $guiBin = Join-Path (Split-Path -Parent $guiCargoPath) "target\release\fzcomputerai.exe"
            Set-FzCodeSigning -exePath $guiBin
            Write-Success "Interface grafica FzComputerAI compilada e assinada em $guiBin"
        } else {
            Write-Warn "Falha ao compilar a GUI Rust. O servidor MCP continuara funcionando normalmente."
        }
    } finally {
        Pop-Location
    }
}

# 2.2 Compilar o motor cua-driver
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
            Set-FzCodeSigning -exePath $BinPath
            Write-Success "Compilacao do motor concluida e assinada em: $BinPath"
        } else {
            Write-Warn "Falha na compilacao local via Cargo. Buscando instalacoes alternativas..."
        }
    } finally {
        Pop-Location
    }
}

if (-not $BinPath -or -not (Test-Path $BinPath)) {
    if ($hasCuaDriver) {
        $BinPath = (Get-Command cua-driver).Source
        Write-Info "Utilizando binario do PATH: $BinPath"
    } else {
        Write-Info "Baixando e instalando o binario oficial cua-driver via PowerShell..."
        $installUrl = "https://cua.ai/driver/install.ps1"
        try {
            Invoke-Expression (Invoke-RestMethod -Uri $installUrl)
            Write-Success "Instalacao oficial concluida!"
            $BinPath = Join-Path $env:LOCALAPPDATA "Programs\Cua\cua-driver\bin\cua-driver.exe"
        } catch {
            Write-Err "Nao foi possivel baixar ou compilar o cua-driver."
            exit 1
        }
    }
}

# Adicionar ao PATH se necessario
if (-not $NoPathUpdate -and $BinPath) {
    $binDir = Split-Path -Parent $BinPath
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$binDir*") {
        Write-Info "Adicionando $binDir ao PATH do Usuario..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$binDir", "User")
        $env:Path = "$env:Path;$binDir"
        Write-Success "PATH atualizado com sucesso."
    }
}

# Gerar arquivo de configuracao MCP local (.mcp.json)
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

# Registrar Autostart
if ($AutoStart -and $BinPath -and (Test-Path $BinPath)) {
    Write-Info "Verificando/Registrando servico de inicializacao automatica..."
    try {
        & "$BinPath" stop 2>$null
        & "$BinPath" autostart enable 2>$null
        & "$BinPath" autostart kick 2>$null
        Write-Success "Autostart ativado e reiniciado com porta TCP $HttpPort."
    } catch {
        Write-Warn "Nao foi possivel ativar o autostart automaticamente."
    }
}

# Diagnostico de saude
Write-Info "Executando verificacao de saude do sistema (cua-driver doctor)..."
if (Get-Command cua-driver -ErrorAction SilentlyContinue) {
    & cua-driver doctor
} elseif (Test-Path $BinPath) {
    & "$BinPath" doctor
}

Write-Host ""
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   Instalacao do FzComputerAI concluida com sucesso!" -ForegroundColor Green
Write-Host "   Servidor MCP em Stdio: cua-driver mcp" -ForegroundColor Yellow
Write-Host "   Servidor MCP em TCP/IP: http://127.0.0.1:$HttpPort/mcp" -ForegroundColor Yellow
Write-Host "   Patrocinadores: www.webstorage.com.br | www.imovelsite.com.br" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host ""

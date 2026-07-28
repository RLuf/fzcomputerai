# ==============================================================================
# FzComputerAI - Verificacao POS-INSTALACAO (honesta: testa, nao supoe)
#
# Executado pelo instalador grafico (checkbox da pagina final) ou manualmente:
#   powershell -ExecutionPolicy Bypass -File "%LOCALAPPDATA%\Programs\FzComputerAI\verify-install.ps1"
#
# O relatorio responde, com TESTES REAIS (nunca suposicao):
#   1. A GUI foi instalada?               (arquivo no diretorio de instalacao)
#   2. O programa esta abrindo/aberto?    (processo em execucao)
#   3. O motor cua-driver foi instalado?  (where no PATH)
#   4. Esta no inicializar do Windows?    (HKCU\...\Run\FzComputerAI)
#   5. Qual porta/bind estao configurados (HKCU\Environment)
#   6. Em QUAIS enderecos a porta esta OUVINDO de fato (netstat)
#   7. O MCP esta FUNCIONAL?              (conexao TCP real em 127.0.0.1)
#
# Texto em ASCII de proposito: o Windows PowerShell 5.1 le este arquivo com a
# codepage ANSI local e acentuacao viraria mojibake.
# ==============================================================================
param()
$ErrorActionPreference = 'Continue'

function Ok($m)    { Write-Host "[OK]     $m" -ForegroundColor Green }
function Falha($m) { Write-Host "[FALHA]  $m" -ForegroundColor Red }
function Info($m)  { Write-Host "[INFO]   $m" -ForegroundColor Cyan }

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   FzComputerAI - Relatorio de verificacao da instalacao" -ForegroundColor Yellow
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host ""

# Da tempo ao daemon recem instalado/reiniciado de abrir o listener antes dos
# testes de porta (o instalador do motor roda logo antes deste relatorio).
Start-Sleep -Seconds 3

# --- 1. GUI instalada (este script e instalado no MESMO diretorio do exe) ---
$appDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$gui = Join-Path $appDir 'fzcomputerai.exe'
if (Test-Path $gui) {
    Ok "GUI instalada: $gui"
} else {
    Falha "GUI nao encontrada em $appDir"
}

# --- 2. Programa aberto? ----------------------------------------------------
if (Get-Process fzcomputerai -ErrorAction SilentlyContinue) {
    Ok "Programa em execucao (processo fzcomputerai ativo)."
} else {
    Info "Programa nao esta aberto neste momento (nao e um erro)."
}

# --- 3. Motor cua-driver ----------------------------------------------------
$cua = Get-Command cua-driver.exe -ErrorAction SilentlyContinue
if ($cua) {
    Ok "Motor cua-driver instalado: $($cua.Source)"
} else {
    Falha "Motor cua-driver NAO encontrado no PATH. Sem ele NENHUMA acao funciona. Use o botao 'Instalar motor cua-driver' da GUI ou rode o instalador de novo com a task do motor marcada."
}

# --- 4. Inicializar com o Windows ------------------------------------------
$run = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'FzComputerAI' -ErrorAction SilentlyContinue
if ($run) {
    Ok "Autostart ATIVO no registro: HKCU Run\FzComputerAI = $($run.FzComputerAI)"
} else {
    Info "Autostart desativado (task 'Iniciar com o Windows' nao marcada; pode ser ligado na GUI)."
}

# --- 5. Configuracao de porta/bind ------------------------------------------
$port = [Environment]::GetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', 'User')
if (-not $port) { $port = '8000' }
$bind = [Environment]::GetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_BIND', 'User')
if (-not $bind) { $bind = '(nao definido - default do driver)' }
Info "Porta MCP configurada: $port | Bind configurado: $bind"

# --- 6. Em quais enderecos a porta esta OUVINDO de fato ---------------------
$listeners = netstat -ano -p tcp | Select-String 'LISTENING' | Select-String (":" + $port + "\s")
if ($listeners) {
    Ok "Listeners REAIS na porta ${port} (netstat):"
    $listeners | ForEach-Object { Write-Host ("         " + $_.Line.Trim()) }
    if ($listeners | Select-String ("0\.0\.0\.0:" + $port)) {
        Ok "Ouvindo em 0.0.0.0 - TODAS as interfaces (acessivel pela LAN)."
    } else {
        Info "NAO esta em 0.0.0.0 - provavelmente apenas loopback (fallback 127.0.0.1). Use a GUI (aba MCP & Rede) para publicar na LAN."
    }
} else {
    Falha "NENHUM listener na porta $port. O daemon nao esta rodando. Inicie pela GUI ou: cua-driver autostart kick"
}

# --- 7. MCP funcional? POST JSON-RPC REAL em CADA endereco -------------------
# TCP aceitar nao prova MCP: o teste envia um initialize JSON-RPC de verdade
# e exige resposta com "jsonrpc". Testa 127.0.0.1 e o IP da LAN (se houver).
$mcpBody = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"verify-install","version":"2.0.0"}}}'
$mcpHeaders = @{ 'Accept' = 'application/json, text/event-stream' }

function Test-McpEndpoint([string]$addr, [string]$p, [bool]$obrigatorio) {
    try {
        $resp = Invoke-WebRequest -Uri "http://${addr}:${p}/mcp" -Method Post -Body $mcpBody `
            -ContentType 'application/json' -Headers $mcpHeaders -UseBasicParsing -TimeoutSec 5
        if ($resp.Content -match 'jsonrpc') {
            Ok "MCP FUNCIONAL em ${addr}:${p} - POST /mcp respondeu JSON-RPC (HTTP $($resp.StatusCode))."
        } else {
            Falha "HTTP $($resp.StatusCode) em ${addr}:${p} mas SEM corpo JSON-RPC - nao parece MCP."
        }
    } catch {
        if ($obrigatorio) {
            Falha "MCP NAO respondeu POST em ${addr}:${p} ($($_.Exception.Message))."
        } else {
            Info "MCP nao acessivel em ${addr}:${p} - fallback em 127.0.0.1 (publique na LAN pela GUI se desejar)."
        }
    }
}

Test-McpEndpoint '127.0.0.1' $port $true

$lanIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254*' } |
    Select-Object -First 1).IPAddress
if ($lanIp) {
    Test-McpEndpoint $lanIp $port $false
}

Write-Host ""
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   Relatorio concluido. Linhas [FALHA] indicam o que corrigir." -ForegroundColor Yellow
Write-Host "======================================================================" -ForegroundColor Cyan
Read-Host "Pressione ENTER para fechar"

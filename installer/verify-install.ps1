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
#   8. Certificado auto-assinado do endpoint HTTPS (gerado no setup)
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
# Caminho existir NAO prova nada: junction pendurada passa no Test-Path com o
# motor removido. Teste REAL: executar '--version' e exigir exit 0 com saida.
$cuaCanonico = Join-Path $env:LOCALAPPDATA 'Programs\Cua\cua-driver\bin\cua-driver.exe'
$cuaExe = $null
$cua = Get-Command cua-driver.exe -ErrorAction SilentlyContinue
if ($cua) {
    $cuaExe = $cua.Source
} elseif (Test-Path $cuaCanonico) {
    $cuaExe = $cuaCanonico
}
if ($cuaExe) {
    $cuaVer = $null
    $cuaOk = $false
    try {
        $cuaVer = (& $cuaExe --version 2>$null | Out-String).Trim()
        if ($LASTEXITCODE -eq 0 -and $cuaVer) { $cuaOk = $true }
    } catch {
        $cuaOk = $false
    }
    if ($cuaOk) {
        Ok "Motor cua-driver instalado e FUNCIONAL: $cuaVer ($cuaExe)"
    } else {
        Falha "Motor cua-driver EXISTE em $cuaExe mas NAO executa ('--version' falhou). Provavel junction pendurada ou instalacao incompleta: reinstale pelo instalador oficial (https://cua.ai/driver/install.ps1)."
    }
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

# Motor 0.16+ exige 'Authorization: Bearer <token>' (verificado no binario
# 0.17.0 desta maquina: POST sem token = 401 {"code":-32001}, com token = 200;
# sem token no ambiente o daemon nem sobe). O token vive em HKCU\Environment.
# Se existir, o teste envia o header — sem isto, um motor perfeitamente
# saudavel sairia [FALHA] por 401, que e exatamente o tipo de relatorio
# mentiroso que este script existe para evitar.
$mcpToken = [Environment]::GetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_TOKEN', 'User')
if ($mcpToken) { $mcpHeaders['Authorization'] = "Bearer $mcpToken" }

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
        $httpCode = $null
        try { $httpCode = [int]$_.Exception.Response.StatusCode } catch {}
        if ($httpCode -eq 401) {
            # 401 = o daemon ESTA de pe e fala JSON-RPC, mas barrou a autenticacao.
            if ($mcpToken) {
                Falha "MCP em ${addr}:${p} respondeu 401 MESMO com token configurado - token divergente do que o daemon carregou. Regrave CUA_DRIVER_RS_MCP_HTTP_TOKEN e reinicie o daemon."
            } else {
                Falha "MCP em ${addr}:${p} EXIGE token (motor 0.16+): grave CUA_DRIVER_RS_MCP_HTTP_TOKEN (32-4096 chars) em HKCU\Environment e reinicie o daemon."
            }
        } elseif ($obrigatorio) {
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

# --- 8. HTTPS do endpoint: certificado auto-assinado gerado na instalacao ---
# O setup roda "fzcomputerai --tls-init" (ou o primeiro run da GUI gera). O
# cert e de SERVIDOR TLS e NAO e instalado em nenhuma store de confianca -
# o cliente confia pelo fingerprint SHA-256 ou pelo arquivo .crt.
Write-Host ""
$tlsDir = Join-Path $env:APPDATA 'FzComputerAI\tls'
$tlsCrt = Join-Path $tlsDir 'selfsigned.crt'
if (Test-Path $tlsCrt) {
    try {
        $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($tlsCrt)
        $dias = [int]($cert.NotAfter - (Get-Date)).TotalDays
        Ok "Certificado HTTPS auto-assinado presente: $tlsCrt"
        Info "   valido ate $($cert.NotAfter.ToString('yyyy-MM-dd')) ($dias dias) - SHA-1 $($cert.Thumbprint)"
        $sha256 = (Get-FileHash -Algorithm SHA256 -InputStream ([IO.MemoryStream]::new($cert.RawData))).Hash -replace '(..)(?!$)','$1:'
        Info "   SHA-256: $sha256"
        Info "   O listener HTTPS (padrao :8443) e ligado na GUI, aba MCP & Rede > HTTPS."
    } catch {
        Falha "selfsigned.crt existe mas nao pode ser lido: $($_.Exception.Message)"
    }
} else {
    Info "Certificado HTTPS ainda nao gerado ($tlsCrt) - a GUI gera no primeiro run. Log: $tlsDir\tls-init.log"
}

Write-Host ""
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "   Relatorio concluido. Linhas [FALHA] indicam o que corrigir." -ForegroundColor Yellow
Write-Host "======================================================================" -ForegroundColor Cyan
Read-Host "Pressione ENTER para fechar"

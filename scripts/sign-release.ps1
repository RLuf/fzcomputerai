#Requires -Version 5.1
<#
.SYNOPSIS
    Assina digitalmente (Authenticode) os executáveis do FzComputerAI usando o certificado
    de code signing que está no token USB / HSM plugado NESTA máquina, e verifica o resultado.

.DESCRIPTION
    FLUXO COMPLETO DE RELEASE (rodar tudo na máquina onde o token está plugado):

        1) Compilar:
             cargo build --release --manifest-path fzcomputerai/Cargo.toml

        2) ASSINAR O GUI ANTES DE EMPACOTAR.  <-- a ordem importa
           O instalador embute o .exe do GUI. Se ele for assinado só depois, o instalador
           sai assinado mas o binário que fica instalado na máquina do usuário continua sem
           assinatura — e é ELE que o SmartScreen e o antivírus vão avaliar no dia a dia.
             .\scripts\sign-release.ps1 -Path fzcomputerai\target\release\fzcomputerai.exe -Thumbprint <THUMB>

        3) Gerar o instalador (Inno Setup) — ele empacota o .exe já assinado do passo 2.
           O installer\fzcomputerai.iss emite em ..\dist:
             & "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DAppVersion=1.0.3 installer\fzcomputerai.iss

        4) Colocar o GUI assinado também em .\dist (distribuição standalone) e ASSINAR
           tudo que estiver lá — na prática, o instalador recém-gerado:
             Copy-Item fzcomputerai\target\release\fzcomputerai.exe .\dist\fzcomputerai-windows-x64.exe
             .\scripts\sign-release.ps1 -Path .\dist -Thumbprint <THUMB>
           (re-assinar um arquivo já assinado é inofensivo: o signtool substitui a assinatura)

        5) Gerar os checksums (recomendado — publique junto com os binários):
             Get-ChildItem .\dist\*.exe | ForEach-Object {
                 "$((Get-FileHash $_ -Algorithm SHA256).Hash.ToLower())  $($_.Name)" |
                     Set-Content "$($_.FullName).sha256" -Encoding ascii
             }

        6) Subir no release do GitHub (só depois que este script sair com código 0):
             gh release upload vX.Y.Z .\dist\*.exe .\dist\*.sha256 --clobber

    POR QUE ASSINAR LOCALMENTE E NÃO NO CI:
    Desde jun/2023 (baseline requirements do CA/Browser Forum) certificados de code signing
    OV/EV só são emitidos em token USB criptográfico ou HSM. A chave privada NÃO é exportável
    e não sai do token — logo não existe .pfx para colocar em secret do GitHub Actions.
    Assinar exige a máquina física com o token plugado. Por isso este script existe.

    O QUE ESTE SCRIPT DELIBERADAMENTE NÃO FAZ (e não deve passar a fazer):
      - NÃO gera certificado self-signed (New-SelfSignedCertificate). Assinatura com certificado
        auto-assinado não é confiada por ninguém: o Windows continua bloqueando igual e ainda
        cria a falsa impressão de que o binário está assinado.
      - NÃO instala CA raiz própria no store da máquina do usuário. Isso é comportamento de
        malware, é detectado por antivírus e altera a configuração de segurança do usuário.
      - NÃO existe "assinar durante a instalação". A assinatura Authenticode é aplicada pelo
        publisher ANTES da distribuição, com a chave privada dele. Embutir chave privada no
        instalador significa chave comprometida e certificado revogado pela CA.

    SOBRE O SMARTSCREEN (expectativa realista):
      - Ter instalador (.msi/.exe de setup) NÃO evita o aviso. Binário não assinado recebe
        exatamente o mesmo bloqueio, seja .exe solto ou instalador.
      - Com certificado OV o aviso pode persistir até o binário acumular reputação de downloads.
        Com EV a reputação normalmente já nasce estabelecida.
      - Enquanto não houver certificado, o caminho honesto é publicar o SHA256 e documentar
        "Mais informações > Executar assim mesmo" no README.

.PARAMETER Path
    Arquivo .exe ou pasta contendo os .exe a assinar. Padrão: .\dist
    Quando for pasta, todos os .exe da pasta (recursivamente) são assinados — GUI e instalador.

.PARAMETER Thumbprint
    Thumbprint (SHA1) do certificado de code signing a usar. Recomendado em release oficial,
    porque deixa explícito qual certificado assinou. Se omitido, o script usa 'signtool /a'
    (seleção automática do melhor certificado do store) — e nesse caso ele SE RECUSA a rodar
    caso não exista nenhum certificado de code signing confiável, justamente para não assinar
    por acidente com um certificado auto-assinado de teste.

.PARAMETER TimestampUrl
    Servidor RFC 3161 de carimbo de tempo. Padrão: http://timestamp.digicert.com
    O carimbo é OBRIGATÓRIO: sem ele a assinatura deixa de ser válida quando o certificado expira.
    Alternativas: http://timestamp.sectigo.com , http://timestamp.globalsign.com/tsa/r6advanced1

.PARAMETER TimestampRetries
    Tentativas em caso de falha do servidor de timestamp (eles caem com frequência). Padrão: 3.

.EXAMPLE
    .\scripts\sign-release.ps1 -Path .\dist -WhatIf
    Mostra exatamente o que seria assinado e com qual comando, sem tocar em nada.
    Em -WhatIf, problemas de certificado (token não plugado, cert auto-assinado) viram apenas
    aviso e o ensaio continua — assim dá para revisar o comando mesmo sem o token na máquina.

.EXAMPLE
    .\scripts\sign-release.ps1 -Path .\dist -Thumbprint A1B2C3D4E5F6...

.NOTES
    Projeto : FzComputerAI - Webstorage Tecnologia
    Requer  : Windows SDK (signtool.exe) + token USB do certificado plugado + driver/middleware
              do token instalado (ex.: SafeNet Authentication Client / eToken).
    Saída   : 0 = tudo assinado e verificado
              1 = pelo menos um arquivo falhou na assinatura ou na verificação
              2 = signtool.exe não encontrado
              3 = nenhum certificado de code signing utilizável
              4 = caminho inválido ou nenhum .exe encontrado
#>

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Position = 0)]
    [string] $Path = '.\dist',

    [Parameter()]
    [string] $Thumbprint,

    [Parameter()]
    [string] $TimestampUrl = 'http://timestamp.digicert.com',

    [Parameter()]
    [ValidateRange(1, 10)]
    [int] $TimestampRetries = 3
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$global:LASTEXITCODE = 0

# Extensões assinadas. O instalador do Inno Setup também é .exe, então .exe cobre GUI + setup.
# Se um dia o projeto passar a distribuir .msi ou .dll, é só acrescentar aqui.
$script:SignableExtensions = @('.exe')

$script:CodeSigningEku = '1.3.6.1.5.5.7.3.3'

# --------------------------------------------------------------------------------------------
# Helpers de saída
# --------------------------------------------------------------------------------------------

function Write-Section {
    param([string] $Text)
    Write-Host ''
    Write-Host "=== $Text ===" -ForegroundColor Cyan
}

function Write-Ok   { param([string] $Text) Write-Host "  [OK]    $Text" -ForegroundColor Green }
function Write-Warn { param([string] $Text) Write-Host "  [AVISO] $Text" -ForegroundColor Yellow }
function Write-Bad  { param([string] $Text) Write-Host "  [FALHA] $Text" -ForegroundColor Red }
function Write-Info { param([string] $Text) Write-Host "  $Text" -ForegroundColor Gray }

function Stop-WithCode {
    param([string] $Message, [int] $Code)
    Write-Host ''
    Write-Host $Message -ForegroundColor Red
    Write-Host ''
    exit $Code
}

# Em -WhatIf o script é um ensaio: problemas de certificado viram aviso e a execução segue,
# para que dê para revisar o comando exato mesmo com o token ainda não plugado.
function Stop-WithCodeUnlessWhatIf {
    param([string] $Message, [int] $Code)
    if ($WhatIfPreference) {
        Write-Host ''
        Write-Host $Message -ForegroundColor Yellow
        Write-Warn 'Como -WhatIf foi usado, o script segue apenas para exibir o que seria executado.'
        return
    }
    Stop-WithCode -Message $Message -Code $Code
}

# --------------------------------------------------------------------------------------------
# Invocação de executável nativo com captura de saída e exit code.
# Necessário porque o PowerShell 7.3+ transforma exit code != 0 de comando nativo em erro
# terminante quando $ErrorActionPreference = 'Stop' — aqui queremos tratar falha por arquivo.
# --------------------------------------------------------------------------------------------

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)] [string]   $FilePath,
        [Parameter(Mandatory = $true)] [string[]] $Arguments
    )

    $previousEap = $ErrorActionPreference
    $hadNativePref = Test-Path -LiteralPath 'Variable:PSNativeCommandUseErrorActionPreference'
    $previousNativePref = if ($hadNativePref) { $PSNativeCommandUseErrorActionPreference } else { $null }

    try {
        $ErrorActionPreference = 'Continue'
        if ($hadNativePref) { $PSNativeCommandUseErrorActionPreference = $false }

        # signtool intercala linhas em branco na saída; limpamos para o log ficar legível.
        $output = & $FilePath @Arguments 2>&1 |
                    ForEach-Object { "$_".TrimEnd() } |
                    Where-Object { $_ -ne '' }
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousEap
        if ($hadNativePref) { $PSNativeCommandUseErrorActionPreference = $previousNativePref }
    }

    return [pscustomobject]@{
        ExitCode = $code
        Output   = if ($null -eq $output) { @() } else { @($output) }
        Text     = if ($null -eq $output) { '' } else { (@($output) -join [Environment]::NewLine) }
    }
}

# --------------------------------------------------------------------------------------------
# Localização do signtool.exe (Windows SDK)
# --------------------------------------------------------------------------------------------

function Resolve-SignTool {
    $archOrder = switch -Wildcard ("$env:PROCESSOR_ARCHITECTURE") {
        'ARM64' { @('arm64', 'x64', 'x86'); break }
        'x86'   { @('x86', 'x64'); break }
        default { @('x64', 'x86', 'arm64') }
    }

    $roots = New-Object System.Collections.Generic.List[string]
    foreach ($programFiles in @(${env:ProgramFiles(x86)}, $env:ProgramFiles, ${env:ProgramW6432})) {
        if ([string]::IsNullOrWhiteSpace($programFiles)) { continue }
        foreach ($kit in @('Windows Kits\10\bin', 'Windows Kits\8.1\bin')) {
            $candidate = Join-Path $programFiles $kit
            if ((Test-Path -LiteralPath $candidate) -and -not $roots.Contains($candidate)) {
                $roots.Add($candidate)
            }
        }
    }

    $found = New-Object System.Collections.Generic.List[object]

    foreach ($root in $roots) {
        # Layout novo: bin\10.0.26100.0\x64\signtool.exe
        # Layout antigo: bin\x64\signtool.exe
        $searchDirs = @(Get-Item -LiteralPath $root)
        $searchDirs += @(Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue)

        foreach ($dir in $searchDirs) {
            $parsedVersion = [version]'0.0.0.0'
            [void][version]::TryParse($dir.Name, [ref] $parsedVersion)

            for ($i = 0; $i -lt $archOrder.Count; $i++) {
                $exe = Join-Path $dir.FullName (Join-Path $archOrder[$i] 'signtool.exe')
                if (Test-Path -LiteralPath $exe -PathType Leaf) {
                    $found.Add([pscustomobject]@{
                        Path     = $exe
                        Version  = $parsedVersion
                        ArchRank = $i
                    })
                }
            }
        }
    }

    if ($found.Count -gt 0) {
        return ($found | Sort-Object -Property @{ Expression = 'Version';  Descending = $true },
                                               @{ Expression = 'ArchRank'; Descending = $false } |
                         Select-Object -First 1).Path
    }

    # Último recurso: signtool no PATH (ex.: "Developer Command Prompt for VS")
    $inPath = Get-Command -Name 'signtool.exe' -CommandType Application -ErrorAction SilentlyContinue |
              Select-Object -First 1
    if ($inPath) { return $inPath.Source }

    return $null
}

# --------------------------------------------------------------------------------------------
# Certificados de code signing disponíveis no store
# --------------------------------------------------------------------------------------------

function Test-CertificateChainTrusted {
    param([Parameter(Mandatory = $true)] $Certificate)

    # Auto-assinado: emissor igual ao sujeito. Nunca é confiável para distribuição.
    if ($Certificate.Subject -eq $Certificate.Issuer) { return $false }

    $chain = $null
    try {
        $chain = New-Object System.Security.Cryptography.X509Certificates.X509Chain
        $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
        return [bool] $chain.Build($Certificate)
    }
    catch {
        return $false
    }
    finally {
        if ($null -ne $chain -and $chain -is [System.IDisposable]) { $chain.Dispose() }
    }
}

function Get-CodeSigningCertificateInfo {
    $now = Get-Date
    $result = New-Object System.Collections.Generic.List[object]

    foreach ($store in @('Cert:\CurrentUser\My', 'Cert:\LocalMachine\My')) {
        $certs = @()
        try {
            $certs = @(Get-ChildItem -Path $store -CodeSigningCert -ErrorAction SilentlyContinue)
        }
        catch {
            Write-Warn "Não foi possível ler $store ($($_.Exception.Message))"
            continue
        }

        foreach ($cert in $certs) {
            if ($result.Where({ $_.Thumbprint -eq $cert.Thumbprint }, 'First').Count -gt 0) { continue }

            $result.Add([pscustomobject]@{
                Thumbprint = $cert.Thumbprint
                Subject    = $cert.Subject
                Issuer     = $cert.Issuer
                NotAfter   = $cert.NotAfter
                Store      = $store
                Valido     = ($cert.NotBefore -le $now -and $cert.NotAfter -ge $now)
                Confiavel  = (Test-CertificateChainTrusted -Certificate $cert)
                HasKey     = $cert.HasPrivateKey
            })
        }
    }

    return $result
}

function Show-CertificateTable {
    param($Certificates)

    foreach ($cert in $Certificates) {
        $status = if (-not $cert.Valido)        { 'EXPIRADO/NÃO VÁLIDO' }
                  elseif (-not $cert.Confiavel) { 'AUTO-ASSINADO / CADEIA NÃO CONFIÁVEL' }
                  elseif (-not $cert.HasKey)    { 'SEM CHAVE PRIVADA' }
                  else                          { 'utilizável' }

        $color = if ($status -eq 'utilizável') { 'Green' } else { 'Yellow' }
        Write-Host ("  - {0}" -f $cert.Subject) -ForegroundColor $color
        Write-Host ("      thumbprint : {0}" -f $cert.Thumbprint) -ForegroundColor Gray
        Write-Host ("      validade   : até {0:yyyy-MM-dd}   store: {1}" -f $cert.NotAfter, $cert.Store) -ForegroundColor Gray
        Write-Host ("      situação   : {0}" -f $status) -ForegroundColor $color
    }
}

$script:NoCertMessage = @'
Nenhum certificado de code signing UTILIZÁVEL foi encontrado no store do Windows.

Checklist:
  1. O token USB do certificado está plugado nesta máquina?
  2. O driver/middleware do token está instalado? (ex.: SafeNet Authentication Client,
     eToken PKI Client, YubiKey Minidriver — conforme a CA que emitiu o certificado)
     Sem o middleware o Windows não expõe o certificado do token no store.
  3. O certificado aparece em "certmgr.msc > Pessoal > Certificados"?
     Ou rode:  Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert
  4. O certificado ainda está dentro da validade?

Lembrete: certificados OV/EV de code signing só existem em token USB ou HSM desde jun/2023.
Não há .pfx para importar, e não se pode assinar em runner de CI sem hardware.

Este script NÃO cria certificado auto-assinado como substituto: assinatura auto-assinada
não é confiada pelo Windows, o bloqueio continua igual e o resultado é enganoso.
'@

# --------------------------------------------------------------------------------------------
# Execução
# --------------------------------------------------------------------------------------------

Write-Host ''
Write-Host 'FzComputerAI - assinatura Authenticode (local, com token plugado)' -ForegroundColor White
Write-Host '-----------------------------------------------------------------' -ForegroundColor DarkGray

# --- 1. Alvos ---------------------------------------------------------------------------------

Write-Section 'Arquivos a assinar'

$resolvedPath = $null
try {
    $resolvedPath = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
}
catch {
    Stop-WithCode -Code 4 -Message @"
Caminho não encontrado: $Path

Rode primeiro a compilação e a geração do instalador, e confirme que os .exe estão na pasta:
  cargo build --release --manifest-path fzcomputerai/Cargo.toml
"@
}

$targets = @()
if (Test-Path -LiteralPath $resolvedPath -PathType Container) {
    $targets = @(
        Get-ChildItem -LiteralPath $resolvedPath -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object { $script:SignableExtensions -contains $_.Extension.ToLowerInvariant() } |
            Sort-Object FullName
    )
}
else {
    $item = Get-Item -LiteralPath $resolvedPath
    if ($script:SignableExtensions -notcontains $item.Extension.ToLowerInvariant()) {
        Stop-WithCode -Code 4 -Message "O arquivo '$($item.Name)' não é assinável por este script (esperado: $($script:SignableExtensions -join ', '))."
    }
    $targets = @($item)
}

if ($targets.Count -eq 0) {
    Stop-WithCode -Code 4 -Message "Nenhum arquivo $($script:SignableExtensions -join '/') encontrado em: $resolvedPath"
}

Write-Info "Pasta/arquivo: $resolvedPath"
foreach ($target in $targets) {
    Write-Info (" - {0}  ({1:N0} KB)" -f $target.Name, ($target.Length / 1KB))
}

# --- 2. signtool ------------------------------------------------------------------------------

Write-Section 'signtool.exe'

$signTool = Resolve-SignTool
if (-not $signTool) {
    Stop-WithCode -Code 2 -Message @'
signtool.exe não encontrado.

Instale o Windows SDK (basta o componente "Windows SDK Signing Tools for Desktop Apps"):
  winget install --id Microsoft.WindowsSDK.10.0.26100 -e
  ou baixe em https://developer.microsoft.com/windows/downloads/windows-sdk/

Alternativa: se você tem Visual Studio instalado, abra o
"Developer Command Prompt for VS" (o signtool entra no PATH) e rode este script de lá.

Procurado em:
  %ProgramFiles(x86)%\Windows Kits\10\bin\<versão>\<arch>\signtool.exe
  %ProgramFiles(x86)%\Windows Kits\8.1\bin\<arch>\signtool.exe
  e no PATH.
'@
}
Write-Ok $signTool

# --- 3. Certificado ---------------------------------------------------------------------------

Write-Section 'Certificado de code signing'

$certificates = @(Get-CodeSigningCertificateInfo)

if ($certificates.Count -eq 0) {
    Stop-WithCodeUnlessWhatIf -Code 3 -Message $script:NoCertMessage
}
else {
    Show-CertificateTable -Certificates $certificates
}

$usable = @($certificates | Where-Object { $_.Valido -and $_.Confiavel -and $_.HasKey })

if ($Thumbprint) {
    $normalizedThumbprint = ($Thumbprint -replace '[^0-9A-Fa-f]', '').ToUpperInvariant()
    $selected = @($certificates | Where-Object { $_.Thumbprint -eq $normalizedThumbprint })

    if ($selected.Count -eq 0) {
        Stop-WithCodeUnlessWhatIf -Code 3 -Message @"
Nenhum certificado de code signing com o thumbprint informado foi encontrado.

  Informado: $normalizedThumbprint

Os thumbprints disponíveis estão listados acima. Se o certificado esperado não aparece,
o token provavelmente não está plugado ou o middleware do token não está instalado.
"@
    }
    else {
        $cert = $selected[0]

        if (-not $cert.Valido) {
            Write-Warn 'O certificado informado está fora do período de validade. A assinatura vai falhar ou nascer inválida.'
        }
        if (-not $cert.Confiavel) {
            Write-Warn 'O certificado informado é AUTO-ASSINADO / não encadeia em uma CA confiável.'
            Write-Warn 'Isso NÃO remove o aviso do SmartScreen e a verificação abaixo (signtool verify /pa) vai falhar.'
            Write-Warn 'Use o certificado OV/EV do token para um release real.'
        }

        Write-Ok "Selecionado por thumbprint: $($cert.Subject)"
    }

    $Thumbprint = $normalizedThumbprint
    $selectionArgs = @('/sha1', $Thumbprint)
}
else {
    if ($usable.Count -eq 0) {
        if ($certificates.Count -gt 0) {
            Stop-WithCodeUnlessWhatIf -Code 3 -Message @"
Foram encontrados certificados de code signing no store, mas NENHUM é utilizável para um
release real (auto-assinado, expirado ou sem chave privada) — veja a lista acima.

Este script se recusa a rodar 'signtool /a' nessa situação para não assinar por acidente com
um certificado de teste e gerar um binário "assinado" que o Windows continua bloqueando
do mesmo jeito.

Se você realmente quer usar um desses certificados (ex.: teste interno), passe o thumbprint
explicitamente:  .\scripts\sign-release.ps1 -Path '$resolvedPath' -Thumbprint <THUMBPRINT>

$($script:NoCertMessage)
"@
        }
    }
    else {
        Write-Ok "Seleção automática (signtool /a) entre $($usable.Count) certificado(s) utilizável(is)."
        if ($certificates.Count -gt $usable.Count) {
            Write-Warn 'Existem certificados NÃO confiáveis no store. Para garantir qual será usado, prefira -Thumbprint.'
        }
    }

    $selectionArgs = @('/a')
}

# --- 4. Assinar -------------------------------------------------------------------------------

Write-Section 'Assinando'

Write-Info "Timestamp (RFC 3161): $TimestampUrl"

$baseSignArgs = @('sign', '/fd', 'SHA256', '/tr', $TimestampUrl, '/td', 'SHA256', '/v') + $selectionArgs

$results = New-Object System.Collections.Generic.List[object]
$whatIfMode = $false

foreach ($target in $targets) {
    $callArgs = $baseSignArgs + @($target.FullName)
    $displayCommand = '"{0}" {1} "{2}"' -f $signTool, ($baseSignArgs -join ' '), $target.FullName

    $record = [pscustomobject]@{
        Arquivo     = $target.Name
        Assinatura  = 'nao executado'
        Verificacao = 'nao executado'
        SHA256      = ''
        Detalhe     = ''
    }

    Write-Host ''
    Write-Host "  $($target.Name)" -ForegroundColor White

    if (-not $PSCmdlet.ShouldProcess($target.FullName, 'Assinar com Authenticode (SHA256 + carimbo de tempo)')) {
        $whatIfMode = $true
        $record.Assinatura  = 'WhatIf'
        $record.Verificacao = 'WhatIf'
        Write-Info "comando: $displayCommand"
        $results.Add($record)
        continue
    }

    # -- assinatura, com retentativa quando a falha é do servidor de timestamp
    $signResult = $null
    for ($attempt = 1; $attempt -le $TimestampRetries; $attempt++) {
        $signResult = Invoke-Native -FilePath $signTool -Arguments $callArgs
        if ($signResult.ExitCode -eq 0) { break }

        if ($signResult.Text -match '(?i)timestamp|carimbo') {
            if ($attempt -lt $TimestampRetries) {
                Write-Warn "Falha no carimbo de tempo (tentativa $attempt/$TimestampRetries). Nova tentativa em 5s..."
                Start-Sleep -Seconds 5
                continue
            }
        }
        break
    }

    if ($signResult.ExitCode -ne 0) {
        $record.Assinatura  = 'FALHOU'
        $record.Verificacao = 'pulada'
        $record.Detalhe     = ($signResult.Output | Select-Object -Last 3) -join ' | '
        Write-Bad "assinatura falhou (exit $($signResult.ExitCode))"
        foreach ($line in $signResult.Output) { Write-Info $line }
        $results.Add($record)
        continue
    }

    $record.Assinatura = 'assinado'
    Write-Ok 'assinado'

    # -- verificação
    $verifyResult = Invoke-Native -FilePath $signTool -Arguments @('verify', '/pa', '/v', $target.FullName)

    if ($verifyResult.ExitCode -eq 0) {
        $record.Verificacao = 'verificado'
        Write-Ok 'verificado (signtool verify /pa /v)'
    }
    else {
        $record.Verificacao = 'FALHOU'
        $record.Detalhe     = ($verifyResult.Output | Select-Object -Last 3) -join ' | '
        Write-Bad "verificação falhou (exit $($verifyResult.ExitCode))"
        foreach ($line in $verifyResult.Output) { Write-Info $line }
    }

    try {
        $record.SHA256 = (Get-FileHash -LiteralPath $target.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        Write-Info "sha256: $($record.SHA256)"
    }
    catch {
        Write-Warn "não foi possível calcular o SHA256: $($_.Exception.Message)"
    }

    $results.Add($record)
}

# --- 5. Resumo --------------------------------------------------------------------------------

Write-Section 'Resumo'

$results | Format-Table -AutoSize -Property Arquivo, Assinatura, Verificacao, SHA256 | Out-String |
    ForEach-Object { Write-Host $_ }

foreach ($record in $results) {
    if ($record.Detalhe) {
        Write-Host ("  {0}: {1}" -f $record.Arquivo, $record.Detalhe) -ForegroundColor DarkYellow
    }
}

if ($whatIfMode) {
    Write-Host ''
    Write-Host 'Modo -WhatIf: nada foi assinado. Rode sem -WhatIf para assinar de verdade.' -ForegroundColor Yellow
    Write-Host ''
    exit 0
}

$failed = @($results | Where-Object { $_.Assinatura -ne 'assinado' -or $_.Verificacao -ne 'verificado' })

Write-Host ''
if ($failed.Count -gt 0) {
    Write-Host "$($failed.Count) de $($results.Count) arquivo(s) com problema. NÃO publique este release." -ForegroundColor Red
    Write-Host ''
    exit 1
}

Write-Host "$($results.Count) arquivo(s) assinado(s) e verificado(s) com sucesso." -ForegroundColor Green
Write-Host 'Próximo passo: gerar os .sha256 e rodar  gh release upload vX.Y.Z .\dist\*.exe .\dist\*.sha256 --clobber' -ForegroundColor Gray
Write-Host ''
exit 0

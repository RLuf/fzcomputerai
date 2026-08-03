# Monta o pacote PORTATIL do FzComputerAI (.zip).
#
# O que faz o pacote ser portatil: o arquivo-marcador `fzcomputerai.portable`.
# Com ele presente ao lado do executavel, o app grava as preferencias num
# `fzcomputerai.ini` na propria pasta em vez do registro, e desabilita o
# "Iniciar com o Windows" (que exigiria HKCU\...\Run).
#
# LIMITE HONESTO, escrito tambem no README do pacote: o RASTREIO de limpeza
# (regras portproxy e PIDs de tunel) continua no registro mesmo no modo
# portatil. Nao e preferencia do usuario: e a trava que impede deixar a maquina
# exposta se o app morrer. Trocar por arquivo quebraria o watchdog que a le.
param(
    [string]$Version = "",
    [string]$OutDir  = ""
)
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
if (-not $Version) {
    # Fonte da verdade da versao: o Cargo.toml (nunca hardcode).
    $cargo = Get-Content (Join-Path $repo 'fzcomputerai\Cargo.toml') -Raw
    if ($cargo -match '(?m)^version\s*=\s*"([^"]+)"') { $Version = $Matches[1] }
    else { throw "Nao consegui ler a versao de fzcomputerai/Cargo.toml" }
}
if (-not $OutDir) { $OutDir = Join-Path $repo 'dist' }

$exe = Join-Path $repo 'fzcomputerai\target\release\fzcomputerai.exe'
if (-not (Test-Path $exe)) {
    throw "Binario nao encontrado: $exe`nCompile antes: cargo build --release --manifest-path fzcomputerai/Cargo.toml"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stage = Join-Path $env:TEMP "fzcomputerai-portable-$Version"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $exe (Join-Path $stage 'fzcomputerai.exe')
Copy-Item (Join-Path $repo 'installer\LICENSE.txt') (Join-Path $stage 'LICENSE.txt')

# O marcador: sua PRESENCA e o que liga o modo portatil.
@"
Este arquivo liga o MODO PORTATIL do FzComputerAI.

Enquanto ele existir nesta pasta, o programa:
  - grava as preferencias em fzcomputerai.ini, aqui do lado (nao no registro);
  - desabilita "Iniciar com o Windows" (exigiria escrever no registro).

Apague este arquivo para o programa voltar a se comportar como instalado.
"@ | Set-Content (Join-Path $stage 'fzcomputerai.portable') -Encoding UTF8

@"
FzComputerAI v$Version - PACOTE PORTATIL
========================================

COMO USAR
  1. Extraia esta pasta onde quiser (pendrive, disco externo, qualquer lugar).
  2. Execute fzcomputerai.exe. Nao precisa instalar nada.

O QUE E "PORTATIL" AQUI
  As suas preferencias ficam em fzcomputerai.ini, nesta mesma pasta.
  Nada de atalhos, nada de "Iniciar com o Windows", nada de desinstalador.

O QUE AINDA TOCA O REGISTRO (sem enrolacao)
  O programa registra em HKCU\Software\FzComputerAI as regras de rede e os
  tuneis que ELE cria, enquanto estao ativos. Isso NAO e preferencia: e a
  trava de seguranca que garante remover essas regras e derrubar os tuneis
  se o programa for encerrado a forca. Sem esse registro, uma queda poderia
  deixar a sua maquina exposta na rede. Os valores sao apagados sozinhos
  quando a regra/tunel termina.

O MOTOR NAO VEM AQUI
  A automacao (clique, digitacao, captura de tela) e feita pelo motor
  cua-driver, do projeto Cua (MIT, Cua AI, Inc.), que e instalado
  separadamente pelo instalador oficial dele. Sem o motor o programa abre,
  mas nenhuma acao funciona - a propria interface oferece instalar.

LICENCA
  MIT. Veja LICENSE.txt (inclui a licenca e o credito ao projeto Cua).

  https://github.com/RLuf/fzcomputerai
"@ | Set-Content (Join-Path $stage 'LEIA-ME.txt') -Encoding UTF8

$zip = Join-Path $OutDir "fzcomputerai-portable-v$Version-windows-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -CompressionLevel Optimal

$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
"$hash  $(Split-Path $zip -Leaf)" | Set-Content "$zip.sha256" -Encoding ascii

Remove-Item $stage -Recurse -Force

Write-Host "Pacote portatil: $zip"
Write-Host "Tamanho: $([math]::Round((Get-Item $zip).Length/1MB,2)) MB"
Write-Host "SHA256: $hash"

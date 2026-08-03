; ============================================================================
;  FzComputerAI - Instalador Windows (Inno Setup 6)
;  Webstorage Tecnologia - https://www.webstorage.com.br
; ============================================================================
;
;  COMO COMPILAR
;  -------------
;    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\fzcomputerai.iss
;  Instalado via "winget install JRSoftware.InnoSetup" sem elevacao, o ISCC fica em
;  %LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe (o caminho acima vale para o pacote
;  choco/instalacao para todos os usuarios, que e o usado no CI).
;
;  REQUISITO: Inno Setup 6.3.0 ou superior. As diretivas
;  ArchitecturesAllowed=x64compatible e ArchitecturesInstallIn64BitMode=
;  x64compatible foram introduzidas na 6.3. Em Inno 6.0-6.2 troque os dois
;  valores por "x64" (o comportamento e equivalente para este projeto, que
;  so distribui binario x86_64).
;  Compilado e validado com Inno Setup 6.7.3 (ISPP habilitado - o bloco que
;  deriva VersionInfoVersion usa #sub/#for/#expr do pre-processador).
;
;  Parametros opcionais de COMPILACAO (todos tem default):
;    /DAppVersion=1.0.3          versao exibida/gravada (default: 2.0.0)
;    /DSourceExe=caminho.exe     binario a empacotar
;                                (default: ..\fzcomputerai\target\release\fzcomputerai.exe)
;    /DExeName=fzcomputerai.exe  nome final do executavel dentro de {app}
;
;  NAO EXISTE MAIS /DCuaDriverVersion: o motor e SEMPRE instalado na ultima
;  versao estavel publicada pelo projeto Cua. O alvo vem de
;  `cua-driver check-update --json` (latest_version) e e passado EXPLICITO ao
;  install.ps1 oficial via -Release. ATENCAO (verificado no script real): sem
;  -Release o install.ps1 NAO consulta o GitHub - instala o BAKED_VERSION
;  congelado dentro do proprio script (o embarcado ja esteve em 0.8.3 contra
;  0.17.0 publicado). Fixar versao em compile-time ja produziu um instalador
;  que "atualizava" para um motor 9 versoes atras do publicado.
;
;  Exemplo no CI (a partir da raiz do repositorio):
;    ISCC.exe /DAppVersion=%VERSION% installer\fzcomputerai.iss
;
;  Saida: ..\dist\fzcomputerai-setup-windows-x64.exe
;
;  PARAMETROS DE LINHA DE COMANDO DO SETUP GERADO
;  ----------------------------------------------
;  Alem dos padroes do Inno (/SILENT, /VERYSILENT, /NORESTART, /LOG=arquivo,
;  /DIR=, /COMPONENTS=, /TASKS=, /ALLUSERS):
;
;    /SKIPENGINE    NAO executa o passo de instalacao do motor cua-driver,
;                   mesmo que o componente "engine" esteja selecionado.
;                   Use em deploy em massa onde o motor e provisionado
;                   separadamente (pelo install.ps1 oficial do projeto Cua),
;                   ou quando nao ha internet na maquina de destino.
;
;    /FORCEENGINE   Executa o instalador oficial do motor MESMO que ele ja
;                   esteja na ultima versao publicada. Sem este parametro o
;                   passo consulta `cua-driver check-update` e nao baixa nada
;                   quando instalado == latest (o download do motor tem ~27 MB
;                   - repetir isso em cada auto-atualizacao seria desperdicio).
;
;  Exemplo de deploy desassistido sem motor:
;    fzcomputerai-setup-windows-x64.exe /VERYSILENT /NORESTART /SKIPENGINE
;
;  SOBRE ASSINATURA DE CODIGO (leia antes de perguntar)
;  ----------------------------------------------------
;  Este instalador NAO elimina o aviso do SmartScreen. Um .exe e um
;  instalador nao assinados recebem exatamente o mesmo bloqueio. Nao existe
;  "assinar durante a instalacao": a assinatura Authenticode exige a chave
;  privada do publisher ANTES da distribuicao. Embutir chave privada no
;  instalador significa chave comprometida, e instalar uma CA raiz propria
;  na maquina do usuario e comportamento de malware. Nada disso e feito aqui
;  e nada disso deve ser adicionado (AGENTS.md secao 4).
;  O caminho legitimo e assinar o .exe e este setup com um certificado de
;  code signing OV/EV em token USB/HSM, localmente, na maquina onde o token
;  esta plugado (via SignTool), ANTES de publicar. Ate la: publique o SHA256
;  e oriente "Mais informacoes > Executar assim mesmo".
; ============================================================================


; ---------------------------------------------------------------------------
; Parametros de linha de comando (com defaults)
; ---------------------------------------------------------------------------
#ifndef AppVersion
  ; Fallback: mantido em sincronia manual com fzcomputerai/Cargo.toml.
  ; No CI a versao vem do tag via /DAppVersion=x.y.z.
  #define AppVersion "2.0.0"
#endif

#ifndef SourceExe
  #define SourceExe "..\fzcomputerai\target\release\fzcomputerai.exe"
#endif

#ifndef ExeName
  #define ExeName "fzcomputerai.exe"
#endif

; O motor cua-driver NAO tem mais versao fixada em tempo de compilacao.
; O alvo e sempre a ULTIMA versao estavel publicada: quem resolve isso e o
; install.ps1 OFICIAL do projeto Cua (sem -Release) e a decisao de "ha algo a
; fazer?" vem de `cua-driver check-update --json` na hora da instalacao.

; ---------------------------------------------------------------------------
; VersionInfoVersion NUMERICA (derivada de AppVersion)
; ---------------------------------------------------------------------------
; A diretiva VersionInfoVersion grava o campo FILEVERSION do recurso VERSIONINFO
; do Windows, que aceita SOMENTE digitos e pontos. AppVersion, por outro lado,
; vem do tag do git (o workflow dispara em "tags: v*" e passa
; /DAppVersion=<tag sem o v>), entao pode ser um pre-release como "1.0.3-rc1".
; Usar {#AppVersion} direto em VersionInfoVersion faz o ISCC abortar com
; "Value of [Setup] section directive VersionInfoVersion is invalid" e derruba
; o job Windows inteiro.
;
; Solucao: pular o que vier antes do primeiro digito (tolera um tag passado
; como "v1.0.3") e entao copiar caractere a caractere ate o primeiro que NAO
; seja digito nem ponto:
;     "1.0.2"         -> "1.0.2"
;     "1.0.3-rc1"     -> "1.0.3"
;     "2.10.0-beta.4" -> "2.10.0"
;     "1.0.3+build7"  -> "1.0.3"
;     "v1.0.3"        -> "1.0.3"
; AppVersion continua completo (com o sufixo) em AppVersion/AppVerName/
; UninstallDisplayName/OutputBaseFilename e tambem em VersionInfoTextVersion -
; so o campo binario FILEVERSION, que nao aceita texto, e truncado.
;
; IMPORTANTE (pegadinha do ISPP): dentro de um #sub, "#define X ..." cria uma
; variavel LOCAL e o valor se perde ao sair do sub. Para acumular resultado
; entre as iteracoes do #for e obrigatorio usar "#expr X = ..." , que atribui
; a variavel ja existente no escopo externo. A primeira versao deste bloco
; usava #define e sempre produzia string vazia (FILEVERSION 0.0.0.0) sem
; nenhum erro de compilacao - um jeito silencioso de errar.
#define VerInfoNum ""
#define VerInfoStop 0
#define VerInfoStart 0
; ISPP exige que a variavel de controle do #for ja exista antes do laco.
#define VerInfoIdx 0

; Passo 1: posicao (1-based) do primeiro digito; 0 se nao houver nenhum.
#sub VerInfoFindStart
  #expr VerInfoStart = ((VerInfoStart == 0) && (Pos(Copy(AppVersion, VerInfoIdx, 1), "0123456789") > 0)) ? VerInfoIdx : VerInfoStart
#endsub
#for {VerInfoIdx = 1; VerInfoIdx <= Len(AppVersion); VerInfoIdx++} VerInfoFindStart

; Passo 2: acumula digitos e pontos ate o primeiro caractere invalido.
#sub VerInfoScanChar
  #expr VerInfoStop = VerInfoStop || (Pos(Copy(AppVersion, VerInfoIdx, 1), "0123456789.") == 0)
  #expr VerInfoNum  = VerInfoStop ? VerInfoNum : VerInfoNum + Copy(AppVersion, VerInfoIdx, 1)
#endsub
#if VerInfoStart > 0
  #for {VerInfoIdx = VerInfoStart; VerInfoIdx <= Len(AppVersion); VerInfoIdx++} VerInfoScanChar
#endif

; Passo 3: normalizacao defensiva - ponto sobrando no fim tambem e invalido
; para VersionInfoVersion (ex.: AppVersion="1.0.-beta" -> "1.0." -> "1.0").
#sub VerInfoStripTrailingDot
  #expr VerInfoNum = ((Len(VerInfoNum) > 0) && (Copy(VerInfoNum, Len(VerInfoNum), 1) == ".")) ? Copy(VerInfoNum, 1, Len(VerInfoNum) - 1) : VerInfoNum
#endsub
#for {VerInfoIdx = 1; VerInfoIdx <= 4; VerInfoIdx++} VerInfoStripTrailingDot

; Passo 4: rede de seguranca. AppVersion sem nenhum digito ("nightly") daria
; string vazia, que o ISCC rejeita. Melhor um valor valido e obviamente falso
; do que derrubar o build de release inteiro.
#if Len(VerInfoNum) == 0
  #expr VerInfoNum = "0"
#endif

; Deixa a versao derivada visivel no log do CI - facilita auditar o que foi
; gravado no recurso binario sem precisar abrir o .exe.
#pragma message "VersionInfoVersion derivada: AppVersion=" + AppVersion + " -> " + VerInfoNum

#define AppName          "FzComputerAI"
#define AppPublisher     "Webstorage Tecnologia"
#define AppPublisherURL  "https://www.webstorage.com.br"
#define AppRepoURL       "https://github.com/RLuf/fzcomputerai"

; Scripts oficiais do cua-driver dentro do repositorio (submodulo `cua`).
; Se o submodulo nao estiver inicializado eles nao existem - o [Files] usa
; skipifsourcedoesntexist e o passo do motor cai no instalador oficial via rede.
#define CuaScriptsDir    "..\cua\libs\cua-driver\scripts"
; Aviso VISIVEL no log de compilacao quando o submodulo nao esta inicializado:
; sem ele, o skipifsourcedoesntexist do [Files] esconde que o setup foi gerado
; SEM o install.ps1 embarcado - o passo do motor e o botao da GUI passam a
; depender exclusivamente do endpoint https://cua.ai (sem fallback auditavel).
#ifnexist "..\cua\libs\cua-driver\scripts\install.ps1"
  #pragma message "AVISO: submodulo cua ausente - o install.ps1 oficial NAO sera embarcado neste setup (o passo do motor dependera 100% do endpoint cua.ai). Rode: git submodule update --init cua"
#endif

; Documentacao do repositorio (componente "docs").
#define DocsDir          "..\docs"


[Setup]
; ATENCAO: AppId e a identidade do produto para upgrade/desinstalacao.
; NUNCA altere este GUID entre versoes - mudar quebra o upgrade in-place e
; deixa entradas orfas em "Aplicativos instalados".
AppId={{F3EC4826-531E-4B4D-ADB3-7467D65AAEA8}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
; NAO use {#AppVersion} aqui: tags de pre-release quebram o compilador.
; Veja o bloco "VersionInfoVersion NUMERICA" no topo deste arquivo.
VersionInfoVersion={#VerInfoNum}
; VersionInfoTextVersion e livre - mostra a versao real (com sufixo) nas
; propriedades do arquivo, sem a restricao numerica do FILEVERSION.
VersionInfoTextVersion={#AppVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} Setup
VersionInfoProductName={#AppName}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppPublisherURL}
AppSupportURL={#AppRepoURL}
AppUpdatesURL={#AppRepoURL}
AppCopyright=Copyright (C) Roger Luft - {#AppPublisher}

; Instalacao por-usuario: com PrivilegesRequired=lowest o {autopf} resolve
; para %LOCALAPPDATA%\Programs, entao o caso comum NAO dispara UAC.
;
; ARMADILHA DA HIVE HKCU (motivo de NAO oferecermos o dialogo "para todos"):
; a entrada [Registry] de autostart grava em HKCU, e a GUI le o mesmo HKCU
; quando desenha o checkbox "Iniciar com o Windows". Se o setup for elevado
; com as credenciais de OUTRO usuario administrador (o que o dialogo do Inno
; permite: "Instalar para todos os usuarios" -> UAC com outra conta), o
; processo do instalador passa a rodar sob AQUELA conta e o HKCU escrito e o
; da conta do admin, nao o de quem vai usar o programa. Resultado: o valor
; Run\FzComputerAI nasce na hive errada, o app nunca inicia com o Windows para
; o usuario real e o checkbox da GUI le "false" mesmo com a task marcada -
; instalador e GUI contando historias diferentes, sem nenhum erro visivel.
;
; Por isso usamos "commandline" em vez de "dialog": o dialogo de escolha some
; da interface (ninguem cai nesse caminho por acidente), mas quem realmente
; precisa de uma instalacao para toda a maquina ainda pode passar /ALLUSERS na
; linha de comando - de forma deliberada e ciente do efeito acima. Nao ha
; perda de capacidade, so de exposicao acidental.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
DefaultDirName={autopf}\FzComputerAI
DefaultGroupName={#AppName}
AllowNoIcons=yes
DisableProgramGroupPage=auto
UninstallDisplayName={#AppName} {#AppVersion}
UninstallDisplayIcon={app}\{#ExeName}
LicenseFile=LICENSE.txt

; --- Paginas do wizard ------------------------------------------------------
; DisableWelcomePage=no: com WizardStyle=modern o Inno esconde a pagina de
; boas-vindas por default. Um instalador de produto (e nao de patch) se
; apresenta antes de pedir a licenca - a pagina identifica versao e publisher.
DisableWelcomePage=no
; DisableDirPage=no (EXPLICITO): o default e "auto", que ESCONDE a pagina de
; diretorio quando existe instalacao anterior do mesmo AppId. Como o upgrade
; e o caso comum deste produto, na pratica a pagina nunca aparecia e o usuario
; nao tinha como escolher/ver o destino. Com "no" a pagina (com Procurar...)
; aparece sempre; o default continua sendo {autopf}\FzComputerAI.
DisableDirPage=no
; DirExistsWarning=no: com a pagina de diretorio agora sempre visivel, o Inno
; passaria a perguntar "A pasta ja existe. Deseja instalar nela mesmo assim?"
; em TODA atualizacao - a pasta existir e o caso NORMAL deste produto, nao uma
; anomalia (medido: o dialogo aparecia em cima da pagina de destino num teste
; real de upgrade). A pagina "Versao anterior encontrada" logo em seguida
; explica a situacao com precisao e oferece a acao certa.
DirExistsWarning=no
; Lista de componentes sempre visivel, com tamanho em disco por item.
AlwaysShowComponentsList=yes
ShowComponentSizes=yes
; A pagina "Pronto para instalar" resume destino, componentes e tarefas.
DisableReadyPage=no
DisableReadyMemo=no

; Fecha a GUI se ela estiver rodando antes de sobrescrever o .exe.
; RestartApplications=no: nao queremos reabrir o app automaticamente.
CloseApplications=yes
RestartApplications=no

; ===========================================================================
; RedirectionGuard=no - LEIA ANTES DE "CORRIGIR" ISTO PARA yes
; ---------------------------------------------------------------------------
; A partir do Inno Setup 6.5 o Setup e o Uninstall LIGAM por default a
; mitigacao RedirectionGuard do Windows, que BLOQUEIA a travessia de junctions
; e symlinks NTFS criados por usuarios sem privilegio - e a mitigacao e
; HERDADA pelos processos filhos.
;
; O instalador oficial do motor cua-driver monta exatamente esse tipo de
; layout: %LOCALAPPDATA%\Programs\Cua\cua-driver\bin e um junction para
; %USERPROFILE%\.cua-driver\packages\current, que por sua vez e um junction
; para packages\releases\<versao>-x86_64-pc-windows-msvc.
;
; Com RedirectionGuard ligado, MEDIDO NESTA MAQUINA (Inno 6.7.3, Windows
; 11 26200) com o motor 0.8.3 corretamente instalado:
;   - "cua-driver --version" via cmd filho: "'cua-driver' nao e reconhecido
;     como um comando interno" (o diretorio do PATH e um junction e nao pode
;     ser percorrido);
;   - executar o binario pelo caminho canonico: "O caminho nao pode ser
;     atravessado porque contem um ponto de montagem nao confiavel";
;   - FileExists() no mesmo caminho: FALSO;
;   - e o install.ps1 OFICIAL do motor, lancado como filho do setup, TRAVOU
;     (7 minutos sem CPU e sem rede) ao tentar manipular seus proprios
;     junctions.
; Ou seja: com a mitigacao ligada o instalador nao consegue nem detectar nem
; instalar o motor - exatamente as duas coisas que esta revisao existe para
; fazer.
;
; Por que desligar aqui e aceitavel: a mitigacao protege contra escalada de
; privilegio, quando um instalador ELEVADO segue um junction plantado por um
; usuario sem privilegio. Este setup e per-user (PrivilegesRequired=lowest),
; roda no mesmo nivel de integridade do dono dos arquivos que ele le, e os
; junctions em questao foram criados pelo proprio usuario, pelo instalador
; oficial do motor. Nao ha fronteira de privilegio a ser cruzada.
; ATENCAO: quem usar /ALLUSERS roda elevado - nesse cenario a protecao faz
; falta. Se precisar dela, passe /REDIRECTIONGUARD na linha de comando
; (parametro nativo do Inno 6.5+) ciente de que o passo do motor vai falhar,
; e provisione o motor separadamente com o install.ps1 oficial.
; ===========================================================================
RedirectionGuard=no

; Log de detalhes SEMPRE, sem depender de /LOG na linha de comando: o arquivo
; nasce em %TEMP%\Setup Log <data> #N.txt. E o unico registro do que o passo do
; motor cua-driver fez numa instalacao silenciosa (onde nao ha MsgBox nem
; pagina final). /LOG=arquivo continua funcionando e apenas escolhe o destino.
SetupLogging=yes

OutputDir=..\dist
; CONTRATO COM O CI E COM A GUI: este nome TEM de casar com INSTALLER_NAME em
; .github/workflows/build-release.yml (o step "Build Windows Installer" falha
; se dist\fzcomputerai-setup-windows-x64.exe nao existir apos o ISCC) E com o
; nome do asset que a propria GUI baixa no auto-upgrade (check_for_updates /
; start_update_download em fzcomputerai/src/app.rs procuram exatamente
; "fzcomputerai-setup-windows-x64.exe" na release do GitHub).
; NAO coloque versao no nome: renomear quebra a atualizacao automatica de
; TODAS as versoes ja instaladas na base. A versao do release vem do tag.
OutputBaseFilename=fzcomputerai-setup-windows-x64
Compression=lzma2/max
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
WizardStyle=modern
MinVersion=10.0
ShowLanguageDialog=auto

; Icone do instalador e das entradas de Programas e Recursos.
; #ifexist: se algum dia o arquivo for removido do repositorio o build nao quebra.
#ifexist "fzcomputerai.ico"
SetupIconFile=fzcomputerai.ico
#endif
; NAO referenciamos WizardImageFile/WizardSmallImageFile: o repositorio nao tem
; os .bmp correspondentes e apontar para arquivo inexistente aborta o ISCC.
; O visual usado e o "modern" nativo do Inno 6.


[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"


[CustomMessages]
; ATENCAO: toda mensagem nova precisa das DUAS linhas (brazilianportuguese + english).
; Valores sao de UMA linha: use %n para quebra e %1/%2 para argumentos (FmtMessage).
; Texto em ASCII de proposito - os .isl do compilador sao ANSI e acentos viram
; mojibake dependendo da codepage da maquina que compila.

; --- Tipos de instalacao ---
brazilianportuguese.TypeFull=Instalacao completa (interface + motor + documentacao)
brazilianportuguese.TypeCompact=Instalacao minima (somente a interface)
brazilianportuguese.TypeCustom=Instalacao personalizada
english.TypeFull=Full installation (interface + engine + documentation)
english.TypeCompact=Minimal installation (interface only)
english.TypeCustom=Custom installation

; --- Componentes ---
brazilianportuguese.CompMain=Interface FzComputerAI (obrigatorio)
brazilianportuguese.CompEngine=Motor de automacao cua-driver (instalador oficial do projeto Cua; requer internet)
brazilianportuguese.CompDocs=Documentacao, licenca e script de verificacao
english.CompMain=FzComputerAI interface (required)
english.CompEngine=cua-driver automation engine (official Cua project installer; needs internet)
english.CompDocs=Documentation, license and verification script

; --- Tarefas ---
brazilianportuguese.GroupAdditional=Opcoes adicionais:
brazilianportuguese.TaskAutostart=Iniciar o FzComputerAI com o Windows
english.GroupAdditional=Additional options:
english.TaskAutostart=Start FzComputerAI with Windows

; --- Pagina de PRE-REQUISITOS ---
brazilianportuguese.PrereqCaption=Pre-requisitos
brazilianportuguese.PrereqDesc=Estado real detectado nesta maquina antes de instalar
brazilianportuguese.PrereqEngineTitle=Motor de automacao (cua-driver)
brazilianportuguese.PrereqChecking=Verificando...
brazilianportuguese.PrereqEngineMissing=NAO ENCONTRADO nesta maquina.
brazilianportuguese.PrereqEngineLocal=instalado %1 (a versao final e resolvida na instalacao: o passo do motor atualiza para a ultima estavel se preciso)
brazilianportuguese.PrereqEngineInstalled=instalado %1 (nenhuma atualizacao pendente)
brazilianportuguese.PrereqEngineUpdate=instalado %1 - disponivel %2
brazilianportuguese.PrereqEngineHint=O cua-driver e o MOTOR que executa clique, digitacao e captura de tela. Ele NAO faz parte deste pacote: e instalado pelo instalador OFICIAL do projeto Cua (MIT, Cua AI, Inc.), que tem gerenciador e desinstalador proprios. Sem ele a interface abre e NENHUMA acao funciona.
brazilianportuguese.PrereqPlanInstall=Acao prevista: instalar/atualizar o motor para a ULTIMA versao estavel publicada pelo projeto Cua (a versao exata e resolvida na hora da instalacao pelo proprio motor).
brazilianportuguese.PrereqPlanNothing=Acao prevista: nada a fazer - o motor ja esta na ultima versao estavel (%1). Use /FORCEENGINE para reinstalar.
brazilianportuguese.PrereqPlanSkip=Acao prevista: nenhuma - /SKIPENGINE foi informado na linha de comando.
brazilianportuguese.PrereqDestTitle=Destino da instalacao
brazilianportuguese.PrereqDest=%1 (ajustavel na proxima etapa)
brazilianportuguese.PrereqPrevTitle=Instalacao anterior do FzComputerAI
brazilianportuguese.PrereqPrevNone=nenhuma detectada - esta sera uma instalacao nova.
brazilianportuguese.PrereqPrevFound=detectada em %1
brazilianportuguese.PrereqPrevOrphan=detectada em %1 (sem registro de desinstalacao - instalacao orfa)
brazilianportuguese.PrereqPortTitle=Porta do endpoint MCP (HKCU\Environment)
english.PrereqCaption=Prerequisites
english.PrereqDesc=Real state detected on this machine before installing
english.PrereqEngineTitle=Automation engine (cua-driver)
english.PrereqChecking=Checking...
english.PrereqEngineMissing=NOT FOUND on this machine.
english.PrereqEngineLocal=installed %1 (the final version is resolved at install time: the engine step updates to the latest stable if needed)
english.PrereqEngineInstalled=installed %1 (no update pending)
english.PrereqEngineUpdate=installed %1 - available %2
english.PrereqEngineHint=cua-driver is the ENGINE that performs clicking, typing and screen capture. It is NOT part of this package: it is installed by the OFFICIAL installer of the Cua project (MIT, Cua AI, Inc.), which ships its own manager and uninstaller. Without it the interface still opens, but NO action works.
english.PrereqPlanInstall=Planned action: install/update the engine to the LATEST stable version published by the Cua project (the exact version is resolved at install time by the engine itself).
english.PrereqPlanNothing=Planned action: nothing to do - the engine is already at the latest stable version (%1). Use /FORCEENGINE to reinstall.
english.PrereqPlanSkip=Planned action: none - /SKIPENGINE was given on the command line.
english.PrereqDestTitle=Installation destination
english.PrereqDest=%1 (adjustable on the next page)
english.PrereqPrevTitle=Previous FzComputerAI installation
english.PrereqPrevNone=none detected - this will be a fresh installation.
english.PrereqPrevFound=detected at %1
english.PrereqPrevOrphan=detected at %1 (no uninstall registration - orphaned installation)
english.PrereqPortTitle=MCP endpoint port (HKCU\Environment)

; --- Pagina de VERSAO ANTERIOR ---
brazilianportuguese.PrevCaption=Versao anterior encontrada
brazilianportuguese.PrevDesc=O que fazer com a instalacao que ja existe nesta maquina
brazilianportuguese.PrevText=Foi encontrada uma instalacao anterior do FzComputerAI em:%n%n    %1%n%nO recomendado e desinstala-la antes de gravar a nova versao: evita arquivos orfaos de layouts antigos e entradas duplicadas em "Aplicativos instalados". Sua configuracao (porta MCP e "Iniciar com o Windows") e PRESERVADA - o instalador salva e restaura esses valores.
brazilianportuguese.PrevCheck=Desinstalar a versao anterior antes de instalar
brazilianportuguese.PrevConfirm=Confirma a desinstalacao da versao anterior em:%n%n    %1%n%nIsto sera feito em modo silencioso, imediatamente antes de gravar a nova versao. Responder Nao mantem a instalacao anterior e apenas sobrescreve os arquivos.
english.PrevCaption=Previous version found
english.PrevDesc=What to do with the installation already present on this machine
english.PrevText=A previous FzComputerAI installation was found at:%n%n    %1%n%nUninstalling it before writing the new version is recommended: it avoids orphaned files from older layouts and duplicated entries in "Installed apps". Your configuration (MCP port and "Start with Windows") is PRESERVED - Setup saves and restores those values.
english.PrevCheck=Uninstall the previous version before installing
english.PrevConfirm=Confirm uninstalling the previous version at:%n%n    %1%n%nThis runs silently, immediately before writing the new version. Answering No keeps the previous installation and simply overwrites the files.

; --- Progresso durante a instalacao ---
brazilianportuguese.StatusEngineProbe=Verificando o motor cua-driver...
brazilianportuguese.StatusEngineInstall=Instalando a ultima versao estavel do motor cua-driver pelo instalador oficial do projeto Cua...
brazilianportuguese.StatusEngineVerify=Conferindo o motor cua-driver recem instalado...
english.StatusEngineProbe=Checking the cua-driver engine...
english.StatusEngineInstall=Installing the latest stable cua-driver engine through the official Cua project installer...
english.StatusEngineVerify=Verifying the freshly installed cua-driver engine...

; --- Resultado do passo do motor ---
brazilianportuguese.EngineOk=Motor cua-driver: OK (versao %1).
brazilianportuguese.EngineSkipped=Motor cua-driver: passo nao executado (componente desmarcado ou /SKIPENGINE).
brazilianportuguese.EngineFailShort=Motor cua-driver: NAO DISPONIVEL - a interface abre, mas nenhuma acao funciona.
brazilianportuguese.EngineFailMsg=O FzComputerAI foi instalado, mas o motor cua-driver NAO ficou disponivel.%n%nSem o motor a interface abre e NENHUMA acao funciona (clique, digitacao, captura de tela).%n%nComo resolver, em ordem:%n%n1) Rode o relatorio de verificacao para ver o que falhou:%n     %1%n%n2) Instale o motor pela propria interface: botao "Instalar motor cua-driver".%n%n3) Ou execute este instalador novamente com o componente "Motor de automacao cua-driver" marcado (requer internet).%n%nDetalhes tecnicos do que foi tentado estao no log desta instalacao.
english.EngineOk=cua-driver engine: OK (version %1).
english.EngineSkipped=cua-driver engine: step not executed (component cleared or /SKIPENGINE).
english.EngineFailShort=cua-driver engine: NOT AVAILABLE - the interface opens, but no action works.
english.EngineFailMsg=FzComputerAI was installed, but the cua-driver engine did NOT become available.%n%nWithout the engine the interface opens and NO action works (click, typing, screen capture).%n%nHow to fix, in order:%n%n1) Run the verification report to see what failed:%n     %1%n%n2) Install the engine from the interface itself: "Install cua-driver engine" button.%n%n3) Or run this installer again with the "cua-driver automation engine" component selected (needs internet).%n%nTechnical details of what was attempted are in this installation's log.

; --- Avisos ---
brazilianportuguese.WarnNoEngine=Voce desmarcou o componente do motor cua-driver.%n%nO cua-driver NAO e um extra: e o motor que executa clique, digitacao, captura de tela e todas as demais acoes. Sem ele o FzComputerAI abre normalmente, mas NENHUM botao funciona - toda acao termina em "nao foi possivel executar 'cua-driver'".%n%nA instalacao vai continuar. Se preferir, instale o motor depois pelo proprio aplicativo (botao "Instalar motor cua-driver") ou execute novamente este instalador com o componente marcado.
brazilianportuguese.UninstallDriverNotice=O FzComputerAI foi removido.%n%nO motor cua-driver NAO foi desinstalado: ele possui gerenciador e desinstalador proprios.%n%nPara remove-lo, consulte https://github.com/trycua/cua
brazilianportuguese.RunVerifyDesc=Verificar a instalacao (relatorio: MCP funcional, porta/interfaces, autostart, motor)
english.WarnNoEngine=You cleared the cua-driver engine component.%n%ncua-driver is not an extra: it is the engine that performs clicking, typing, screen capture and every other action. Without it FzComputerAI still opens, but NO button works - every action ends in "cannot execute 'cua-driver'".%n%nSetup will continue. You can install the engine later from the application itself (the "Install cua-driver engine" button) or by running this installer again with the component selected.
english.UninstallDriverNotice=FzComputerAI has been removed.%n%nThe cua-driver engine was NOT uninstalled: it ships its own manager and uninstaller.%n%nTo remove it, see https://github.com/trycua/cua
english.RunVerifyDesc=Verify the installation (report: MCP working, port/interfaces, autostart, engine)


[Types]
; A primeira entrada e o tipo default (usado tambem em instalacao silenciosa
; quando nao se passa /COMPONENTS=): "full" seleciona interface + motor + docs.
Name: "full";    Description: "{cm:TypeFull}"
Name: "compact"; Description: "{cm:TypeCompact}"
Name: "custom";  Description: "{cm:TypeCustom}"; Flags: iscustom


[Components]
; Arvore de componentes (estilo "Custom Setup"): cada linha diz o que e e
; quanto ocupa. Substituiu a task "cuadriver" da versao anterior, que
; escondia o motor numa lista de checkboxes de "opcoes adicionais".
;
; main: a interface. Flags: fixed => nunca desmarcavel, e por isso precisa
; aparecer em TODOS os tipos declarados em [Types].
Name: "main";   Description: "{cm:CompMain}";   Types: full compact custom; Flags: fixed
;
; engine: o motor cua-driver. NAO copia arquivos - representa a ACAO de
; executar o instalador OFICIAL do projeto Cua (ver InstallEngineStep na
; secao [Code]). Por isso o tamanho vem de ExtraDiskSpaceRequired em vez da
; soma dos [Files]: 30 MB e a medida real de uma release do motor nesta
; plataforma (~26,5 MB em %USERPROFILE%\.cua-driver\packages, binario de
; ~19 MB + helper UIA), arredondada para cima.
; Vem MARCADO por default (esta em "full", o primeiro tipo). A pagina de
; pre-requisitos pode DESMARCAR automaticamente quando a versao fixada ja
; esta instalada - ai nao ha nada para baixar (ver ApplyEngineComponentDefault).
Name: "engine"; Description: "{cm:CompEngine}"; Types: full custom; ExtraDiskSpaceRequired: 30000000
;
; docs: licenca, script de verificacao e a documentacao em Markdown.
Name: "docs";   Description: "{cm:CompDocs}";   Types: full custom


[Tasks]
; [Tasks] agora contem SO opcoes de comportamento (atalho e autostart). O que
; e "parte do produto" virou [Components] acima - a distincao que o usuario
; espera: componente = o que instalar, tarefa = como se comportar depois.
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Components: main
Name: "autostart";   Description: "{cm:TaskAutostart}";     GroupDescription: "{cm:GroupAdditional}"; Components: main


[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#ExeName}"; Components: main; Flags: ignoreversion

; Instalador OFICIAL do cua-driver, embarcado a partir do repositorio em vez
; de baixado em runtime (auditavel: o script vai junto com o setup e pode ser
; inspecionado em {app}\cua-driver\). Se o submodulo `cua` nao estiver
; inicializado no momento da compilacao, estes arquivos simplesmente nao
; entram no pacote e o passo do motor usa o endpoint oficial cua.ai como
; fallback.
;
; ===== CONTRATO COM A GUI - NAO MUDE ESTES CAMINHOS =====================
; A GUI detecta a ausencia do motor e oferece o botao "Instalar motor
; cua-driver", que procura o script embarcado em:
;
;     <diretorio do executavel>\cua-driver\install.ps1
;
; Como o executavel e instalado em {app}, o DestDir aqui TEM de ser exatamente
; "{app}\cua-driver" - nao "{app}\cua", nem "{app}\scripts", nem um subnivel a
; mais. O modulo _install-common.psm1 e importado pelo install.ps1 por caminho
; relativo ao proprio script, entao ele precisa ficar no MESMO diretorio.
; Se algum dia o layout mudar, mude junto o caminho lido pela GUI em
; fzcomputerai/src/app.rs - os dois lados formam um contrato so.
;
; POR QUE "Components: main" E NAO "engine": estes dois arquivos sao o caminho
; que a GUI usa para instalar o motor DEPOIS, a qualquer momento. Amarra-los ao
; componente "engine" faria o botao da GUI perder o script embarcado justamente
; para quem escolheu instalar o motor mais tarde. Sao 80 KB - o custo de manter
; o contrato sempre valido e desprezivel.
; ========================================================================
Source: "{#CuaScriptsDir}\install.ps1";          DestDir: "{app}\cua-driver"; Components: main; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#CuaScriptsDir}\_install-common.psm1"; DestDir: "{app}\cua-driver"; Components: main; Flags: ignoreversion skipifsourcedoesntexist

; --- Componente "docs" -----------------------------------------------------
; Licenca e relatorio de verificacao pos-instalacao (testes reais: POST
; initialize no endpoint MCP, listeners via netstat, autostart, motor).
; Instalados junto com o app para poderem ser reexecutados a qualquer hora.
Source: "LICENSE.txt";        DestDir: "{app}"; Components: docs; Flags: ignoreversion
Source: "verify-install.ps1"; DestDir: "{app}"; Components: docs; Flags: ignoreversion
; Documentacao em Markdown. Todos os caminhos abaixo foram conferidos no
; repositorio antes de serem escritos aqui; skipifsourcedoesntexist evita que
; a remocao futura de qualquer um deles derrube o build.
Source: "{#DocsDir}\*.md";    DestDir: "{app}\docs"; Components: docs; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\INSTALL.md";      DestDir: "{app}\docs"; Components: docs; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\INSTALL_EN.md";   DestDir: "{app}\docs"; Components: docs; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\README.md";       DestDir: "{app}\docs"; Components: docs; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\README_EN.md";    DestDir: "{app}\docs"; Components: docs; Flags: ignoreversion skipifsourcedoesntexist


[UninstallDelete]
; Binarios de tunel baixados sob demanda pela aba Tunel (cloudflared/ngrok) e
; artefatos correlatos (token-file, ngrok-policy.yml) vivem em {app}\tunnel.
; Nao sao instalados pelo setup (download sob demanda), entao precisam ser
; removidos explicitamente na desinstalacao.
Type: filesandordirs; Name: "{app}\tunnel"


[Icons]
Name: "{group}\{#AppName}";       Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"; Components: main
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"; Components: main; Tasks: desktopicon


[Registry]
; Autostart. O nome do valor e o formato do dado (caminho ENTRE ASPAS) sao
; IDENTICOS aos que a GUI grava em fzcomputerai/src/app.rs (set_autostart ->
; reg add HKCU\...\Run /v FzComputerAI /t REG_SZ /d "\"<exe>\"").
; Se mudar aqui, o checkbox "Iniciar com o Windows" da GUI dessincroniza.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "FzComputerAI"; ValueData: """{app}\{#ExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

; NAO adicione aqui uma entrada "ValueType: none; Flags: deletevalue;
; Tasks: not autostart". Ela APAGARIA silenciosamente a preferencia do
; usuario: quem instalou sem marcar a task, ligou "Iniciar com o Windows"
; pela GUI e depois roda o instalador de uma versao nova (com a task
; desmarcada, que e o estado default do wizard) perderia o autostart sem
; qualquer aviso. O instalador so CRIA o valor quando a task esta marcada;
; desligar o autostart e prerrogativa da GUI.
; A limpeza na desinstalacao e feita em CurUninstallStepChanged (secao
; [Code]), que cobre inclusive o valor criado pela GUI - ver comentario la.

; Configuracao MCP do daemon em HKCU\Environment. createvalueifdoesntexist:
; o instalador NUNCA sobrescreve a porta de quem ja usava o sistema - so cria
; o default na primeira instalacao. A porta e a UNICA variavel que o motor
; oficial le para o endpoint HTTP.
;
; ===== NAO REINTRODUZA CUA_DRIVER_RS_MCP_HTTP_BIND =====================
; Uma versao anterior deste instalador criava tambem
;   CUA_DRIVER_RS_MCP_HTTP_BIND = 0.0.0.0
; afirmando que isso "publica o MCP em todas as interfaces". ISSO ERA FALSO.
; O motor oficial do projeto Cua escuta SOMENTE em 127.0.0.1: o endereco esta
; fixo no codigo deles (`([127,0,0,1], port)` em mcp_http.rs) e NAO existe
; variavel de bind no upstream. Verificado de duas formas: (1) a string
; CUA_DRIVER_RS_MCP_HTTP_BIND nao aparece no binario oficial instalado; (2) a
; busca por essa variavel no repositorio trycua/cua retorna zero ocorrencia.
; A variavel so tem efeito num motor com patch local, que nao e o que o
; usuario roda. Criar configuracao morta no ambiente do usuario e enganoso e
; polui o diagnostico.
; Acesso pela LAN = netsh portproxy (aba MCP & Rede). Acesso pela internet =
; tunel de saida (aba Tunel). Se algum dia o upstream aceitar bind
; configuravel, reintroduza com verificacao real no netstat - nunca por
; suposicao.
; ========================================================================
;
; Sem uninsdeletevalue de proposito: o motor cua-driver tem ciclo de vida
; proprio e pode continuar instalado depois da desinstalacao da GUI.
Root: HKCU; Subkey: "Environment"; ValueType: string; ValueName: "CUA_DRIVER_RS_MCP_HTTP_PORT"; ValueData: "8000";    Flags: createvalueifdoesntexist


[Run]
; ===========================================================================
; O MOTOR cua-driver NAO ESTA MAIS AQUI - ELE E UM PASSO REAL DA INSTALACAO.
; ---------------------------------------------------------------------------
; Antes havia duas entradas [Run] com "postinstall skipifsilent" para instalar
; o motor. Isso tinha um defeito grave e silencioso: o auto-upgrade da GUI
; executa este setup com /VERYSILENT (ver install_update_and_restart em
; fzcomputerai/src/app.rs), e "skipifsilent" fazia o motor NUNCA ser instalado
; nem atualizado nesse caminho - ou seja, no caminho que a maior parte da base
; instalada realmente usa.
; Agora a instalacao do motor acontece em InstallEngineStep (secao [Code],
; disparada em CurStepChanged/ssPostInstall), que roda em modo interativo E em
; modo silencioso, condicionada ao componente "engine" e ao parametro
; /SKIPENGINE. As entradas abaixo sao apenas conveniencias de fim de wizard.
; ===========================================================================

; --- Abrir a GUI ao final --------------------------------------------------
Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Components: main; Flags: nowait postinstall skipifsilent

; --- Verificacao pos-instalacao (relatorio completo no PowerShell) ----------
; O passo do motor ja fez a verificacao objetiva ("cua-driver --version" de
; novo, resultado no log e na pagina final). Este relatorio e o exame COMPLETO
; e opcional: POST initialize real no endpoint MCP, listeners no netstat,
; autostart, porta. nowait: o wizard fecha e a janela do relatorio fica aberta.
; Componente docs porque e ele que instala o verify-install.ps1.
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\verify-install.ps1"""; WorkingDir: "{app}"; Description: "{cm:RunVerifyDesc}"; Components: docs; Flags: nowait postinstall skipifsilent skipifdoesntexist


[Code]

const
  AutostartKey  = 'Software\Microsoft\Windows\CurrentVersion\Run';
  AutostartName = 'FzComputerAI';
  // Chave de desinstalacao do PROPRIO AppId. Escrita como string Pascal, entao
  // as chaves { } aqui NAO sao constantes do Inno - nada e expandido.
  AppUninstKey  = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{F3EC4826-531E-4B4D-ADB3-7467D65AAEA8}_is1';
  // Diretorio canonico do instalador OFICIAL do motor no Windows (verificado
  // na maquina: %LOCALAPPDATA%\Programs\Cua\cua-driver\bin\cua-driver.exe, um
  // junction para %USERPROFILE%\.cua-driver\packages\current).
  EngineCanonRel = '\Programs\Cua\cua-driver\bin\cua-driver.exe';

var
  { Paginas customizadas }
  PrereqPage: TWizardPage;
  PrevPage:   TWizardPage;

  { Controles da pagina de pre-requisitos }
  LblEngineState, LblEngineHint, LblEnginePlan: TNewStaticText;
  LblDest, LblPrev, LblPort: TNewStaticText;

  { Controles da pagina de versao anterior }
  LblPrevText:   TNewStaticText;
  ChkUninstPrev: TNewCheckBox;

  { Estado do motor (detectado, nunca suposto) }
  EngineProbed:       Boolean;
  EngineFound:        Boolean;
  EngineVersion:      String;
  EngineLatest:       String;
  EngineUpdateAvail:  Boolean;
  EngineUpdateChecked: Boolean;
  EngineProbeSource:  String;
  { Comando RESOLVIDO do motor ('cua-driver' se veio do PATH, ou o caminho
    canonico entre aspas). Toda invocacao do motor passa por ele - o nome puro
    depende de um PATH que este processo pode nao ter herdado. }
  EngineExe:          String;

  { Estado da instalacao anterior }
  PrevFound:      Boolean;
  PrevRegistered: Boolean;
  PrevUninstStr:  String;
  PrevLocation:   String;

  { Decisoes }
  UninstallPrevious:    Boolean;
  EngineDefaultApplied: Boolean;
  EngineWarningShown:   Boolean;

  { Resultado do passo do motor (usado na pagina final e no log) }
  EngineStepRan:      Boolean;
  EngineStepOk:       Boolean;
  EngineFinalVersion: String;
  FinishedLabelDone:  Boolean;

  { O daemon do motor estava registrado no autostart ANTES da limpeza? }
  DaemonAutostartWasRegistered: Boolean;


{ ---------------------------------------------------------------------------
  Utilitarios
  --------------------------------------------------------------------------- }

function YesNo(B: Boolean): String;
begin
  if B then Result := 'sim' else Result := 'nao';
end;

// Duas quebras de linha. Existe como funcao porque o ISPP trata QUALQUER linha
// cujo primeiro caractere nao-branco seja "#" como diretiva de pre-processador:
// um literal Pascal escrito no inicio da linha ("#13#10 + ...") aborta a
// compilacao com "Unknown preprocessor directive". Aqui o literal fica no meio
// da linha, longe da coluna inicial.
function ParagraphBreak: String;
begin
  Result := Chr(13) + Chr(10) + Chr(13) + Chr(10);
end;

// Presenca de um parametro na linha de comando, comparacao case-insensitive.
// Implementado com ParamCount/ParamStr de proposito: nao depende de helper de
// versao especifica do compilador.
function HasCmdLineFlag(const Flag: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(Trim(ParamStr(I)), Flag) = 0 then
    begin
      Result := True;
      Exit;
    end;
end;

function SkipEngineRequested: Boolean;
begin
  Result := HasCmdLineFlag('/SKIPENGINE');
end;

function ForceEngineRequested: Boolean;
begin
  Result := HasCmdLineFlag('/FORCEENGINE');
end;

// Executa um comando e devolve a saida (stdout+stderr) em Output.
// Retorna o exit code, ou -1 se nem deu para iniciar o processo.
//
// Detalhes que importam:
//  - O diretorio de trabalho e {tmp} e o arquivo de captura tem nome RELATIVO
//    sem espacos: assim nenhum caminho com espaco entra na linha do cmd.exe.
//  - A linha inteira e envolvida por um par extra de aspas ("/C "..."") porque
//    e a unica forma confiavel de o cmd.exe aceitar um executavel entre aspas
//    seguido de redirecionamento.
//  - SW_HIDE: nada pisca na tela.
function RunAndCapture(const CmdLine: String; var Output: String): Integer;
var
  TmpDir, OutFile: String;
  Lines: TArrayOfString;
  I, RC: Integer;
begin
  Result := -1;
  Output := '';
  TmpDir  := ExpandConstant('{tmp}');
  OutFile := TmpDir + '\fzprobe.txt';
  DeleteFile(OutFile);

  if not Exec(ExpandConstant('{cmd}'),
              '/C "' + CmdLine + ' > fzprobe.txt 2>&1"',
              TmpDir, SW_HIDE, ewWaitUntilTerminated, RC) then
  begin
    Log('[exec] NAO INICIOU: ' + CmdLine + ' -> ' + SysErrorMessage(DLLGetLastError));
    Exit;
  end;

  Result := RC;
  if LoadStringsFromFile(OutFile, Lines) then
    for I := 0 to GetArrayLength(Lines) - 1 do
      Output := Output + Lines[I] + Chr(10);
  DeleteFile(OutFile);

  // Log da saida real de cada sonda. Sem isto, o diagnostico do bloqueio do
  // RedirectionGuard (ver a nota da diretiva em [Setup]) era impossivel: a
  // sonda "so retornava nao encontrado", sem dizer por que.
  Log('[exec] ' + CmdLine + ' -> exit=' + IntToStr(RC) + ' saida=[' +
      Trim(Output) + ']');
end;

// Extrator de valor de JSON plano (o payload de `cua-driver check-update
// --json` e um objeto de um nivel). Serve para string e para booleano:
// devolve o texto bruto entre o ':' e o proximo delimitador, sem as aspas.
// Nao e um parser de JSON completo e nao pretende ser - o formato consumido
// e o contrato publicado pela CLI oficial do motor.
function JsonValue(const Json, Key: String): String;
var
  P, I: Integer;
  S: String;
begin
  Result := '';
  P := Pos('"' + Key + '"', Json);
  if P = 0 then
    Exit;
  S := Copy(Json, P + Length(Key) + 2, Length(Json));
  P := Pos(':', S);
  if P = 0 then
    Exit;
  S := Copy(S, P + 1, Length(S));
  I := 1;
  while (I <= Length(S)) and (S[I] <> ',') and (S[I] <> '}') and
        (S[I] <> #10) and (S[I] <> #13) do
    I := I + 1;
  S := Trim(Copy(S, 1, I - 1));
  if (Length(S) >= 2) and (S[1] = '"') and (S[Length(S)] = '"') then
    S := Copy(S, 2, Length(S) - 2);
  Result := Trim(S);
end;

// "cua-driver 0.8.3" -> "0.8.3" (primeira linha, texto depois do primeiro
// espaco). Tolera saida com CRLF e linhas extras.
function ParseVersionOutput(const Raw: String): String;
var
  S: String;
begin
  S := Trim(Raw);
  if Pos(#10, S) > 0 then
    S := Trim(Copy(S, 1, Pos(#10, S) - 1));
  if Pos(#13, S) > 0 then
    S := Trim(Copy(S, 1, Pos(#13, S) - 1));
  if Pos(' ', S) > 0 then
    S := Trim(Copy(S, Pos(' ', S) + 1, Length(S)));
  Result := S;
end;

procedure SetStatus(const S: String);
begin
  // Em modo silencioso nao existe wizard para atualizar.
  if WizardSilent then
    Exit;
  WizardForm.StatusLabel.Caption := S;
end;


{ ---------------------------------------------------------------------------
  Deteccao do motor cua-driver - pela CLI OFICIAL dele, nunca por suposicao
  --------------------------------------------------------------------------- }

// Descobre versao instalada (e, opcionalmente, se ha versao nova) usando
// exclusivamente a CLI oficial do motor:
//    cua-driver --version
//    cua-driver check-update --json   (chaves current_version, latest_version,
//                                      update_available, install_command,
//                                      release_notes_url)
//
// WithUpdateCheck E OPCIONAL DE PROPOSITO - E NAO E LIGADO AUTOMATICAMENTE.
// `check-update` consulta o GitHub (tem cache de 20h, mas pode sair para a
// rede). MEDIDO: essa chamada, redirecionada por cmd, chegou a passar de 120
// segundos sem retornar nesta maquina; e num outro momento retornou exit=0 com
// saida VAZIA. Uma pagina de wizard que abre e trava esperando a internet e
// pior do que uma pagina que mostra so o estado local. Por isso a abertura da
// pagina de pre-requisitos usa apenas `--version` (local, ~200 ms) e a consulta
// online acontece SOMENTE no passo do motor (InstallEngineStep), durante a
// pagina de progresso - onde esperar e aceitavel e a resposta decide se ha
// algo a baixar. Falha ou saida vazia do check-update NUNCA e tratada como
// erro: apenas nao ha informacao de atualizacao para mostrar.
//
// DUAS TENTATIVAS, nesta ordem, e o motivo importa:
//  1) "cua-driver" pelo PATH herdado por este processo;
//  2) o caminho canonico do instalador oficial
//     (%LOCALAPPDATA%\Programs\Cua\cua-driver\bin\cua-driver.exe).
// A tentativa 2 nao e redundancia: o install.ps1 oficial acrescenta o bin ao
// PATH do USUARIO, e o processo do instalador (ja em execucao) NAO herda essa
// mudanca. Sem ela, a verificacao pos-instalacao acusaria falha justamente
// numa instalacao do motor que deu certo.
procedure ProbeEngine(Force: Boolean; WithUpdateCheck: Boolean);
var
  Raw, Json, CanonExe, V, Plain: String;
  RC: Integer;
begin
  if EngineProbed and (not Force) then
    Exit;

  EngineProbed      := True;
  EngineFound       := False;
  EngineVersion     := '';
  EngineLatest      := '';
  EngineUpdateAvail := False;
  // Cada sonda invalida a consulta anterior: sem isto, um Checked=True antigo
  // combinado com Avail/Latest zerados pela sonda nova fazia
  // EngineAtLatestVersion afirmar "ja esta na ultima" sem consulta nenhuma.
  EngineUpdateChecked := False;
  EngineProbeSource := '';
  EngineExe         := '';

  RC := RunAndCapture('cua-driver --version', Raw);
  if RC = 0 then
  begin
    V := ParseVersionOutput(Raw);
    if V <> '' then
    begin
      EngineFound       := True;
      EngineVersion     := V;
      EngineProbeSource := 'PATH';
      EngineExe         := 'cua-driver';
    end;
  end;

  if not EngineFound then
  begin
    CanonExe := ExpandConstant('{localappdata}') + EngineCanonRel;
    if FileExists(CanonExe) then
    begin
      RC := RunAndCapture('"' + CanonExe + '" --version', Raw);
      if RC = 0 then
      begin
        V := ParseVersionOutput(Raw);
        if V <> '' then
        begin
          EngineFound       := True;
          EngineVersion     := V;
          EngineProbeSource := CanonExe;
          EngineExe         := '"' + CanonExe + '"';
        end;
      end;
    end;
  end;

  if EngineFound and WithUpdateCheck then
  begin
    // Caminho RESOLVIDO, nunca o nome puro: quando o motor so foi encontrado
    // pelo caminho canonico, "cua-driver check-update" falharia por PATH e a
    // pagina afirmaria "sem informacao" para um motor perfeitamente instalado.
    //
    // E com TIMEOUT de 20s: check-update sai para a rede e ja foi MEDIDO
    // travando >120s nesta maquina. Exec/ewWaitUntilTerminated nao tem
    // timeout, entao um travamento aqui penduraria TODA instalacao com o
    // componente do motor — inclusive o auto-upgrade /VERYSILENT da GUI, em
    // que o app ja foi fechado e o usuario ficaria sem GUI e sem feedback.
    // Estouro do limite NAO e erro: o estado fica DESCONHECIDO (saida vazia)
    // e o passo do motor executa o instalador oficial mesmo assim.
    if EngineProbeSource = 'PATH' then
      Plain := 'cua-driver'
    else
      Plain := EngineProbeSource;
    // Apostrofo em caminho quebraria a string single-quoted do PowerShell.
    StringChangeEx(Plain, '''', '''''', True);
    // WaitForExit(20000) + WaitForExit() + pausa + releitura: ja foi MEDIDO
    // (log de 2026-08-03 16:34) o wrapper devolver exit=0 com saida vazia -
    // corrida de flush do redirect. Saida vazia continua NAO sendo erro.
    RC := RunAndCapture(
      'powershell -NoProfile -ExecutionPolicy Bypass -Command ' +
      '$o = Join-Path $env:TEMP ''fzchk.txt''; ' +
      '$p = Start-Process -FilePath ''' + Plain +
      ''' -ArgumentList ''check-update'',''--json'' -NoNewWindow -RedirectStandardOutput $o -PassThru; ' +
      'if ($p.WaitForExit(20000)) { $null = $p.WaitForExit(); Start-Sleep -Milliseconds 300; ' +
      '$j = Get-Content $o -Raw; if (-not $j) { Start-Sleep -Milliseconds 700; $j = Get-Content $o -Raw }; $j } ' +
      'else { try { $p.Kill() } catch {} }',
      Json);
    if (RC = 0) and (Trim(Json) <> '') then
    begin
      V := JsonValue(Json, 'current_version');
      if V <> '' then
        EngineVersion := V;
      EngineLatest      := JsonValue(Json, 'latest_version');
      EngineUpdateAvail := CompareText(JsonValue(Json, 'update_available'), 'true') = 0;
      // So consideramos "consultado" quando a CLI oficial realmente respondeu.
      EngineUpdateChecked := EngineLatest <> '';
    end
    else
      Log('[prereq] check-update nao respondeu (exit=' + IntToStr(RC) +
          '). Estado de atualizacao permanece DESCONHECIDO - nao inventamos.');
  end;

  if EngineFound then
    Log('[prereq] motor cua-driver: encontrado versao=' + EngineVersion +
        ' origem=' + EngineProbeSource +
        ' atualizacao_disponivel=' + YesNo(EngineUpdateAvail) +
        ' ultima=' + EngineLatest)
  else
    Log('[prereq] motor cua-driver: NAO ENCONTRADO (nem no PATH nem no caminho canonico do instalador oficial).');
end;

// Verdadeiro quando o motor esta instalado E a consulta oficial (check-update)
// respondeu E confirmou que nao ha versao mais nova. "Nada a fazer" significa
// "ja esta na ultima publicada" - nunca "e igual a um numero cravado no setup".
function EngineAtLatestVersion: Boolean;
begin
  Result := EngineFound and EngineUpdateChecked and (not EngineUpdateAvail);
end;

// Comando do motor com caminho resolvido, para uso fora da sonda (stop,
// autostart kick/enable/disable). Garante que a sonda local ja rodou; se o
// motor nunca foi encontrado devolve o nome puro - o chamador ja tolera falha.
function EngineCmd: String;
begin
  ProbeEngine(False, False);
  if EngineExe <> '' then
    Result := EngineExe
  else
    Result := 'cua-driver';
end;


{ ---------------------------------------------------------------------------
  Deteccao da instalacao anterior
  --------------------------------------------------------------------------- }

// Procura, nesta ordem:
//  1) o registro de desinstalacao do proprio AppId em HKCU (instalacao
//     per-user, o caso normal) e depois em HKLM (feita com /ALLUSERS);
//  2) um unins000.exe dentro do diretorio de destino SEM registro
//     correspondente - instalacao ORFA. Acontece de verdade (visto nesta
//     maquina): os arquivos ficam, a entrada em "Aplicativos instalados"
//     desaparece. Detectar isso e o que permite limpar antes de reinstalar.
procedure DetectPreviousInstall(const TargetDir: String);
var
  S, Loc: String;
begin
  PrevFound      := False;
  PrevRegistered := False;
  PrevUninstStr  := '';
  PrevLocation   := '';

  S := '';
  if (not RegQueryStringValue(HKEY_CURRENT_USER, AppUninstKey, 'UninstallString', S)) then
    RegQueryStringValue(HKEY_LOCAL_MACHINE, AppUninstKey, 'UninstallString', S);

  S := RemoveQuotes(Trim(S));
  if (S <> '') and FileExists(S) then
  begin
    PrevFound      := True;
    PrevRegistered := True;
    PrevUninstStr  := S;
    Loc := '';
    if (not RegQueryStringValue(HKEY_CURRENT_USER, AppUninstKey, 'InstallLocation', Loc)) then
      RegQueryStringValue(HKEY_LOCAL_MACHINE, AppUninstKey, 'InstallLocation', Loc);
    if Trim(Loc) = '' then
      Loc := ExtractFilePath(S);
    PrevLocation := RemoveBackslashUnlessRoot(Trim(Loc));
    Log('[prev] instalacao anterior REGISTRADA em: ' + PrevLocation);
    Exit;
  end;

  if Trim(TargetDir) <> '' then
  begin
    S := AddBackslash(TargetDir) + 'unins000.exe';
    if FileExists(S) then
    begin
      PrevFound      := True;
      PrevRegistered := False;
      PrevUninstStr  := S;
      PrevLocation   := RemoveBackslashUnlessRoot(TargetDir);
      Log('[prev] instalacao anterior ORFA (sem registro) em: ' + PrevLocation);
      Exit;
    end;
  end;

  Log('[prev] nenhuma instalacao anterior detectada.');
end;

// Remove a VERSAO ANTERIOR antes de instalar a nova: executa o desinstalador
// detectado, em modo silencioso. Detalhes que importam:
//  - /VERYSILENT tambem suprime o MsgBox de usPostUninstall do desinstalador
//    antigo (UninstallSilent = True la dentro).
//  - PRESERVACAO DO AUTOSTART: o desinstalador antigo apaga o valor
//    Run\FzComputerAI (por design, ver RemoveOwnAutostartValue). Num UPGRADE
//    isso destruiria a preferencia que o usuario ligou pela GUI. Por isso o
//    valor e salvo antes e restaurado depois - o caminho continua valido
//    porque o DefaultDirName e o mesmo entre as versoes.
procedure RunPreviousUninstallerSilently;
var
  SavedAutostart: String;
  HadAutostart: Boolean;
  ResultCode: Integer;
begin
  if (not PrevFound) or (PrevUninstStr = '') or (not FileExists(PrevUninstStr)) then
    Exit;

  SavedAutostart := '';
  HadAutostart := RegQueryStringValue(HKEY_CURRENT_USER, AutostartKey,
                                      AutostartName, SavedAutostart);

  Log('[prev] executando desinstalador anterior: ' + PrevUninstStr);
  if Exec(PrevUninstStr, '/VERYSILENT /NORESTART /SUPPRESSMSGBOXES', '',
          SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    Log('[prev] desinstalador anterior retornou exit=' + IntToStr(ResultCode))
  else
    Log('[prev] FALHA ao iniciar o desinstalador anterior: ' +
        SysErrorMessage(DLLGetLastError));

  if HadAutostart and (SavedAutostart <> '') then
  begin
    RegWriteStringValue(HKEY_CURRENT_USER, AutostartKey, AutostartName, SavedAutostart);
    Log('[prev] valor de autostart restaurado apos a desinstalacao anterior.');
  end;
end;


{ ---------------------------------------------------------------------------
  Pagina de PRE-REQUISITOS
  --------------------------------------------------------------------------- }

function MakeLabel(APage: TWizardPage; ATop, AHeight: Integer;
                   ABold: Boolean): TNewStaticText;
begin
  Result := TNewStaticText.Create(APage);
  Result.Parent   := APage.Surface;
  Result.Left     := 0;
  Result.Top      := ScaleY(ATop);
  Result.Width    := APage.SurfaceWidth;
  Result.AutoSize := False;
  Result.WordWrap := True;
  Result.Height   := ScaleY(AHeight);
  if ABold then
    Result.Font.Style := [fsBold];
end;

procedure RefreshPrereqPage;
var
  S: String;
begin
  if PrereqPage = nil then
    Exit;

  { --- estado do motor --- }
  if not EngineFound then
    S := CustomMessage('PrereqEngineMissing')
  else if EngineUpdateAvail and (EngineLatest <> '') then
    S := FmtMessage(CustomMessage('PrereqEngineUpdate'), [EngineVersion, EngineLatest])
  else if EngineUpdateChecked then
    S := FmtMessage(CustomMessage('PrereqEngineInstalled'), [EngineVersion])
  else
    // Ninguem consultou a rede ainda: dizer "nenhuma atualizacao pendente"
    // aqui seria afirmar algo que nao foi verificado.
    S := FmtMessage(CustomMessage('PrereqEngineLocal'), [EngineVersion]);
  LblEngineState.Caption := S;

  { --- acao prevista --- }
  if SkipEngineRequested then
    S := CustomMessage('PrereqPlanSkip')
  else if EngineAtLatestVersion and (not ForceEngineRequested) then
    S := FmtMessage(CustomMessage('PrereqPlanNothing'), [EngineVersion])
  else
    S := CustomMessage('PrereqPlanInstall');
  LblEnginePlan.Caption := S;

  { --- destino --- }
  LblDest.Caption := FmtMessage(CustomMessage('PrereqDest'), [WizardDirValue]);

  { --- instalacao anterior --- }
  DetectPreviousInstall(WizardDirValue);
  if not PrevFound then
    S := CustomMessage('PrereqPrevNone')
  else if PrevRegistered then
    S := FmtMessage(CustomMessage('PrereqPrevFound'), [PrevLocation])
  else
    S := FmtMessage(CustomMessage('PrereqPrevOrphan'), [PrevLocation]);
  LblPrev.Caption := S;

  { --- porta MCP configurada (valor REAL do registro, nao o default) --- }
  S := '';
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment',
                             'CUA_DRIVER_RS_MCP_HTTP_PORT', S) then
    S := '';
  if Trim(S) = '' then
    S := '8000 (sera criado agora / will be created now)';
  LblPort.Caption := S;
end;

// NAO EXISTE MAIS botao "Verificar atualizacoes" nesta pagina: o passo do
// motor instala/atualiza SEMPRE para a ultima versao estavel no final da
// instalacao (decidido la pelo check-update do proprio motor). Um botao de
// consulta aqui so duplicava essa decisao - e ficava tosco sem motor.

// Decide o DEFAULT do componente "engine" a partir do estado REAL da maquina.
// Roda uma unica vez (EngineDefaultApplied): depois disso a escolha e do
// usuario e nao pode ser sobrescrita por navegar para tras e para a frente.
//
// Regra: so ha "nada a baixar" quando o check-update do motor CONFIRMOU que a
// versao instalada e a ultima publicada. Na abertura da pagina essa consulta
// ainda nao rodou (e local por design - ver ProbeEngine), entao o default e
// MARCADO; o passo do motor decide na hora da instalacao, com dado real, se
// pula o download. /FORCEENGINE sempre marca.
procedure ApplyEngineComponentDefault;
begin
  if EngineDefaultApplied then
    Exit;
  EngineDefaultApplied := True;

  if EngineAtLatestVersion and (not ForceEngineRequested) then
  begin
    WizardSelectComponents('!engine');
    Log('[prereq] componente "engine" DESMARCADO por default: o motor ja esta na ultima versao (' + EngineVersion + ').');
  end
  else
  begin
    WizardSelectComponents('engine');
    Log('[prereq] componente "engine" MARCADO por default.');
  end;
end;

procedure CreatePrereqPage;
var
  Y: Integer;
begin
  PrereqPage := CreateCustomPage(wpLicense,
                                 CustomMessage('PrereqCaption'),
                                 CustomMessage('PrereqDesc'));

  Y := 0;
  MakeLabel(PrereqPage, Y, 14, True).Caption := CustomMessage('PrereqEngineTitle');
  Y := Y + 16;
  LblEngineState := MakeLabel(PrereqPage, Y, 14, False);
  Y := Y + 18;
  LblEngineHint := MakeLabel(PrereqPage, Y, 46, False);
  LblEngineHint.Caption := CustomMessage('PrereqEngineHint');
  Y := Y + 50;
  LblEnginePlan := MakeLabel(PrereqPage, Y, 28, False);
  Y := Y + 32;

  MakeLabel(PrereqPage, Y, 14, True).Caption := CustomMessage('PrereqDestTitle');
  Y := Y + 16;
  LblDest := MakeLabel(PrereqPage, Y, 14, False);
  Y := Y + 22;

  MakeLabel(PrereqPage, Y, 14, True).Caption := CustomMessage('PrereqPrevTitle');
  Y := Y + 16;
  LblPrev := MakeLabel(PrereqPage, Y, 26, False);
  Y := Y + 30;

  MakeLabel(PrereqPage, Y, 14, True).Caption := CustomMessage('PrereqPortTitle');
  Y := Y + 16;
  LblPort := MakeLabel(PrereqPage, Y, 14, False);
end;


{ ---------------------------------------------------------------------------
  Pagina de VERSAO ANTERIOR
  --------------------------------------------------------------------------- }

procedure CreatePrevPage;
begin
  // Depois da pagina de diretorio de proposito: assim o texto mostra a
  // instalacao anterior encontrada NO DESTINO que o usuario acabou de
  // confirmar (inclusive o caso da instalacao orfa, que so existe como
  // diretorio).
  PrevPage := CreateCustomPage(wpSelectDir,
                               CustomMessage('PrevCaption'),
                               CustomMessage('PrevDesc'));

  LblPrevText := MakeLabel(PrevPage, 0, 110, False);

  ChkUninstPrev := TNewCheckBox.Create(PrevPage);
  ChkUninstPrev.Parent   := PrevPage.Surface;
  ChkUninstPrev.Left     := 0;
  ChkUninstPrev.Top      := ScaleY(120);
  ChkUninstPrev.Width    := PrevPage.SurfaceWidth;
  ChkUninstPrev.Height   := ScaleY(17);
  ChkUninstPrev.Caption  := CustomMessage('PrevCheck');
  ChkUninstPrev.Checked  := True;   // marcado por default
end;

procedure RefreshPrevPage;
begin
  if PrevPage = nil then
    Exit;
  LblPrevText.Caption := FmtMessage(CustomMessage('PrevText'), [PrevLocation]);
end;


{ ---------------------------------------------------------------------------
  Eventos do wizard
  --------------------------------------------------------------------------- }

// Destino default SEMPRE %LOCALAPPDATA%\Programs\FzComputerAI - NUNCA uma
// pasta dentro de %TEMP%. O Inno lembra o diretorio da instalacao anterior
// (UsePreviousAppDir): se alguem um dia instalou a partir de um setup rodando
// em %TEMP% (ou passou /DIR= para la), TODO upgrade seguinte herdaria um
// destino que o Windows limpa sozinho - o produto some sem desinstalacao.
// A deteccao de instalacao anterior/orfa (DetectPreviousInstall) tambem parte
// do WizardDirValue, entao um destino herdado errado contamina a deteccao.
// Prefixo de caminho tolerante as formas 8.3 x longas do MESMO diretorio
// (ex.: C:\Users\RUNNER~1\... x C:\Users\runneradmin\...): a comparacao
// textual pura nao casa entre as duas formas e o guarda viraria no-op
// silencioso exatamente onde mais aparece forma curta (servicos/CI).
// GetShortName converte quando o caminho existe (o %TEMP% sempre existe; o
// destino herdado de instalacao anterior normalmente tambem).
function DirIsUnder(const Dir, Root: String): Boolean;
begin
  Result := False;
  if (Trim(Dir) = '') or (Trim(Root) = '') then
    Exit;
  if CompareText(Copy(AddBackslash(Dir), 1, Length(AddBackslash(Root))),
                 AddBackslash(Root)) = 0 then
  begin
    Result := True;
    Exit;
  end;
  Result := CompareText(Copy(AddBackslash(GetShortName(Dir)), 1,
                             Length(AddBackslash(GetShortName(Root)))),
                        AddBackslash(GetShortName(Root))) = 0;
end;

procedure EnforceDefaultDirOutsideTemp;
var
  T: String;
  I: Integer;
begin
  for I := 0 to 1 do
  begin
    if I = 0 then
      T := Trim(GetEnv('TEMP'))
    else
      T := Trim(GetEnv('TMP'));
    if DirIsUnder(Trim(WizardForm.DirEdit.Text), T) then
    begin
      Log('[dir] destino herdado/informado dentro da pasta temporaria (' +
          WizardForm.DirEdit.Text + ') - corrigido para o default ' +
          ExpandConstant('{autopf}\FzComputerAI'));
      WizardForm.DirEdit.Text := ExpandConstant('{autopf}\FzComputerAI');
      Exit;
    end;
  end;
end;

procedure InitializeWizard;
begin
  UninstallPrevious := True;   // default: comportamento automatico preservado
  CreatePrereqPage;
  CreatePrevPage;
  EnforceDefaultDirOutsideTemp;
end;

// Paginas customizadas NAO aparecem em instalacao silenciosa (o wizard nao e
// navegado). Todo o comportamento continua definido sem elas:
//   - componente "engine": vem do tipo default "full" (ou de /COMPONENTS=);
//   - desinstalar versao anterior: UninstallPrevious = True desde
//     InitializeWizard, que e exatamente o comportamento automatico anterior.
function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := False;
  if WizardSilent then
    Exit;
  if (PrevPage <> nil) and (PageID = PrevPage.ID) then
  begin
    DetectPreviousInstall(WizardDirValue);
    Result := not PrevFound;
    if not Result then
      RefreshPrevPage;
  end;
end;

// EM MODO SILENCIOSO O INNO AINDA PERCORRE AS PAGINAS INTERNAMENTE.
// Comprovado no log de um /VERYSILENT real: CurPageChanged foi chamado para a
// pagina de pre-requisitos e NextButtonClick para a pagina de versao anterior,
// e o MsgBox de confirmacao chegou a ser exibido (73 segundos de instalacao
// parada esperando um clique que ninguem ia dar, numa instalacao
// "desassistida"). Por isso TODO manipulador de pagina comeca com uma saida
// antecipada em WizardSilent: paginas nao existem para o usuario nesse modo e
// nenhuma decisao pode depender delas.
procedure CurPageChanged(CurPageID: Integer);
var
  S: String;
begin
  if WizardSilent then
    Exit;

  if (PrereqPage <> nil) and (CurPageID = PrereqPage.ID) then
  begin
    LblEngineState.Caption := CustomMessage('PrereqChecking');
    // Somente sonda LOCAL aqui (--version). A consulta online fica no botao -
    // ver a nota sobre WithUpdateCheck em ProbeEngine.
    ProbeEngine(False, False);
    ApplyEngineComponentDefault;
    RefreshPrereqPage;
    Exit;
  end;

  if (PrevPage <> nil) and (CurPageID = PrevPage.ID) then
  begin
    RefreshPrevPage;
    Exit;
  end;

  // Resultado do passo do motor na pagina final - visivel sem abrir log.
  if (CurPageID = wpFinished) and (not FinishedLabelDone) then
  begin
    FinishedLabelDone := True;
    if not EngineStepRan then
      S := CustomMessage('EngineSkipped')
    else if EngineStepOk then
      S := FmtMessage(CustomMessage('EngineOk'), [EngineFinalVersion])
    else
      S := CustomMessage('EngineFailShort');
    WizardForm.FinishedLabel.Caption :=
      WizardForm.FinishedLabel.Caption + ParagraphBreak + S;
  end;
end;

// AVISA, mas NAO BLOQUEIA. Quem desmarca o componente do motor recebe uma
// explicacao do que exatamente vai deixar de funcionar e de como instalar
// depois. Result e sempre True: a escolha continua sendo do usuario.
// Mostrado no maximo uma vez por execucao e omitido quando o motor ja esta
// instalado (quem ja tem o motor e apenas evitando reinstalar).
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;

  // Ver a nota em CurPageChanged: em silencioso o Inno percorre as paginas e um
  // MsgBox aqui travaria a instalacao desassistida.
  if WizardSilent then
    Exit;

  if (CurPageID = wpSelectComponents) and (not EngineWarningShown) and
     (not WizardIsComponentSelected('engine')) and (not EngineFound) then
  begin
    EngineWarningShown := True;
    MsgBox(CustomMessage('WarnNoEngine'), mbInformation, MB_OK);
    Exit;
  end;

  if (PrevPage <> nil) and (CurPageID = PrevPage.ID) then
  begin
    UninstallPrevious := ChkUninstPrev.Checked;
    if UninstallPrevious then
    begin
      // Confirmacao explicita ANTES de agendar a remocao.
      if MsgBox(FmtMessage(CustomMessage('PrevConfirm'), [PrevLocation]),
                mbConfirmation, MB_YESNO) <> IDYES then
      begin
        UninstallPrevious      := False;
        ChkUninstPrev.Checked  := False;
      end;
    end;
    Log('[prev] decisao do usuario: desinstalar versao anterior = ' +
        YesNo(UninstallPrevious));
  end;
end;


{ ---------------------------------------------------------------------------
  PASSO REAL: instalacao do motor cua-driver
  --------------------------------------------------------------------------- }

// Roda em ssPostInstall, ou seja: os arquivos de {app} JA estao no disco
// (inclusive {app}\cua-driver\install.ps1) e o wizard ainda esta na pagina de
// progresso. Roda em modo interativo E em modo silencioso - e o ponto central
// desta revisao do instalador.
//
// Ordem de preferencia do caminho OFICIAL (nunca baixamos binario do motor por
// conta propria - so executamos o instalador oficial do projeto Cua):
//   1) alvo CONHECIDO (check-update respondeu): script embarcado
//      {app}\cua-driver\install.ps1 com -Release <latest_version> EXPLICITO.
//      Sem -Release o script NAO consulta o GitHub: instala o BAKED_VERSION
//      congelado nele (verificado: embarcado com baked 0.8.3 contra 0.17.0
//      publicado) - por isso o alvo nunca e omitido quando conhecido.
//   2) alvo DESCONHECIDO (sem motor / check-update mudo): script do endpoint
//      oficial cua.ai, cujo baked e atualizado pelo CD do projeto Cua a cada
//      release (verificado hoje: baked 0.17.0 == latest). O script embarcado
//      (baked congelado) fica como fallback OFFLINE, quando existir.
// "Nada a fazer" = o check-update do proprio motor confirmou instalado==latest
// (nunca uma versao cravada neste arquivo). Sem rede, tudo falha rapido e
// NAO-FATAL - o motor existente permanece intacto.
//
// JANELA VISIVEL em modo interativo (SW_SHOWNORMAL, sem runhidden): o
// install.ps1 oficial baixa uma release do GitHub (pode demorar) e, com
// -AutoStart (default), faz Start-Process -Verb RunAs para registrar a
// Scheduled Task com RunLevel=Highest. Sob janela oculta o usuario veria um
// UAC surgindo "do nada". Com a janela visivel ele ve progresso e contexto.
//
// -NoAutoStart em modo SILENCIOSO: numa instalacao desassistida um UAC
// bloquearia o processo indefinidamente, esperando um clique que ninguem vai
// dar. Como `cua-driver autostart enable` exige admin (a task e registrada com
// RunLevel=Highest - ver o proprio install.ps1 oficial), a unica escolha
// honesta em silencioso e nao tentar registrar. O daemon e religado por
// `cua-driver autostart kick` quando a task ja existia antes (ver
// RestoreDaemonAutostart) - e a GUI tambem chama kick apos o auto-upgrade.
//
// NAO-FATAL por design: falha aqui NAO derruba a instalacao da GUI. Sem
// internet, release indisponivel ou UAC recusado, o setup termina com a GUI
// intacta e utilizavel (sem controlar a maquina) e o motivo fica no log.
// Versao "nua" segura para linha de comando (so digitos e pontos, ate 32
// chars): o valor vem do JSON do check-update e vai parar em -Release -
// nada alem de [0-9.] passa (defesa contra payload malformado).
function IsBareVersion(const S: String): Boolean;
var
  I: Integer;
begin
  Result := (Length(S) > 0) and (Length(S) <= 32);
  if not Result then
    Exit;
  for I := 1 to Length(S) do
    if Pos(S[I], '0123456789.') = 0 then
    begin
      Result := False;
      Exit;
    end;
end;

procedure InstallEngineStep;
var
  PSExe, ScriptPath, Params, FailHelp: String;
  TargetRel, AutoFlag, EmbPS, LockFix: String;
  RC: Integer;
  ShowMode: Integer;
  DoInstall: Boolean;
begin
  EngineStepRan      := False;
  EngineStepOk       := False;
  EngineFinalVersion := '';

  if not WizardIsComponentSelected('engine') then
  begin
    Log('[engine] PASSO DO MOTOR IGNORADO: componente "engine" nao selecionado.');
    Exit;
  end;
  if SkipEngineRequested then
  begin
    Log('[engine] PASSO DO MOTOR IGNORADO: /SKIPENGINE informado na linha de comando.');
    Exit;
  end;

  EngineStepRan := True;
  Log('[engine] ===== PASSO DO MOTOR cua-driver: INICIO (silencioso=' +
      YesNo(WizardSilent) + ', alvo=ultima versao estavel publicada) =====');

  SetStatus(CustomMessage('StatusEngineProbe'));
  // Sonda COM check-update (caminho resolvido do exe): e o dado real que
  // decide se ha algo a fazer. Aqui a espera e aceitavel - estamos na pagina
  // de progresso, nao numa pagina de wizard aberta na frente do usuario.
  ProbeEngine(True, True);
  if EngineFound then
    Log('[engine] versao detectada ANTES do passo: ' + EngineVersion +
        ' (origem: ' + EngineProbeSource + ', ultima publicada: ' +
        EngineLatest + ', consulta respondeu: ' + YesNo(EngineUpdateChecked) + ')')
  else
    Log('[engine] motor NAO detectado antes do passo.');

  DoInstall := True;
  if EngineAtLatestVersion and (not ForceEngineRequested) then
  begin
    DoInstall := False;
    Log('[engine] instalador oficial NAO executado: o motor ja esta na ultima ' +
        'versao estavel (' + EngineVersion + ') segundo o check-update dele ' +
        '(download de ~27 MB desnecessario). Passe /FORCEENGINE para reinstalar.');
  end
  else if EngineFound and (not EngineUpdateChecked) then
    Log('[engine] check-update nao respondeu (sem rede? timeout?). O instalador ' +
        'oficial sera executado mesmo assim: com rede ele resolve a latest, sem ' +
        'rede ele falha sem derrubar a instalacao da GUI nem o motor atual.');

  if DoInstall then
  begin
    PSExe := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
    if not FileExists(PSExe) then
    begin
      Log('[engine] FALHA: Windows PowerShell 5.x nao encontrado em ' + PSExe +
          '. Instale o motor manualmente (https://cua.ai/driver).');
    end
    else
    begin
      SetStatus(CustomMessage('StatusEngineInstall'));
      ScriptPath := ExpandConstant('{app}\cua-driver\install.ps1');

      // Alvo explicito quando o check-update respondeu (ver o bloco de
      // comentario acima desta procedure): sem -Release o script oficial
      // instala o BAKED_VERSION dele, nao a latest.
      TargetRel := '';
      if (EngineLatest <> '') and IsBareVersion(EngineLatest) then
        TargetRel := ' -Release ' + EngineLatest;

      AutoFlag := '';
      if WizardSilent then
        AutoFlag := ' -NoAutoStart';

      // Lock orfao: o install.ps1 oficial espera o lock em
      // ~/.cua-driver/install.lock PARA SEMPRE. Uma instalacao morta que
      // deixou o lock penduraria este passo (e o setup inteiro, ja que o
      // Exec usa ewWaitUntilTerminated sem timeout). Antes de executar o
      // instalador, remove o lock se ele tiver mais de 30 minutos: uma
      // instalacao viva tem lock recente; uma morta deixa um arquivo velho.
      LockFix := '$lk = Join-Path $env:USERPROFILE ''.cua-driver/install.lock''; ' +
                 'if ((Test-Path $lk) -and (((Get-Date) - (Get-Item $lk)' +
                 '.LastWriteTime).TotalMinutes -gt 30)) { Remove-Item $lk -Force }; ';

      if (TargetRel <> '') and FileExists(ScriptPath) then
      begin
        // Caminho 1: alvo conhecido + script OFICIAL embarcado (auditavel em
        // {app}\cua-driver), apontado para a versao exata publicada.
        // Via -Command (e nao -File) para preceder a limpeza de lock orfao;
        // exit $LASTEXITCODE preserva no log o codigo de saida do script.
        EmbPS := ScriptPath;
        StringChangeEx(EmbPS, '''', '''''', True);
        Params := '-NoProfile -ExecutionPolicy Bypass -Command "' + LockFix +
                  '& ''' + EmbPS + '''' + TargetRel + AutoFlag +
                  '; exit $LASTEXITCODE"';
        Log('[engine] caminho 1 (script embarcado,' + TargetRel + '): ' + ScriptPath);
      end
      else
      begin
        // Caminho 2: alvo desconhecido OU sem script embarcado - o script do
        // endpoint oficial tem o baked SEMPRE atualizado pelo CD do projeto
        // Cua. Baixado para arquivo (o one-liner "irm | iex" nao aceita
        // parametros). O embarcado, quando existe, e o fallback OFFLINE.
        if FileExists(ScriptPath) then
        begin
          EmbPS := ScriptPath;
          StringChangeEx(EmbPS, '''', '''''', True);
          EmbPS := ' catch { & ''' + EmbPS + '''' + TargetRel + AutoFlag + ' }';
        end
        else
          EmbPS := ' catch { exit 1 }';
        Params := '-NoProfile -ExecutionPolicy Bypass -Command ' +
                  '"' + LockFix +
                  'try { $s = Join-Path $env:TEMP ''cua-driver-install.ps1''; ' +
                  'irm https://cua.ai/driver/install.ps1 -OutFile $s; ' +
                  '& $s' + TargetRel + AutoFlag + ' }' + EmbPS + '"';
        Log('[engine] caminho 2 (endpoint oficial cua.ai' + TargetRel +
            '; fallback offline embarcado=' + YesNo(FileExists(ScriptPath)) + ').');
      end;

      if WizardSilent then
        ShowMode := SW_HIDE
      else
        ShowMode := SW_SHOWNORMAL;

      if Exec(PSExe, Params, ExpandConstant('{app}'), ShowMode,
              ewWaitUntilTerminated, RC) then
        Log('[engine] instalador oficial do motor retornou exit=' + IntToStr(RC))
      else
        Log('[engine] FALHA ao iniciar o PowerShell: ' +
            SysErrorMessage(DLLGetLastError));
    end;
  end;

  // --- VERIFICACAO POS-INSTALACAO (roda o motor de novo, nao supoe nada) ---
  SetStatus(CustomMessage('StatusEngineVerify'));
  ProbeEngine(True, False);
  EngineStepOk       := EngineFound;
  EngineFinalVersion := EngineVersion;

  if EngineStepOk then
    Log('[engine] VERIFICACAO OK: cua-driver responde --version = ' +
        EngineFinalVersion + ' (origem: ' + EngineProbeSource + ')')
  else
    Log('[engine] VERIFICACAO FALHOU: cua-driver nao respondeu --version nem ' +
        'pelo PATH nem pelo caminho canonico do instalador oficial. A interface ' +
        'vai abrir, mas nenhuma acao vai funcionar.');

  Log('[engine] ===== PASSO DO MOTOR cua-driver: FIM (ok=' + YesNo(EngineStepOk) + ') =====');

  // Mensagem clara para o usuario SOMENTE em modo interativo. Em silencioso o
  // resultado fica no log do Inno (SetupLogging=yes / /LOG=arquivo) - um
  // MsgBox numa instalacao desassistida travaria o processo.
  // NOTA DE SINTAXE: nao inicie linha com "[" dentro de [Code] - o compilador
  // le uma linha que comeca com "[" como abertura de secao ("Invalid section
  // tag"). Por isso o array de argumentos do FmtMessage fica na mesma linha.
  if (not EngineStepOk) and (not WizardSilent) then
  begin
    FailHelp := ExpandConstant('{app}\verify-install.ps1');
    MsgBox(FmtMessage(CustomMessage('EngineFailMsg'), [FailHelp]), mbError, MB_OK);
  end;
end;

// Religa o daemon do motor quando ele ESTAVA registrado no autostart antes da
// limpeza pre-instalacao.
//
// Sem isto (e sem a excecao de modo silencioso em CurStepChanged/ssInstall),
// cada atualizacao silenciosa - que e o caminho do auto-upgrade da GUI - deixava
// o usuario sem daemon no proximo logon: a limpeza removia a Scheduled Task e,
// como o passo do motor era `skipifsilent`, nada a recriava. Um produto que se
// auto-atualiza e volta pior.
// `kick` nao exige admin; `enable` exige (a task e registrada com
// RunLevel=Highest) e por isso so e tentado quando ha wizard interativo capaz
// de explicar o UAC.
procedure RestoreDaemonAutostart;
var
  RC: Integer;
  Out1: String;
begin
  if not DaemonAutostartWasRegistered then
    Exit;

  RC := RunAndCapture('schtasks /Query /TN "cua-driver-serve"', Out1);
  if RC = 0 then
  begin
    Log('[daemon] Scheduled Task cua-driver-serve ja existe apos a instalacao - apenas kick.');
    // Sonda de novo (Force): o passo do motor pode ter acabado de instalar o
    // exe num caminho que o PATH deste processo nao conhece.
    ProbeEngine(True, False);
    RunAndCapture(EngineCmd + ' autostart kick', Out1);
    Exit;
  end;

  if WizardSilent then
  begin
    Log('[daemon] AVISO: a task cua-driver-serve estava registrada antes e nao ' +
        'existe mais. Registrar de novo exige admin (RunLevel=Highest) e um UAC ' +
        'nao pode ser mostrado em instalacao silenciosa. Rode uma vez: ' +
        'cua-driver autostart enable');
    Exit;
  end;

  Log('[daemon] recriando a Scheduled Task do daemon (cua-driver autostart enable).');
  ProbeEngine(True, False);
  RunAndCapture(EngineCmd + ' autostart enable', Out1);
  RunAndCapture(EngineCmd + ' autostart kick', Out1);
end;


{ ---------------------------------------------------------------------------
  Ciclo de vida da instalacao
  --------------------------------------------------------------------------- }

// Limpeza pre-instalacao - executada em CurStepChanged(ssInstall), ou seja,
// SOMENTE DEPOIS que o usuario clicou "Instalar" na pagina Pronto para
// Instalar. NUNCA mova isto para InitializeSetup: la roda ANTES do wizard
// aparecer, e quem abrisse o setup so para ler a licenca e cancelar perderia
// a scheduled task e o autostart do cua-driver sem instalar nada (estado de
// OUTRO produto destruido sem consentimento). Passos:
//  1. anota se o daemon do motor estava no autostart (para religar depois);
//  2. para o daemon de forma LIMPA (cua-driver stop);
//  3. SOMENTE EM MODO INTERATIVO, remove a scheduled task do daemon
//     (autostart disable + schtasks /Delete) para a reinstalacao do cua
//     comecar do zero. Em modo SILENCIOSO a task e apenas ENCERRADA, nunca
//     removida: registrar a task de novo exige admin (RunLevel=Highest) e
//     numa instalacao desassistida nao existe ninguem para aprovar o UAC -
//     apagar a task ali significaria tirar o daemon do usuario para sempre,
//     que e exatamente o que acontecia no auto-upgrade da GUI (/VERYSILENT);
//  4. encerra a forca qualquer instancia remanescente (taskkill);
//  5. remove entradas legadas do registro;
//  6. desinstala a versao anterior (mesmo AppId), se decidido.
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
  Out1: String;
begin
  if CurStep = ssInstall then
  begin
    DaemonAutostartWasRegistered :=
      RunAndCapture('schtasks /Query /TN "cua-driver-serve"', Out1) = 0;
    Log('[daemon] Scheduled Task cua-driver-serve registrada antes da instalacao: ' +
        YesNo(DaemonAutostartWasRegistered));

    // Caminho RESOLVIDO do motor (EngineCmd): o nome puro depende de um PATH
    // que este processo pode nao ter herdado - "stop" que falha em silencio
    // deixa o daemon antigo vivo durante a troca de versao.
    RunAndCapture(EngineCmd + ' stop', Out1);
    Exec(ExpandConstant('{cmd}'), '/C schtasks /End /TN "cua-driver-serve" >nul 2>&1', '',
         SW_HIDE, ewWaitUntilTerminated, ResultCode);
    if not WizardSilent then
    begin
      Log('[daemon] modo interativo: removendo a Scheduled Task do daemon (o ' +
          'instalador oficial do motor a recria com UAC visivel).');
      RunAndCapture(EngineCmd + ' autostart disable', Out1);
      Exec(ExpandConstant('{cmd}'), '/C schtasks /Delete /TN "cua-driver-serve" /F >nul 2>&1', '',
           SW_HIDE, ewWaitUntilTerminated, ResultCode);
    end
    else
      Log('[daemon] modo silencioso: Scheduled Task do daemon PRESERVADA ' +
          '(recria-la exigiria admin/UAC, impossivel numa instalacao desassistida).');
    Exec(ExpandConstant('{cmd}'), '/C taskkill /F /IM cua-driver.exe /IM fzcomputerai.exe >nul 2>&1', '',
         SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(ExpandConstant('{cmd}'), '/C reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v cua-driver-serve /f >nul 2>&1', '',
         SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(ExpandConstant('{cmd}'), '/C reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v CuaDriver /f >nul 2>&1', '',
         SW_HIDE, ewWaitUntilTerminated, ResultCode);

    // Instalacao silenciosa mantem o comportamento AUTOMATICO de sempre:
    // UninstallPrevious ja e True e nenhuma pagina precisa ser navegada.
    DetectPreviousInstall(ExpandConstant('{app}'));
    if UninstallPrevious then
      RunPreviousUninstallerSilently
    else
      Log('[prev] desinstalacao da versao anterior NAO solicitada - os arquivos serao sobrescritos.');

    Exit;
  end;

  if CurStep = ssPostInstall then
  begin
    InstallEngineStep;
    RestoreDaemonAutostart;
    Exit;
  end;
end;


{ ---------------------------------------------------------------------------
  Desinstalacao
  --------------------------------------------------------------------------- }

// ATENCAO ao editar comentarios nesta secao: comentarios Pascal { ... } sao
// encerrados pelo primeiro "}", entao uma constante do Inno escrita como
// {app} dentro deles quebra a compilacao. Por isso usamos "//" aqui.
//
// Remove o valor de autostart na desinstalacao SEMPRE que ele apontar para
// dentro de {app} - inclusive quando foi a GUI que o criou (set_autostart em
// fzcomputerai/src/app.rs), e nao a task do instalador.
//
// Por que nao basta o flag "uninsdeletevalue" da secao [Registry]: aquele
// flag so e registrado quando a entrada e efetivamente processada, ou seja,
// quando a task "autostart" estava marcada na instalacao. Quem instalou sem a
// task e ligou "Iniciar com o Windows" pela GUI ficava, apos desinstalar, com
// um Run\FzComputerAI apontando para um .exe que nao existe mais - o Windows
// tenta executa-lo em todo logon.
//
// A comparacao de caminho evita apagar a configuracao de OUTRA instalacao do
// FzComputerAI (ex.: uma copia portatil em outro diretorio, ou a instalacao
// per-user quando se desinstala a de outro local): so removemos o valor se o
// executavel referenciado estiver sob o {app} que esta sendo removido.
// Comparacao case-insensitive porque caminhos no Windows nao diferenciam
// maiusculas. A barra final acrescentada por AddBackslash e essencial: sem
// ela, {app}="...\FzComputerAI" casaria tambem com um valor apontando para
// "...\FzComputerAI2\fzcomputerai.exe", de outra instalacao.
procedure RemoveOwnAutostartValue;
var
  Data, ExePath, AppDir: String;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, AutostartKey, AutostartName, Data) then
    Exit;

  // A GUI e o [Registry] gravam o caminho ENTRE ASPAS; RemoveQuotes tolera
  // tambem o formato sem aspas.
  ExePath := RemoveQuotes(Trim(Data));
  // AddBackslash nao duplica a barra se {app} ja terminar com uma.
  AppDir  := AddBackslash(ExpandConstant('{app}'));

  if CompareText(Copy(ExePath, 1, Length(AppDir)), AppDir) = 0 then
    RegDeleteValue(HKEY_CURRENT_USER, AutostartKey, AutostartName);
end;

{ O cua-driver tem ciclo de vida proprio (junctions em %LOCALAPPDATA%\Programs\Cua
  e uma Scheduled Task). Desinstalar a GUI NAO o remove - apenas informamos. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  // usUninstall: antes da remocao dos arquivos, com {app} ainda resolvido.
  if CurUninstallStep = usUninstall then
    RemoveOwnAutostartValue;

  if (CurUninstallStep = usPostUninstall) and (not UninstallSilent) then
    MsgBox(ExpandConstant('{cm:UninstallDriverNotice}'), mbInformation, MB_OK);
end;

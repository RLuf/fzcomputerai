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
;  Parametros opcionais (todos tem default):
;    /DAppVersion=1.0.3          versao exibida/gravada (default: 1.0.2)
;    /DSourceExe=caminho.exe     binario a empacotar
;                                (default: ..\fzcomputerai\target\release\fzcomputerai.exe)
;    /DExeName=fzcomputerai.exe  nome final do executavel dentro de {app}
;    /DCuaDriverVersion=0.8.3    versao do motor cua-driver a instalar
;
;  Exemplo no CI (a partir da raiz do repositorio):
;    ISCC.exe /DAppVersion=%VERSION% installer\fzcomputerai.iss
;
;  Saida: ..\dist\fzcomputerai-setup-windows-x64.exe
;
;  SOBRE ASSINATURA DE CODIGO (leia antes de perguntar)
;  ----------------------------------------------------
;  Este instalador NAO elimina o aviso do SmartScreen. Um .exe e um
;  instalador nao assinados recebem exatamente o mesmo bloqueio. Nao existe
;  "assinar durante a instalacao": a assinatura Authenticode exige a chave
;  privada do publisher ANTES da distribuicao. Embutir chave privada no
;  instalador significa chave comprometida, e instalar uma CA raiz propria
;  na maquina do usuario e comportamento de malware. Nada disso e feito aqui
;  e nada disso deve ser adicionado.
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
  #define AppVersion "1.0.2"
#endif

#ifndef SourceExe
  #define SourceExe "..\fzcomputerai\target\release\fzcomputerai.exe"
#endif

#ifndef ExeName
  #define ExeName "fzcomputerai.exe"
#endif

#ifndef CuaDriverVersion
  ; Pinada na versao declarada pelo workspace local
  ; (cua/libs/cua-driver/rust/Cargo.toml e o BAKED_VERSION do install.ps1 oficial).
  #define CuaDriverVersion "0.8.3"
#endif

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
; skipifsourcedoesntexist e o [Run] cai no instalador oficial via rede.
#define CuaScriptsDir    "..\cua\libs\cua-driver\scripts"

; Icone: opcional. Se um dia existir installer\fzcomputerai.ico, ele passa a
; ser usado automaticamente (ver SetupIconFile no fim de [Setup]).


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

; Fecha a GUI se ela estiver rodando antes de sobrescrever o .exe.
; RestartApplications=no: nao queremos reabrir o app automaticamente.
CloseApplications=yes
RestartApplications=no

OutputDir=..\dist
OutputBaseFilename=fzcomputerai-setup-windows-x64
Compression=lzma2/max
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
WizardStyle=modern
MinVersion=10.0
ShowLanguageDialog=auto

; O repositorio ainda nao tem um .ico. Basta colocar um em
; installer\fzcomputerai.ico e recompilar - nada mais precisa mudar.
#ifexist "fzcomputerai.ico"
SetupIconFile=fzcomputerai.ico
#endif


[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"


[CustomMessages]
brazilianportuguese.GroupAdditional=Opcoes adicionais:
brazilianportuguese.GroupComponents=Motor de automacao:
brazilianportuguese.TaskAutostart=Iniciar o FzComputerAI com o Windows
brazilianportuguese.TaskCuaDriver=Instalar o motor cua-driver (NECESSARIO para controlar a maquina; requer internet)
brazilianportuguese.RunCuaDriverDesc=Instalar agora o motor cua-driver {#CuaDriverVersion} (abre uma janela do PowerShell)
brazilianportuguese.WarnNoEngine=Voce desmarcou a instalacao do motor cua-driver.%n%nO cua-driver NAO e um extra: e o motor que executa clique, digitacao, captura de tela e todas as demais acoes. Sem ele o FzComputerAI abre normalmente, mas NENHUM botao funciona - toda acao termina em "nao foi possivel executar 'cua-driver'".%n%nA instalacao vai continuar. Se preferir, instale o motor depois pelo proprio aplicativo (botao "Instalar motor cua-driver") ou execute novamente este instalador com a opcao marcada.
brazilianportuguese.UninstallDriverNotice=O FzComputerAI foi removido.%n%nO motor cua-driver NAO foi desinstalado: ele possui gerenciador e desinstalador proprios.%n%nPara remove-lo, consulte https://github.com/trycua/cua

english.GroupAdditional=Additional options:
english.GroupComponents=Automation engine:
english.TaskAutostart=Start FzComputerAI with Windows
english.TaskCuaDriver=Install the cua-driver engine (REQUIRED to control the machine; needs internet)
english.RunCuaDriverDesc=Install the cua-driver {#CuaDriverVersion} engine now (opens a PowerShell window)
english.WarnNoEngine=You unchecked the cua-driver engine.%n%ncua-driver is not an extra: it is the engine that performs clicking, typing, screen capture and every other action. Without it FzComputerAI still opens, but NO button works - every action ends in "cannot execute 'cua-driver'".%n%nSetup will continue. You can install the engine later from the application itself (the "Install cua-driver engine" button) or by running this installer again with the option checked.
english.UninstallDriverNotice=FzComputerAI has been removed.%n%nThe cua-driver engine was NOT uninstalled: it ships its own manager and uninstaller.%n%nTo remove it, see https://github.com/trycua/cua


[Tasks]
; O motor cua-driver vem MARCADO por padrao - NAO acrescente "Flags: unchecked"
; nesta entrada. Toda acao da GUI e um Command::new("cua-driver") (ver
; fzcomputerai/src/app.rs): sem o motor no PATH o programa abre e nenhum botao
; funciona. Instalar so a GUI por padrao era prometer um produto que nao
; controla a maquina. Quem desmarcar recebe o aviso de WarnNoEngine
; (NextButtonClick, secao [Code]) - avisa, mas nao bloqueia.
;
; Por que Tasks e nao Components: [Components] sugere "partes do produto que eu
; copio para o disco", e o motor nao e copiado - ele e baixado e instalado pelo
; instalador OFICIAL do cua, que tem gerenciador e desinstalador proprios (ver
; UninstallDriverNotice). A ligacao "Tasks: cuadriver" das entradas [Run] ja
; expressa exatamente isso: uma ACAO pos-instalacao. Trocar para Components
; exigiria [Types] + Components em todas as entradas sem ganho funcional.
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart";   Description: "{cm:TaskAutostart}";     GroupDescription: "{cm:GroupAdditional}"
Name: "cuadriver";   Description: "{cm:TaskCuaDriver}";     GroupDescription: "{cm:GroupComponents}"


[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#ExeName}"; Flags: ignoreversion
Source: "LICENSE.txt";  DestDir: "{app}"; Flags: ignoreversion

; Instalador OFICIAL do cua-driver, embarcado a partir do repositorio em vez
; de baixado em runtime (auditavel: o script vai junto com o setup e pode ser
; inspecionado em {app}\cua-driver\). Se o submodulo `cua` nao estiver
; inicializado no momento da compilacao, estes arquivos simplesmente nao
; entram no pacote e o [Run] usa o endpoint oficial cua.ai como fallback.
;
; ===== CONTRATO COM A GUI - NAO MUDE ESTES CAMINHOS =====================
; A GUI detecta a ausencia do motor e oferece o botao "Instalar motor
; cua-driver", que procura o script embarcado em:
;
;     <diretorio do executavel>\cua-driver\install.ps1
;
; Como o executavel e instalado em {app} (entrada Source do topo desta secao),
; o DestDir aqui TEM de ser exatamente "{app}\cua-driver" - nao "{app}\cua",
; nem "{app}\scripts", nem um subnivel a mais. O modulo _install-common.psm1 e
; importado pelo install.ps1 por caminho relativo ao proprio script, entao ele
; precisa ficar no MESMO diretorio.
; Se algum dia o layout mudar, mude junto o caminho lido pela GUI em
; fzcomputerai/src/app.rs - os dois lados formam um contrato so.
; ========================================================================
Source: "{#CuaScriptsDir}\install.ps1";          DestDir: "{app}\cua-driver"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#CuaScriptsDir}\_install-common.psm1"; DestDir: "{app}\cua-driver"; Flags: ignoreversion skipifsourcedoesntexist


[Icons]
Name: "{group}\{#AppName}";      Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"; Tasks: desktopicon


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


[Run]
; --- Motor cua-driver (marcado por padrao, mas NAO-FATAL) ------------------
; A task vem marcada (o motor nao e opcional - ver [Tasks]), o que torna ainda
; mais importante que uma falha aqui NAO derrube a instalacao da GUI. E o que
; acontece hoje, por tres razoes somadas:
;   1) `postinstall`: estas entradas so rodam DEPOIS que os arquivos ja foram
;      gravados e o setup ja e um sucesso - aparecem como checkbox na pagina
;      final. Cancelar ali nao desfaz nada.
;   2) O Inno IGNORA o codigo de saida de entradas [Run]. Sem internet, release
;      indisponivel, UAC cancelado ou script abortado: o instalador termina
;      normalmente, com a GUI intacta e utilizavel (sem controlar a maquina,
;      como o aviso WarnNoEngine explica).
;   3) `skipifdoesntexist`: se o powershell.exe do caminho indicado nao existir
;      (Windows sem Windows PowerShell 5.x), a entrada e pulada em silencio em
;      vez de gerar "Unable to execute file".
; Nao acrescente aqui nada que altere isso - em especial NAO use uma entrada
; nao-postinstall com checagem de erro para o driver.
;
; INSTALACAO SILENCIOSA (documentado de proposito): com /VERYSILENT a task
; `cuadriver` continua marcada por padrao, mas estas entradas tem
; `skipifsilent` e portanto NAO rodam - deploy silencioso instala a GUI e nao
; o motor. E deliberado: baixar uma release da internet e disparar um UAC
; (Scheduled Task do driver) no meio de uma instalacao desassistida seria pior.
; Em massa, instale o motor separadamente (o proprio install.ps1 do cua) ou
; deixe o usuario usar o botao "Instalar motor cua-driver" da GUI.
;
; NOTA - por que NAO usamos "runhidden" aqui: o install.ps1 oficial do cua
; baixa uma release do GitHub (pode demorar) e, com -AutoStart (default), faz
; Start-Process -Verb RunAs para registrar a Scheduled Task com
; RunLevel=Highest. Sob runhidden o usuario veria um prompt de UAC surgindo
; "do nada", sem nenhuma janela explicando a origem. Com a janela visivel ele
; ve o progresso e o contexto do UAC. Para instalacao silenciosa nada disso
; roda: as entradas sao postinstall + skipifsilent.
; Para desativar o registro da Scheduled Task do driver, acrescente
; " -NoAutoStart" aos Parameters da entrada Caminho 1.
;
; Caminho 1 - script oficial embarcado em {app}\cua-driver\install.ps1.
; (Entradas em uma unica linha de proposito: nao dependemos de continuacao
;  de linha, que varia entre versoes do compilador.)
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\cua-driver\install.ps1"" -Release {#CuaDriverVersion}"; WorkingDir: "{app}\cua-driver"; Description: "{cm:RunCuaDriverDesc}"; Tasks: cuadriver; Check: CuaScriptEmbedded; Flags: postinstall skipifsilent waituntilterminated skipifdoesntexist

; Caminho 2 - fallback quando o submodulo `cua` nao foi empacotado: endpoint
; oficial do projeto (o mesmo ja usado pelo install.ps1 da raiz do repositorio).
; A versao e pinada por CUA_DRIVER_RS_VERSION, que tem precedencia sobre
; qualquer default do proprio script.
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""$env:CUA_DRIVER_RS_VERSION='{#CuaDriverVersion}'; irm https://cua.ai/driver/install.ps1 | iex"""; Description: "{cm:RunCuaDriverDesc}"; Tasks: cuadriver; Check: CuaScriptNotEmbedded; Flags: postinstall skipifsilent waituntilterminated skipifdoesntexist

; --- Abrir a GUI ao final --------------------------------------------------
Filename: "{app}\{#ExeName}"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent


[Code]

{ Verdadeiro quando o script oficial do cua-driver foi empacotado junto com
  o setup (submodulo `cua` presente no momento da compilacao). Usado para
  escolher entre a instalacao embarcada e o fallback via endpoint oficial. }
function CuaScriptEmbedded: Boolean;
begin
  Result := FileExists(ExpandConstant('{app}\cua-driver\install.ps1'));
end;

{ Negacao explicita em vez de "Check: not CuaScriptEmbedded" - funcao
  dedicada elimina qualquer duvida de parsing do parametro Check. }
function CuaScriptNotEmbedded: Boolean;
begin
  Result := not CuaScriptEmbedded;
end;

var
  EngineWarningShown: Boolean;
  DriverProbeDone:    Boolean;
  DriverProbeFound:   Boolean;

// Detecta se o motor cua-driver JA existe nesta maquina, perguntando ao
// proprio Windows onde ele esta ("where" percorre o PATH). Serve so para
// decidir se vale a pena avisar: quem ja tem o motor instalado e desmarca a
// task esta apenas evitando reinstalar, e nao merece um alerta.
//
// Detalhes que importam:
//  - SW_HIDE + redirecionamento: nenhuma janela de console pisca na tela.
//  - O resultado e cacheado (DriverProbeDone) porque NextButtonClick pode ser
//    chamado varias vezes se o usuario navegar para tras e para a frente.
//  - Se o proprio Exec falhar, assumimos "nao instalado" e avisamos. Errar
//    para o lado do aviso e melhor do que deixar o usuario com uma GUI muda.
//  - PATH aqui e o herdado pelo processo do instalador. Um motor instalado
//    depois que este setup abriu pode nao ser visto - de novo, o pior caso e
//    um aviso a mais.
function CuaDriverAlreadyInstalled: Boolean;
var
  ResultCode: Integer;
begin
  if not DriverProbeDone then
  begin
    DriverProbeDone  := True;
    DriverProbeFound := False;
    if Exec(ExpandConstant('{cmd}'), '/C where cua-driver.exe >nul 2>&1', '',
            SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      DriverProbeFound := (ResultCode = 0);
  end;
  Result := DriverProbeFound;
end;

// AVISA, mas NAO BLOQUEIA. Se o usuario desmarcar o motor na pagina de tarefas
// ele recebe uma explicacao do que exatamente vai deixar de funcionar (a GUI
// abre, os botoes nao) e de como instalar depois. Result e sempre True: a
// escolha continua sendo dele.
//
// Mostrado no maximo uma vez por execucao (EngineWarningShown) para nao virar
// um pedagio a cada ida e volta no wizard, e omitido quando o motor ja esta
// instalado na maquina.
//
// Em instalacao silenciosa nada disso roda - o wizard nao tem paginas e
// NextButtonClick nao e chamado. Ver a nota sobre /VERYSILENT em [Run].
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;

  if (CurPageID = wpSelectTasks) and (not EngineWarningShown) and
     (not WizardIsTaskSelected('cuadriver')) and
     (not CuaDriverAlreadyInstalled) then
  begin
    EngineWarningShown := True;
    MsgBox(ExpandConstant('{cm:WarnNoEngine}'), mbInformation, MB_OK);
  end;
end;

const
  AutostartKey  = 'Software\Microsoft\Windows\CurrentVersion\Run';
  AutostartName = 'FzComputerAI';

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

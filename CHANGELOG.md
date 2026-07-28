# Changelog

Todas as alterações notáveis do projeto **FzComputerAI / CUA Driver Computer Vision MCP** serão documentadas neste arquivo.

O formato baseia-se no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/spec/v2.0.0.html).

---

## [2.0.0] - 2026-07-28

### Adicionado & Aprimorado
- **Catálogo MCP Tools na GUI (`mcp_tools.rs`)**: Nova aba visual para listar, filtrar por categoria e invocar interativamente chamadas de visão e automação do motor CUA Driver.
- **Mecanismo de Auto-Upgrade de Versão**: Botão *Verificar Atualizações* na GUI que consulta o GitHub Releases API, baixa o instalador em **background** para `%TEMP%` com **verificação SHA256 obrigatória** (o `.sha256` publicado pelo CI), pede confirmação para **fechar** o aplicativo, garante os processos encerrados, instala silenciosamente e reabre a GUI e o motor. Só atualiza se a release for **estritamente mais nova** (comparação semver) — rollback de release no GitHub nunca vira downgrade silencioso.
- **Regras Inteligentes de Firewall & PortProxy**:
  - `netsh interface portproxy` direciona obrigatoriamente para a porta CUA confirmada (`127.0.0.1:$CUA_PORT_CONFIRMADA`).
  - Aplicação de regras do Windows Firewall com checagem deduplicada de nomes para evitar regras duplicadas.
  - Suporte a bind em `0.0.0.0` com fallback gracioso para `127.0.0.1`.
- **Limpeza Automática no Instalador (Inno Setup)**: o `InitializeSetup()` do `installer/fzcomputerai.iss` encerra instâncias em execução (`fzcomputerai`, `cua-driver`), remove a scheduled task do daemon (`cua-driver-serve`), limpa registros de autostart legados em `HKCU\...\Run` e executa silenciosamente o desinstalador de versões anteriores antes de instalar a nova.
- **Console Debug Formatado**: Logs organizados com autoscroll e 2 linhas em branco de espaçamento após cada lote de execução. Console **sempre visível** como faixa fixa no rodapé da aba MCP & Rede; controles do daemon e de porta/IP fixos no topo; só o diagnóstico rola.
- **Diagnóstico honesto na aba MCP & Rede**: host e URL exibidos derivam do **estado real** (netstat + teste TCP), listeners reais impressos na tela (incluindo portas órfãs no IP da LAN) e regras portproxy existentes listadas cruas do `netsh show v4tov4`. Badge do encaminhamento com **3 estados**: REGRA FUNCIONANDO (config + listener confirmados), REGRA SEM EFEITO (config sem listener — dica do IP Helper) e SEM REGRA.
- **Encerramento limpo ao fechar a GUI**: o `on_exit` para o daemon `cua-driver` e remove as regras portproxy **criadas pelo próprio app** (registradas em `HKCU\Software\FzComputerAI`) — nunca regras de outros serviços com padrão parecido. Elevação única (UAC) apenas se necessária.
- **Workflow CI/Release Atualizado**: Compilação multiplataforma e publicação automatizada de instaladores e binários pré-compilados em releases com checksum `.sha256`.

### Removido
- **`install.ps1` da raiz do repositório** — o Windows agora instala **exclusivamente** pelo instalador gráfico
  Inno Setup (`fzcomputerai-setup-windows-x64.exe`, baixado dos releases ou gerado localmente com
  `ISCC.exe /DAppVersion=<versao> installer\fzcomputerai.iss`). Manter dois instaladores Windows duplicava
  lógica e abria margem para comportamentos divergentes. O `install.sh` permanece para Linux/macOS.
  *Não confundir com o `install.ps1` oficial do motor cua-driver (`cua/libs/cua-driver/scripts/install.ps1`),
  que continua existindo e é o que o instalador gráfico executa na task opcional do motor.*

### Corrigido
- **Nome de saída do instalador no `.iss`**: `OutputBaseFilename` fixado em `fzcomputerai-setup-windows-x64`
  (sem a versão no nome), para casar com o `INSTALLER_NAME` esperado pelo workflow de release — antes o job
  Windows falharia por não encontrar o arquivo na hora de publicar o asset.

---

### Segurança
- **Removida a autoassinatura de código do `install.ps1`** (função `Set-FzCodeSigning`, presente da v1.0.0 à v1.0.2).
  A função gerava um certificado auto-assinado `CN=FzComputerAI (Webstorage Tecnologia)` em `Cert:\CurrentUser\My`,
  **instalava esse certificado no store de Raiz Confiável do usuário** (`Cert:\CurrentUser\Root`) e assinava
  `fzcomputerai.exe` e `cua-driver.exe` com ele. Instalar uma raiz confiável na máquina de quem instala altera a
  configuração de segurança do usuário final e é a técnica clássica usada por malware para legitimar binários
  arbitrários; além disso, a assinatura resultante **não removia o aviso do SmartScreen**, porque o certificado não
  encadeia em nenhuma CA pública. **Quem executou a versão antiga do script deve seguir o procedimento de remediação
  descrito em `SIGNING.md` (seção 10, "Perguntas frequentes")** para localizar e remover o certificado dos stores
  `Root` e `My`.
- **Removido do workflow o step "Auto-Sign"**, que aplicava a mesma autoassinatura no runner do GitHub Actions.
  O comentário em `.github/workflows/build-release.yml` registra a remoção para que o step não seja reintroduzido.

### Adicionado
- **Instalador Windows (Inno Setup)**: `installer/fzcomputerai.iss` gera o `fzcomputerai-setup-windows-x64.exe`, com
  atalhos no Menu Iniciar, opção de iniciar com o Windows, instalação opcional do motor `cua-driver`, desinstalador
  registrado em *Aplicativos instalados* e upgrade in-place. **O instalador não é assinado e, portanto, exibe o mesmo
  aviso do SmartScreen que o `.exe` avulso** — contornar o SmartScreen nunca foi objetivo dele.
- **Script de assinatura local `scripts/sign-release.ps1`**: assina e verifica os artefatos com `signtool`, exigindo um
  certificado de code signing real (token USB / HSM) e carimbo de tempo RFC 3161. **Recusa-se a executar** se
  encontrar apenas certificados auto-assinados no store.
- **`SIGNING.md`**: documento de referência sobre assinatura de código, SmartScreen e distribuição — estado atual,
  custos e requisitos das opções reais de certificado, e as práticas que este projeto se recusa a adotar.
- **Console Debug na GUI**, execução de comandos sem janela de console, autostart e versão dinâmica a partir do
  `Cargo.toml`; CI com stamp de versão e `install.sh` via `curl | bash`.

---

## [1.0.2] - 2026-07-25

### Adicionado
- **Interface Gráfica (GUI) Nativa em Rust (`fzcomputerai`)**: Painel de controle completo com abas para configuração do servidor MCP, testes de calibração de tela e visão, gerenciador de janelas e processos, gravação de trajetória e diagnóstico (Doctor).
- **Autoassinatura de código (Authenticode) no `install.ps1`** — *recurso removido depois por questão de segurança; ver [2.0.0], seção "Segurança"*. O script passou a gerar um certificado auto-assinado `CN=FzComputerAI (Webstorage Tecnologia)`, a instalá-lo na Raiz Confiável do usuário (`Cert:\CurrentUser\Root`) e a assinar os binários com ele.
  > **Correção de registro:** esta entrada, como publicada originalmente, afirmava que o recurso eliminava os avisos do Windows Defender. **Isso era falso.** Um certificado auto-assinado não encadeia em nenhuma CA pública: o SmartScreen e o Defender continuam avisando exatamente igual. O texto original é mantido aqui apenas como registro histórico da promessa incorreta.
- **Workflow CI/CD Multiplataforma**: Configuração completa do GitHub Actions para compilação nativa em Windows, macOS e Linux a cada release de tag `v*`. A versão 1.0.2 incluía também um step "Auto-Sign" que aplicava a mesma autoassinatura no runner — *step removido depois; ver [2.0.0], seção "Segurança"*.
- **Pacote NPM**: Publicação global via `npm install -g fzcomputerai`.

### Corrigido
- Correção no workflow do GitHub Actions (`fail-fast: false`, remoção de `submodules: recursive` desnecessário e adição de dependências Linux).

---

## [1.0.1] - 2026-07-24

### Corrigido
- Ajustes de pipeline no GitHub Actions.

---

## [1.0.0] - 2026-07-24

### Adicionado
- **Integração de Visão Computacional MCP**: Suporte nativo ao Model Context Protocol (MCP) permitindo que agentes de IA inspecionem visualmente o desktop e controlem a UI em tempo real.
- **Ferramentas de Inspeção Visual Multimodal**: `get_desktop_state`, `get_window_state`, `take_screenshot`.
- **Ferramentas de Controle de Ponteiro & Teclado**: `mouse_click`, `mouse_move`, `keyboard_type`, `shortcut`, etc.
- **Suporte a Transporte HTTP TCP/IP Nativo**: Configuração da porta `8000` via `CUA_DRIVER_RS_MCP_HTTP_PORT` para orquestradores remotos como FazAI-NG.
- **Documentação Multilíngue**: Guias de instalação e uso completos em Português e Inglês.
- **Scripts de Instalação Automatizados**: `install.ps1` e `install.sh`.
- **Seção de Patrocinadores Oficiais**: Inclusão de Webstorage Tecnologia e Imóvel Site.

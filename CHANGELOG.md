# Changelog

Todas as alterações notáveis do projeto **FzComputerAI / CUA Driver Computer Vision MCP** serão documentadas neste arquivo.

O formato baseia-se no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/spec/v2.0.0.html).

---

## [1.0.2] - 2026-07-25

### Adicionado
- **Interface Gráfica (GUI) Nativa em Rust (`fzcomputerai`)**: Painel de controle completo com abas para configuração do servidor MCP, testes de calibração de tela e visão, gerenciador de janelas e processos, gravação de trajetória e diagnóstico (Doctor).
- **Autoassinatura Digital (Authenticode) para Windows**: O script `install.ps1` agora gera e aplica um certificado CodeSigning (`CN=FzComputerAI (Webstorage Tecnologia)`) ao binário, eliminando avisos do Windows Defender.
- **Workflow CI/CD Multiplataforma**: Configuração completa do GitHub Actions para compilação nativa em Windows (com autoassinatura), macOS e Linux a cada release de tag `v*`.
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

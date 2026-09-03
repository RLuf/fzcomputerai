# Desenvolvimento

Para quem vai compilar, alterar ou empacotar o FzComputerAI.

## 1. Compilar

Requisito: toolchain Rust estável. No Linux, as dependências de GUI do `eframe` (X11/Wayland, GL) precisam estar instaladas — o workflow de release faz isso no runner Ubuntu.

```bash
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

O binário sai em `fzcomputerai/target/release/fzcomputerai` (`.exe` no Windows).

Durante o desenvolvimento, `cargo run --manifest-path fzcomputerai/Cargo.toml` funciona, mas note que `main.rs` tem `#![windows_subsystem = "windows"]`: no Windows não há console anexado, e **toda** saída de diagnóstico vai para o console interno do app, não para o terminal.

No Windows, `build.rs` embute o recurso `VERSIONINFO` no `.exe` usando `winresource` (declarado como *build-dependency* apenas para host Windows). Sem isso, a aba Propriedades do arquivo sai em branco. Em Linux/macOS o `build.rs` é no-op.

## 2. Estrutura de pastas

```text
fzcomcontrol/
├─ fzcomputerai/              o crate da GUI
│  ├─ Cargo.toml              versão do produto (fonte única) e dependências
│  ├─ build.rs                VERSIONINFO do Windows (no-op fora do Windows)
│  ├─ assets/icon64.rgba      ícone da JANELA, RGBA cru 64x64 (include_bytes!)
│  └─ src/
│     ├─ main.rs              NativeOptions, ícone da janela, run_native, flag --tls-init
│     ├─ app.rs               AppState + toda a lógica + o shell da UI e o tema
│     ├─ tls.rs               HTTPS do endpoint: proxy TLS (rustls), auto-assinado (rcgen),
│     │                       Let's Encrypt (instant-acme), inspeção x509, sonda; teste em cargo test tls::
│     ├─ oauth.rs             OAuth 2.1 para conectores (metadata, DCR, authorize, token); teste em cargo test oauth::
│     ├─ tray.rs              ícone na bandeja (Windows)
│     └─ tabs/
│        ├─ mod.rs
│        ├─ network.rs        aba MCP & Rede
│        ├─ tunnel.rs         aba Túnel
│        ├─ mcp_tools.rs      catálogo de tools MCP
│        ├─ calibration.rs    aba Calibração
│        ├─ windows.rs        aba Janelas
│        ├─ recording.rs      aba Gravação
│        └─ doctor_skills.rs  aba Doctor & Skills
├─ installer/
│  ├─ fzcomputerai.iss        script Inno Setup (nome do artefato é contrato)
│  ├─ fzcomputerai.ico        ícone do .exe e do instalador
│  ├─ LICENSE.txt             licença exibida no wizard
│  └─ verify-install.ps1      verificação pós-instalação (testes reais)
├─ scripts/
│  ├─ make-icon.ps1           gera o .ico e o .rgba (determinístico)
│  └─ sign-release.ps1        assinatura (ver SIGNING.md)
├─ .github/workflows/
│  ├─ ci.yml                  build + test + clippy (Windows e Linux) em push/PR na master
│  └─ build-release.yml       build multiplataforma, ISCC, SHA256, release (tag v*)
├─ docs/                      esta documentação
├─ install.sh                 instalação em Linux/macOS
├─ dist/                      saída local do instalador (OutputDir do .iss)
├─ archived/                  arquivos preservados antes de sobrescrever
└─ cua/                       submódulo do projeto Cua (motor, MIT) — NÃO editar
```

Duas pastas que **não** são do produto e não devem ser lidas nem varridas por ferramenta automática: `cua/` (repositório de terceiro) e `target/` (artefatos de build). O mesmo vale para exports de conversa em `.claude/` e `.claude-code-history/`, e para `archived/`.

**Divisão de responsabilidade entre `app.rs` e `tabs/`:** `tabs/*.rs` desenha e, no `clicked()`, chama um método de `AppState`. A camada de UI **não** executa processo, não fala com a rede e não escreve no registro. Toda essa lógica vive em `app.rs`. Um `tabs/` que chame `Command::new` está errado.

## 3. Convenções obrigatórias

Estas não são preferências de estilo — quebrá-las produz bug visível ao usuário.

| Convenção | Por quê | Como |
| --- | --- | --- |
| **Spawn só via `quiet_cmd`** | `Command::new` direto abre uma janela preta de console que pisca na tela do usuário a cada ação | `quiet_cmd("netsh")` aplica `CREATE_NO_WINDOW` no Windows e é transparente nos outros sistemas |
| **Todo handler loga** | o console é o único lugar onde o usuário vê o que aconteceu; ação silenciosa é ação não auditável | use `run_logged()` (registra comando + `exit` + `stdout` + `stderr`) ou, quando não houver processo, `log_debug()` com o resultado real |
| **Versão sempre de `env!("CARGO_PKG_VERSION")`** | versão escrita à mão em dois lugares divergiu no passado; o CI estampa a versão no `Cargo.toml` a partir do tag | nunca escreva a versão como literal em código, string de UI, header HTTP ou snippet |
| **Status honesto** | badge que mente é pior que badge ausente | nenhum estado exibido pode vir da intenção. Depois de agir, **releia a fonte de verdade** (`netstat`, `netsh show`, `reg query`, POST real) e exiba o que ela disse |
| **Sem dependência nova** | cada crate nova é superfície de auditoria e de build; o projeto resolve HTTP com `TcpStream` e TLS com `curl.exe` justamente para não crescer | resolva com a std, com `serde_json` (já presente) ou delegando a um binário do sistema — o encaminhamento LAN, por exemplo, é `std::net::TcpListener` + `std::io::copy`, zero crate nova. Se não houver alternativa, isso é decisão do dono do projeto, não do PR |
| **Sem emoji e sem glifo ausente** | a fonte do tema não tem `→`, `●` nem emoji: renderizam caixa vazia, com cara de placeholder quebrado | use `->` em texto e `status_dot()` (ponto desenhado pelo painter) para status. Formas como o coração do diálogo Sobre são **desenhadas**, não caracteres |
| **i18n via `match state.language`** | não há arquivo de tradução nem recarga: a troca de idioma é em tempo real, no mesmo frame | todo texto visível nasce como `match state.language { Language::PtBr => "...", Language::English => "..." }`. Para mensagens curtas em `app.rs`, use o helper `self.tr(pt, en)` |
| **Segredo não vai para argv, log nem registro** | `Win32_Process.CommandLine` é legível por outros processos, e o console é copiável pelo usuário | token do Cloudflare em arquivo com ACL restrita (só o caminho persiste); senha do ngrok no arquivo de policy; senha do porteiro só em memória e mascarada como `/s/***/`; URL com senha vai ao `curl` por `--config`. Única exceção consciente: o token do endpoint MCP vive em `HKCU\Environment` porque **é assim que o motor o recebe** — e mesmo ele nunca é impresso no console |
| **Nunca matar por nome de imagem** | `taskkill /IM cloudflared.exe` mataria o `cloudflared` legítimo do usuário; pelo mesmo motivo o `taskkill /F /IM cua-driver.exe` foi removido do app e do instalador — ele derrubava também o motor que o usuário tivesse rodando para outro fim | túnel: identidade de 3 fatores (imagem + `CreationDate` + marcador `run_id` na linha de comando). Motor: `cua-driver stop` (comando oficial) e, se sobrar processo, kill **por PID** com o caminho do executável conferido |
| **Só remover o que registramos** | há regras `portproxy` de outros serviços nesta mesma máquina (hoje só o caminho de *fallback* chega a criar uma) | percorra os valores `portproxy:*` / `tunnel:*` de `HKCU\Software\FzComputerAI`; nunca case por "padrão parecido" |
| **Não bloquear o `on_exit` — nem terceirizar a limpeza para o PowerShell** | a versão bloqueante com `Start-Process -Verb RunAs -Wait` fazia o app **não fechar**: janela sumia, processo vivo, portas abertas. A versão seguinte — um `powershell -WindowStyle Hidden` de ~2 KB que esperava o processo morrer, matava processos, escrevia no registro, rodava `netsh` e ainda chamava `-Verb RunAs` — foi **flagrada pelo Microsoft Defender** por essa linha de comando (detecção 2147941383, 2026-08-03) | a limpeza é Rust nativo: chamadas diretas e curtas, sem elevação e sem PowerShell. O que ainda for pesado vai para processo auxiliar destacado com `spawn()`, sem `wait()` |
| **Trabalho longo não roda na thread da UI** | congela a janela | processo destacado + arquivo de flag em `%TEMP%` + um `poll_*` com *throttle* de 1 s, chamado do `update()`. Enquanto houver pendência, `request_repaint_after(1s)` |
| **Sem threads, salvo exceção justificada** | previsibilidade | as exceções hoje são o porteiro do túnel (um servidor não é implementável por poll de arquivo) e o encaminhador LAN. Ambos morrem com o app: `AtomicBool` + conexão dummy destravam o `accept` |
| **Encaminhamento LAN é do app, não do `netsh`** | a regra `portproxy` é estática, do serviço IP Helper: exigia admin/UAC para criar e remover, continuava mostrando a porta **LISTENING na LAN mesmo com o motor morto** (aceitava conexões que morriam no destino — falso positivo de serviço no ar) e sobrevivia ao fechamento do app e ao reboot, obrigando uma rotina de limpeza | uma thread do próprio processo escuta em `<ip_lan>:porta` e copia bytes contra `127.0.0.1:porta` (`std::net::TcpListener` + `std::io::copy`). É TCP puro: `curl`, `telnet` e `nc` atravessam igual. O `netsh portproxy` continua **apenas como fallback**, quando o bind no IP da LAN falha |
| **A GUI é dona do motor** | enquanto `start_daemon` chamava `cua-driver autostart kick`, quem subia o motor era a Scheduled Task e o processo nascia filho do **Agendador**: (a) o `stdout` pertencia à task e os logs sumiam — a atividade de clientes MCP externos (conector do Claude, Antigravity, Cursor) não aparecia no console; (b) a task herda o ambiente do **logon**, então token/porta gravados depois de logar não eram vistos e o motor 0.16+ morria no ato, deixando a porta muda com a GUI dizendo só "PARADO"; (c) a GUI não controlava o ciclo de vida do que gerencia | lance `cua-driver serve` como **processo filho**, com porta e token injetados no ambiente, e redirecione `stdout`+`stderr` para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, que o console segue como `tail -f` (prefixo `[motor]`). A Scheduled Task fica como último recurso — e nesse caso o console avisa que o processo não é da GUI e que não haverá logs |
| **"Iniciar" não derruba motor saudável** | a função parava o motor e subia outro **sem checar se o endpoint já respondia**; no Windows, sockets da porta que já tiveram conexão ficam retidos em `TIME_WAIT` por minutos, então o `serve` novo não conseguia o bind (`MCP HTTP transport disabled — bind 127.0.0.1:8000 failed (os error 10048)`) e sobrava um daemon zumbi: pipe vivo, porta muda. Na prática, clicar Iniciar **quebrava** o que estava funcionando | com o endpoint respondendo, o botão não encosta no processo. Forçar troca de processo é papel do "Reiniciar" |
| **Token do endpoint é gerado pela GUI** | o motor 0.16+ chama o valor de *host-generated bearer token* — o host é o app; sem token o `serve` sai com erro e **nenhuma porta abre** | a GUI gera 32 bytes do RNG do Windows (64 chars hex) e persiste em `HKCU\Environment` na primeira vez que precisa; o usuário não precisa saber que a variável existe. O valor **nunca** é exibido em log — para ler: `reg query HKCU\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN` |
| **Salto de rolagem tem precedência sobre a detecção de posição** | o botão "Ir ao fim" do console não funcionava: o clique setava `console_follow = true` e a detecção da posição do scroll, no **mesmo frame**, sobrescrevia o valor com a posição antiga | quando o frame tem um salto explícito, ele vence a heurística de posição |

Antes de abrir PR, releia também `AGENTS.md` na raiz: ele é a fonte normativa do projeto e detalha as decisões que **não** devem ser revertidas.

## 4. Gerar o ícone

Há dois ícones distintos, gerados juntos:

| Saída | Uso |
| --- | --- |
| `installer/fzcomputerai.ico` | ícone do `.exe` (aplicado pelo `build.rs` como recurso Win32) e do instalador — é o que o Explorer, a busca e o atalho mostram |
| `fzcomputerai/assets/icon64.rgba` | RGBA **cru** 64x64 embutido por `include_bytes!` — é o ícone da **janela** e da barra de tarefas enquanto o app roda |

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\make-icon.ps1
```

O `.rgba` é cru de propósito: decodificar PNG exigiria a feature `image` do `eframe`, ou seja, dependência nova. O desenho é determinístico (semente fixa), então rodar o script duas vezes produz o mesmo arquivo. Depois de regenerar, recompile — o `include_bytes!` só pega o novo conteúdo em novo build.

## 5. Montar o instalador localmente

Requisito: Inno Setup 6 (`ISCC.exe`).

```powershell
# 1. compilar a GUI
cargo build --release --manifest-path fzcomputerai\Cargo.toml

# 2. gerar o instalador (ajuste a versão e o caminho do exe)
& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' `
    "/DAppVersion=2.1.0" `
    "/DSourceExe=..\fzcomputerai\target\release\fzcomputerai.exe" `
    .\installer\fzcomputerai.iss
```

Saída: `dist\fzcomputerai-setup-windows-x64.exe` (`OutputDir=..\dist`, `OutputBaseFilename=fzcomputerai-setup-windows-x64` no `.iss`).

Pontos do `.iss` que **são contrato** e não devem mudar sem mudar o outro lado:

| Item | Contrato com |
| --- | --- |
| `OutputBaseFilename=fzcomputerai-setup-windows-x64` | o workflow de release **e** o auto-upgrade da GUI, que baixa exatamente esse nome |
| valor `FzComputerAI` em `HKCU\...\Run`, com o caminho **entre aspas** | o checkbox "Iniciar com o Windows" da GUI (`set_autostart`), que grava no mesmo formato — divergir dessincroniza o checkbox |
| `{app}\cua-driver\install.ps1` como destino do script embarcado do Cua | o caminho que a GUI procuraria para instalar o motor; o `_install-common.psm1` precisa ficar no **mesmo** diretório |

Outros comportamentos deliberados do instalador: `PrivilegesRequired=lowest` (instalação por usuário, em `{autopf}`); o componente **motor** (`engine`) vem **marcado** por padrão, porque sem o motor nenhum botão funciona; o passo do motor **não pina versão** — o alvo é a última versão estável publicada, obtida de `cua-driver check-update --json` na hora da instalação e passada **explícita** ao `install.ps1` oficial via `-Release` (atenção: sem `-Release` o script oficial **não** consulta o GitHub — instala o `BAKED_VERSION` congelado dentro dele; com alvo desconhecido, o instalador usa o script do endpoint oficial cua.ai, cujo baked o CD do Cua mantém na latest, com o embarcado como fallback offline); "nada a fazer" = instalado == latest confirmado pelo check-update (`/FORCEENGINE` força a reinstalação, `/SKIPENGINE` pula o passo); o passo do motor roda em `ssPostInstall` **inclusive em instalação silenciosa** (com `-NoAutoStart`, pois registrar a Scheduled Task exige admin/UAC); no fim da instalação o setup roda `cua-driver skills install` — sem os symlinks o agente conecta no MCP e **não enxerga ferramenta nenhuma**, e quem acabou de instalar não tem como adivinhar que precisa clicar num botão da aba Doctor & Skills; o comando é idempotente e, pelo help oficial do motor, "Never overwrites existing user links"; e `{app}\tunnel` (binários baixados sob demanda, token-file, policy do ngrok) é removido na desinstalação.

Para conferir o resultado numa máquina, `installer/verify-install.ps1` fica instalado junto com o app e pode ser reexecutado a qualquer momento: ele faz testes reais de MCP, porta, autostart e motor.

## 6. CI de release

`.github/workflows/build-release.yml`, em resumo:

1. **estampa a versão** do tag no `Cargo.toml` e no `package.json` (por isso a versão em código vem sempre de `env!("CARGO_PKG_VERSION")`);
2. compila `cargo build --release --manifest-path fzcomputerai/Cargo.toml` nas três plataformas;
3. no Windows, **assina** o executável e o instalador com `signtool` **se** houver certificado configurado em segredo; sem certificado, o job emite um aviso explícito de que os binários **não** estão assinados — e é essa a situação dos artefatos publicados hoje (ver [`SIGNING.md`](../SIGNING.md));
4. localiza/instala o `ISCC` e gera o instalador com `/DAppVersion` e `/DSourceExe`, falhando se o `.exe` esperado não aparecer;
5. gera um `.sha256` **por artefato**, com o caminho gravado sem diretório, para que `sha256sum -c` funcione para quem baixa o instalador e o `.sha256` soltos na mesma pasta;
6. publica os artefatos.

O `.sha256` não é enfeite: é exatamente o que o auto-upgrade da GUI confere antes de executar o instalador baixado.

## 7. Checklist antes do PR

- [ ] compila com `cargo build --release --manifest-path fzcomputerai/Cargo.toml` sem warning novo;
- [ ] nenhuma dependência nova em `Cargo.toml`;
- [ ] nenhum `Command::new` fora de `quiet_cmd`;
- [ ] todo caminho novo escreve algo no console (`run_logged` ou `log_debug`);
- [ ] nenhum estado exibido sem releitura da fonte de verdade;
- [ ] texto novo tem as duas línguas;
- [ ] nenhum emoji, `→` ou `●` em string;
- [ ] nenhum segredo em argv, log ou registro;
- [ ] se mexeu em nome de artefato, chave de registro ou caminho de script: os dois lados do contrato foram atualizados;
- [ ] se um arquivo existente foi sobrescrito, a versão anterior foi copiada para `archived/`.

## Ver também

- [arquitetura.md](arquitetura.md) — o desenho que essas convenções sustentam.
- [atualizacao.md](atualizacao.md) — por que o nome do instalador é contrato fixo.
- [solucao-de-problemas.md](solucao-de-problemas.md) — os sintomas que aparecem quando uma convenção é quebrada.

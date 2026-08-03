# Atualização

Para quem clicou em **Verificar e Atualizar** e quer saber o que vai acontecer com cada componente.

## 1. Dois componentes, dois caminhos

O botão **Verificar e Atualizar** (aba MCP & Rede) cuida de **duas** coisas — e **age**, em vez de só relatar — e a **Central de Atualizações** mostra o estado real das duas — versão instalada x disponível:

| Componente | O que é | Como é verificado | Como é aplicado |
| --- | --- | --- | --- |
| **Interface (FzComputerAI)** | esta GUI | GitHub Releases de `RLuf/fzcomputerai` (`releases/latest`), comparando com `env!("CARGO_PKG_VERSION")` | download do instalador **com SHA256 conferido** + instalação silenciosa |
| **Motor (`cua-driver`)** | quem faz o trabalho | `cua-driver check-update --json` — a **API oficial do próprio motor** | `cua-driver update --apply` — o **atualizador oficial dele**, em processo destacado; se o subcomando não existir ou falhar (ex.: motor 0.8.3), **fallback automático** para o instalador oficial do projeto Cua, que instala a última versão estável do GitHub |

Por que o motor entra aqui: enquanto o botão olhava apenas a GUI, o motor podia ficar dezenas de versões atrás sem ninguém perceber — e foi o que aconteceu. Na máquina de referência, **0.8.3 instalado** contra **0.17.0 publicado** (em 2026-08-02). Versões novas do motor mudam contrato — o caso concreto, **medido no 0.17.0 em 2026-08-03**: sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` o `serve` **nem sobe**, e com o daemon no ar toda requisição sem `Authorization: Bearer` recebe **401**. Saber a versão real, portanto, não é cosmético: é o que evita a GUI reportar estado errado.

**Nunca baixamos binário do motor por conta própria.** Quem publica e instala o motor é o projeto Cua.

## 2. Fluxo da GUI, passo a passo

1. **Verificação.** `Invoke-RestMethod` na API do GitHub lê o `tag_name` da release marcada como *latest*.
2. **Comparação numérica** de `major.minor.patch`. Só considera atualização se a remota for **estritamente mais nova**. Se a *latest* apontar para um tag **mais antigo** que a instalada (rollback de release no GitHub, que é um ponteiro mutável), a GUI **informa e não faz nada** — downgrade silencioso não acontece.
3. **Download em processo separado** (a UI não trava), para `%TEMP%\fzcomputerai-update\`:
   - baixa `fzcomputerai-setup-windows-x64.exe`;
   - baixa `fzcomputerai-setup-windows-x64.exe.sha256` publicado pelo CI;
   - compara `Get-FileHash -Algorithm SHA256` com o valor esperado;
   - **hash confere** ⇒ grava `ready.flag`. **Hash divergente** ⇒ grava `error.flag` **e apaga o instalador**. Executável baixado sem integridade conferida nunca roda.
4. **A GUI observa as flags** (poll de ~1 s) e, quando `ready.flag` e o `.exe` existem, abre o diálogo **Pronto para atualizar**.
5. **Instalação.** Ao confirmar "Fechar e instalar agora", a GUI dispara um processo em background e **fecha**. Esse processo:
   - espera o `fzcomputerai` sair (até 30 s) e então garante o encerramento (`Stop-Process -Force`);
   - encerra também o `cua-driver`;
   - executa o instalador com `/VERYSILENT /NORESTART`;
   - reabre a GUI (`%LOCALAPPDATA%\Programs\FzComputerAI\fzcomputerai.exe`, com fallback para o caminho do executável anterior);
   - religa o motor com `cua-driver autostart kick`.

Nada de "instalar por cima" com o app aberto. Se você escolher **Depois**, o instalador fica em `%TEMP%` e o diálogo volta na próxima verificação.

O nome `fzcomputerai-setup-windows-x64.exe` é **contrato fixo** entre o workflow de release, o `.iss` do Inno Setup e este auto-upgrade. Renomear o artefato quebra a atualização automática.

## 3. Fluxo do motor, passo a passo

1. **Verificação:** `cua-driver check-update --json`. A GUI lê os campos `current_version`, `latest_version`, `update_available` e `release_notes_url`. Se o motor reportar `error`, isso vai para o console.
2. Se o comando não puder ser executado, a Central mostra honestamente "não foi possível consultar o motor (instalado? no PATH?)" — não inventa versão. A GUI resolve o caminho do executável do motor por conta própria (`engine_exe()`: PATH ou o caminho canônico de instalação), então isso funciona logo após instalar o motor, mesmo com o PATH da sessão desatualizado.
3. **Aplicação:** se há versão mais nova, a atualização é **automática de ponta a ponta** — sem mais cliques. Um processo destacado executa, nesta ordem:

```powershell
cua-driver stop
cua-driver update --apply
cua-driver autostart kick
cua-driver check-update --json     # relê a versão para a GUI
```

   Se `update --apply` não existir ou falhar (ex.: motor **0.8.3**, anterior ao subcomando), o processo faz **fallback automático** para o instalador oficial do projeto Cua, que instala a última versão estável publicada no GitHub — e o autostart é religado do mesmo jeito, com o daemon novo no ar.

4. O resultado é gravado em `drv-ready.flag` (ou `drv-error.flag`) em `%TEMP%\fzcomputerai-update\`, e a GUI observa com poll de ~1 s.
5. Ao terminar, a GUI **relê a versão real** e **retesta o endpoint**. O estado exibido nunca vem de "mandei atualizar, então atualizou".

Fora do Windows, a chamada é direta (`cua-driver update --apply`), sem o envelope de autostart, que é específico do Windows.

## 4. Aviso importante: motor novo exige token (verificado)

Isto foi **medido no binário `cua-driver` 0.17.0 em 2026-08-03** — não é mais "a documentação diz". Dois níveis, que na tela parecem o mesmo problema mas não são:

- **Sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do processo, o daemon nem sobe:** `cua-driver serve` sai com código 1 e o erro `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`. A porta simplesmente não abre.
- **Com o daemon no ar, requisição sem `Authorization: Bearer <token>` recebe 401**, corpo `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` — igual em `POST /mcp`, `GET /mcp` e `GET /`, e sem header `WWW-Authenticate`. Com o header correto, **200** e o `result` do `initialize`.

O token é **gerado por você**: qualquer string aleatória serve — o próprio motor a chama de *host-generated bearer token*. Não há comando no `cua-driver` nem no instalador oficial do Cua que gere um. Versões antigas (como a **0.8.3**) não têm token nenhum.

Consequências práticas depois de atualizar o motor:

- se você **não** configurar o token, não é "o endpoint responde 401": o daemon morre ao subir e a aba MCP & Rede mostrará **PARADO** — corretamente, porque o teste é um POST real e não há ninguém escutando;
- **grave o token em `HKCU\Environment` (`setx`) e considere o logon.** A Scheduled Task que sobe o daemon herda o ambiente do **logon**: token gravado depois de você logar só é enxergado por ela no próximo. Desde a GUI **v2.1.0**, quando o `autostart kick` não abre a porta, a GUI lê porta e token do registro, para o daemon anterior e lança o `serve` com as variáveis injetadas no processo filho — que é o que destrava o caso "task rodou, porta muda";
- a GUI **lê** o token de `HKCU\Environment` na abertura e passa a enviar o header `Authorization: Bearer` em todos os testes. Ela **não gera nem grava** o token: escolher o segredo é papel de quem opera a máquina;
- se você configurou o token com o app aberto, **reabra o app** para ele reler a variável;
- clientes MCP que já estavam conectados precisam passar a enviar o header. Snippets antigos sem `Authorization` deixam de funcionar.

A própria Central de Atualizações mostra esse aviso ao lado do botão do motor, e traz o link para as notas da versão quando o motor informa a URL.

## 5. Onde as coisas ficam

| Caminho | Conteúdo |
| --- | --- |
| `%TEMP%\fzcomputerai-update\` | instalador baixado, `.sha256`, `ready.flag` / `error.flag` (GUI) e `drv-ready.flag` / `drv-error.flag` (motor) |
| `%LOCALAPPDATA%\Programs\FzComputerAI\` | destino padrão da instalação por usuário (o `.iss` usa `{autopf}` com `PrivilegesRequired=lowest`) |

## 6. Instalação e atualização manual

**Windows.** Exclusivamente pelo instalador gráfico Inno Setup: `fzcomputerai-setup-windows-x64.exe`. Cada artefato de release tem um `.sha256` ao lado — confira antes de executar:

```powershell
Get-FileHash .\fzcomputerai-setup-windows-x64.exe -Algorithm SHA256
Get-Content .\fzcomputerai-setup-windows-x64.exe.sha256
```

**Linux/macOS.** Por `install.sh`, na raiz do repositório.

**O motor é instalado à parte**, pelo instalador oficial do Cua (baixado da rede) — ele **não é embarcado** na GUI. O instalador do FzComputerAI traz o componente **Motor de automação cua-driver** (`engine`), marcado por padrão, que executa o `install.ps1` oficial do Cua ao final da instalação — sempre resolvendo a **última versão estável** publicada (não há versão fixada do motor). Se o motor instalado já é a mais recente — decidido na hora via `cua-driver check-update --json` —, não há nada a fazer; `/FORCEENGINE` força a reinstalação e `/SKIPENGINE` pula a etapa. Sem rede, a etapa falha rápido e de forma **não-fatal**: o motor atual permanece intacto. Duas ressalvas documentadas de propósito:

- o passo do motor roda **também em instalação silenciosa** (`/VERYSILENT`, que é o caminho do auto-upgrade da GUI) — porém com `-NoAutoStart`: registrar a Scheduled Task do daemon exige admin, e um UAC numa instalação desassistida travaria o processo esperando um clique que ninguém vai dar. Para deploy em massa sem rede ou com motor provisionado à parte, use `/SKIPENGINE`;
- falha nessa etapa **não derruba** a instalação da GUI — o passo roda após a cópia dos arquivos (`ssPostInstall`), e o resultado é que você fica com a interface funcionando e sem controle da máquina, o que o aviso do próprio instalador explica.

**Assinatura.** Os binários publicados **não são assinados**: o SmartScreen do Windows vai avisar. O motivo, as alternativas e o procedimento estão em [`SIGNING.md`](../SIGNING.md). A verificação de integridade disponível é o `.sha256` de cada artefato — e é exatamente o que o auto-upgrade confere antes de executar qualquer coisa.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — onde fica o botão e como conferir o endpoint depois de atualizar.
- [solucao-de-problemas.md](solucao-de-problemas.md) — "MCP parado" com o motor rodando (o caso do token) e SmartScreen bloqueando o instalador.
- [acesso-remoto.md](acesso-remoto.md) — o que o token do motor novo muda, e o que ele não resolve.
- [desenvolvimento.md](desenvolvimento.md) — como o instalador é gerado e por que o nome do artefato é contrato.

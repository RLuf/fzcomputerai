# Atualização

Para quem clicou em **Verificar Atualizações** e quer saber o que vai acontecer com cada componente.

## 1. Dois componentes, dois caminhos

O botão **Verificar Atualizações** (aba MCP & Rede) cuida de **duas** coisas, e a **Central de Atualizações** mostra o estado real das duas — versão instalada x disponível:

| Componente | O que é | Como é verificado | Como é aplicado |
| --- | --- | --- | --- |
| **Interface (FzComputerAI)** | esta GUI | GitHub Releases de `RLuf/fzcomputerai` (`releases/latest`), comparando com `env!("CARGO_PKG_VERSION")` | download do instalador **com SHA256 conferido** + instalação silenciosa |
| **Motor (`cua-driver`)** | quem faz o trabalho | `cua-driver check-update --json` — a **API oficial do próprio motor** | `cua-driver update --apply` — o **atualizador oficial dele**, em processo destacado |

Por que o motor entra aqui: enquanto o botão olhava apenas a GUI, o motor podia ficar dezenas de versões atrás sem ninguém perceber — e foi o que aconteceu. Na máquina de referência, **0.8.3 instalado** contra **0.17.0 publicado** (em 2026-08-02). Versões novas do motor mudam contrato (por exemplo, passaram a exigir token no endpoint HTTP), então saber a versão real não é cosmético: é o que evita a GUI reportar estado errado.

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
   - reabre a GUI (`%LOCALAPPDATA%\Programs\FzComputerAI\fzcomputerai.exe`, com fallback para o caminho do executável anterior) — e é ela que sobe o motor de novo, como processo filho. **Não há `autostart kick`**: a tarefa agendada saiu do fluxo do aplicativo na v2.3.0.

Nada de "instalar por cima" com o app aberto. Se você escolher **Depois**, o instalador fica em `%TEMP%` e o diálogo volta na próxima verificação.

O nome `fzcomputerai-setup-windows-x64.exe` é **contrato fixo** entre o workflow de release, o `.iss` do Inno Setup e este auto-upgrade. Renomear o artefato quebra a atualização automática.

## 3. Fluxo do motor, passo a passo

1. **Verificação:** `cua-driver check-update --json`. A GUI lê os campos `current_version`, `latest_version`, `update_available` e `release_notes_url`. Se o motor reportar `error`, isso vai para o console.
2. Se o comando não puder ser executado, a Central mostra honestamente "não foi possível consultar o motor (instalado? no PATH?)" — não inventa versão.
3. **Aplicação:** ao clicar em **Atualizar motor**, um processo destacado executa, nesta ordem:

```powershell
cua-driver stop
cua-driver update --apply
cua-driver check-update --json     # relê a versão para a GUI
```

4. O resultado é gravado em `drv-ready.flag` (ou `drv-error.flag`) em `%TEMP%\fzcomputerai-update\`, e a GUI observa com poll de ~1 s.
5. Ao terminar, a GUI **relê a versão real** e **retesta o endpoint**. O estado exibido nunca vem de "mandei atualizar, então atualizou".

Depois do `--apply`, quem religa o motor e a propria GUI, subindo-o como processo filho — nao ha etapa de autostart.

## 4. Aviso importante: motor novo pode exigir token

Versões da série **`0.16+`** do `cua-driver` **exigem** `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32 a 4096 caracteres, sem espaço nem caractere de controle) e respondem **401** a qualquer `POST /mcp` sem `Authorization: Bearer <token>`; elas também rejeitam requisições com origem de navegador. Versões antigas (**<= 0.8.x**) não têm token nenhum — e por isso o instalador passou a exigir **0.16.0 como versão mínima** (comparação numérica, "igual ou mais novo": quem já tem 0.17+ não é rebaixado).

Consequências práticas depois de atualizar o motor:

- se você **não** configurar o token, o endpoint HTTP responderá **401 para tudo** (fail-closed — não existe "aberto sem token" no 0.16+) e a GUI mostrará o estado real;
- a GUI **lê** o token de `HKCU\Environment` na abertura **e ao abrir a aba Túnel**, e envia o header `Authorization: Bearer` em todos os testes. Desde a **2.2.0** ela também **gera e grava** o token por você: botão **"Gerar e ativar token do motor"** na aba Túnel (CSPRNG, gravação confirmada em `HKCU\Environment`, reinício do daemon);
- clientes MCP que já estavam conectados precisam passar a enviar o header. Snippets antigos sem `Authorization` deixam de funcionar — o snippet copiado da aba Túnel já sai com o header.

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

**O motor é instalado à parte**, pelo instalador oficial do Cua (baixado da rede) — ele **não é embarcado** na GUI. O instalador do FzComputerAI traz o **componente `engine`** (marcado por padrão; a página de pré-requisitos o desmarca sozinha quando a versão fixada já está instalada), que executa o instalador oficial do Cua como **passo real da instalação** (`InstallEngineStep`). Duas ressalvas documentadas de propósito:

- a etapa do motor roda **também na instalação silenciosa** (`/VERYSILENT`) — o `skipifsilent` da versão antiga era um defeito, porque o auto-upgrade da GUI usa exatamente esse caminho. Para deploy desassistido **sem** o motor, passe `/SKIPENGINE`;
- falha nessa etapa **não derruba** a instalação da GUI — o resultado é a interface funcionando e sem controle da máquina, o que o aviso do próprio instalador explica.

**Assinatura.** Os binários publicados **não são assinados**: o SmartScreen do Windows vai avisar. O motivo, as alternativas e o procedimento estão em [`SIGNING.md`](../SIGNING.md). A verificação de integridade disponível é o `.sha256` de cada artefato — e é exatamente o que o auto-upgrade confere antes de executar qualquer coisa.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — onde fica o botão e como conferir o endpoint depois de atualizar.
- [solucao-de-problemas.md](solucao-de-problemas.md) — "MCP parado" com o motor rodando (o caso do token) e SmartScreen bloqueando o instalador.
- [acesso-remoto.md](acesso-remoto.md) — o que o token do motor novo muda, e o que ele não resolve.
- [desenvolvimento.md](desenvolvimento.md) — como o instalador é gerado e por que o nome do artefato é contrato.

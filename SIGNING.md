# Assinatura de Código, SmartScreen e Distribuição — FzComputerAI

> **Resumo em uma linha:** os binários do FzComputerAI **não são assinados digitalmente** hoje.
> O Windows exibe o aviso do SmartScreen, o usuário precisa clicar em **Mais informações → Executar assim mesmo**,
> e **o instalador não muda isso**. Este documento explica por quê, o que é possível fazer, quanto custa e o que
> este projeto se recusa a fazer.

Este arquivo existe para não haver promessa falsa nem improviso. Se você chegou aqui procurando um jeito de
"remover o aviso do Windows sem comprar certificado": não existe. A parte final do documento lista as opções
reais, com custo e requisito de cada uma.

---

## Índice

1. [Estado atual](#1-estado-atual)
2. [O que o usuário final vê](#2-o-que-o-usuário-final-vê)
3. [Como conferir a integridade do download (SHA256)](#3-como-conferir-a-integridade-do-download-sha256)
4. [Por que o instalador NÃO resolve o SmartScreen](#4-por-que-o-instalador-não-resolve-o-smartscreen)
5. [Como a reputação do SmartScreen realmente funciona](#5-como-a-reputação-do-smartscreen-realmente-funciona)
6. [Por que NÃO existe "assinar durante a instalação" — decisão de projeto](#6-por-que-não-existe-assinar-durante-a-instalação--decisão-de-projeto)
7. [Opções reais de assinatura](#7-opções-reais-de-assinatura)
8. [Fluxo de release assinando localmente com token USB](#8-fluxo-de-release-assinando-localmente-com-token-usb)
9. [Carimbo de tempo (timestamp) é obrigatório](#9-carimbo-de-tempo-timestamp-é-obrigatório)
10. [Perguntas frequentes](#10-perguntas-frequentes)
11. [Arquivos relacionados neste repositório](#11-arquivos-relacionados-neste-repositório)

---

## 1. Estado atual

| Item | Situação |
| :--- | :--- |
| `fzcomputerai-windows-x64.exe` (portátil) | **Não assinado** |
| `fzcomputerai-setup-windows-x64.exe` (instalador Inno Setup) | **Não assinado** |
| Binários macOS / Linux | **Não assinados** (sem Authenticode; sem notarização Apple) |
| Certificado de code signing do projeto | **Não existe** — nunca foi comprado |
| Checksums `.sha256` | **Publicados** em todo release, para cada artefato |
| Assinatura no CI | **Preparada, porém inativa** — só roda se os segredos `WINDOWS_CERT_PFX_BASE64` / `WINDOWS_CERT_PASSWORD` existirem no repositório |

O workflow `.github/workflows/build-release.yml` emite um `::warning` explícito em todo build sem certificado,
avisando que os binários saíram sem assinatura. O build **não falha** por isso — é o estado esperado hoje.

> **Nota sobre macOS e Linux:** este documento trata do Windows/Authenticode, que é onde está o problema visível
> hoje. macOS tem um caminho próprio (Developer ID + notarização, exige conta paga no Apple Developer Program) que
> **ainda não foi avaliado neste projeto**. Linux normalmente não exige assinatura para executar binários baixados.

---

## 2. O que o usuário final vê

Ao baixar e executar o `.exe` **ou o instalador** no Windows 10/11:

```
  Windows protegeu o seu PC
  O Microsoft Defender SmartScreen impediu a inicialização de um aplicativo não reconhecido.
  A execução desse aplicativo pode colocar o computador em risco.

  [ Não executar ]
```

O botão para prosseguir está escondido atrás do link **"Mais informações"**:

1. Clique em **Mais informações**;
2. Confirme que aparece o nome do arquivo esperado (`fzcomputerai-setup-windows-x64.exe`);
3. Clique em **Executar assim mesmo**.

Além disso, o Windows pode marcar o arquivo com a *Mark of the Web* (zona de download). Se o Explorer mostrar um
botão **Desbloquear** nas propriedades do arquivo, é o mesmo fenômeno.

**Antes de clicar em "Executar assim mesmo", confira o hash.** É a única verificação de integridade que este
projeto pode oferecer hoje — e ela é real, não um paliativo cosmético.

---

## 3. Como conferir a integridade do download (SHA256)

Todo release publica, ao lado de cada binário, um arquivo `.sha256` com o hash do artefato correspondente.
Baixe os dois arquivos na mesma pasta e compare.

**PowerShell (Windows):**

```powershell
# Calcula o hash do arquivo baixado
Get-FileHash .\fzcomputerai-setup-windows-x64.exe -Algorithm SHA256

# Mostra o hash publicado no release, para comparar visualmente
Get-Content .\fzcomputerai-setup-windows-x64.exe.sha256
```

**Git Bash / WSL / Linux:**

```bash
sha256sum -c fzcomputerai-setup-windows-x64.exe.sha256
```

**macOS:**

```bash
shasum -a 256 -c fzcomputerai-macos.sha256
```

Se os hashes **não** baterem, apague o arquivo e baixe de novo direto de
<https://github.com/RLuf/fzcomputerai/releases>. Não execute.

> **Limite honesto desta verificação:** o SHA256 prova que o arquivo que você baixou é *bit a bit* o mesmo que o
> CI publicou. Ele **não** prova quem produziu o arquivo — se a conta do GitHub fosse comprometida, o atacante
> publicaria binário e hash juntos. É exatamente esse problema de *autoria* que a assinatura de código resolve e
> o checksum não. Por isso o checksum é um mitigador, não um substituto.

---

## 4. Por que o instalador NÃO resolve o SmartScreen

Esta é a pergunta mais frequente e a resposta é curta: **não resolve, e nunca resolveria.**

O SmartScreen avalia o **arquivo executável que o usuário abriu**. Um instalador (`.exe` de setup gerado pelo
Inno Setup, ou um `.msi`) é, ele próprio, um executável baixado da internet. Se ele não estiver assinado, recebe
**exatamente o mesmo bloqueio** que um `.exe` avulso não assinado. Não há "modo instalador" que dispense o
Authenticode.

Ou seja:

| Distribuição | Assinado? | SmartScreen |
| :--- | :---: | :--- |
| `.exe` portátil | não | avisa |
| Instalador `.exe` (Inno Setup) | não | avisa **igual** |
| `.msi` | não | avisa **igual** |
| Qualquer um dos três | sim, com cert OV/EV válido | não avisa (ou para de avisar rapidamente — ver seção 5) |

O instalador deste projeto (`installer/fzcomputerai.iss`) existe por outros motivos, todos legítimos:
atalhos no Menu Iniciar, opção de iniciar com o Windows, instalação opcional do motor `cua-driver`, desinstalador
registrado em *Aplicativos instalados* e upgrade in-place entre versões. **Contornar o SmartScreen não é, e não
poderia ser, um deles.** Esse aviso está escrito também no cabeçalho do próprio `.iss`, para que ninguém
"otimize" o arquivo achando que resolveria.

---

## 5. Como a reputação do SmartScreen realmente funciona

Entender isso muda a decisão de compra, então vale o parágrafo:

- **Arquivo não assinado → reputação por hash.** O SmartScreen acumula reputação para *aquele arquivo exato*.
  Como cada nova versão gera um binário diferente, a reputação **recomeça do zero a cada release**. Você nunca
  sai do aviso, por mais downloads que o projeto tenha, porque nunca fica tempo suficiente no mesmo hash.
- **Arquivo assinado → reputação por certificado.** A reputação passa a ser atribuída ao **publisher** (ao
  certificado). Ela **acumula entre versões**: se você assinar sempre com o mesmo certificado, cada release novo
  já nasce herdando a reputação construída pelos anteriores.

Consequências práticas:

- Com certificado **OV**, o aviso **pode continuar aparecendo nos primeiros tempos**, até o certificado acumular
  reputação (volume de downloads sem incidentes). Quem promete "compra o OV e o aviso some no mesmo dia" está
  vendendo, não informando.
- Com certificado **EV**, a reputação normalmente já nasce estabelecida — é essa a diferença que se paga.
- **Trocar de certificado zera a reputação acumulada.** Como desde 2026 os certificados valem no máximo ~15 meses
  (ver seção 7), a renovação vira um evento recorrente: renove **com a mesma CA e mesma identidade jurídica**, que
  é o que preserva melhor a reputação.

---

## 6. Por que NÃO existe "assinar durante a instalação" — decisão de projeto

Aparece com frequência a ideia de "o instalador assina o programa na máquina do usuário". **Isso não existe.**
A assinatura Authenticode é aplicada pelo publisher, com a chave privada dele, **antes** da distribuição. As três
formas de tentar burlar isso são todas inaceitáveis, e nenhuma delas está implementada aqui:

**(a) Embutir a chave privada no instalador.**
A chave privada estaria no disco de qualquer pessoa que baixasse o programa — trivialmente extraível. Isso é, por
definição, **chave comprometida**. As regras da CA obrigam a revogação imediata do certificado ao primeiro
indício de exposição, e revogação invalida todas as assinaturas emitidas com ele. Resultado: você paga o
certificado, o perde em dias, e ainda entrega ao primeiro atacante a capacidade de assinar malware em seu nome.
Fora que, para certificados OV/EV emitidos desde **junho/2023**, **a chave privada nem sequer é exportável** (seção 7) —
não há o que embutir.

**(b) Gerar um certificado autoassinado (`New-SelfSignedCertificate`) e assinar com ele.**
Não é confiado por ninguém: não encadeia em nenhuma CA pública. O Windows continua bloqueando **exatamente
igual**, e ainda cria a falsa impressão de que o binário "está assinado". Este projeto já teve **duas**
implementações assim, ambas **removidas** justamente por serem enganosas: um passo "Auto-Sign" no
`.github/workflows/build-release.yml` e a função `Set-FzCodeSigning` do `install.ps1` (esta última também
incorria em **(c)**, abaixo). O workflow traz hoje um comentário registrando a remoção para que não seja
reintroduzida; o `install.ps1` foi depois removido do repositório por inteiro (a instalação no Windows passou a
ser exclusivamente pelo instalador gráfico Inno Setup). O histórico e o **procedimento de remediação para quem
executou a versão antiga do `install.ps1`** estão na [seção 10](#10-perguntas-frequentes).

**(c) Instalar uma CA raiz própria no repositório de confiança da máquina do usuário.**
Isto é **comportamento de malware**. É a técnica clássica para interceptar TLS e legitimar binários arbitrários,
é detectada por antivírus como tal, e altera a **configuração de segurança da máquina do usuário** — que não
pertence a este projeto. Além de ser eticamente indefensável, quebraria a confiança de qualquer auditoria.

> ### Decisão de projeto (vinculante)
> **Nenhuma das três abordagens acima será implementada no FzComputerAI.** Não em código, não no instalador, não
> em script auxiliar, não "temporariamente para testar". O `installer/fzcomputerai.iss` e o
> `scripts/sign-release.ps1` trazem essa mesma proibição escrita no cabeçalho, e o `sign-release.ps1` se **recusa
> a executar** quando encontra apenas certificados autoassinados no store, para não produzir um binário
> "assinado" que o Windows bloqueia do mesmo jeito.

O caminho legítimo é um só: obter um certificado de code signing de uma CA pública e assinar os artefatos
**antes** de publicá-los.

---

## 7. Opções reais de assinatura

Contexto obrigatório antes da tabela — dois fatos do setor que restringem tudo:

- **Chave em hardware é obrigatória.** Desde **junho/2023** (1º de junho de 2023, Baseline Requirements do
  CA/Browser Forum), certificados de code signing **OV e EV** só são
  emitidos com a chave privada em **token USB criptográfico ou HSM**. Não existe mais o `.pfx` simples que se
  copiava para qualquer máquina. Consequência direta: **não há como assinar em runner de CI** com um certificado
  OV/EV comprado depois dessa data — não existe arquivo para colocar em secret.
- **Validade curta.** O Ballot CSC-31 do CA/Browser Forum reduziu o teto de validade de 39 meses para **460 dias**
  (~15 meses), em vigor desde 1º de março de 2026; na prática as CAs emitem com **459 dias** para ficar dentro do
  teto (a DigiCert já desde fevereiro de 2026). Ou seja: **renovação anual passa a ser rotina**, não exceção.

### Tabela comparativa

| # | Opção | Custo aproximado | Requisito | Onde assina | Situação para o FzComputerAI |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | **Certificado OV em token USB** (CA brasileira ou revenda) | a partir de **~US$ 219/ano** | CNPJ + validação da empresa pela CA; token físico enviado pelo correio | máquina local com o token plugado | ✅ **Recomendado** |
| 2 | **Certificado EV em token USB** | mais caro que o OV (a diferença varia por CA) | validação mais rigorosa da empresa | máquina local com o token plugado | ⚪ Considerar se o aviso do OV incomodar |
| 3 | **HSM em nuvem / eSigner / KeyLocker** | mensalidade + custo por assinatura | contrato com a CA; certificado hospedado em HSM da CA | **no CI** (GitHub Actions) | ⚪ Viável, mais caro e mais complexo |
| 4 | **Azure Trusted Signing** (renomeado *Azure Artifact Signing*) | **US$ 9,99/mês** (plano básico) | elegibilidade **geográfica** da entidade | no CI | ❌ **Brasil não é elegível hoje** |
| 5 | **Não assinar e documentar o aviso** | US$ 0 | — | — | 🟡 **É o que está em vigor** |

### 1. Certificado OV em token USB — recomendado

É o caminho realista para uma empresa brasileira hoje.

- **O que é:** certificado *Organization Validation* emitido em nome da pessoa jurídica (Webstorage Tecnologia),
  após a CA validar CNPJ, endereço e existência da empresa.
- **Custo:** a partir de aproximadamente **US$ 219/ano** em revendas. O preço varia bastante por CA, por prazo
  contratado e pela inclusão ou não do token no pacote. **Confirme o preço vigente e o que está incluso no
  momento da compra** — inclusive frete e eventual imposto de importação do token.
- **Prazo:** conte com dias a semanas entre o pagamento e ter o token em mãos (validação da empresa + envio
  físico).
- **Requisitos técnicos na máquina que vai assinar:**
  - o **token USB plugado**;
  - o **middleware/driver do token** instalado (ex.: SafeNet Authentication Client, eToken PKI Client, YubiKey
    Minidriver — conforme a CA). Sem o middleware o Windows nem exibe o certificado no store;
  - o **`signtool.exe`** (componente *Windows SDK Signing Tools for Desktop Apps* do Windows SDK).
- **Como assinar:** `scripts/sign-release.ps1` — ver o passo a passo na [seção 8](#8-fluxo-de-release-assinando-localmente-com-token-usb).
- **Limitação a aceitar:** o aviso do SmartScreen pode persistir por um tempo, até o certificado acumular
  reputação (seção 5).

### 2. Certificado EV em token USB

Mesma mecânica do OV — token, middleware, assinatura local — com validação mais rigorosa da empresa e preço
maior. A vantagem concreta é a **reputação imediata** no SmartScreen: em geral não há período de "aquecimento".
Se o incômodo do aviso durante as primeiras semanas do OV for inaceitável para o público-alvo, é o EV que resolve
— não há atalho intermediário.

### 3. HSM em nuvem / eSigner / KeyLocker — assinar no CI

Serviços em que a CA hospeda a chave num HSM próprio e expõe uma API/ferramenta de assinatura, permitindo assinar
dentro do pipeline sem hardware local. Cobram mensalidade e, muitas vezes, por assinatura.

O workflow deste repositório **já está preparado** para o caso em que exista um material de chave utilizável no
CI. Os segredos suportados são:

| Segredo do repositório | Conteúdo |
| :--- | :--- |
| `WINDOWS_CERT_PFX_BASE64` | o `.pfx` codificado em Base64 |
| `WINDOWS_CERT_PASSWORD` | a senha do `.pfx` |

Quando `WINDOWS_CERT_PFX_BASE64` está presente, o workflow localiza o `signtool.exe`, decodifica o PFX num arquivo
temporário, **assina o `.exe` da GUI, depois compila o instalador e assina o instalador**, verifica ambos com
`signtool verify /pa` e **apaga o PFX temporário** ao final (passo com `if: always()`). Quando o segredo está
ausente, nada disso roda e o build apenas emite o aviso de "binários não assinados".

> ⚠️ **Ressalva importante:** esse caminho via `.pfx` **não** serve para um certificado OV/EV comprado depois de
> **junho/2023**, porque a chave desses certificados não é exportável (não existe `.pfx`). Ele existe para
> certificados antigos ainda válidos ou para serviços de HSM em nuvem que ofereçam um material equivalente.
> Se a solução de HSM contratada usar uma *action* ou CLI própria em vez de `.pfx`, o workflow precisará de um
> passo específico — isso **não** está implementado.

### 4. Azure Trusted Signing / Azure Artifact Signing — indisponível para o Brasil hoje

Seria, de longe, a opção mais barata: **US$ 9,99/mês** no plano básico, assinatura direto no CI, sem hardware, sem
gerenciar chave. O problema é elegibilidade.

O serviço da Microsoft valida a identidade apenas de entidades sediadas em um conjunto restrito de países. A
pesquisa feita para este documento (**julho/2026**, fontes na seção de Referências) confirmou a elegibilidade de
**EUA, Canadá, União Europeia e Reino Unido**, e **não** encontrou o Brasil em nenhuma lista de países elegíveis.
A Microsoft pode ter ampliado a lista desde então — **a lista vigente só vale consultada na documentação oficial
da Microsoft no momento da contratação**, e este documento não reproduz uma lista completa por não ter fonte
verificada para ela.

**O Brasil não aparece como elegível.** Portanto: **não é uma opção disponível para a Webstorage
Tecnologia hoje.** Fica registrada como **opção futura e condicional** — reavaliar se e quando a Microsoft
estender a elegibilidade ao Brasil; a alternativa de usar uma entidade jurídica em região elegível é uma decisão
societária, não técnica, e não é recomendada aqui.

O bloco correspondente está **comentado** no `.github/workflows/build-release.yml`, com essa mesma ressalva
escrita ao lado, pronto para ser habilitado se o cenário mudar.

### 5. Não assinar e documentar o aviso — o que está em vigor

É a situação atual, e é uma escolha defensável enquanto não houver certificado:

- publicar o `.sha256` de cada artefato (**já feito** pelo CI);
- explicar o aviso do SmartScreen nas notas do release (**já feito** — o corpo do release traz a instrução
  "Mais informações → Executar assim mesmo");
- explicar o mesmo aviso no README, com link para este documento (**já feito**);
- não fingir que instalador, autoassinatura ou qualquer truque resolvem (**este documento**).

---

## 8. Fluxo de release assinando localmente com token USB

Rodar **tudo na máquina onde o token está plugado**, a partir da raiz do repositório.
`<THUMB>` é o thumbprint SHA1 do certificado; para descobri-lo:
`Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert`.

```powershell
# 0) Ensaio (opcional, dispensa o token): mostra o que seria assinado e com qual comando
.\scripts\sign-release.ps1 -Path .\dist -WhatIf

# 1) Compilar a GUI
cargo build --release --manifest-path fzcomputerai\Cargo.toml

# 2) ASSINAR A GUI ANTES DE EMPACOTAR  <-- a ordem importa (ver nota abaixo)
.\scripts\sign-release.ps1 -Path fzcomputerai\target\release\fzcomputerai.exe -Thumbprint <THUMB>

# 3) Gerar o instalador (ele empacota o .exe JÁ ASSINADO do passo 2)
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DAppVersion=1.0.3 installer\fzcomputerai.iss
#    saída: .\dist\fzcomputerai-setup-windows-x64.exe

# 4) Colocar a GUI assinada também em .\dist e assinar tudo que estiver lá
#    (na prática, o instalador recém-gerado; re-assinar arquivo já assinado é inofensivo)
Copy-Item fzcomputerai\target\release\fzcomputerai.exe .\dist\fzcomputerai-windows-x64.exe
.\scripts\sign-release.ps1 -Path .\dist -Thumbprint <THUMB>

# 5) Gerar os checksums
Get-ChildItem .\dist\*.exe | ForEach-Object {
    "$((Get-FileHash $_ -Algorithm SHA256).Hash.ToLower())  $($_.Name)" |
        Set-Content "$($_.FullName).sha256" -Encoding ascii
}

# 6) Publicar — SÓ depois que o passo 4 sair com código 0
gh release upload vX.Y.Z .\dist\*.exe .\dist\*.sha256 --clobber
```

> ### Por que assinar a GUI **antes** de gerar o instalador
> O instalador **embute** o `.exe` da GUI. Se você assinar só o instalador, o binário que fica instalado na
> máquina do usuário permanece **sem assinatura** — e é ele que o SmartScreen e o antivírus avaliam no dia a dia,
> toda vez que o programa é aberto. Assinar o `.exe` primeiro e o instalador depois cobre os dois.
> (O `.github/workflows/build-release.yml` segue exatamente essa ordem no caminho automatizado.)

**Códigos de saída do `scripts/sign-release.ps1`:**

| Código | Significado |
| :-: | :--- |
| `0` | tudo assinado e verificado |
| `1` | pelo menos um arquivo falhou na assinatura ou na verificação — **não publique o release** |
| `2` | `signtool.exe` não encontrado (instale o Windows SDK) |
| `3` | nenhum certificado de code signing utilizável (token não plugado? middleware ausente? cert expirado?) |
| `4` | caminho inválido ou nenhum `.exe` encontrado |

O script também **verifica** cada arquivo com `signtool verify /pa /v` após assinar e imprime o SHA256 resultante,
então o passo 4 já dá o retorno de que o release está apto a ser publicado.

---

## 9. Carimbo de tempo (timestamp) é obrigatório

Assinar **sem** carimbo de tempo faz a assinatura **deixar de ser válida no dia em que o certificado expirar** —
e, com o teto de ~15 meses de validade (seção 7), isso significa que todo release antigo quebraria em pouco mais
de um ano. Com carimbo RFC 3161, a assinatura continua válida indefinidamente, porque fica provado que ela foi
feita enquanto o certificado ainda estava vigente.

Por isso o `scripts/sign-release.ps1` e o workflow **sempre** passam `/tr <url> /td SHA256` ao `signtool`, e o
script ainda **repete a tentativa** quando o servidor de carimbo falha (eles caem com alguma frequência).

Servidor padrão: `http://timestamp.digicert.com`.
Alternativas: `http://timestamp.sectigo.com`, `http://timestamp.globalsign.com/tsa/r6advanced1`.

```powershell
# Trocar o servidor de carimbo, se o padrão estiver fora do ar
.\scripts\sign-release.ps1 -Path .\dist -Thumbprint <THUMB> -TimestampUrl http://timestamp.sectigo.com
```

---

## 10. Perguntas frequentes

**"Se eu gerar um `.msi` em vez de `.exe`, o aviso some?"**
Não. `.msi` não assinado recebe o mesmo tratamento. O formato do pacote é irrelevante; o que conta é a assinatura.

**"Dá para pedir à Microsoft para liberar o binário?"**
Existe o formulário de *submissão de arquivo* do Microsoft Defender, útil para contestar um **falso positivo de
malware**. Ele não serve para "pular" o SmartScreen de um binário não assinado — a reputação continua sendo
construída pelos mecanismos da seção 5.

**"Comprar o certificado uma vez resolve para sempre?"**
Não. Desde 2026 a validade máxima é de ~15 meses (seção 7). É renovação recorrente, e é preciso renovar **com a
mesma CA e mesma identidade** para não zerar a reputação acumulada.

**"Posso assinar no GitHub Actions com o certificado do token?"**
Não. A chave não sai do token — é esse o ponto do token. Assinar no CI exige HSM em nuvem (opção 3) ou um serviço
como o Azure Artifact Signing (opção 4, indisponível para o Brasil hoje).

**"O instalador gráfico / `install.sh` contornam o aviso?"**
Não, e não deveriam. O `install.sh` (Linux/macOS) apenas verifica dependências, compila/instala os binários e
escreve a configuração MCP local; o instalador gráfico do Windows (`installer/fzcomputerai.iss`) copia arquivos,
cria atalhos e oferece a instalação do motor — nenhum dos dois toca em certificado, assina qualquer coisa ou
altera o repositório de confiança da máquina. O binário continua sendo o mesmo binário não assinado.
(O antigo `install.ps1` da raiz, citado nas perguntas seguintes, foi removido do repositório: a instalação no
Windows é exclusivamente pelo instalador gráfico.)

**"Mas eu li que o `install.ps1` assinava o binário. O que mudou?"**
Mudou, e é importante que quem rodou a versão antiga saiba disso. **Até a versão 1.0.2, inclusive**, o `install.ps1`
tinha uma função `Set-FzCodeSigning` que, na primeira execução:

1. gerava um certificado **auto-assinado** `CN=FzComputerAI (Webstorage Tecnologia)` em `Cert:\CurrentUser\My`;
2. **instalava esse certificado no repositório de Raiz Confiável do usuário** (`Cert:\CurrentUser\Root`);
3. assinava `fzcomputerai.exe` e `cua-driver.exe` com ele.

Isso era a prática **(b) + (c)** proibidas na [seção 6](#6-por-que-não-existe-assinar-durante-a-instalação--decisão-de-projeto):
não removia o aviso do SmartScreen (o certificado não encadeia em nenhuma CA pública, então o Windows bloqueia
igual), dava a falsa impressão de binário "assinado" e — o mais grave — **acrescentava uma âncora de confiança na
máquina do usuário final**. Uma raiz confiável a mais significa que qualquer coisa assinada com aquela chave passa
a ser tratada como confiável naquele computador. **A função foi removida** e, mais tarde, o próprio `install.ps1`
foi removido do repositório. A proibição de reintrodução vive hoje no `AGENTS.md` (seção 4) e nos comentários de
`installer/fzcomputerai.iss` e `.github/workflows/build-release.yml`.

#### Remediação — se você executou a versão antiga do `install.ps1`

Vale a pena checar mesmo em caso de dúvida; os comandos abaixo são somente de leitura até o `Remove-Item`.
Rode **no mesmo usuário do Windows** que executou o script (o certificado vai para o store do usuário, não da
máquina):

```powershell
# 1) Verificar — a raiz confiável do usuário é o que realmente importa
Get-ChildItem Cert:\CurrentUser\Root | Where-Object Subject -like '*FzComputerAI*' |
    Select-Object Subject, Thumbprint, NotAfter

# 2) Verificar também o store pessoal, onde ficam o certificado e a CHAVE PRIVADA
Get-ChildItem Cert:\CurrentUser\My | Where-Object Subject -like '*FzComputerAI*' |
    Select-Object Subject, Thumbprint, NotAfter

# 3) Remover de ambos (confira a saída dos passos 1 e 2 antes de rodar)
Get-ChildItem Cert:\CurrentUser\Root | Where-Object Subject -like '*FzComputerAI*' | Remove-Item
Get-ChildItem Cert:\CurrentUser\My   | Where-Object Subject -like '*FzComputerAI*' | Remove-Item
```

Se preferir a interface gráfica: `certmgr.msc` → **Autoridades de Certificação Raiz Confiáveis → Certificados**
(e depois **Pessoal → Certificados**), localize `FzComputerAI (Webstorage Tecnologia)`, clique com o botão direito
e escolha **Excluir**.

Notas:

- **Se os comandos não retornarem nada, não há o que remover** — a versão executada já era a corrigida, o
  certificado já foi removido antes, ou a escrita no store falhou (a função inteira era envolvida por um
  `try/catch` silencioso).
- **É comum aparecer em `My` e não em `Root`.** O script só escrevia na Raiz Confiável ao *gerar* um certificado
  novo; em execuções seguintes ele reaproveitava o que já estava em `My` sem tocar em `Root`. Encontrar o
  certificado só no store pessoal ainda justifica removê-lo: ele carrega a **chave privada** que assinava os
  binários.
- Remover o certificado **não desfaz a assinatura** já aplicada aos `.exe` locais. A assinatura simplesmente deixa
  de ser reconhecida, que é o comportamento correto. Para ficar com binários limpos, apague os executáveis e
  recompile (ou baixe de novo do release).
- **Confira o `Subject` antes de apagar.** Não remova certificados de outros emissores que apareçam na listagem —
  a Raiz Confiável do usuário pode conter itens legítimos de outros softwares.

**"E o antivírus, que às vezes bloqueia também?"**
É um problema correlato, mas distinto. Um programa de automação de desktop que injeta input (`SendInput`) e captura
tela tem, por natureza, comportamento parecido com o de ferramentas de acesso remoto — algumas soluções de AV
reagem a isso. Assinatura ajuda (dá autoria verificável e acumula reputação), mas não é garantia. Se ocorrer,
o caminho é submeter o arquivo como falso positivo ao fabricante do AV.

---

## 11. Arquivos relacionados neste repositório

| Arquivo | Papel |
| :--- | :--- |
| `scripts/sign-release.ps1` | Assinatura local com o token plugado + verificação. Recusa-se a assinar com certificado autoassinado. |
| `installer/fzcomputerai.iss` | Script do instalador (Inno Setup 6.3+). Cabeçalho documenta a mesma proibição descrita na seção 6. |
| `install.ps1` *(removido)* | Instalador de console para Windows que existia na raiz do repositório. Continha até a v1.0.2 a função `Set-FzCodeSigning`, removida por segurança — histórico e remediação na [seção 10](#10-perguntas-frequentes). O arquivo inteiro foi removido depois: a instalação no Windows é exclusivamente pelo instalador gráfico (`installer/fzcomputerai.iss`). |
| `install.sh` | Equivalente para Linux/macOS. Apenas baixa/compila e posiciona o binário; nunca assinou nada. |
| `.github/workflows/build-release.yml` | Build multiplataforma, geração do instalador, checksums, assinatura **condicional** e aviso quando não há certificado. O antigo step "Auto-Sign" (autoassinado) foi removido; bloco do Azure mantido comentado. |
| `AGENTS.md` | Regras normativas para agentes de IA, incluindo a proibição de reintroduzir autoassinatura ou instalar CA na máquina do usuário. Aponta para este documento como fonte da verdade. |
| `CHANGELOG.md` | Registro da remoção da autoassinatura e da correção da entrada da v1.0.2. |
| `README.md` / `README_EN.md` | Seção de instalação, com o aviso curto de SmartScreen apontando para este documento. |

---

## Referências

- [Understanding the New Code-Signing Certificate Validity Change — DigiCert](https://www.digicert.com/blog/understanding-the-new-code-signing-certificate-validity-change)
- [Moving to 459-day validity for public Code Signing certificates — DigiCert](https://knowledge.digicert.com/alerts/code-signing-certificates-459-day-validity)
- [Shorter validity periods for Code Signing certificates — Sectigo](https://www.sectigo.com/resource-library/shorter-validity-periods-for-code-signing-certificates)
- [Code signing options for Windows app developers — Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
- [Quickstart: Set up Artifact Signing — Microsoft Learn](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)
- [Trusted Signing: disponibilidade fora de EUA/Canadá — Azure/artifact-signing-action, issue #81](https://github.com/Azure/artifact-signing-action/issues/81)

> **Sobre preços e elegibilidade:** os valores e as listas de países citados aqui refletem a pesquisa feita em
> **julho de 2026** e mudam com frequência. **Confirme diretamente com a CA ou com a Microsoft antes de comprar.**

---

<div align="center">

**FzComputerAI** — Roger Luft / Webstorage Tecnologia
[www.webstorage.com.br](https://www.webstorage.com.br)

</div>

---

## 12. Registro de falsos positivos

| Data | Arquivo | Detecção | Contexto | Resultado |
| :--- | :--- | :--- | :--- | :--- |
| 2026-09-03 | `fzcomputerai-setup-windows-x64.exe` compilado **localmente** (v2.3.3, `%TEMP%\fz-v233`) | `Trojan:Win32/Bearfoos.B!ml` (Defender, quarentena) | binário sem assinatura, gerado minutos antes, sem prevalência. `.iss` e workflow idênticos à v2.2.0; o payload ganhou listener TLS, ACME, chamadas HTTPS à API do Cloudflare/DoH e OAuth/OIDC | liberado manualmente pelo usuário; instalador da release v2.3.2 no GitHub também sem Authenticode |

O sufixo `!ml` é classificação heurística por aprendizado de máquina. O caminho que resolve é o da seção 8: assinar o GUI **antes** de empacotar, assinar o instalador, carimbo de tempo, e submeter o arquivo em <https://www.microsoft.com/wdsi/filesubmission> como falso positivo (a reavaliação costuma sair em 1–2 dias).

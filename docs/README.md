# Documentação do FzComputerAI

Para quem vai instalar, operar ou desenvolver o FzComputerAI — a interface gráfica nativa que gerencia o motor `cua-driver`.

## O que é (resumo de uma linha)

`fzcomputerai` é uma GUI nativa em Rust (egui/eframe) que **inicia, para, configura, diagnostica e expõe** o motor `cua-driver` do projeto [Cua](https://github.com/trycua/cua). A GUI **não é** o motor: sem o `cua-driver` instalado e no PATH, os botões não têm o que executar.

## Índice

| Documento | Serve para |
| --- | --- |
| [arquitetura.md](arquitetura.md) | Entender a separação GUI x motor, o transporte MCP (stdio e HTTP), o caminho de uma ação do clique até o console, onde vive o estado e o princípio de status honesto. |
| [uso-mcp-rede.md](uso-mcp-rede.md) | Operar a aba **MCP & Rede**: iniciar/parar/reiniciar o motor, aplicar a porta, testar o endpoint, autostart, ler o diagnóstico cru e resolver o badge "REGRA SEM EFEITO". |
| [uso-tunel.md](uso-tunel.md) | Operar a aba **Túnel**: escolher o provedor (Cloudflare, ngrok, SSH reverso), proteger a URL com senha, verificar a exposição real e entender a limpeza automática. |
| [acesso-remoto.md](acesso-remoto.md) | Decidir **como** sair do loopback: LAN por encaminhamento, internet por túnel ou VPN — e por que **não existe** bind `0.0.0.0` no motor oficial. |
| [atualizacao.md](atualizacao.md) | Usar a Central de Atualizações: os dois componentes (GUI e motor), o que é verificado em cada um e o que acontece durante a instalação. |
| [solucao-de-problemas.md](solucao-de-problemas.md) | Ir de sintoma a correção: MCP "parado" com o motor rodando, listener só em loopback, portproxy sem efeito, túnel com 404/502, `cua-driver` fora do PATH, SmartScreen. |
| [desenvolvimento.md](desenvolvimento.md) | Compilar, navegar no código, seguir as convenções obrigatórias do projeto, gerar o ícone e montar o instalador localmente. |
| [faq.md](faq.md) | Respostas curtas às perguntas que aparecem sempre: preciso do motor? é seguro? por que o MCP cai ao fechar o app? |

## Convenções desta documentação

- Onde o comportamento **depende da versão do motor**, isso está dito no texto. As duas gerações relevantes são: `<= 0.8.x` (sem autenticação no endpoint HTTP) e `>= 0.16` (token obrigatório). O contrato do token foi **medido no binário 0.17.0 em 2026-08-03**, e não é herdado de documentação anterior: sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente, `cua-driver serve` **nem sobe**; com o daemon no ar, requisição sem `Authorization: Bearer` recebe **401** (`-32001 Authentication required`). Detalhes em [arquitetura.md](arquitetura.md) e [atualizacao.md](atualizacao.md).
- Caminhos de código aparecem relativos à raiz do repositório, por exemplo `fzcomputerai/src/app.rs`.
- Nada aqui afirma que algo "é seguro" sem dizer **sob qual condição**.

## Licença e crédito

O FzComputerAI é MIT — `Copyright (c) 2026 Roger Luft (VeilWalker) — Webstorage Tecnologia`. O motor `cua-driver` é MIT de **Cua AI, Inc.** e é instalado à parte, pelo instalador oficial do Cua. O texto completo das duas licenças, a citação formal e o agradecimento estão em [`LICENSE.md`](../LICENSE.md) na raiz do repositório.

# Documentação do FzComputerAI

Para quem vai instalar, operar ou desenvolver o FzComputerAI — a interface gráfica nativa que gerencia o motor `cua-driver`.

## O que é (resumo de uma linha)

`fzcomputerai` é uma GUI nativa em Rust (egui/eframe) que **inicia, para, configura, diagnostica e expõe** o motor `cua-driver` do projeto [Cua](https://github.com/trycua/cua). A GUI **não é** o motor: sem o `cua-driver` instalado e no PATH, os botões não têm o que executar. Desde a 2.3.0 ela também é **dona do ciclo de vida**: sobe o motor como processo filho e o encerra ao fechar — ver [arquitetura.md](arquitetura.md).

## Índice

| Documento | Serve para |
| --- | --- |
| [arquitetura.md](arquitetura.md) | Entender a separação GUI x motor, o transporte MCP (stdio e HTTP), o ciclo de vida dos filhos pelo Job Object, o relay da LAN, o executor de segundo plano, onde vive o estado e o princípio de status honesto. |
| [uso-mcp-rede.md](uso-mcp-rede.md) | Operar a aba **MCP & Rede**: iniciar/parar/reiniciar o motor, aplicar a porta, testar o endpoint, publicar na rede local, autostart e ler o diagnóstico cru. |
| [uso-tunel.md](uso-tunel.md) | Operar a aba **Túnel**: escolher o provedor (Cloudflare quick ou nomeado com domínio próprio, ngrok, SSH reverso), gerar e ativar o token do motor (`0.16+`), proteger a URL com senha, verificar a exposição real com a sonda em 2 fases (sem credencial e com Bearer) e entender a limpeza automática. |
| [acesso-remoto.md](acesso-remoto.md) | Decidir **como** sair do loopback: LAN pelo relay do app, internet por túnel (inclusive com URL fixa no seu domínio) ou VPN — e por que **não existe** bind `0.0.0.0` no motor oficial. |
| [atualizacao.md](atualizacao.md) | Usar a Central de Atualizações: os dois componentes (GUI e motor), o que é verificado em cada um e o que acontece durante a instalação. |
| [solucao-de-problemas.md](solucao-de-problemas.md) | Ir de sintoma a correção: MCP "parado" com o motor rodando, janela "(Não Respondendo)", listener só em loopback, túnel com 404/502, cliente que não aceita header, DNS que não foi criado, `cua-driver` fora do PATH, SmartScreen. |
| [desenvolvimento.md](desenvolvimento.md) | Compilar, navegar no código, seguir as convenções obrigatórias do projeto, gerar o ícone e montar o instalador localmente. |
| [faq.md](faq.md) | Respostas curtas às perguntas que aparecem sempre: preciso do motor? preciso deixar o app aberto? é seguro? como uso meu domínio? |

Fora de `docs/`: `scripts/remote-teste.py` prova o endpoint **de outra rede**, só com a biblioteca padrão do Python 3 — `initialize`, `tools/list` e uma tarefa real de ponta a ponta na máquina remota. Uso e limites em [acesso-remoto.md](acesso-remoto.md) e [solucao-de-problemas.md](solucao-de-problemas.md).

## Convenções desta documentação

- Onde o comportamento **depende da versão do motor**, isso está dito no texto. As duas gerações relevantes são: `<= 0.8.x` (sem autenticação no endpoint HTTP) e `>= 0.16` (token obrigatório).
- Caminhos de código aparecem relativos à raiz do repositório, por exemplo `fzcomputerai/src/app.rs`.
- Nada aqui afirma que algo "é seguro" sem dizer **sob qual condição**.

## Licença e crédito

O FzComputerAI é MIT — `Copyright (c) 2026 Roger Luft (VeilWalker) — Webstorage Tecnologia`. O motor `cua-driver` é MIT de **Cua AI, Inc.** e é instalado à parte, pelo instalador oficial do Cua. O texto completo das duas licenças, a citação formal e o agradecimento estão em [`LICENSE.md`](../LICENSE.md) na raiz do repositório.

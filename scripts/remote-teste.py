#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Teste REMOTO do FzComputerAI MCP — rode de FORA da rede.

O que ele faz, de ponta a ponta, contra a máquina do outro lado do túnel:
  1. initialize            — abre a sessão MCP e mostra a versão do motor
  2. tools/list            — confirma que as ferramentas chegaram
  3. abre o navegador      — launch_app / browser_navigate
  4. vai para o Yahoo      — https://search.yahoo.com
  5. digita "Roger Luft"   — no campo de busca
  6. clica no botão        — descobre o rótulo real (Search/Pesquisar/Buscar...)
  7. confere o resultado   — lê a tela de volta e diz se a busca aconteecu

Uso:
    python remote-teste.py <URL> [--token TOKEN] [--termo TEXTO]

    URL    endpoint MCP completo, ex.:
           https://mcphome.rogerluft.com.br/mcp
           https://algo.trycloudflare.com/mcp
           http://192.168.0.101:8000/mcp          (dentro da LAN)

    --token  Bearer do motor (CUA_DRIVER_RS_MCP_HTTP_TOKEN). Pode vir também
             da variável de ambiente FZ_MCP_TOKEN. Se a URL tiver senha no
             caminho (/s/<senha>/mcp), o porteiro injeta o token sozinho e
             este parâmetro não é necessário.

Só usa a biblioteca padrão do Python 3 — nada para instalar.
"""

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request

TIMEOUT = 60

# O motor responde com emoji ("✅ Main display: ...") e o console do Windows
# costuma estar em cp1252 — sem isto, imprimir a resposta estoura
# UnicodeEncodeError e o teste "falha" sem ter falhado. Medido na prática.
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


class Mcp:
    """Cliente MCP mínimo sobre HTTP JSON-RPC (o transporte do cua-driver)."""

    def __init__(self, url, token=None):
        self.url = url
        self.token = token
        self.n = 0

    def call(self, method, params=None):
        self.n += 1
        payload = {"jsonrpc": "2.0", "id": self.n, "method": method, "params": params or {}}
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(self.url, data=data, method="POST")
        req.add_header("Content-Type", "application/json")
        req.add_header("Accept", "application/json, text/event-stream")
        # ngrok mostra um interstitial para navegadores; este header o dispensa.
        req.add_header("ngrok-skip-browser-warning", "1")
        if self.token:
            req.add_header("Authorization", "Bearer " + self.token)
        try:
            with urllib.request.urlopen(req, timeout=TIMEOUT) as r:
                return json.loads(r.read().decode("utf-8", "replace"))
        except urllib.error.HTTPError as e:
            body = e.read().decode("utf-8", "replace")
            if e.code == 401:
                raise SystemExit(
                    "\n[401] O endpoint exigiu credencial e a que foi enviada não serve.\n"
                    "      - Motores cua-driver 0.16+ exigem 'Authorization: Bearer <token>'.\n"
                    "      - Passe --token, ou use a URL com senha (/s/<senha>/mcp), em que\n"
                    "        o porteiro do app injeta o token por você.\n"
                    "      Resposta do servidor: " + body[:200]
                )
            raise SystemExit("[HTTP %s] %s" % (e.code, body[:400]))
        except urllib.error.URLError as e:
            raise SystemExit(
                "Não consegui falar com %s\n  Motivo: %s\n"
                "  Verifique se o túnel está ativo na outra ponta." % (self.url, e.reason)
            )

    def tool(self, name, args=None):
        """Executa uma ferramenta e devolve (ok, texto, estruturado)."""
        r = self.call("tools/call", {"name": name, "arguments": args or {}})
        if "error" in r:
            return False, str(r["error"]), None
        res = r.get("result", {})
        text = " ".join(
            c.get("text", "") for c in res.get("content", []) if isinstance(c, dict)
        )
        return not res.get("isError", False), text, res.get("structuredContent")


def passo(n, titulo):
    print("\n[%d] %s" % (n, titulo))


def achar_botao_busca(elems):
    """Procura o botão de pesquisa por rótulo, em vários idiomas."""
    alvos = ("search", "pesquisar", "buscar", "buscá", "pesquisa", "submit")
    for e in elems:
        papel = (e.get("role") or "").lower()
        rotulo = (e.get("label") or "").lower()
        if not rotulo:
            continue
        if papel in ("button", "menuitem") or "button" in papel:
            if any(a in rotulo for a in alvos):
                return e
    return None


def main():
    ap = argparse.ArgumentParser(
        description="Teste remoto do FzComputerAI MCP (rode de FORA da rede)",
        epilog="Exemplo: python remote-teste.py https://mcphome.rogerluft.com.br/s/SENHA/mcp",
    )
    ap.add_argument("url", help="endpoint MCP, ex.: https://host/mcp")
    ap.add_argument("--token", default=os.environ.get("FZ_MCP_TOKEN"), help="Bearer do motor")
    ap.add_argument("--termo", default="Roger Luft", help='o que pesquisar (padrão: "Roger Luft")')
    ap.add_argument(
        "--no-browser-tools",
        action="store_true",
        help="não usar as ferramentas browser_*; dirigir o navegador só por teclado/mouse",
    )
    a = ap.parse_args()

    m = Mcp(a.url, a.token)
    print("Endpoint: %s" % a.url)
    print("Credencial: %s" % ("Bearer informado" if a.token else "nenhuma (URL com senha ou motor aberto)"))

    passo(1, "initialize")
    r = m.call(
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "teste-remoto-fzcomputerai", "version": "1"},
        },
    )
    if "error" in r:
        raise SystemExit("initialize falhou: %s" % r["error"])
    info = r.get("result", {}).get("serverInfo", {})
    print("    OK — servidor: %s %s" % (info.get("name", "cua-driver"), info.get("version", "")))

    passo(2, "tools/list")
    r = m.call("tools/list")
    tools = [t["name"] for t in r.get("result", {}).get("tools", [])]
    print("    OK — %d ferramentas. Ex.: %s" % (len(tools), ", ".join(tools[:6])))
    tem_browser = ("browser_navigate" in tools) and not a.no_browser_tools

    passo(3, "abrindo uma janela NOVA de navegador na máquina remota")
    # Fotografa as janelas ANTES para não sequestrar uma janela que já estava
    # aberta (o dono da máquina pode estar usando o navegador agora).
    _, _, st_antes = m.tool("list_windows", {})
    antes = {
        w.get("window_id")
        for w in ((st_antes or {}).get("windows", []) if st_antes else [])
    }
    ok, txt, _ = m.tool("launch_app", {"app": "chrome"})
    print("    launch_app chrome: %s" % ("OK" if ok else txt[:120]))
    time.sleep(7)

    ok, txt, st = m.tool("list_windows", {})
    janelas = (st or {}).get("windows", []) if st else []
    navegadores = [
        w
        for w in janelas
        if any(n in (w.get("app_name") or "").lower() for n in ("chrome", "msedge", "firefox"))
    ]
    # Preferência absoluta: janela que NÃO existia antes.
    alvo = next((w for w in navegadores if w.get("window_id") not in antes), None)
    if not alvo and navegadores:
        print("    AVISO: nenhuma janela nova apareceu; usando a existente (pode interromper quem usa a máquina).")
        alvo = navegadores[0]
    if not alvo:
        raise SystemExit("Não achei uma janela de navegador na máquina remota. Saída: %s" % txt[:200])
    pid, wid = alvo["pid"], alvo["window_id"]
    print("    janela: pid=%s '%s'" % (pid, (alvo.get("title") or "")[:60]))

    passo(4, "navegando para o Yahoo")
    if tem_browser:
        ok, txt, _ = m.tool("browser_prepare", {"pid": pid})
        print("    browser_prepare: %s" % ("OK" if ok else txt[:120]))
        ok, txt, _ = m.tool("browser_navigate", {"pid": pid, "url": "https://search.yahoo.com"})
        print("    browser_navigate: %s" % ("OK" if ok else txt[:160]))
    else:
        m.tool("bring_to_front", {"pid": pid, "window_id": wid})
        m.tool("hotkey", {"pid": pid, "keys": "ctrl+l"})
        m.tool("type_text", {"pid": pid, "text": "https://search.yahoo.com\n", "delivery_mode": "foreground"})
        print("    navegação por barra de endereços enviada")
    time.sleep(7)

    passo(5, 'digitando "%s"' % a.termo)
    escreveu = False
    if tem_browser:
        ok, txt, _ = m.tool(
            "browser_type",
            {"pid": pid, "selector": "input[name='p']", "text": a.termo},
        )
        escreveu = ok
        print("    browser_type: %s" % ("OK" if ok else txt[:160]))
    if not escreveu:
        # Caminho universal: clica no campo de busca visto na árvore e digita.
        ok, txt, st = m.tool("get_window_state", {"pid": pid, "window_id": wid})
        elems = (st or {}).get("elements", []) if st else []
        campo = next(
            (
                e
                for e in elems
                if (e.get("role") or "").lower() in ("edit", "textbox", "searchbox", "combobox")
                and "endere" not in (e.get("label") or "").lower()
                and "address" not in (e.get("label") or "").lower()
            ),
            None,
        )
        if campo:
            m.tool("click", {"pid": pid, "window_id": wid, "element_index": campo["element_index"],
                             "snapshot_id": (st or {}).get("snapshot_id")})
        m.tool("type_text", {"pid": pid, "text": a.termo, "delivery_mode": "foreground"})
        print("    texto digitado pelo caminho de teclado")

    passo(6, "clicando no botão de pesquisa")
    clicou = False
    ok, txt, st = m.tool("get_window_state", {"pid": pid, "window_id": wid})
    elems = (st or {}).get("elements", []) if st else []
    botao = achar_botao_busca(elems)
    if botao:
        print("    botão encontrado: '%s'" % botao.get("label"))
        ok, txt, _ = m.tool(
            "click",
            {
                "pid": pid,
                "window_id": wid,
                "element_index": botao["element_index"],
                "snapshot_id": (st or {}).get("snapshot_id"),
                "delivery_mode": "foreground",
            },
        )
        clicou = ok
    if not clicou:
        print("    sem botão na árvore — enviando Enter (equivale a clicar em pesquisar)")
        m.tool("press_key", {"pid": pid, "key": "enter", "delivery_mode": "foreground"})
    time.sleep(7)

    passo(7, "conferindo o resultado")
    ok, txt, st = m.tool("get_window_state", {"pid": pid, "window_id": wid})
    titulo = ((st or {}).get("title") or "") if st else ""
    if not titulo:
        ok2, _, st2 = m.tool("list_windows", {})
        for w in ((st2 or {}).get("windows", []) if st2 else []):
            if w.get("pid") == pid:
                titulo = w.get("title") or ""
                break
    print("    título da janela: %s" % (titulo[:90] or "(vazio)"))
    termo_ok = a.termo.split()[0].lower() in titulo.lower()
    yahoo_ok = "yahoo" in titulo.lower()

    print("\n" + "=" * 62)
    if termo_ok and yahoo_ok:
        print("RESULTADO: SUCESSO — a busca por '%s' aconteceu no Yahoo." % a.termo)
        rc = 0
    elif yahoo_ok:
        print("RESULTADO: PARCIAL — o Yahoo abriu, mas o termo não apareceu no título.")
        print("           Rode de novo; se repetir, veja a tela remota para conferir.")
        rc = 2
    else:
        print("RESULTADO: FALHOU — não cheguei à página de resultados.")
        print("           O MCP respondeu (passos 1 e 2 OK), então o túnel está bom;")
        print("           o problema está na automação do navegador remoto.")
        rc = 1
    print("=" * 62)
    sys.exit(rc)


if __name__ == "__main__":
    main()

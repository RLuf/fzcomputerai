# License

## FzComputerAI (GUI, integrações e documentação deste repositório)

MIT License

Copyright (c) 2026 Roger Luft (VeilWalker) — Webstorage Tecnologia

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Third-party components / Componentes de terceiros

### Cua (`trycua/cua`) — o motor `cua-driver`

O motor de automação e visão computacional **`cua-driver`** é parte do projeto
open-source **Cua**, desenvolvido e mantido por **Cua AI, Inc.** (equipe
[cua.ai](https://cua.ai)). O FzComputerAI é uma **interface gráfica
independente** construída *sobre* esse motor — ele não substitui, não modifica
e não redistribui o binário oficial do `cua-driver` (a instalação do motor é
opcional e feita pelo instalador oficial do próprio projeto Cua).

O projeto Cua é licenciado sob a **MIT License**, reproduzida integralmente
abaixo conforme exigido pela própria licença:

```
MIT License

Copyright (c) 2025 Cua AI, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Repositório oficial: <https://github.com/trycua/cua>
Documentação: <https://cua.ai/docs> · Comunidade: <https://discord.gg/mVnXXpdE85>

> **Nota:** o projeto Cua inclui, por sua vez, componentes de terceiros com
> licenças próprias (por exemplo Kasm sob MIT, OmniParser sob CC-BY-4.0 e
> ultralytics sob AGPL-3.0). Consulte o repositório oficial deles para os
> termos aplicáveis a cada componente.

#### Citação / Citation

Conforme solicitado pelos autores do Cua:

```bibtex
@software{cua2025,
  author  = {{Cua AI, Inc.}},
  title   = {Cua},
  year    = {2025},
  url     = {https://github.com/trycua/cua},
  license = {MIT}
}
```

### Bibliotecas Rust

A GUI usa `egui`/`eframe` (licenciados sob MIT OR Apache-2.0) e outras crates
do ecossistema Rust, cada uma sob a sua própria licença. Os termos de cada
dependência ficam disponíveis em `fzcomputerai/Cargo.toml` e no registro
crates.io correspondente.

### Ferramentas de túnel (opcionais, baixadas sob demanda)

`cloudflared` (Cloudflare, Apache-2.0) e `ngrok` (ngrok Inc., proprietário) e o
cliente OpenSSH do Windows **não são redistribuídos** por este projeto. Quando
usados pela aba Túnel, são obtidos das fontes oficiais na máquina do usuário,
por ação explícita dele, e permanecem sujeitos aos termos dos respectivos
fornecedores.

---

## Agradecimentos / Acknowledgements

Agradecimento sincero à **Cua AI, Inc.** e a toda a equipe e comunidade do
projeto [**Cua**](https://github.com/trycua/cua): o motor `cua-driver` é a base
sobre a qual o FzComputerAI foi construído, e a decisão deles de publicá-lo
como software livre sob MIT é o que tornou este trabalho possível. Obrigado.

Our sincere thanks to **Cua AI, Inc.** and the entire team and community behind
the [**Cua**](https://github.com/trycua/cua) project. The `cua-driver` engine is
the foundation FzComputerAI is built upon, and their choice to release it as
open source under the MIT License is what made this work possible. Thank you.

---

**FzComputerAI** — Roger Luft (VeilWalker) / Webstorage Tecnologia
<https://www.webstorage.com.br>

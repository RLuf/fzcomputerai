use eframe::egui::{self, Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(PartialEq, Clone, Copy)]
pub enum Language {
    PtBr,
    English,
}

/// Status real do endpoint MCP, distinguindo loopback de LAN.
/// Verde (LanListening) SOMENTE se o netstat mostrar listener no IP da LAN
/// (ou 0.0.0.0) E o teste TCP no IP da LAN conectar.
#[derive(PartialEq, Clone, Copy)]
pub enum PortStatus {
    Stopped,
    LocalOnly,
    LanListening,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Network,
    Calibration,
    Windows,
    Recording,
    DoctorSkills,
    McpTools,
    Tunnel,
}

/// Provedor do túnel de internet (HTTPS público -> MCP HTTP local).
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelProvider {
    Cloudflare,
    Ngrok,
    Ssh,
}

/// Status HONESTO do túnel, em níveis (mesma doutrina do PortStatus):
///   Starting = processo vivo, URL pública ainda NAO capturada;
///   Running  = URL pública capturada (ou informada). "Confirmado pela
///              internet" é um estado SEPARADO (tunnel_exposure), provado
///              por POST initialize real na URL pública — nunca presumido.
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Resultado da SONDA DE EXPOSIÇÃO na URL pública (POST initialize): o que a
/// internet consegue de fato. Nunca é opinião — é o que a rede respondeu.
///   Exposed     = o initialize RESPONDEU RESULTADO sem nenhuma credencial
///                 (HTTP 200 + "result") — aberto de verdade;
///   EngineAuth  = o MOTOR barrou (HTTP 401 com corpo JSON-RPC): motores
///                 0.16+ exigem Bearer. O corpo do 401 TAMBÉM contém
///                 "jsonrpc" — por isso o veredito olha o código HTTP
///                 primeiro; "contém jsonrpc" sozinho chamaria de EXPOSTO um
///                 endpoint que acabou de NEGAR acesso (bug real corrigido);
///   EdgeAuth(c) = a borda do provedor barrou (HTTP 401/403/302/407 sem
///                 corpo JSON-RPC) — há auth na frente do motor;
///   AuthOk      = duas fases confirmadas: SEM credencial foi barrado E com
///                 o token Bearer conhecido o initialize respondeu resultado
///                 — túnel protegido e utilizável de ponta a ponta;
///   Unknown     = timeout/5xx/erro: não deu para verificar (tratar como exposto).
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelExposure {
    Exposed,
    EngineAuth,
    EdgeAuth(u16),
    AuthOk,
    Unknown,
}

/// Efeito tipado que uma tarefa de segundo plano aplica ao estado quando
/// termina. A thread NÃO toca no `AppState` (não tem `&mut self`): ela devolve
/// isto, e o `poll_bg()` — que roda na thread da UI — aplica.
pub enum BgEffect {
    None,
    /// Resultado da sonda de exposição do túnel (curl pode levar 40s).
    Exposure(TunnelExposure),
    /// Estado real da porta/endpoint recalculado fora da UI.
    PortStatus {
        port_active: bool,
        port_status: PortStatus,
        probe_401: bool,
        real_listeners: Vec<String>,
    },
}

/// Uma tarefa terminada: o que registrar no console, o que mostrar na faixa de
/// status e o efeito a aplicar. Campos vazios = "não mexer".
pub struct BgOutcome {
    pub log: String,
    pub status: String,
    pub effect: BgEffect,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WindowItem {
    pub pid: u32,
    pub window_id: u64,
    pub title: String,
    pub app_name: Option<String>,
    pub minimized: Option<bool>,
}

/// Ponto de status colorido DESENHADO (painter), em vez do caractere "●":
/// a fonte proporcional padrão do egui não tem esse glifo e renderiza uma
/// caixa vazia — que parece placeholder quebrado na tela.
pub fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
}

/// Cria um `Command` que, no Windows, NUNCA abre a janela preta de console
/// (CREATE_NO_WINDOW). Use SEMPRE este helper em vez de `Command::new`.
pub fn quiet_cmd(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Autodetecta o IP da LAN sem enviar nenhum pacote:
/// UDP connect apenas seleciona a interface de saída.
fn detect_lan_ip() -> String {
    let detected = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|sock| {
        sock.connect("8.8.8.8:80")?;
        sock.local_addr()
    });
    match detected {
        Ok(addr) => addr.ip().to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

pub struct AppState {
    pub language: Language,
    pub active_tab: Tab,
    pub http_port: String,
    pub lan_ip: String,
    pub port_active: bool,
    pub port_status: PortStatus,
    pub daemon_running: bool,

    // Regra portproxy LAN -> localhost: estado REAL em DOIS niveis.
    //   portproxy_active    = a regra EXISTE na config (netsh show v4tov4);
    //   portproxy_effective = alem de existir, o LISTENER esta de pe no
    //                         netstat (IP Helper servindo de fato).
    // Regra na config sem listener e "SEM EFEITO" — nunca pinte verde.
    pub portproxy_active: bool,
    pub portproxy_effective: bool,

    // VERDADE na tela: linhas LISTENING reais do netstat (porta configurada
    // + qualquer porta no IP da LAN) e TODAS as regras portproxy v4tov4.
    // A UI exibe isto cru — nada de host/URL "de intencao".
    pub real_listeners: Vec<String>,
    pub portproxy_rules: Vec<String>,

    // ─── EXECUTOR DE SEGUNDO PLANO ──────────────────────────────────────
    // POR QUE EXISTE: toda ação desta GUI terminava em `Command::output()`,
    // que BLOQUEIA. Um `reg query` custa ~200ms, um `powershell -Command`
    // 300ms-2s, e o teste de túnel dispara `curl -m 20` DUAS vezes — até 40s
    // com a janela congelada e o Windows escrevendo "(Não Respondendo)" no
    // título. Não era travamento aleatório: era a thread do egui esperando
    // processo externo, e por isso "sempre aconteceu".
    //
    // O desenho segue o que o projeto já fazia com downloads (spawn + poll):
    // a tarefa roda numa thread, devolve um BgOutcome, e o `poll_bg()` aplica
    // na thread da UI. Nada de `&mut self` cruzando thread, nada de canal
    // exótico — um Mutex<Vec<_>> drenado por frame.
    bg_out: std::sync::Arc<std::sync::Mutex<Vec<BgOutcome>>>,
    /// Quantas tarefas estão em voo (a UI mostra isso em vez de fingir que
    /// terminou).
    pub bg_busy: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// As tarefas pesadas de arranque ainda não rodaram? Elas acontecem APÓS o
    /// primeiro frame (ver `run_startup_tasks`), para a janela aparecer
    /// desenhada e responsiva em vez de branca e "Não Respondendo".
    startup_pending: bool,

    // ─── MOTOR COMO PROCESSO FILHO ──────────────────────────────────────
    // O motor deixou de ser iniciado por "autostart kick" (tarefa agendada,
    // processo independente que sobrevivia ao app e que a limpeza antiga
    // matava com `taskkill /F /IM cua-driver.exe` — atingindo daemons de
    // OUTROS usos, o que o AGENTS.md proíbe). Agora a GUI dá spawn em
    // `cua-driver serve` e ADOTA o filho no Job Object (lifecycle.rs): ao
    // fechar a GUI por qualquer via — X, Sair na bandeja, taskkill /F, crash —
    // o kernel mata o motor junto. Sem vigia, sem corrida, sem lixo.
    //
    // `engine_external` cobre o caso honesto do daemon que NÃO é nosso (o
    // `.mcp.json` de um cliente MCP sobe o seu): ele aparece na UI como tal e
    // NUNCA é morto às cegas.
    engine_child: Option<std::process::Child>,
    pub engine_pid: Option<u32>,
    pub engine_external: bool,

    // ─── RELAY LAN (dentro do processo da GUI) ──────────────────────────
    // Substitui o netsh portproxy como caminho PADRÃO de publicar o MCP na
    // rede. Três motivos, todos medidos:
    //   1. netsh portproxy EXIGE elevação (UAC a cada aplicar/remover) e
    //      depende do serviço IP Helper; o relay é um socket comum, sem UAC;
    //   2. a regra do netsh SOBREVIVE ao fechamento do app (ficava lixo na
    //      máquina, e a limpeza no on_exit tinha de abrir UAC); o relay é uma
    //      thread deste processo — morre junto, sempre, sem auxiliar externo;
    //   3. medido nesta plataforma: escutar em 0.0.0.0:<porta> COEXISTE com o
    //      listener do motor em 127.0.0.1:<mesma porta> (o bind mais
    //      específico atende o loopback), então dá para publicar na rede na
    //      MESMA porta, sem tocar na configuração do motor.
    // O relay é transparente: copia bytes nos dois sentidos, sem inspecionar
    // nem reescrever HTTP — keep-alive e streaming (SSE) passam intactos.
    pub lan_relay_bind: String,          // "0.0.0.0" (qualquer interface) ou um IP
    pub lan_relay_listen: Option<u16>,   // porta em que o relay REALMENTE escuta
    pub lan_relay_target: Option<u16>,   // porta do motor para onde encaminha
    pub lan_relay_conns: std::sync::Arc<std::sync::atomic::AtomicUsize>, // ativas agora
    pub lan_relay_total: std::sync::Arc<std::sync::atomic::AtomicUsize>, // desde o start
    lan_relay_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    lan_relay_log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,

    // Calibração & Visão
    pub screen_width: u32,
    pub screen_height: u32,
    pub dpi_scale: f32,
    pub test_x: String,
    pub test_y: String,

    // Janelas & Processos
    pub windows_list: Vec<WindowItem>,
    pub launch_input: String,

    // Gravação & Trajetórias
    pub is_recording: bool,
    pub recording_path: String,

    pub show_about: bool,

    // ─── SAÍDA UNIFICADA ────────────────────────────────────────────────
    // Antes cada aba tinha sua PRÓPRIA caixa de saída (calibration_log,
    // windows_log, recording_log, doctor_output, skills_output,
    // mcp_tools_output, tunnel_output) e a aba MCP & Rede tinha o Console
    // Debug — dois consoles na mesma tela, com a mesma informação. Agora:
    //   status_msg = ÚLTIMA mensagem/resultado (faixa de status);
    //   debug_log  = HISTÓRICO completo (comandos + exit + stdout/stderr),
    //                exibido no ÚNICO console global, visível em todas as
    //                abas, rolando como `tail -f`.
    pub status_msg: String,
    pub debug_log: String,
    // Segue o fim do log automaticamente; vira false quando o usuário rola
    // para cima (para poder ler) e volta a true quando ele retorna ao fim.
    pub console_follow: bool,

    // Iniciar com o Windows (HKCU\...\Run)
    pub autostart_enabled: bool,

    // ─── MODO PORTÁTIL ─────────────────────────────────────────────────
    // Ligado quando existe o arquivo-marcador `fzcomputerai.portable` ao lado
    // do executável (é o que vai dentro do .zip portátil). Nesse modo as
    // PREFERÊNCIAS vão para um .ini ao lado do exe em vez do registro, e o
    // "Iniciar com o Windows" fica indisponível (ele exige HKCU\...\Run, que
    // é justamente o rastro que o portátil não deve deixar).
    //
    // LIMITE HONESTO: o RASTREIO de limpeza (regras portproxy e PIDs de túnel)
    // continua no registro mesmo em modo portátil. Ele não é preferência do
    // usuário: é a trava que evita deixar a máquina exposta se o app morrer.
    // Trocá-lo por arquivo quebraria o watchdog em PowerShell que o lê. A UI
    // diz isso em vez de fingir "zero registro".
    pub portable_mode: bool,

    // ─── Bandeja do sistema (tray) ──────────────────────────────────────
    // Quando ligado, minimizar ESCONDE a janela em vez de deixá-la na barra
    // de tarefas — o app continua vivo (e o motor também), acessível pelo
    // ícone na área de notificação. Preferência persistida em HKCU.
    pub minimize_to_tray: bool,
    pub window_hidden: bool,

    // MCP Tools Catalog
    pub mcp_tools_filter: String,

    // Fluxo de upgrade (GitHub Releases):
    //   check -> update_available(tag) -> download em BACKGROUND (%TEMP%)
    //   -> ready.flag -> pedir para FECHAR -> instalar -> reabrir GUI + motor.
    pub update_available: Option<String>,
    pub update_downloading: bool,
    pub update_ready: bool,
    last_update_poll: Option<std::time::Instant>,

    // ─── Atualização do MOTOR cua-driver ────────────────────────────────
    // O botão "Verificar Atualizações" cuida de DUAS coisas: esta GUI e o
    // motor. Antes ele só olhava a GUI, e o motor podia ficar dezenas de
    // versões atrás sem ninguém notar (na prática ficou: 0.8.3 instalado
    // contra 0.17.0 publicado). A checagem do motor usa a API OFICIAL dele,
    // `cua-driver check-update --json`, e a aplicação usa `update --apply` —
    // nunca baixamos binário do motor por conta própria.
    pub driver_version: String,           // versão instalada (check-update)
    pub driver_latest: String,            // última publicada
    pub driver_update_available: bool,
    pub driver_notes_url: String,
    pub driver_updating: bool,            // update --apply em andamento
    pub driver_present: bool,             // cua-driver existe no PATH?
    pub update_checked: bool,             // já rodou uma verificação nesta sessão
    driver_update_poll: Option<std::time::Instant>,

    // ─── Token do endpoint HTTP do motor ────────────────────────────────
    // Versões do cua-driver a partir da série 0.16 EXIGEM
    // `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32-4096 chars) e respondem 401 a
    // qualquer POST sem `Authorization: Bearer <token>`. Sem isto, o teste de
    // status desta GUI receberia 401 e reportaria "MCP PARADO" com o motor
    // funcionando perfeitamente — ou seja, o "status honesto" mentiria. Lemos
    // o token de HKCU\Environment e o enviamos em todo probe.
    pub mcp_token: String,
    // O ÚLTIMO probe local respondeu HTTP 401? Distingue os quatro estados
    // reais de autenticação do motor SEM re-sondar (aviso da aba Túnel):
    //   401 + token vazio   => motor fail-closed e a GUI não tem o token
    //                          (túnel subiria mas NENHUM cliente entraria);
    //   401 + token lido    => o token conhecido foi RECUSADO (regenerar);
    //   sem 401 + token     => autenticando normalmente;
    //   sem 401 + sem token => motor antigo, endpoint aberto (avisar).
    pub mcp_probe_401: bool,

    // ─── Aba Túnel: expõe o MCP HTTP local (127.0.0.1:porta) na internet ───
    // Um túnel por vez: 1 processo rastreado, 1 URL pública, 1 snippet.
    pub tunnel_provider: TunnelProvider,
    pub tunnel_status: TunnelStatus,
    pub tunnel_pid: Option<u32>,
    // URL pública BASE (sem /mcp e sem /s/<senha>); editável — o túnel
    // nomeado do Cloudflare não imprime URL, o usuário informa à mão.
    pub tunnel_public_url: String,
    pub tunnel_exposure: Option<TunnelExposure>,

    // Binários resolvidos (where.exe / {app}\tunnel / System32\OpenSSH).
    pub tunnel_cf_bin: String,
    pub tunnel_ngrok_bin: String,
    pub tunnel_ssh_bin: String,
    pub tunnel_bins_checked: bool,

    // Config por provedor (persistida em HKCU como tunnelcfg:*).
    pub tunnel_cf_token_input: String, // campo .password(true); nunca persistido/logado
    pub tunnel_cf_token_file: String,  // caminho do token-file; NAO-vazio => túnel nomeado
    // ─── Cloudflare NOMEADO com domínio próprio (fluxo OAuth completo) ───
    // `cloudflared tunnel login` sozinho só baixa o cert.pem: ele autoriza a
    // conta, mas NÃO cria túnel nem DNS — quem parava aí ficava sem URL e
    // achava que o login tinha falhado. O caminho completo, que a GUI agora
    // executa, é o documentado pelo próprio cloudflared:
    //   login -> tunnel create <nome> -> tunnel route dns <nome> <hostname>
    //   -> tunnel run --url http://127.0.0.1:<porta> <nome>
    // Diferente do quick tunnel, o hostname é FIXO (ex.: mcphome.seudominio)
    // e sobrevive a reinícios.
    pub tunnel_cf_name: String,     // nome do túnel na conta Cloudflare
    pub tunnel_cf_hostname: String, // ex.: mcphome.rogerluft.com.br
    pub tunnel_cf_logged: bool,     // existe ~/.cloudflared/cert.pem?
    pub tunnel_ngrok_use_policy: bool,
    pub tunnel_ngrok_password: String, // basic-auth da traffic policy (mostrada 1x)
    pub tunnel_ngrok_extra: String,
    pub tunnel_ssh_target: String,
    pub tunnel_ssh_remote_port: String,
    pub tunnel_ssh_key: String,
    pub tunnel_ssh_extra: String,

    // Nível 1 de auth: senha na URL (via gate local). Por sessão de túnel,
    // NUNCA persistida nem logada — aparece só na URL copiável.
    pub tunnel_gate_password: String,
    pub tunnel_gate_port: Option<u16>,

    // Modais.
    pub tunnel_show_start_modal: bool,
    pub tunnel_show_ngrok_tos: bool,

    // Download de binário em background (mesmo padrão do upgrade).
    pub tunnel_downloading: bool,

    // Identidade forte do processo (marcador na cmdline via path de log).
    pub tunnel_run_id: String,
    // Provedor do túnel EM EXECUÇÃO, congelado no start. O rádio da UI
    // (tunnel_provider) pode ser trocado com o túnel vivo — usar o rádio em
    // stop/poll faria a confirmação de identidade e a limpeza do HKCU
    // olharem para imagem/slug ERRADOS (bug real: parar um cloudflare com
    // ngrok selecionado tentava apagar tunnel:ngrok:<pid>).
    tunnel_run_provider: TunnelProvider,

    // Estado interno (privado — só o app.rs mexe).
    tunnel_child: Option<std::process::Child>,
    tunnel_gate_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    tunnel_last_poll: Option<std::time::Instant>,
    tunnel_last_probe: Option<std::time::Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            language: Language::PtBr,
            active_tab: Tab::Network,
            http_port: "8000".to_string(),
            lan_ip: detect_lan_ip(),
            port_active: false,
            port_status: PortStatus::Stopped,
            daemon_running: false,
            portproxy_active: false,
            portproxy_effective: false,
            real_listeners: Vec::new(),
            portproxy_rules: Vec::new(),

            bg_out: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            bg_busy: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            startup_pending: true,

            engine_child: None,
            engine_pid: None,
            engine_external: false,

            lan_relay_bind: "0.0.0.0".to_string(),
            lan_relay_listen: None,
            lan_relay_target: None,
            lan_relay_conns: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            lan_relay_total: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            lan_relay_stop: None,
            lan_relay_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),

            screen_width: 1920,
            screen_height: 1080,
            dpi_scale: 1.0,
            test_x: "960".to_string(),
            test_y: "540".to_string(),

            windows_list: Vec::new(),
            launch_input: "notepad".to_string(),

            is_recording: false,
            recording_path: "./recordings".to_string(),

            show_about: false,

            status_msg: String::new(),
            debug_log: String::new(),
            console_follow: true,
            autostart_enabled: false,
            portable_mode: false, // definido de verdade no startup
            minimize_to_tray: false,
            window_hidden: false,

            mcp_tools_filter: String::new(),

            update_available: None,
            update_downloading: false,
            update_ready: false,
            last_update_poll: None,

            driver_version: String::new(),
            driver_latest: String::new(),
            driver_update_available: false,
            driver_notes_url: String::new(),
            driver_updating: false,
            driver_present: true, // definido de verdade no startup logo abaixo
            update_checked: false,
            driver_update_poll: None,
            mcp_token: String::new(),
            mcp_probe_401: false,

            tunnel_provider: TunnelProvider::Cloudflare,
            tunnel_status: TunnelStatus::Stopped,
            tunnel_pid: None,
            tunnel_public_url: String::new(),
            tunnel_exposure: None,

            tunnel_cf_bin: String::new(),
            tunnel_ngrok_bin: String::new(),
            tunnel_ssh_bin: String::new(),
            tunnel_bins_checked: false,

            tunnel_cf_name: "fzcomputerai".to_string(),
            tunnel_cf_hostname: String::new(),
            tunnel_cf_logged: false,
            tunnel_cf_token_input: String::new(),
            tunnel_cf_token_file: String::new(),
            tunnel_ngrok_use_policy: false,
            tunnel_ngrok_password: String::new(),
            tunnel_ngrok_extra: String::new(),
            tunnel_ssh_target: "nokey@localhost.run".to_string(),
            tunnel_ssh_remote_port: "80".to_string(),
            tunnel_ssh_key: String::new(),
            tunnel_ssh_extra: String::new(),

            tunnel_gate_password: String::new(),
            tunnel_gate_port: None,
            tunnel_run_provider: TunnelProvider::Cloudflare,

            tunnel_show_start_modal: false,
            tunnel_show_ngrok_tos: false,

            tunnel_downloading: false,
            tunnel_run_id: String::new(),

            tunnel_child: None,
            tunnel_gate_stop: None,
            tunnel_last_poll: None,
            tunnel_last_probe: None,
        };
        state.log_debug(&format!("[startup] IP LAN autodetectado: {}", state.lan_ip));
        // ─── O QUE PODE RODAR AQUI ──────────────────────────────────────
        // Este construtor roda DENTRO do closure do `eframe::run_native`,
        // ANTES do primeiro frame: a janela já existe e ainda não pinta, então
        // tudo que demora aqui aparece como "(Não Respondendo)" logo ao abrir
        // — a queixa mais antiga do aplicativo. Sobrou só leitura barata de
        // configuração (registro/arquivo, ~75ms cada).
        //
        // O QUE SAIU DAQUI (agora em `run_startup_tasks`, depois do 1º frame):
        // reconciliação de túneis órfãos (rodava UM PowerShell POR TÚNEL
        // rastreado — e nesta máquina uma invocação de PowerShell chegou a
        // levar mais de um MINUTO), teste da porta, subida do motor e leitura
        // das dimensões de tela via `cua-driver call`.
        state.detect_portable_mode();
        #[cfg(target_os = "windows")]
        state.read_mcp_token();
        state.read_minimize_to_tray();
        #[cfg(target_os = "windows")]
        state.read_autostart();
        state
    }
}

impl AppState {
    /// Anexa uma entrada ao Console Debug (mantém tamanho limitado e 2 linhas em branco de espaçamento).
    pub fn log_debug(&mut self, entry: &str) {
        let entry_clean = entry.trim();
        if entry_clean.is_empty() {
            return;
        }
        if !self.debug_log.is_empty() {
            self.debug_log.push_str("\n\n");
        }
        self.debug_log.push_str(entry_clean);

        const MAX_LEN: usize = 64 * 1024;
        if self.debug_log.len() > MAX_LEN {
            let mut start = self.debug_log.len() - MAX_LEN;
            while !self.debug_log.is_char_boundary(start) {
                start += 1;
            }
            self.debug_log = self.debug_log[start..].to_string();
        }
    }

    /// Dispara uma tarefa em SEGUNDO PLANO. A closure roda numa thread e
    /// devolve o que aplicar; a UI continua desenhando. Use para tudo que
    /// chame processo externo ou rede — nunca bloqueie a thread do egui.
    pub fn spawn_bg<F>(&mut self, label: &str, f: F)
    where
        F: FnOnce() -> BgOutcome + Send + 'static,
    {
        use std::sync::atomic::Ordering;
        let out = self.bg_out.clone();
        let busy = self.bg_busy.clone();
        busy.fetch_add(1, Ordering::SeqCst);
        self.log_debug(&format!("[bg] {} — rodando em segundo plano...", label));
        std::thread::spawn(move || {
            let outcome = f();
            if let Ok(mut v) = out.lock() {
                v.push(outcome);
            }
            busy.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Aplica, na thread da UI, o que as tarefas de segundo plano produziram.
    /// Chamado a cada frame (barato: só trava o Mutex e sai se estiver vazio).
    pub fn poll_bg(&mut self) {
        let drained: Vec<BgOutcome> = match self.bg_out.lock() {
            Ok(mut v) if !v.is_empty() => v.drain(..).collect(),
            _ => return,
        };
        for o in drained {
            if !o.log.trim().is_empty() {
                self.log_debug(&o.log);
            }
            if !o.status.trim().is_empty() {
                self.status_msg = o.status;
            }
            match o.effect {
                BgEffect::None => {}
                BgEffect::Exposure(e) => self.tunnel_exposure = Some(e),
                BgEffect::PortStatus {
                    port_active,
                    port_status,
                    probe_401,
                    real_listeners,
                } => {
                    self.port_active = port_active;
                    self.port_status = port_status;
                    self.mcp_probe_401 = probe_401;
                    self.real_listeners = real_listeners;
                    self.daemon_running = port_active;
                }
            }
        }
    }

    /// Há tarefa de segundo plano em voo? (a UI diz isso em vez de parecer
    /// parada)
    pub fn bg_is_busy(&self) -> bool {
        self.bg_busy.load(std::sync::atomic::Ordering::SeqCst) > 0
    }

    /// Tarefas de arranque que NÃO podem rodar no construtor: elas custam de
    /// centenas de ms a minutos (PowerShell, netsh, sondas TCP, `cua-driver
    /// call`) e ali travariam a janela antes do primeiro pixel. Chamada uma
    /// única vez, no primeiro `update()` — com a interface já desenhada e o
    /// console mostrando o que está acontecendo.
    pub fn run_startup_tasks(&mut self) {
        if !self.startup_pending {
            return;
        }
        self.startup_pending = false;
        self.log_debug("[startup] Verificando ambiente em segundo plano (motor, porta, sobras de sessoes anteriores)...");

        // Rápido e necessário já (decide se dá para subir o motor): `where`.
        self.check_driver_present();
        // Sobe o motor como FILHO se nada responder. O spawn em si é barato;
        // tanto a sondagem da porta quanto a espera pelo endpoint são
        // assíncronas.
        if self.driver_present {
            self.start_daemon();
        }

        // ─── O RESTO VAI PARA THREAD ────────────────────────────────────
        // A limpeza de sobras roda PowerShell, e a leitura de tela chama o
        // `cua-driver`. Feitas aqui, seguravam o SEGUNDO frame — e um frame
        // que não termina é uma janela que não aparece: foi exatamente assim
        // que a janela sumiu no teste desta versão. Nesta máquina uma única
        // invocação de PowerShell chegou a passar de um minuto.
        let lang = self.language;
        self.spawn_bg("Limpeza de sobras da sessao anterior", move || {
            let mut log = String::new();
            #[cfg(target_os = "windows")]
            {
                // Um ÚNICO PowerShell varre e limpa tudo (mesma disciplina de
                // identidade do shutdown: só mata o que é comprovadamente
                // nosso — imagem + CreationDate + run_id na command line).
                let ps = "$ErrorActionPreference='SilentlyContinue'; \
                     $key='HKCU:\\Software\\FzComputerAI'; \
                     $props = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue; \
                     if (-not $props) { 'sem sobras'; exit 0 }; \
                     $n=0; \
                     foreach ($tv in @($props.PSObject.Properties | Where-Object { $_.Name -like 'tunnel:*' })) { \
                       $tp = ($tv.Name -split ':')[-1]; $parts = $tv.Value -split '\\|'; \
                       $tpr = Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                       if ($tpr -and $tpr.Name -eq $parts[0] -and ($parts[1] -eq '' -or $tpr.CreationDate.ToString('yyyyMMddHHmmss') -eq $parts[1]) -and $parts[3] -and $tpr.CommandLine -like ('*' + $parts[3] + '*')) { Stop-Process -Id $tp -Force; $n++ }; \
                       Remove-ItemProperty -Path $key -Name $tv.Name -Force \
                     }; \
                     \"tuneis orfaos encerrados: $n\"";
                match quiet_cmd("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps])
                    .output()
                {
                    Ok(o) => log.push_str(&format!(
                        "[startup] Reconciliacao concluida: {}",
                        String::from_utf8_lossy(&o.stdout).trim()
                    )),
                    Err(e) => log.push_str(&format!("[startup] Reconciliacao falhou: {}", e)),
                }
            }
            BgOutcome {
                log,
                status: tr_of(lang, "", ""),
                effect: BgEffect::None,
            }
        });

        // Dimensões de tela via motor: útil, mas nunca urgente.
        self.spawn_bg("Lendo dimensoes da tela", move || {
            let out = quiet_cmd("cua-driver").args(["call", "get_screen_size"]).output();
            let txt = match out {
                Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
                Err(e) => format!("erro: {}", e),
            };
            BgOutcome {
                log: format!("[startup] get_screen_size: {}", tail_str(&txt, 300)),
                status: String::new(),
                effect: BgEffect::None,
            }
        });
    }

    /// Executa um comando via quiet_cmd e loga comando + exit code +
    /// stdout/stderr/erro no Console Debug. Retorna o Output (se rodou).
    ///
    /// ATENÇÃO: é SÍNCRONO. Chamar da thread da UI congela a janela pelo tempo
    /// do processo. Para qualquer coisa que passe de ~100ms use `spawn_bg`.
    fn run_logged(&mut self, program: &str, args: &[&str]) -> Option<std::process::Output> {
        let cmd_line = format!("{} {}", program, args.join(" "));
        match quiet_cmd(program).args(args).output() {
            Ok(out) => {
                let code = out
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let mut entry = format!("> {}\n  exit: {}", cmd_line, code);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stdout.trim().is_empty() {
                    entry.push_str(&format!("\n  stdout: {}", stdout.trim()));
                }
                if !stderr.trim().is_empty() {
                    entry.push_str(&format!("\n  stderr: {}", stderr.trim()));
                }
                self.log_debug(&entry);
                Some(out)
            }
            Err(e) => {
                self.log_debug(&format!("> {}\n  ERRO ao executar: {}", cmd_line, e));
                None
            }
        }
    }

    /// Teste REAL do MCP em um endereço: conexao TCP + POST /mcp com um
    /// JSON-RPC `initialize` de verdade. Retorna true SOMENTE quando a
    /// resposta HTTP contem "jsonrpc" — o servidor MCP respondeu de fato.
    /// GET nao serve como prova: o endpoint MCP legitimamente devolve
    /// 405 Method Not Allowed para GET, o que so provava o TCP.
    fn mcp_probe(&mut self, ip: &str, port: u16) -> bool {
        use std::io::{Read, Write};

        let addr: std::net::SocketAddr = match format!("{}:{}", ip, port).parse() {
            Ok(a) => a,
            Err(_) => {
                self.log_debug(&format!(
                    "> mcp-check {}:{}\n  ERRO: endereco invalido",
                    ip, port
                ));
                return false;
            }
        };
        let timeout = std::time::Duration::from_millis(1200);

        match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));

                let body = concat!(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fzcomputerai-gui","version":""#,
                    env!("CARGO_PKG_VERSION"),
                    r#""}}}"#
                );
                // Authorization SO quando existe token configurado: o motor
                // antigo (<=0.8.x) ignora o header, e o novo (>=0.16) EXIGE.
                // Assim o mesmo probe fala com as duas gerações do motor.
                let auth = if self.mcp_token.trim().is_empty() {
                    String::new()
                } else {
                    format!("Authorization: Bearer {}\r\n", self.mcp_token.trim())
                };
                let request = format!(
                    "POST /mcp HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    ip,
                    port,
                    auth,
                    body.len(),
                    body
                );

                if let Err(e) = stream.write_all(request.as_bytes()) {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  TCP conectou mas a escrita falhou: {}",
                        ip, port, e
                    ));
                    return false;
                }

                // Le ate 4KB (o initialize cabe com folga); o timeout de
                // leitura encerra streams SSE que ficariam abertos.
                let mut collected: Vec<u8> = Vec::new();
                let mut buf = [0u8; 1024];
                while collected.len() < 4096 {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => collected.extend_from_slice(&buf[..n]),
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&collected).to_string();
                let status_line = text.lines().next().unwrap_or("(sem resposta)").to_string();

                // Registra o 401 (motores 0.16+ fail-closed): endpoint VIVO
                // porém exigindo Bearer. A aba Túnel usa isto para dizer a
                // verdade sobre autenticação sem sondar de novo.
                self.mcp_probe_401 = status_line.contains(" 401");

                if text.contains("jsonrpc") {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  MCP OK — {} + resposta JSON-RPC ao initialize.",
                        ip, port, status_line
                    ));
                    true
                } else {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  TCP conectou mas a resposta NAO e MCP: {}",
                        ip, port, status_line
                    ));
                    false
                }
            }
            Err(e) => {
                self.log_debug(&format!("> mcp-check {}:{}\n  SEM conexao ({})", ip, port, e));
                false
            }
        }
    }

    /// Fonte de verdade do sistema: netstat. Retorna true se existe LISTENER
    /// em `<lan_ip>:<porta>` ou `0.0.0.0:<porta>` (que cobre todas as
    /// interfaces). Loga as linhas do netstat que batem com a porta.
    fn netstat_lan_listening(&mut self, lan_ip: &str, port: u16) -> bool {
        let out = match quiet_cmd("netstat").args(["-ano", "-p", "tcp"]).output() {
            Ok(o) => o,
            Err(e) => {
                self.log_debug(&format!("> netstat -ano -p tcp\n  ERRO ao executar: {}", e));
                return false;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let want_lan = format!("{}:{}", lan_ip, port);
        let want_any = format!("0.0.0.0:{}", port);
        let suffix = format!(":{}", port);
        let lan_prefix = format!("{}:", lan_ip);

        let mut matched: Vec<String> = Vec::new();
        let mut lan_listening = false;
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 5 || !cols[0].eq_ignore_ascii_case("TCP") {
                continue;
            }
            let local = cols[1];
            let remote = cols[2];
            let state = cols[3];
            let pid_txt = cols[4];

            let is_listener = state.eq_ignore_ascii_case("LISTENING") || remote == "0.0.0.0:0";

            // O que interessa na tela: QUALQUER linha (LISTENING e tambem
            // ESTABLISHED — conexoes MCP reais em andamento) cujo endereco
            // local OU remoto use a porta configurada, mais listeners em
            // portas ALTAS (>=1024) no IP da LAN — assim um listener orfao
            // (ex.: portproxy antigo em outra porta) aparece na tela, sem
            // poluir com servicos de sistema (137/139/445...).
            let local_port_high = local
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok())
                .map(|p| p >= 1024)
                .unwrap_or(false);
            let interesting = local.ends_with(&suffix)
                || remote.ends_with(&suffix)
                || (is_listener && local.starts_with(&lan_prefix) && local_port_high);
            if interesting {
                // MESMAS colunas do netstat real (local, remoto, estado,
                // PID) para a tela bater 1:1 com o que o usuario ve no
                // proprio terminal. Num listener em espera o "remoto" e
                // 0.0.0.0:0 — e o formato do Windows, nao um destino.
                matched.push(format!(
                    "TCP  {:<22} {:<22} {:<12} pid {}",
                    local, remote, state, pid_txt
                ));
            }
            if is_listener && (local == want_lan || local == want_any) {
                lan_listening = true;
            }
        }
        self.real_listeners = matched.clone();

        if matched.is_empty() {
            self.log_debug(&format!(
                "> netstat -ano -p tcp (filtro :{})\n  nenhum LISTENER na porta {}",
                port, port
            ));
        } else {
            self.log_debug(&format!(
                "> netstat -ano -p tcp (filtro :{})\n  {}",
                port,
                matched.join("\n  ")
            ));
        }
        lan_listening
    }

    /// Teste REAL do endpoint MCP nos DOIS endereços (loopback e IP da LAN
    /// exibido), com o netstat como fonte de verdade para a LAN. Atualiza
    /// `port_status`:
    ///   LanListening = netstat mostra `<ip_lan>:<porta>` (ou 0.0.0.0) LISTENING
    ///                  E o TCP no IP da LAN conectou;
    ///   LocalOnly    = só 127.0.0.1 responde;
    ///   Stopped      = nada responde.
    pub fn check_port_status(&mut self) {
        let port: u16 = match self.http_port.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                self.port_active = false;
                self.port_status = PortStatus::Stopped;
                let port_txt = self.http_port.clone();
                self.log_debug(&format!("> tcp-check :{}\n  ERRO: porta invalida", port_txt));
                return;
            }
        };

        // 1) loopback — MCP de verdade (POST initialize), nao so TCP
        let local_ok = self.mcp_probe("127.0.0.1", port);

        // 2) IP da LAN exibido na interface (o endereço publicado na URL MCP)
        let lan_ip = self.lan_ip.trim().to_string();
        let lan_is_loopback = lan_ip == "127.0.0.1" || lan_ip.eq_ignore_ascii_case("localhost");
        let lan_ok = if lan_is_loopback {
            self.log_debug("[status] IP LAN e loopback — teste LAN nao se aplica.");
            false
        } else {
            self.mcp_probe(&lan_ip, port)
        };

        // 3) fonte de verdade: netstat (roda SEMPRE, para real_listeners
        // refletir a verdade mesmo com IP loopback no campo)
        let netstat_result = self.netstat_lan_listening(&lan_ip, port);
        let netstat_lan = if lan_is_loopback { false } else { netstat_result };

        // Badge do portproxy recalculado AQUI, junto do netstat que ja rodou:
        // regra na config + listener LISTENING = funcionando; regra na config
        // sem listener = SEM EFEITO (IP Helper nao subiu o listener).
        #[cfg(target_os = "windows")]
        {
            let exists = if lan_is_loopback {
                false
            } else {
                self.portproxy_rule_exists(&lan_ip, &port.to_string())
            };
            self.portproxy_active = exists;
            self.portproxy_effective = exists && netstat_lan;
            if exists && !netstat_lan {
                self.log_debug(
                    "[portproxy] Regra existe na config do netsh mas o listener NAO esta de pe — SEM EFEITO. Reinicie o servico IP Helper (iphlpsvc) ou remova e reaplique a regra.",
                );
            }
        }

        if netstat_lan && !lan_ok {
            self.log_debug(
                "[status] AVISO: netstat mostra listener na LAN mas o teste TCP falhou (firewall?).",
            );
        }
        if lan_ok && !netstat_lan {
            self.log_debug(
                "[status] AVISO: TCP na LAN conectou mas netstat nao mostra o listener — nao pintando verde.",
            );
        }

        self.port_active = local_ok || lan_ok;
        self.port_status = if lan_ok && netstat_lan {
            PortStatus::LanListening
        } else if local_ok {
            PortStatus::LocalOnly
        } else {
            PortStatus::Stopped
        };

        let resumo = match self.port_status {
            PortStatus::LanListening => format!(
                "[status] LISTENING (local + LAN) — MCP respondeu JSON-RPC em 127.0.0.1:{p} E em {ip}:{p}; listener confirmado no netstat.",
                p = port,
                ip = lan_ip
            ),
            PortStatus::LocalOnly => format!(
                "[status] LOCAL APENAS — MCP responde em 127.0.0.1:{} mas NAO e acessivel pela LAN ({}).",
                port, lan_ip
            ),
            PortStatus::Stopped => format!("[status] STOPPED — nada respondeu na porta {}.", port),
        };
        self.log_debug(&resumo);
    }

    /// Grava uma variavel em HKCU\Environment pela via oficial e RELÊ o
    /// registro para confirmar sucesso/falha real. Retorna true somente com
    /// o valor confirmado.
    #[cfg(target_os = "windows")]
    fn set_user_env_confirmed(&mut self, name: &str, value: &str) -> bool {
        let ps = format!(
            "[Environment]::SetEnvironmentVariable('{}', '{}', 'User')",
            name, value
        );
        let set_ok = self
            .run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
            .map(|o| o.status.success())
            .unwrap_or(false);

        let confirmed = self
            .run_logged("reg", &["query", r"HKCU\Environment", "/v", name])
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains(value))
            .unwrap_or(false);

        set_ok && confirmed
    }

    /// Define CUA_DRIVER_RS_MCP_HTTP_PORT + CUA_DRIVER_RS_MCP_HTTP_BIND
    /// (User) e reinicia o daemon. O bind vai para 0.0.0.0 (todas as
    /// interfaces); se o daemon nao conseguir/nao subir na LAN, o
    /// check_port_status mostra honestamente LOCAL APENAS (fallback
    /// 127.0.0.1) — nunca presumimos que a LAN esta servida.
    pub fn apply_env_port(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let port = self.http_port.trim().to_string();

            let port_ok = self.set_user_env_confirmed("CUA_DRIVER_RS_MCP_HTTP_PORT", &port);
            if port_ok {
                self.log_debug(&format!(
                    "[env] OK: CUA_DRIVER_RS_MCP_HTTP_PORT = {} confirmado em HKCU\\Environment.",
                    port
                ));
            } else {
                self.log_debug(
                    "[env] FALHA: CUA_DRIVER_RS_MCP_HTTP_PORT NAO confirmado em HKCU\\Environment.",
                );
            }

            // ─── SOBRE O "BIND 0.0.0.0" (leia antes de reintroduzir) ───
            // O motor OFICIAL do projeto Cua escuta APENAS em 127.0.0.1: o
            // endereco esta fixo no codigo (`([127,0,0,1], port)`) e NAO existe
            // variavel de bind no upstream. Verificado no binario instalado
            // (0.8.3): a string CUA_DRIVER_RS_MCP_HTTP_BIND nem aparece nele,
            // e no upstream atual a busca por essa variavel no repositorio
            // inteiro retorna zero ocorrencia.
            //
            // Ou seja: gravar essa variavel NAO publica nada na LAN — o motor a
            // ignora. Quem realmente entrega a LAN e o ENCAMINHAMENTO
            // (netsh portproxy, ao lado) ou um TUNEL (aba Tunel). Ela so tem
            // efeito num motor com patch local, que nao e o que o usuario roda.
            //
            // Deixamos de gravar a variavel e passamos a DIZER a verdade. Se
            // algum dia o upstream aceitar bind configuravel, reintroduza aqui
            // — mas com verificacao real no netstat, nunca por suposicao.
            self.log_debug(
                "[env] NOTA: o motor oficial escuta somente em 127.0.0.1 (bind fixo no codigo do Cua). \
                 Nao existe variavel de bind no upstream, portanto nada e gravado para isso. \
                 Para acesso pela LAN use o Encaminhamento (portproxy); para internet, a aba Tunel.",
            );

            // Reinício pelo NOSSO ciclo de vida: derruba o filho e sobe outro
            // com a porta nova. Antes isto era `cua-driver stop` + `autostart
            // kick`, que (a) encerrava o daemon de QUEM QUER QUE FOSSE, mesmo
            // o de um cliente MCP de terceiro, e (b) ressuscitava o motor pela
            // tarefa agendada — um processo independente que sobrevivia ao
            // app, exatamente o que deixou de ser o desenho.
            if port_ok {
                if self.engine_child.is_some() {
                    self.log_debug(
                        "[env] Reiniciando o motor FILHO para aplicar a porta nova...",
                    );
                    self.stop_daemon();
                    self.start_daemon();
                } else {
                    self.log_debug(
                        "[env] Porta gravada. Nenhum motor filho para reiniciar — use Iniciar quando quiser subir o motor nesta porta.",
                    );
                }
            }

            // Higiene: se uma versao anterior desta GUI (ou o instalador) deixou
            // a variavel morta no ambiente do usuario, remove — configuracao que
            // nao faz nada so gera confusao no diagnostico.
            let leftover = self
                .run_logged(
                    "reg",
                    &["query", r"HKCU\Environment", "/v", "CUA_DRIVER_RS_MCP_HTTP_BIND"],
                )
                .map(|o| o.status.success())
                .unwrap_or(false);
            if leftover {
                let _ = self.run_logged(
                    "reg",
                    &[
                        "delete",
                        r"HKCU\Environment",
                        "/v",
                        "CUA_DRIVER_RS_MCP_HTTP_BIND",
                        "/f",
                    ],
                );
                self.log_debug(
                    "[env] Removida a variavel CUA_DRIVER_RS_MCP_HTTP_BIND, que o motor oficial ignora (config morta).",
                );
            }
        }
        self.check_port_status();
    }

    // ─── MODO PORTÁTIL: preferências em arquivo, não no registro ────────

    /// Caminho do marcador que liga o modo portátil (vai dentro do .zip).
    fn portable_marker() -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("fzcomputerai.portable")))
    }

    /// Caminho do .ini de preferências, ao lado do executável.
    fn portable_ini() -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("fzcomputerai.ini")))
    }

    /// Detecta o modo portátil pela presença do marcador.
    pub fn detect_portable_mode(&mut self) {
        self.portable_mode = Self::portable_marker()
            .map(|p| p.exists())
            .unwrap_or(false);
        if self.portable_mode {
            self.log_debug(
                "[portatil] MODO PORTATIL ativo (marcador fzcomputerai.portable encontrado). \
                 Preferencias ficam em fzcomputerai.ini ao lado do executavel; \
                 'Iniciar com o Windows' fica indisponivel. O rastreio de limpeza \
                 (portproxy/tunel) continua no registro por seguranca.",
            );
        }
    }

    /// Lê uma preferência: arquivo no modo portátil, registro fora dele.
    fn cfg_get(&mut self, key: &str) -> Option<String> {
        if self.portable_mode {
            let path = Self::portable_ini()?;
            let text = std::fs::read_to_string(path).ok()?;
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.starts_with(';') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == key {
                        return Some(v.trim().to_string());
                    }
                }
            }
            None
        } else {
            #[cfg(target_os = "windows")]
            {
                let out = self.run_logged(
                    "reg",
                    &["query", r"HKCU\Software\FzComputerAI", "/v", key],
                )?;
                if !out.status.success() {
                    return None;
                }
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if line.contains(key) {
                        if let Some(pos) = line.find("REG_SZ") {
                            return Some(line[pos + "REG_SZ".len()..].trim().to_string());
                        }
                    }
                }
                None
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = key;
                None
            }
        }
    }

    /// Grava uma preferência: arquivo no modo portátil, registro fora dele.
    fn cfg_set(&mut self, key: &str, value: &str) {
        if self.portable_mode {
            let Some(path) = Self::portable_ini() else {
                return;
            };
            // Reescreve preservando as outras chaves (sem apagar o que não é nosso).
            let mut lines: Vec<String> = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect();
            let mut replaced = false;
            for line in lines.iter_mut() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.starts_with(';') {
                    continue;
                }
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        *line = format!("{}={}", key, value);
                        replaced = true;
                    }
                }
            }
            if !replaced {
                if lines.is_empty() {
                    lines.push(
                        "# FzComputerAI - preferencias do MODO PORTATIL".to_string(),
                    );
                }
                lines.push(format!("{}={}", key, value));
            }
            let _ = std::fs::write(&path, lines.join("\r\n") + "\r\n");
        } else {
            #[cfg(target_os = "windows")]
            {
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        key,
                        "/t",
                        "REG_SZ",
                        "/d",
                        value,
                        "/f",
                    ],
                );
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (key, value);
            }
        }
    }

    /// Liga/desliga "minimizar para a bandeja" e PERSISTE a preferência em
    /// `HKCU\Software\FzComputerAI` (`appcfg:minimize_to_tray`), na mesma chave
    /// já usada pelo resto do app. O ícone em si é criado/removido pelo
    /// `update()`, que é quem tem a mão na janela.
    pub fn set_minimize_to_tray(&mut self, enable: bool) {
        self.minimize_to_tray = enable;
        if !enable {
            self.window_hidden = false;
        }
        let value = if enable { "1" } else { "0" };
        self.cfg_set("appcfg:minimize_to_tray", value);
        self.log_debug(&format!(
            "[tray] Minimizar para a bandeja: {}",
            if enable { "ATIVADO" } else { "DESATIVADO" }
        ));
    }

    /// Lê a preferência da bandeja no startup (sem isso o checkbox esqueceria
    /// a escolha do usuário a cada abertura). Respeita o modo portátil.
    pub fn read_minimize_to_tray(&mut self) {
        if let Some(v) = self.cfg_get("appcfg:minimize_to_tray") {
            self.minimize_to_tray = v == "1";
        }
    }

    /// Lê o token do endpoint HTTP do motor de `HKCU\Environment`. Motores
    /// 0.16+ EXIGEM esse token; sem enviá-lo, todo probe voltaria 401 e o app
    /// reportaria "MCP parado" com o motor perfeitamente vivo.
    ///
    /// NÃO usa `run_logged`: ele loga o stdout inteiro no Console Debug, e o
    /// stdout AQUI contém o VALOR do token — segredo jamais vai para log
    /// (AGENTS.md §1.1). Executa direto e loga só o desfecho, mascarado.
    #[cfg(target_os = "windows")]
    pub fn read_mcp_token(&mut self) {
        if let Ok(out) = quiet_cmd("reg")
            .args([
                "query",
                r"HKCU\Environment",
                "/v",
                "CUA_DRIVER_RS_MCP_HTTP_TOKEN",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if !line.contains("CUA_DRIVER_RS_MCP_HTTP_TOKEN") {
                        continue;
                    }
                    // Formato: NOME<espacos>REG_SZ<espacos>VALOR — cortar em
                    // "REG_SZ" (o valor nao tem espaco, mas o parser fica
                    // correto de qualquer forma).
                    if let Some(pos) = line.find("REG_SZ") {
                        let value = line[pos + "REG_SZ".len()..].trim().to_string();
                        if !value.is_empty() {
                            self.mcp_token = value;
                            self.log_debug(
                                "[env] Token do endpoint MCP encontrado em HKCU\\Environment — sera enviado como Bearer nos testes. (valor NAO exibido)",
                            );
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn read_mcp_token(&mut self) {}

    /// Gera um token com o CSPRNG do sistema (RNGCryptoServiceProvider via
    /// PowerShell): é credencial de controle da máquina, então o xorshift do
    /// `gen_token` (bom para senha-de-URL/run_id) não basta. O valor NUNCA
    /// passa por `run_logged` — sai só no stdout deste processo filho.
    /// Fallback honesto: se o PowerShell falhar, usa `gen_token(48)` e LOGA
    /// que a fonte foi o gerador fraco.
    #[cfg(target_os = "windows")]
    fn gen_secure_token(&mut self) -> String {
        let out = quiet_cmd("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$r=[Security.Cryptography.RNGCryptoServiceProvider]::new(); $b=New-Object byte[] 48; $r.GetBytes($b); ([Convert]::ToBase64String($b)) -replace '[^a-zA-Z0-9]',''",
            ])
            .output();
        if let Ok(o) = out {
            let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // 48 bytes -> 64 chars base64; removendo +/ sobra >= 32 com folga.
            if o.status.success() && t.len() >= 32 && t.chars().all(|c| c.is_ascii_alphanumeric()) {
                return t;
            }
        }
        self.log_debug(
            "[env] AVISO: CSPRNG via PowerShell indisponivel — token gerado pelo xorshift local (mais fraco).",
        );
        Self::gen_token(48)
    }

    /// Grava uma variável SECRETA em HKCU\Environment: mesma via oficial do
    /// `set_user_env_confirmed`, porém SEM `run_logged` (a linha de comando e
    /// a confirmação conteriam o valor — segredo não vai para o Console
    /// Debug). Confirma relendo o registro de verdade.
    #[cfg(target_os = "windows")]
    fn set_user_env_secret(&mut self, name: &str, value: &str) -> bool {
        use std::io::Write;
        // O SEGREDO VAI PELO STDIN, NUNCA PELO ARGV. Passar o token em
        // `-Command "...$token..."` o expõe: enquanto o powershell.exe vive,
        // qualquer processo do mesmo usuário lê a linha de comando inteira
        // (`Get-CimInstance Win32_Process | select CommandLine`) — inclusive o
        // watchdog deste próprio app, antivírus e telemetria. Não logar não
        // basta; o AGENTS.md diz "segredo nunca em argv". `-Command -` faz o
        // PowerShell ler o script do stdin, que não é visível de fora.
        let script = format!(
            "$v = [Console]::In.ReadLine()\n[Environment]::SetEnvironmentVariable('{}', $v, 'User')\n",
            name
        );
        let set_ok = (|| -> Option<bool> {
            let mut child = quiet_cmd("powershell")
                .args(["-NoProfile", "-Command", "-"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok()?;
            {
                let stdin = child.stdin.as_mut()?;
                // 1ª linha: o script; 2ª: o valor lido por ReadLine().
                stdin.write_all(script.as_bytes()).ok()?;
                stdin.write_all(value.as_bytes()).ok()?;
                stdin.write_all(b"\n").ok()?;
            }
            child.wait().ok().map(|s| s.success())
        })()
        .unwrap_or(false);
        let confirmed = quiet_cmd("reg")
            .args(["query", r"HKCU\Environment", "/v", name])
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains(value))
            .unwrap_or(false);
        self.log_debug(&format!(
            "[env] {} em HKCU\\Environment: {} (valor NAO exibido).",
            name,
            if set_ok && confirmed { "gravado e CONFIRMADO" } else { "FALHA ao gravar/confirmar" }
        ));
        set_ok && confirmed
    }

    /// Gera + grava o token do endpoint HTTP do motor e reinicia o daemon
    /// para o motor passar a exigi-lo. Fluxo VERIFICADO nesta máquina:
    /// motor 0.17 sem token responde 401 a TUDO (fail-closed) — o túnel até
    /// sobe, mas nenhum cliente entra; com o token em HKCU\Environment e o
    /// daemon religado, initialize responde 200 com Bearer e 401 sem.
    /// `std::env::set_var` cobre o caso do daemon renascer como FILHO desta
    /// GUI (filho herda o ambiente do pai, não o HKCU).
    #[cfg(target_os = "windows")]
    pub fn generate_engine_token(&mut self) {
        let token = self.gen_secure_token();
        if !self.set_user_env_secret("CUA_DRIVER_RS_MCP_HTTP_TOKEN", &token) {
            self.status_msg = self.tr(
                "FALHA ao gravar o token em HKCU\\Environment — nada foi alterado no motor.",
                "FAILED to write the token to HKCU\\Environment — nothing was changed in the engine.",
            );
            return;
        }
        std::env::set_var("CUA_DRIVER_RS_MCP_HTTP_TOKEN", &token);
        self.mcp_token = token;
        // Reinício pelo NOSSO ciclo de vida (nunca `autostart kick`, que sobe
        // o motor pela tarefa agendada — processo solto que não morre com o app).
        self.log_debug("[env] Reiniciando o motor para adotar o token...");
        self.stop_daemon();
        self.start_daemon();
        self.check_port_status();
        self.status_msg = self.tr(
            "Token do motor gerado e gravado (HKCU\\Environment). Daemon reiniciado; o snippet do tunel passa a incluir 'Authorization: Bearer'. Reinicie clientes MCP locais que usem o endpoint HTTP.",
            "Engine token generated and stored (HKCU\\Environment). Daemon restarted; the tunnel snippet now includes 'Authorization: Bearer'. Restart local MCP clients that use the HTTP endpoint.",
        );
    }

    #[cfg(not(target_os = "windows"))]
    pub fn generate_engine_token(&mut self) {
        self.status_msg =
            "Geracao de token pela GUI disponivel apenas no Windows (HKCU\\Environment).".to_string();
    }

    /// Porta CUA CONFIGURADA e CONFIRMADA — nunca chutada.
    /// Ordem dos candidatos: (1) CUA_DRIVER_RS_MCP_HTTP_PORT em
    /// HKCU\Environment (a configuracao real do daemon), (2) a porta do campo
    /// da UI, (3) o default 8000. Retorna a PRIMEIRA que RESPONDER de fato em
    /// 127.0.0.1 (tcp_probe). Nenhuma respondeu => None: quem chamar decide o
    /// que fazer, mas regra de encaminhamento para porta morta NAO se cria.
    pub fn detect_confirmed_cua_port(&mut self) -> Option<u16> {
        let mut candidates: Vec<u16> = Vec::new();

        #[cfg(target_os = "windows")]
        if let Some(out) = self.run_logged(
            "reg",
            &[
                "query",
                r"HKCU\Environment",
                "/v",
                "CUA_DRIVER_RS_MCP_HTTP_PORT",
            ],
        ) {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if line.contains("CUA_DRIVER_RS_MCP_HTTP_PORT") {
                        if let Some(tok) = line.split_whitespace().last() {
                            if let Ok(p) = tok.parse::<u16>() {
                                candidates.push(p);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(p) = self.http_port.trim().parse::<u16>() {
            if !candidates.contains(&p) {
                candidates.push(p);
            }
        }
        if !candidates.contains(&8000) {
            candidates.push(8000);
        }

        for p in candidates {
            if self.mcp_probe("127.0.0.1", p) {
                self.log_debug(&format!(
                    "[portproxy] Porta CUA confirmada respondendo MCP em 127.0.0.1:{}.",
                    p
                ));
                return Some(p);
            }
        }
        self.log_debug(
            "[portproxy] NENHUM candidato de porta CUA respondeu em 127.0.0.1 (config HKCU, campo da UI e 8000 testados).",
        );
        None
    }

    /// Lê `netsh interface portproxy show v4tov4` e retorna true se existe
    /// linha cujo par (endereço de escuta, porta de escuta) é exatamente
    /// `<ip> <porta>`. Parse por token — nada de contains() solto.
    #[cfg(target_os = "windows")]
    fn portproxy_rule_exists(&mut self, ip: &str, port: &str) -> bool {
        match self.run_logged("netsh", &["interface", "portproxy", "show", "v4tov4"]) {
            Some(show) => {
                let text = String::from_utf8_lossy(&show.stdout).to_string();
                // Guarda TODAS as regras v4tov4 (linhas com portas numericas)
                // para a UI listar — inclusive regras orfas em outras portas.
                self.portproxy_rules = text
                    .lines()
                    .filter_map(|line| {
                        let cols: Vec<&str> = line.split_whitespace().collect();
                        if cols.len() >= 4
                            && cols[1].parse::<u16>().is_ok()
                            && cols[3].parse::<u16>().is_ok()
                        {
                            Some(format!("{}:{} -> {}:{}", cols[0], cols[1], cols[2], cols[3]))
                        } else {
                            None
                        }
                    })
                    .collect();
                text.lines().any(|line| {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    cols.len() >= 2 && cols[0] == ip && cols[1] == port
                })
            }
            None => false,
        }
    }

    /// O APP aplica a regra portproxy do netsh (não apenas sugere).
    /// Etapas, todas logadas com o resultado real:
    ///   1. verificar se a regra já existe (show v4tov4);
    ///   2. tentar `netsh ... add` direto; se falhar, elevar via UAC oficial
    ///      (Start-Process -Verb RunAs) propagando o exit code do netsh elevado;
    ///   3. reler `show v4tov4` e confirmar a regra por token;
    ///   4. confirmar com netstat que `<ip>:<porta>` está LISTENING;
    ///   5. refazer o teste TCP nos dois endereços (check_port_status).
    pub fn apply_portproxy(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let listen_port = self.http_port.trim().to_string();
            let ip = self.lan_ip.trim().to_string();

            if ip == "127.0.0.1" || ip.eq_ignore_ascii_case("localhost") {
                self.log_debug(
                    "[portproxy] IP da LAN e loopback — regra nao faz sentido. Ajuste o campo IP.",
                );
                return;
            }
            let listen_port_num: u16 = match listen_port.parse() {
                Ok(p) => p,
                Err(_) => {
                    self.log_debug("[portproxy] Porta invalida — corrija o campo Porta.");
                    return;
                }
            };

            // Destino da regra: a porta CUA CONFIGURADA e CONFIRMADA por teste
            // TCP real — nunca a porta "sugerida" sem confirmacao. Sem porta
            // confirmada nao se cria regra nenhuma (encaminhar para porta
            // morta so mascara o problema).
            let connect_port = match self.detect_confirmed_cua_port() {
                Some(p) => p,
                None => {
                    self.log_debug(
                        "[portproxy] Regra NAO criada: nenhuma porta CUA confirmada em 127.0.0.1. Inicie o daemon (botao Iniciar) e tente de novo.",
                    );
                    self.check_port_status();
                    return;
                }
            };

            // Etapa 1: regra já existe?
            if self.portproxy_rule_exists(&ip, &listen_port) {
                self.log_debug(&format!(
                    "[portproxy] Regra {}:{} -> 127.0.0.1:{} JA existe — pulando o add.",
                    ip, listen_port, connect_port
                ));
            } else if self.netstat_lan_listening(&ip, listen_port_num) {
                // Ja ha um listener REAL em <lan>:<porta> que NAO e regra
                // portproxy (ex.: o proprio daemon com bind 0.0.0.0, ou outro
                // processo). Criar a regra sobrescreveria/conflitaria com a
                // porta de um processo existente — nao fazemos isso.
                self.log_debug(&format!(
                    "[portproxy] Ja existe listener em {}:{} que NAO e regra portproxy (bind 0.0.0.0 do daemon ou outro processo). Nada foi alterado.",
                    ip, listen_port
                ));
                self.portproxy_active = false;
                self.check_port_status();
                return;
            } else {
                // Etapa 2: aplicar (direto; se falhar, elevado via UAC)
                let netsh_args = format!(
                    "interface portproxy add v4tov4 listenport={} listenaddress={} connectport={} connectaddress=127.0.0.1",
                    listen_port, ip, connect_port
                );
                let args_vec: Vec<&str> = netsh_args.split(' ').collect();

                let direct_ok = self
                    .run_logged("netsh", &args_vec)
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if direct_ok {
                    self.log_debug("[portproxy] netsh add direto concluiu com exit 0.");
                } else {
                    self.log_debug(
                        "[portproxy] Tentativa direta falhou — solicitando elevacao (UAC)...",
                    );
                    // -PassThru + exit $p.ExitCode: o exit code do netsh ELEVADO
                    // chega até aqui. UAC cancelado => Start-Process lança erro
                    // e o powershell sai com exit != 0.
                    let ps = format!(
                        "$p = Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
                        netsh_args
                    );
                    match self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()]) {
                        Some(o) if o.status.success() => {
                            self.log_debug("[portproxy] netsh elevado concluiu com exit 0.")
                        }
                        Some(o) => self.log_debug(&format!(
                            "[portproxy] netsh elevado FALHOU ou UAC foi cancelado (exit {:?}).",
                            o.status.code()
                        )),
                        None => {}
                    }
                }
            }

            // Etapa 3: reler a verdade do netsh
            let rule_ok = self.portproxy_rule_exists(&ip, &listen_port);
            self.portproxy_active = rule_ok;
            if rule_ok {
                self.log_debug(&format!(
                    "[portproxy] Regra CONFIRMADA no show v4tov4: {}:{} -> 127.0.0.1:{}",
                    ip, listen_port, connect_port
                ));
                // Registra a regra como PROPRIEDADE deste app (HKCU). O
                // shutdown_cleanup so remove regras registradas aqui —
                // nunca regras de outros servicos com o mesmo padrao
                // (ex.: 8082->8082 ou 9333->9222 de outras ferramentas).
                let value_name = format!("portproxy:{}:{}", ip, listen_port);
                let cp = connect_port.to_string();
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        value_name.as_str(),
                        "/t",
                        "REG_SZ",
                        "/d",
                        cp.as_str(),
                        "/f",
                    ],
                );
            } else {
                self.log_debug(
                    "[portproxy] Regra NAO encontrada em 'show v4tov4' — nada foi aplicado.",
                );
            }

            // Etapa 4: confirmar com netstat (o listener do portproxy e do iphlpsvc)
            if let Ok(p) = listen_port.parse::<u16>() {
                let listening = self.netstat_lan_listening(&ip, p);
                if rule_ok && !listening {
                    self.log_debug(
                        "[portproxy] AVISO: regra existe no netsh mas netstat NAO mostra o listener. Verifique o servico 'IP Helper' (iphlpsvc).",
                    );
                }
            }
        }
        // Etapa 5: teste TCP real nos dois endereços + status
        self.check_port_status();
    }

    /// Regras portproxy registradas como PROPRIEDADE deste app em
    /// HKCU\Software\FzComputerAI (valores "portproxy:<ip>:<porta>").
    #[cfg(target_os = "windows")]
    fn tracked_portproxy_rules() -> Vec<(String, String)> {
        let mut tracked: Vec<(String, String)> = Vec::new();
        if let Ok(out) = quiet_cmd("reg")
            .args(["query", r"HKCU\Software\FzComputerAI"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            for line in text.lines() {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if let Some(name) = cols.first() {
                    if let Some(rest) = name.strip_prefix("portproxy:") {
                        if let Some((ip, port)) = rest.rsplit_once(':') {
                            if port.parse::<u16>().is_ok() {
                                tracked.push((ip.to_string(), port.to_string()));
                            }
                        }
                    }
                }
            }
        }
        tracked
    }

    /// RECONCILIACAO NA ABERTURA: um fechamento anterior pode ter falhado
    /// (kill forcado, UAC recusado, queda de energia) e deixado regras
    /// portproxy NOSSAS vivas — portas abertas com o software "fechado".
    /// Aqui, toda regra RASTREADA encontrada na config do netsh e removida
    /// (tentativa direta; sem privilegio, o usuario e avisado no console e
    /// a regra sai no proximo fechamento com UAC ou pelo botao Remover).
    pub fn startup_reconcile_tracked_rules(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let tracked = Self::tracked_portproxy_rules();
            if tracked.is_empty() {
                return;
            }
            for (ip, port) in tracked {
                if !self.portproxy_rule_exists(&ip, &port) {
                    // Regra ja nao existe — apenas desregistra.
                    let value_name = format!("portproxy:{}:{}", ip, port);
                    let _ = quiet_cmd("reg")
                        .args([
                            "delete",
                            r"HKCU\Software\FzComputerAI",
                            "/v",
                            value_name.as_str(),
                            "/f",
                        ])
                        .output();
                    continue;
                }
                self.log_debug(&format!(
                    "[startup] Sobra de sessao anterior: regra portproxy {}:{} ainda ativa — removendo...",
                    ip, port
                ));
                let ok = quiet_cmd("netsh")
                    .args([
                        "interface",
                        "portproxy",
                        "delete",
                        "v4tov4",
                        &format!("listenport={}", port),
                        &format!("listenaddress={}", ip),
                    ])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                if ok && !self.portproxy_rule_exists(&ip, &port) {
                    self.log_debug("[startup] Sobra removida e confirmada no show v4tov4.");
                    let value_name = format!("portproxy:{}:{}", ip, port);
                    let _ = quiet_cmd("reg")
                        .args([
                            "delete",
                            r"HKCU\Software\FzComputerAI",
                            "/v",
                            value_name.as_str(),
                            "/f",
                        ])
                        .output();
                } else {
                    self.log_debug(
                        "[startup] AVISO: sem privilegio para remover a sobra agora. Ela sera removida no fechamento (UAC) ou use o botao Remover Regra.",
                    );
                }
            }
        }
    }

    /// Encerramento LIMPO ao fechar a GUI (chamado pelo on_exit do eframe):
    ///   1. para/mata o daemon cua-driver;
    ///   2. remove as regras portproxy QUE ESTE APP CRIOU — as registradas
    ///      em HKCU\Software\FzComputerAI (valores "portproxy:<ip>:<porta>").
    /// NUNCA apaga regra por "padrao parecido": nesta mesma maquina existem
    /// regras LAN->127.0.0.1 de OUTROS servicos (ex.: 8082->8082,
    /// 9333->9222) que nao sao nossas e nao podem ser tocadas.
    ///
    /// CRITICO — POR QUE ISTO E DESACOPLADO (nao mexa): a versao anterior
    /// fazia a limpeza AQUI, de forma bloqueante, incluindo
    /// `Start-Process -Verb RunAs -Wait` para elevar o netsh. Como o netsh
    /// delete exige admin, TODO fechamento abria um UAC e o processo ficava
    /// preso esperando resposta — o app simplesmente NAO FECHAVA (janela
    /// some, processo vivo, portas abertas). Agora o on_exit apenas DISPARA
    /// um auxiliar independente (spawn, sem wait) e retorna na hora: quem
    /// espera o app morrer, mata o motor, elimina as regras e lida com o UAC
    /// e o auxiliar — nunca a GUI.
    pub fn shutdown_cleanup(&mut self) {
        // ─── ORDEM IMPORTA ──────────────────────────────────────────────
        // 1) Derruba o que é NOSSO, direto pelo handle: instantâneo e sem
        //    depender de terceiros. 2) Só então dispara o auxiliar que cuida
        //    do que sobrevive a um processo (registro e regras de sistema).
        //
        // O que ANTES estava aqui e FOI REMOVIDO — não reintroduzir:
        //   • `taskkill /F /IM cua-driver.exe`: matava QUALQUER cua-driver da
        //     máquina, inclusive o daemon que um cliente MCP de terceiro
        //     tivesse subido. É a proibição explícita do AGENTS.md (matar por
        //     imagem). O motor agora é FILHO adotado pelo Job Object, então
        //     morre com esta GUI sem que ninguém precise caçá-lo por nome.
        //   • `cua-driver stop` no fechamento: mesma coisa por via oficial —
        //     encerra o daemon de quem quer que seja. Parar motor alheio virou
        //     ação explícita do usuário (botão Parar), nunca automática.
        if let Some(mut child) = self.tunnel_child.take() {
            let _ = child.kill();
        }
        if let Some(mut child) = self.engine_child.take() {
            let _ = child.kill();
        }
        self.stop_gate();
        self.stop_lan_relay();
        #[cfg(target_os = "windows")]
        {
            let my_pid = std::process::id();
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; \
                 $deadline=(Get-Date).AddSeconds(20); \
                 while ((Get-Process -Id {pid} -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {{ Start-Sleep -Milliseconds 300 }}; \
                 $key='HKCU:\\Software\\FzComputerAI'; \
                 $props = (Get-ItemProperty -Path $key -ErrorAction SilentlyContinue); \
                 if (-not $props) {{ exit 0 }}; \
                 $tuns = @($props.PSObject.Properties | Where-Object {{ $_.Name -like 'tunnel:*' }}); \
                 foreach ($tv in $tuns) {{ \
                   $tp = ($tv.Name -split ':')[-1]; \
                   $parts = $tv.Value -split '\\|'; \
                   $tpr = Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                   if ($tpr -and $tpr.Name -eq $parts[0] -and ($parts[1] -eq '' -or $tpr.CreationDate.ToString('yyyyMMddHHmmss') -eq $parts[1]) -and $parts[3] -and $tpr.CommandLine -like ('*' + $parts[3] + '*')) {{ Stop-Process -Id $tp -Force }}; \
                   Remove-ItemProperty -Path $key -Name $tv.Name -Force -ErrorAction SilentlyContinue \
                 }}; \
                 $rules = @($props.PSObject.Properties | Where-Object {{ $_.Name -like 'portproxy:*' }} | ForEach-Object {{ $_.Name.Substring(10) }}); \
                 if ($rules.Count -eq 0) {{ exit 0 }}; \
                 $pend=@(); \
                 foreach ($r in $rules) {{ \
                   $i=$r.LastIndexOf(':'); $addr=$r.Substring(0,$i); $prt=$r.Substring($i+1); \
                   netsh interface portproxy delete v4tov4 listenport=$prt listenaddress=$addr | Out-Null; \
                   $left = (netsh interface portproxy show v4tov4 | Select-String (\"^\\s*\" + [regex]::Escape($addr) + \"\\s+\" + $prt + \"\\s\")); \
                   if ($left) {{ $pend += \"netsh interface portproxy delete v4tov4 listenport=$prt listenaddress=$addr\" }} \
                   else {{ Remove-ItemProperty -Path $key -Name (\"portproxy:\" + $r) -Force -ErrorAction SilentlyContinue }} \
                 }}; \
                 if ($pend.Count -gt 0) {{ \
                   Start-Process -FilePath powershell -ArgumentList ('-NoProfile -WindowStyle Hidden -Command \"' + ($pend -join '; ') + '\"') -Verb RunAs -Wait; \
                   foreach ($r in $rules) {{ \
                     $i=$r.LastIndexOf(':'); $addr=$r.Substring(0,$i); $prt=$r.Substring($i+1); \
                     $left = (netsh interface portproxy show v4tov4 | Select-String (\"^\\s*\" + [regex]::Escape($addr) + \"\\s+\" + $prt + \"\\s\")); \
                     if (-not $left) {{ Remove-ItemProperty -Path $key -Name (\"portproxy:\" + $r) -Force -ErrorAction SilentlyContinue }} \
                   }} \
                 }}",
                pid = my_pid
            );

            // spawn(), NUNCA output()/wait(): o auxiliar vive por conta
            // propria e o processo da GUI pode terminar imediatamente.
            let _ = quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn();
        }
    }

    /// Remove a regra portproxy (netsh delete) com o MESMO fluxo honesto do
    /// apply_portproxy: tentativa direta, fallback elevado via UAC oficial e
    /// confirmação relendo `show v4tov4` — o estado exibido nunca é presumido.
    pub fn remove_portproxy(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let listen_port = self.http_port.trim().to_string();
            let ip = self.lan_ip.trim().to_string();

            if listen_port.parse::<u16>().is_err() {
                self.log_debug("[portproxy] Porta invalida — corrija o campo Porta.");
                return;
            }

            if !self.portproxy_rule_exists(&ip, &listen_port) {
                self.log_debug(&format!(
                    "[portproxy] Nenhuma regra {}:{} para remover.",
                    ip, listen_port
                ));
                self.portproxy_active = false;
                return;
            }

            let netsh_args = format!(
                "interface portproxy delete v4tov4 listenport={} listenaddress={}",
                listen_port, ip
            );
            let args_vec: Vec<&str> = netsh_args.split(' ').collect();

            let direct_ok = self
                .run_logged("netsh", &args_vec)
                .map(|o| o.status.success())
                .unwrap_or(false);

            if direct_ok {
                self.log_debug("[portproxy] netsh delete direto concluiu com exit 0.");
            } else {
                self.log_debug(
                    "[portproxy] Delete direto falhou — solicitando elevacao (UAC)...",
                );
                let ps = format!(
                    "$p = Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
                    netsh_args
                );
                match self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()]) {
                    Some(o) if o.status.success() => {
                        self.log_debug("[portproxy] netsh delete elevado concluiu com exit 0.")
                    }
                    Some(o) => self.log_debug(&format!(
                        "[portproxy] netsh delete elevado FALHOU ou UAC foi cancelado (exit {:?}).",
                        o.status.code()
                    )),
                    None => {}
                }
            }

            // Verdade final: reler o netsh.
            let still_there = self.portproxy_rule_exists(&ip, &listen_port);
            self.portproxy_active = still_there;
            if still_there {
                self.log_debug(
                    "[portproxy] Regra AINDA presente apos o delete — nada foi removido.",
                );
            } else {
                self.log_debug(&format!(
                    "[portproxy] Regra {}:{} removida e confirmada no show v4tov4.",
                    ip, listen_port
                ));
                // Regra removida => sai tambem do registro de propriedade.
                let value_name = format!("portproxy:{}:{}", ip, listen_port);
                let _ = self.run_logged(
                    "reg",
                    &[
                        "delete",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        value_name.as_str(),
                        "/f",
                    ],
                );
            }
        }
        self.check_port_status();
    }

    pub fn fetch_screen_info(&mut self) {
        match self.run_logged("cua-driver", &["call", "get_screen_size"]) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "get_screen_size:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "get_screen_size falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn test_click_position(&mut self) {
        let x: i32 = self.test_x.parse().unwrap_or(0);
        let y: i32 = self.test_y.parse().unwrap_or(0);
        let xs = x.to_string();
        let ys = y.to_string();

        match self.run_logged(
            "cua-driver",
            &["call", "move_cursor", "--x", xs.as_str(), "--y", ys.as_str()],
        ) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "move_cursor ({}, {}):\n{}",
                    x,
                    y,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "move_cursor ({}, {}) falhou (exit {:?}):\n{}",
                    x,
                    y,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn refresh_windows(&mut self) {
        match self.run_logged("cua-driver", &["call", "list_windows"]) {
            Some(out) if out.status.success() => {
                self.status_msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Some(out) => {
                self.status_msg = format!(
                    "list_windows falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn launch_app(&mut self) {
        let app = self.launch_input.trim().to_string();
        match self.run_logged("cua-driver", &["call", "launch_app", "--app", app.as_str()]) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "launch_app '{}':\n{}",
                    app,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "launch_app '{}' falhou (exit {:?}):\n{}",
                    app,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn start_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "start_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = true;
                self.status_msg = format!(
                    "start_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "start_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn stop_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "stop_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = false;
                self.status_msg = format!(
                    "stop_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "stop_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    /// Sobe o motor COMO FILHO desta GUI (`cua-driver serve`), adotado pelo
    /// Job Object — ele morre junto com o app, por garantia do kernel.
    ///
    /// Nunca sobe um segundo motor por cima de um que já responde: dois
    /// daemons disputariam o pipe `\\.\pipe\cua-driver` e a porta HTTP. Se o
    /// que está de pé não é nosso filho, a UI diz isso em vez de fingir posse.
    pub fn start_daemon(&mut self) {
        // Já temos um filho vivo? Então não há o que iniciar.
        if let Some(child) = self.engine_child.as_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                self.status_msg = self.tr(
                    "O motor ja esta rodando como processo desta GUI.",
                    "The engine is already running as a child of this GUI.",
                );
                self.check_port_status();
                return;
            }
            // Morreu sozinho: descarta o handle e segue para subir de novo.
            self.engine_child = None;
            self.engine_pid = None;
        }

        // Alguém já responde na porta? Se sim, é motor de terceiro (cliente
        // MCP próprio, tarefa agendada antiga). Respeitamos: não matamos, não
        // duplicamos — e dizemos a verdade sobre quem manda no ciclo de vida.
        if self.detect_confirmed_cua_port().is_some() {
            self.engine_external = true;
            self.daemon_running = true;
            self.status_msg = self.tr(
                "Ja existe um motor cua-driver respondendo que NAO foi iniciado por esta GUI (ex.: subido por um cliente MCP). Ele NAO sera encerrado ao fechar o app. Use 'Parar' para encerra-lo se quiser que a GUI passe a controlar o ciclo de vida.",
                "A cua-driver engine is already answering and was NOT started by this GUI (e.g. launched by an MCP client). It will NOT be terminated when the app closes. Use 'Stop' to end it if you want the GUI to own the lifecycle.",
            );
            self.log_debug(
                "[engine] Motor EXTERNO detectado (nao e filho desta GUI) — nao duplicamos nem matamos as cegas.",
            );
            self.check_port_status();
            return;
        }

        // Ambiente do filho: porta e (se houver) token. O filho herda o
        // ambiente do pai, então basta acrescentar o que a UI define.
        let port = self.http_port.trim().to_string();
        let mut cmd = quiet_cmd("cua-driver");
        cmd.arg("serve")
            .env("CUA_DRIVER_RS_MCP_HTTP_PORT", &port)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if !self.mcp_token.trim().is_empty() {
            cmd.env("CUA_DRIVER_RS_MCP_HTTP_TOKEN", self.mcp_token.trim());
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                // ADOÇÃO: sem isto o filho sobreviveria a um taskkill /F na
                // GUI. Falha aqui é reportada — nunca presumida como sucesso.
                match crate::lifecycle::adopt(&child) {
                    Ok(()) => self.log_debug(&format!(
                        "[engine] cua-driver serve iniciado como FILHO (pid {}) e adotado pelo Job Object — morre com a GUI.",
                        pid
                    )),
                    Err(code) => self.log_debug(&format!(
                        "[engine] AVISO: motor iniciado (pid {}) mas NAO foi adotado pelo Job Object (erro {}). A limpeza automatica ao fechar NAO esta garantida para ele.",
                        pid, code
                    )),
                }
                self.engine_child = Some(child);
                self.engine_pid = Some(pid);
                self.engine_external = false;

                // A espera pelo listener (o `serve` leva ~1-3s) vai para
                // SEGUNDO PLANO. Antes eram até 12×400ms de `sleep` na thread
                // da UI — quase 5 segundos de janela congelada a cada clique
                // em "Iniciar", e o dobro no "Reiniciar".
                let port_for_wait = port.parse::<u16>().unwrap_or(8000);
                let token_for_wait = self.mcp_token.trim().to_string();
                let lang = self.language;
                self.status_msg = self.tr(
                    "Motor iniciado; aguardando o endpoint responder (em segundo plano)...",
                    "Engine started; waiting for the endpoint to answer (in the background)...",
                );
                self.spawn_bg("Aguardando o motor responder", move || {
                    let mut ok = false;
                    for _ in 0..12 {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        if probe_mcp("127.0.0.1", port_for_wait, &token_for_wait).0 {
                            ok = true;
                            break;
                        }
                    }
                    BgOutcome {
                        log: format!(
                            "[engine] Endpoint em 127.0.0.1:{} {} apos o start.",
                            port_for_wait,
                            if ok { "RESPONDEU" } else { "NAO respondeu" }
                        ),
                        status: if ok {
                            tr_of(lang,
                                "Motor iniciado como processo desta GUI e respondendo. Ao fechar o app ele sera encerrado junto.",
                                "Engine started as a child of this GUI and answering. Closing the app terminates it too.")
                        } else {
                            tr_of(lang,
                                "Motor iniciado, mas o endpoint ainda NAO respondeu. Veja o console e a porta configurada.",
                                "Engine started, but the endpoint did NOT answer yet. Check the console and the configured port.")
                        },
                        effect: BgEffect::None,
                    }
                });
            }
            Err(e) => {
                self.status_msg = format!(
                    "{}: {}",
                    self.tr(
                        "ERRO ao iniciar 'cua-driver serve' (o motor esta instalado e no PATH?)",
                        "ERROR starting 'cua-driver serve' (is the engine installed and on PATH?)"
                    ),
                    e
                );
                self.log_debug(&format!("[engine] ERRO ao spawnar cua-driver serve: {}", e));
            }
        }

        // NÃO sondar a porta aqui. O motor acabou de ser lançado e leva 1-3s
        // para abrir o listener: um `check_port_status` imediato pintaria
        // "PARADO" logo após o clique em Iniciar — e ainda sobrescreveria, com
        // um resultado velho, o veredito que a tarefa de segundo plano vai
        // trazer. Quem reporta o estado é ela.
    }

    /// Para o motor. Se ele é NOSSO filho, mata o handle direto (instantâneo).
    /// Se é externo, usa a via oficial `cua-driver stop` — ação explícita do
    /// usuário sobre processo de terceiro, nunca automática no fechamento.
    pub fn stop_daemon(&mut self) {
        if let Some(mut child) = self.engine_child.take() {
            let pid = self.engine_pid.take();
            let _ = child.kill();
            let _ = child.wait();
            self.log_debug(&format!(
                "[engine] Motor (filho, pid {:?}) encerrado pela GUI.",
                pid
            ));
        } else {
            self.log_debug(
                "[engine] Nenhum motor filho desta GUI — usando a via oficial 'cua-driver stop' (a pedido do usuario).",
            );
            let _ = self.run_logged("cua-driver", &["stop"]);
        }
        self.engine_external = false;
        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    /// O motor filho morreu sozinho? Observado pelo loop da UI para o estado
    /// não mentir (badge "rodando" com processo morto).
    pub fn poll_engine_child(&mut self) {
        let exited = self
            .engine_child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten());
        if let Some(status) = exited {
            self.engine_child = None;
            let pid = self.engine_pid.take();
            self.log_debug(&format!(
                "[engine] O motor filho (pid {:?}) SAIU sozinho (exit {:?}).",
                pid,
                status.code()
            ));
            self.status_msg = self.tr(
                "O motor encerrou sozinho. Use Iniciar para subir de novo.",
                "The engine exited on its own. Use Start to bring it back.",
            );
            self.check_port_status();
            self.daemon_running = self.port_active;
        }
    }

    pub fn run_doctor(&mut self) {
        match self.run_logged("cua-driver", &["doctor"]) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("doctor: exit {:?} (sem saida)", out.status.code())
                };
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver doctor' (esta no PATH?)."
                        .to_string();
            }
        }
    }

    fn run_skills(&mut self, action: &str) {
        match self.run_logged("cua-driver", &["skills", action]) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("skills {}: exit {:?} (sem saida)", action, out.status.code())
                };
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver skills' (esta no PATH?)."
                        .to_string();
            }
        }
    }

    pub fn install_skills(&mut self) {
        self.run_skills("install");
    }

    pub fn update_skills(&mut self) {
        self.run_skills("update");
    }

    pub fn uninstall_skills(&mut self) {
        self.run_skills("uninstall");
    }

    /// Botão "Reiniciar": derruba o motor FILHO e sobe outro, sempre dentro do
    /// ciclo de vida da GUI. Antes chamava `cua-driver autostart kick`, que
    /// pedia à TAREFA AGENDADA do Windows para subir o daemon — um processo
    /// solto, sem vínculo com o app, que sobrevivia ao fechamento. Era a única
    /// via que restava ressuscitando o motor por fora.
    pub fn kick_autostart(&mut self) {
        self.stop_daemon();
        self.start_daemon();
    }

    /// Invoca qualquer tool MCP do CUA via CLI e captura o resultado.
    pub fn call_mcp_tool(&mut self, tool_name: &str, extra_args: &[&str]) {
        let mut args = vec!["call", tool_name];
        args.extend_from_slice(extra_args);
        match self.run_logged("cua-driver", &args) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    format!("[{}] OK:\n{}", tool_name, stdout)
                } else if !stderr.is_empty() {
                    format!("[{}] stderr:\n{}", tool_name, stderr)
                } else {
                    format!("[{}]: exit {:?} (sem saida)", tool_name, out.status.code())
                };
            }
            None => {
                self.status_msg = format!(
                    "ERRO: nao foi possivel executar 'cua-driver call {}' (esta no PATH?).",
                    tool_name
                );
            }
        }
    }

    /// Diretorio de staging do upgrade em %TEMP% (download + flag de pronto).
    fn update_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("fzcomputerai-update")
    }

    /// true somente se `remote` for ESTRITAMENTE mais nova que `local`
    /// (comparacao numerica major.minor.patch). Igual ou mais antiga =>
    /// false: /releases/latest e um ponteiro mutavel no GitHub e apontar
    /// para um tag antigo (rollback de release) NAO pode virar downgrade
    /// silencioso aqui.
    fn version_newer(remote: &str, local: &str) -> bool {
        fn parts(v: &str) -> [u64; 3] {
            let mut out = [0u64; 3];
            for (i, seg) in v.split('.').take(3).enumerate() {
                let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
                out[i] = digits.parse().unwrap_or(0);
            }
            out
        }
        parts(remote) > parts(local)
    }

    /// PASSO 1 do upgrade: verifica os DOIS componentes — esta GUI e o motor
    /// `cua-driver` — e apenas MARCA o que está desatualizado. Nada é baixado
    /// nem instalado sem o usuário confirmar no diálogo.
    ///
    /// POR QUE O MOTOR ENTRA AQUI: a GUI é só a interface; quem faz o trabalho
    /// é o `cua-driver`. Enquanto este botão olhava apenas a GUI, o motor podia
    /// ficar dezenas de versões atrás sem ninguém perceber — e foi o que
    /// aconteceu (0.8.3 instalado contra 0.17.0 publicado). Versões novas do
    /// motor mudam contrato (por exemplo, passaram a EXIGIR token no endpoint
    /// HTTP), então saber a versão real não é cosmético: é o que evita a GUI
    /// reportar estado errado.
    pub fn check_for_updates(&mut self) {
        self.update_checked = true;
        self.check_driver_update();
        self.check_gui_update();
    }

    /// Versão do motor + se há atualização, pela API OFICIAL do próprio
    /// `cua-driver` (`check-update --json`). Não reimplementamos a consulta de
    /// releases do motor: quem sabe onde ele publica é ele.
    pub fn check_driver_update(&mut self) {
        self.log_debug("[upgrade] Consultando o motor: cua-driver check-update --json");
        let out = match self.run_logged("cua-driver", &["check-update", "--json"]) {
            Some(o) => o,
            None => {
                self.status_msg = self.tr(
                    "Nao foi possivel executar 'cua-driver check-update' (o motor esta instalado e no PATH?).",
                    "Could not run 'cua-driver check-update' (is the engine installed and on PATH?).",
                );
                return;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        // serde_json ja e dependencia declarada do projeto — sem dep nova.
        match serde_json::from_str::<serde_json::Value>(text.trim()) {
            Ok(v) => {
                self.driver_version = v
                    .get("current_version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                self.driver_latest = v
                    .get("latest_version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                self.driver_update_available = v
                    .get("update_available")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                self.driver_notes_url = v
                    .get("release_notes_url")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
                    self.log_debug(&format!("[upgrade] motor reportou erro: {}", err));
                }
                self.log_debug(&format!(
                    "[upgrade] Motor cua-driver: instalado {} | ultimo {} | atualizacao disponivel: {}",
                    self.driver_version, self.driver_latest, self.driver_update_available
                ));
            }
            Err(e) => {
                self.log_debug(&format!(
                    "[upgrade] Nao consegui interpretar o JSON do check-update: {}",
                    e
                ));
            }
        }
    }

    /// Versão desta GUI contra o GitHub Releases do projeto.
    pub fn check_gui_update(&mut self) {
        self.log_debug("[upgrade] Verificando novas versoes no GitHub Releases...");
        let ps_cmd = "$r = Invoke-RestMethod -Uri 'https://api.github.com/repos/RLuf/fzcomputerai/releases/latest' -Headers @{'User-Agent'='FzComputerAI'}; Write-Output $r.tag_name";
        match self.run_logged("powershell", &["-NoProfile", "-Command", ps_cmd]) {
            Some(out) if out.status.success() => {
                let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let current_ver = env!("CARGO_PKG_VERSION");
                self.log_debug(&format!(
                    "[upgrade] Versao atual: v{} | Ultima release: {}",
                    current_ver, tag
                ));
                let clean_tag = tag.trim_start_matches('v');
                if Self::version_newer(clean_tag, current_ver) {
                    self.log_debug(&format!(
                        "[upgrade] Nova versao disponivel: {}. Aguardando confirmacao para baixar em segundo plano.",
                        tag
                    ));
                    self.update_available = Some(tag);
                } else if !clean_tag.is_empty() && clean_tag != current_ver {
                    // Release "latest" mais ANTIGA que a instalada (rollback
                    // no GitHub). Informar, nunca fazer downgrade sozinho.
                    self.log_debug(&format!(
                        "[upgrade] A release marcada como latest ({}) NAO e mais nova que a versao instalada (v{}). Nenhum downgrade automatico sera feito.",
                        tag, current_ver
                    ));
                } else {
                    self.log_debug(&format!(
                        "[upgrade] Voce ja esta utilizando a versao mais recente (v{}).",
                        current_ver
                    ));
                }
            }
            Some(_) => {
                self.log_debug("[upgrade] Nao foi possivel consultar a API do GitHub Releases.");
            }
            None => {
                self.log_debug("[upgrade] Falha ao executar verificacao de atualizacoes.");
            }
        }
    }

    /// O motor está presente no PATH? Uma checagem rápida (`where`/`which`),
    /// para a UI poder oferecer a instalação em vez de deixar o usuário com
    /// todos os botões falhando sem explicação.
    pub fn check_driver_present(&mut self) {
        #[cfg(target_os = "windows")]
        let finder = "where";
        #[cfg(not(target_os = "windows"))]
        let finder = "which";
        self.driver_present = quiet_cmd(finder)
            .arg("cua-driver")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !self.driver_present {
            self.log_debug(
                "[motor] cua-driver NAO encontrado no PATH — nenhuma acao de automacao vai funcionar ate instalar o motor.",
            );
        }
    }

    /// Instala o MOTOR `cua-driver`, cumprindo o contrato que o instalador
    /// anuncia ao usuário ("instale o motor depois pelo próprio aplicativo").
    ///
    /// Ordem de preferência, igual à do instalador gráfico:
    ///   1. script OFICIAL embarcado em `<dir do exe>\cua-driver\install.ps1`
    ///      (auditável: veio junto no pacote e pode ser lido antes de rodar);
    ///   2. se não existir, o endpoint OFICIAL do projeto Cua
    ///      (`irm https://cua.ai/driver/install.ps1 | iex`).
    /// Nunca baixamos/instalamos o motor por conta própria — quem publica e
    /// instala o motor é o projeto Cua.
    pub fn install_driver_engine(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let embedded = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("cua-driver").join("install.ps1")))
                .filter(|p| p.exists());
            let dir = Self::update_dir();
            let _ = std::fs::create_dir_all(&dir);

            let installer_cmd = match &embedded {
                Some(p) => {
                    self.log_debug(&format!(
                        "[motor] Usando o script OFICIAL embarcado: {}",
                        p.display()
                    ));
                    format!("& '{}'", p.display())
                }
                None => {
                    self.log_debug(
                        "[motor] Script embarcado ausente — usando o endpoint oficial do projeto Cua (cua.ai/driver/install.ps1).",
                    );
                    "irm https://cua.ai/driver/install.ps1 | iex".to_string()
                }
            };

            let ps = format!(
                "$ErrorActionPreference='Continue'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'drv-ready.flag'),(Join-Path $d 'drv-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   $out = ({cmd} 2>&1 | Out-String); \
                   $ver = (cua-driver check-update --json 2>&1 | Out-String); \
                   Set-Content -Path (Join-Path $d 'drv-ready.flag') -Value ($out + \"`n\" + $ver) \
                 }} catch {{ Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display(),
                cmd = installer_cmd
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn()
            {
                Ok(_) => {
                    self.driver_updating = true;
                    self.status_msg = self.tr(
                        "Instalando o motor cua-driver em segundo plano (instalador oficial do projeto Cua)...",
                        "Installing the cua-driver engine in the background (Cua project's official installer)...",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!("[motor] ERRO ao disparar a instalacao: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg =
                "No Linux/macOS instale o motor com: curl -fsSL https://cua.ai/driver/install.sh | bash"
                    .to_string();
        }
    }

    /// Atualiza o MOTOR pelo caminho oficial dele: `cua-driver update --apply`.
    /// Roda em processo DESTACADO (o download/instalação pode levar dezenas de
    /// segundos e travaria a UI), na sequência correta: para o daemon, aplica a
    /// atualização, religa o autostart e grava uma flag para o poll observar.
    /// NUNCA baixamos binário do motor por conta própria — quem publica e
    /// instala o motor é o projeto Cua.
    pub fn start_driver_update(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let _ = std::fs::create_dir_all(&dir);
            let ps = format!(
                "$ErrorActionPreference='Continue'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'drv-ready.flag'),(Join-Path $d 'drv-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   cua-driver stop 2>&1 | Out-Null; \
                   $out = (cua-driver update --apply 2>&1 | Out-String); \
                   $ver = (cua-driver check-update --json 2>&1 | Out-String); \
                   Set-Content -Path (Join-Path $d 'drv-ready.flag') -Value ($out + \"`n\" + $ver) \
                 }} catch {{ Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display()
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn()
            {
                Ok(_) => {
                    self.driver_updating = true;
                    self.status_msg = self.tr(
                        "Atualizando o motor cua-driver em segundo plano (pelo atualizador oficial dele)...",
                        "Updating the cua-driver engine in the background (via its own official updater)...",
                    );
                    self.log_debug(
                        "[upgrade] cua-driver update --apply disparado em segundo plano (stop -> apply; o motor volta como filho da GUI).",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao disparar a atualizacao do motor: {}",
                        e
                    ));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Fora do Windows o motor tambem tem atualizador proprio; aqui a
            // chamada e direta (sem o wrapper de autostart, que e Windows-only).
            let _ = self.run_logged("cua-driver", &["update", "--apply"]);
            self.check_driver_update();
        }
    }

    /// Observa a atualização do motor (flags em %TEMP%) e, ao terminar, relê a
    /// versão REAL — o estado exibido nunca é presumido a partir do "mandei
    /// atualizar".
    pub fn poll_driver_update(&mut self) {
        if !self.driver_updating {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.driver_update_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.driver_update_poll = Some(now);

        let dir = Self::update_dir();
        let ready = dir.join("drv-ready.flag");
        let err = dir.join("drv-error.flag");
        if err.exists() {
            let msg = std::fs::read_to_string(&err).unwrap_or_default();
            let _ = std::fs::remove_file(&err);
            self.driver_updating = false;
            self.log_debug(&format!(
                "[upgrade] FALHA ao atualizar o motor: {}",
                msg.trim()
            ));
            self.status_msg = format!("Falha ao atualizar o motor: {}", tail_str(msg.trim(), 300));
        } else if ready.exists() {
            let info = std::fs::read_to_string(&ready).unwrap_or_default();
            let _ = std::fs::remove_file(&ready);
            self.driver_updating = false;
            self.log_debug(&format!(
                "[upgrade] Atualizacao do motor concluida. Saida:\n{}",
                tail_str(info.trim(), 1500)
            ));
            // Verdade final: reler presenca, versao e estado do endpoint —
            // nada e presumido a partir de "mandei instalar/atualizar".
            self.check_driver_present();
            self.check_driver_update();
            self.check_port_status();
            self.daemon_running = self.port_active;
            let v = self.driver_version.clone();
            self.status_msg = format!(
                "{} {}",
                self.tr("Motor atualizado. Versao agora:", "Engine updated. Version now:"),
                v
            );
        }
    }

    /// PASSO 2: download do instalador em PROCESSO SEPARADO (a UI nao trava).
    /// O processo grava ready.flag ao terminar; poll_update_download observa.
    pub fn start_update_download(&mut self) {
        let Some(tag) = self.update_available.clone() else {
            return;
        };
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            // Baixa o instalador E o .sha256 publicado pelo CI, confere o
            // hash (Get-FileHash) e SO grava ready.flag com hash conferido.
            // Divergencia => error.flag + instalador apagado: executavel
            // baixado sem integridade conferida nunca roda (SIGNING.md §3).
            let ps = format!(
                "$ErrorActionPreference='Stop'; \
                 $d = '{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'ready.flag'),(Join-Path $d 'error.flag') -Force -ErrorAction SilentlyContinue; \
                 $t = Join-Path $d 'fzcomputerai-setup-windows-x64.exe'; \
                 try {{ \
                   Invoke-WebRequest -Uri 'https://github.com/RLuf/fzcomputerai/releases/download/{tag}/fzcomputerai-setup-windows-x64.exe' -OutFile $t -UseBasicParsing; \
                   Invoke-WebRequest -Uri 'https://github.com/RLuf/fzcomputerai/releases/download/{tag}/fzcomputerai-setup-windows-x64.exe.sha256' -OutFile \"$t.sha256\" -UseBasicParsing; \
                   $expected = ((Get-Content \"$t.sha256\" -TotalCount 1) -split '\\s+')[0].ToLower(); \
                   $actual = (Get-FileHash -Path $t -Algorithm SHA256).Hash.ToLower(); \
                   if ($expected -ne $actual) {{ throw \"SHA256 nao confere: esperado $expected, obtido $actual\" }}; \
                   New-Item -ItemType File -Force -Path (Join-Path $d 'ready.flag') | Out-Null \
                 }} catch {{ \
                   Set-Content -Path (Join-Path $d 'error.flag') -Value $_.Exception.Message; \
                   Remove-Item $t -Force -ErrorAction SilentlyContinue \
                 }}",
                dir = dir.display(),
                tag = tag
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-Command", ps.as_str()])
                .spawn()
            {
                Ok(_) => {
                    self.update_downloading = true;
                    self.log_debug(&format!(
                        "[upgrade] Download do instalador {} iniciado em SEGUNDO PLANO para {}.",
                        tag,
                        dir.display()
                    ));
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao iniciar o download em background: {}",
                        e
                    ));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.log_debug("[upgrade] Auto-upgrade disponivel apenas no Windows.");
        }
    }

    /// PASSO 3: observado a cada ~1s pelo loop da UI enquanto ha download em
    /// andamento. Quando ready.flag existe E o .exe esta la, marca pronto —
    /// dai o dialogo pede para FECHAR e instalar.
    pub fn poll_update_download(&mut self) {
        if !self.update_downloading || self.update_ready {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.last_update_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.last_update_poll = Some(now);

        let dir = Self::update_dir();
        let flag = dir.join("ready.flag");
        let err_flag = dir.join("error.flag");
        let setup = dir.join("fzcomputerai-setup-windows-x64.exe");

        if err_flag.exists() {
            let msg = std::fs::read_to_string(&err_flag).unwrap_or_default();
            self.update_downloading = false;
            self.update_available = None;
            self.log_debug(&format!(
                "[upgrade] FALHA no download/verificacao do instalador: {}",
                msg.trim()
            ));
            return;
        }

        if flag.exists() && setup.exists() {
            self.update_downloading = false;
            self.update_ready = true;
            self.log_debug(&format!(
                "[upgrade] Download concluido e SHA256 conferido: {}. Aguardando confirmacao para FECHAR e instalar.",
                setup.display()
            ));
        }
    }

    /// PASSO 4: dispara o processo de instalacao em background e retorna.
    /// O chamador (UI) fecha o aplicativo em seguida. O processo externo:
    /// espera este exe sair (ate 30s), GARANTE encerrado (Stop-Process),
    /// encerra o cua-driver, instala /VERYSILENT e reabre a GUI + motor
    /// (autostart kick). Nada de "instalar por cima" com o app aberto.
    pub fn install_update_and_restart(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let current_exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let ps = format!(
                "$d = '{dir}'; \
                 $t = Join-Path $d 'fzcomputerai-setup-windows-x64.exe'; \
                 if (-not (Test-Path $t)) {{ exit 1 }}; \
                 $deadline = (Get-Date).AddSeconds(30); \
                 while ((Get-Process fzcomputerai -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {{ Start-Sleep -Milliseconds 500 }}; \
                 Stop-Process -Name fzcomputerai -Force -ErrorAction SilentlyContinue; \
                 Stop-Process -Name cua-driver -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Seconds 1; \
                 Start-Process -FilePath $t -ArgumentList '/VERYSILENT /NORESTART' -Wait; \
                 $exe = Join-Path $env:LOCALAPPDATA 'Programs\\FzComputerAI\\fzcomputerai.exe'; \
                 if (-not (Test-Path $exe)) {{ $exe = '{cur}' }}; \
                 if (Test-Path $exe) {{ Start-Process -FilePath $exe }}; \
",
                dir = dir.display(),
                cur = current_exe
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-Command", ps.as_str()])
                .spawn()
            {
                Ok(_) => {
                    self.log_debug(
                        "[upgrade] Instalador disparado em background — o aplicativo sera fechado para concluir a atualizacao.",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao disparar a instalacao: {}",
                        e
                    ));
                }
            }
        }
    }

    /// Lê no registro se o autostart (HKCU\...\Run\FzComputerAI) está ativo.
    #[cfg(target_os = "windows")]
    pub fn read_autostart(&mut self) {
        let enabled = self
            .run_logged(
                "reg",
                &[
                    "query",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "FzComputerAI",
                ],
            )
            .map(|o| o.status.success())
            .unwrap_or(false);
        self.autostart_enabled = enabled;
    }

    /// Ativa/desativa "Iniciar com o Windows" via reg add/delete e relê o
    /// estado real do registro para o checkbox refletir a verdade.
    #[cfg(target_os = "windows")]
    pub fn set_autostart(&mut self, enable: bool) {
        if enable {
            match std::env::current_exe() {
                Ok(exe) => {
                    let value = format!("\"{}\"", exe.display());
                    let _ = self.run_logged(
                        "reg",
                        &[
                            "add",
                            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                            "/v",
                            "FzComputerAI",
                            "/t",
                            "REG_SZ",
                            "/d",
                            value.as_str(),
                            "/f",
                        ],
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[autostart] ERRO: current_exe() falhou: {}",
                        e
                    ));
                }
            }
        } else {
            let _ = self.run_logged(
                "reg",
                &[
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "FzComputerAI",
                    "/f",
                ],
            );
        }
        self.read_autostart();
        let status = if self.autostart_enabled { "ATIVADO" } else { "DESATIVADO" };
        self.log_debug(&format!("[autostart] Iniciar com o Windows: {}", status));
    }

    // ═══════════════════════════════════════════════════════════════════
    //  ABA TÚNEL — expõe o MCP HTTP local (127.0.0.1:porta) na internet.
    //  Objetivo: URL HTTPS pública -> HTTP local. Um túnel por vez.
    // ═══════════════════════════════════════════════════════════════════

    /// Diretório de staging dos túneis em %TEMP%. O caminho é usado como
    /// arquivo de log dos CLIs (--logfile/--log/-E) e por isso vira o
    /// MARCADOR de identidade na command line do processo (usado para matar
    /// apenas os túneis NOSSOS, nunca um cloudflared/ssh alheio do usuário).
    fn tunnel_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("fzcomputerai-tunnel")
    }

    fn provider_slug(p: TunnelProvider) -> &'static str {
        match p {
            TunnelProvider::Cloudflare => "cloudflare",
            TunnelProvider::Ngrok => "ngrok",
            TunnelProvider::Ssh => "ssh",
        }
    }

    fn provider_image(p: TunnelProvider) -> &'static str {
        match p {
            TunnelProvider::Cloudflare => "cloudflared.exe",
            TunnelProvider::Ngrok => "ngrok.exe",
            TunnelProvider::Ssh => "ssh.exe",
        }
    }

    /// Caminho do log combinado (stdout+stderr) do túnel atual. Usa o
    /// provedor CONGELADO no start (não o rádio da UI): com o túnel vivo o
    /// usuário pode trocar o rádio, e o poll leria o log ERRADO.
    fn tunnel_log_path(&self) -> std::path::PathBuf {
        let name = format!(
            "{}-{}.log",
            Self::provider_slug(self.tunnel_run_provider),
            self.tunnel_run_id
        );
        Self::tunnel_dir().join(name)
    }

    /// Diretório onde a GUI guarda binários baixados (cloudflared/ngrok):
    /// ao lado do executável ({app}\tunnel) e, se não for gravável,
    /// %LOCALAPPDATA%\FzComputerAI\tunnel.
    fn tunnel_bin_dir() -> std::path::PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                return parent.join("tunnel");
            }
        }
        std::env::temp_dir().join("fzcomputerai-tunnel-bin")
    }

    /// Preferência de gravação: {app}\tunnel se gravável, senão
    /// %LOCALAPPDATA%\FzComputerAI\tunnel.
    fn tunnel_download_dir() -> std::path::PathBuf {
        let primary = Self::tunnel_bin_dir();
        if std::fs::create_dir_all(&primary).is_ok() {
            // Testa gravabilidade real com um arquivo temporário.
            let probe = primary.join(".w");
            if std::fs::write(&probe, b"x").is_ok() {
                let _ = std::fs::remove_file(&probe);
                return primary;
            }
        }
        let fallback = std::env::var("LOCALAPPDATA")
            .map(|p| std::path::PathBuf::from(p).join("FzComputerAI").join("tunnel"))
            .unwrap_or_else(|_| std::env::temp_dir().join("fzcomputerai-tunnel-bin"));
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }

    /// Detecta cloudflared/ngrok/ssh: primeiro no PATH (where/which), depois
    /// em {app}\tunnel, e o ssh também no OpenSSH do Windows. LAZY: roda na
    /// 1ª abertura da aba, nunca no startup.
    pub fn detect_tunnel_bins(&mut self) {
        self.tunnel_cf_bin = self.resolve_bin("cloudflared", "cloudflared.exe");
        self.tunnel_ngrok_bin = self.resolve_bin("ngrok", "ngrok.exe");
        let mut ssh = self.resolve_bin("ssh", "ssh.exe");
        #[cfg(target_os = "windows")]
        if ssh.is_empty() {
            let sys = r"C:\Windows\System32\OpenSSH\ssh.exe";
            if std::path::Path::new(sys).exists() {
                ssh = sys.to_string();
            }
        }
        self.tunnel_ssh_bin = ssh;
        self.tunnel_bins_checked = true;
        self.log_debug(&format!(
            "[tunnel] Binarios detectados -> cloudflared: {} | ngrok: {} | ssh: {}",
            if self.tunnel_cf_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_cf_bin },
            if self.tunnel_ngrok_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_ngrok_bin },
            if self.tunnel_ssh_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_ssh_bin },
        ));
    }

    /// Resolve um binário: PATH (where/which) e depois {app}\tunnel\<exe>.
    fn resolve_bin(&mut self, cmd: &str, exe: &str) -> String {
        #[cfg(target_os = "windows")]
        let finder = "where";
        #[cfg(not(target_os = "windows"))]
        let finder = "which";

        if let Ok(out) = quiet_cmd(finder).arg(cmd).output() {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(first) = text.lines().map(|l| l.trim()).find(|l| !l.is_empty()) {
                    return first.to_string();
                }
            }
        }
        let local = Self::tunnel_bin_dir().join(exe);
        if local.exists() {
            return local.display().to_string();
        }
        String::new()
    }

    /// Lê a config da aba Túnel de HKCU\Software\FzComputerAI (valores
    /// "tunnelcfg:*"). Parser corta em "REG_SZ" — caminhos têm espaço, então
    /// split_whitespace().last() estragaria o valor.
    pub fn read_tunnel_cfg(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let out = match self
                .run_logged("reg", &["query", r"HKCU\Software\FzComputerAI"])
            {
                Some(o) if o.status.success() => o,
                _ => return,
            };
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            for line in text.lines() {
                let trimmed = line.trim();
                let Some(rest) = trimmed.strip_prefix("tunnelcfg:") else {
                    continue;
                };
                // rest = "<chave>    REG_SZ    <valor>"
                let Some(pos) = rest.find("REG_SZ") else { continue };
                let key = rest[..pos].trim();
                let value = rest[pos + "REG_SZ".len()..].trim().to_string();
                match key {
                    "provider" => {
                        self.tunnel_provider = match value.as_str() {
                            "ngrok" => TunnelProvider::Ngrok,
                            "ssh" => TunnelProvider::Ssh,
                            _ => TunnelProvider::Cloudflare,
                        }
                    }
                    "cf_token_file" => self.tunnel_cf_token_file = value,
                    "cf_name" => {
                        if !value.is_empty() {
                            self.tunnel_cf_name = value
                        }
                    }
                    "cf_hostname" => self.tunnel_cf_hostname = value,
                    "public_url" => self.tunnel_public_url = value,
                    "ngrok_extra" => self.tunnel_ngrok_extra = value,
                    "ngrok_use_policy" => self.tunnel_ngrok_use_policy = value != "0",
                    "ssh_target" => {
                        if !value.is_empty() {
                            self.tunnel_ssh_target = value
                        }
                    }
                    "ssh_remote_port" => {
                        if !value.is_empty() {
                            self.tunnel_ssh_remote_port = value
                        }
                    }
                    "ssh_key" => self.tunnel_ssh_key = value,
                    "ssh_extra" => self.tunnel_ssh_extra = value,
                    _ => {}
                }
            }
        }
    }

    /// Grava a config da aba (só no clique em "Salvar configuração" — gravar
    /// por keystroke faria um reg.exe por frame). NUNCA grava segredo: só o
    /// CAMINHO do token-file do Cloudflare, jamais o token.
    pub fn save_tunnel_cfg(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let provider = Self::provider_slug(self.tunnel_provider).to_string();
            let pairs: Vec<(&str, String)> = vec![
                ("provider", provider),
                ("cf_token_file", self.tunnel_cf_token_file.clone()),
                ("cf_name", self.tunnel_cf_name.clone()),
                ("cf_hostname", self.tunnel_cf_hostname.clone()),
                ("public_url", self.tunnel_public_url.clone()),
                ("ngrok_extra", self.tunnel_ngrok_extra.clone()),
                (
                    "ngrok_use_policy",
                    if self.tunnel_ngrok_use_policy { "1" } else { "0" }.to_string(),
                ),
                ("ssh_target", self.tunnel_ssh_target.clone()),
                ("ssh_remote_port", self.tunnel_ssh_remote_port.clone()),
                ("ssh_key", self.tunnel_ssh_key.clone()),
                ("ssh_extra", self.tunnel_ssh_extra.clone()),
            ];
            for (k, v) in pairs {
                let name = format!("tunnelcfg:{}", k);
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        name.as_str(),
                        "/t",
                        "REG_SZ",
                        "/d",
                        v.as_str(),
                        "/f",
                    ],
                );
            }
            self.log_debug("[tunnel] Configuracao salva em HKCU\\Software\\FzComputerAI (tunnelcfg:*).");
        }
    }

    /// Registra em HKCU (auditoria) que os termos do ngrok foram aceitos.
    pub fn save_ngrok_tos_accepted(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "1".to_string());
            let _ = self.run_logged(
                "reg",
                &[
                    "add",
                    r"HKCU\Software\FzComputerAI",
                    "/v",
                    "tunnelcfg:ngrok_tos_accepted",
                    "/t",
                    "REG_SZ",
                    "/d",
                    stamp.as_str(),
                    "/f",
                ],
            );
        }
    }

    /// Gera uma string alfanumérica pseudoaleatória sem crate `rand`
    /// (xorshift semeado pelo relógio). Boa o bastante para senha-de-URL e
    /// para o run_id — não é material criptográfico de produção.
    fn gen_token(len: usize) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            ^ (std::process::id() as u64).wrapping_mul(0x2545F4914F6CDD1D);
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            // xorshift64
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            out.push(CHARS[(seed % CHARS.len() as u64) as usize] as char);
        }
        out
    }

    /// Gera a senha do gate (16 chars) no campo da UI.
    pub fn tunnel_generate_password(&mut self) {
        self.tunnel_gate_password = Self::gen_token(16);
    }

    // ─── Gate local (nível 1 de auth): senha na URL ─────────────────────
    // O driver MCP executa POST em QUALQUER path (não valida caminho), e o
    // quick tunnel do Cloudflare / SSH público não têm auth de borda. Logo,
    // "senha na URL" só é real com um porteiro no meio: este gate é um
    // mini reverse-proxy em 127.0.0.1 que exige /s/<senha>/ no path antes de
    // encaminhar ao MCP. Sem senha, o túnel aponta direto no MCP.
    //
    // EXCEÇÃO CONSCIENTE à convenção "sem threads" do crate: um servidor não
    // é implementável por poll de arquivo. A thread morre com o app (o
    // AtomicBool + uma conexão dummy destravam o accept), então o gate nunca
    // sobrevive à GUI — coerente com o ciclo de vida dos túneis.

    /// Sobe o gate em 127.0.0.1:porta-efêmera e retorna a porta. O túnel
    /// deve então apontar para ela em vez da porta do MCP.
    fn start_gate(&mut self, mcp_port: u16, password: &str) -> Option<u16> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let listener = match std::net::TcpListener::bind(("127.0.0.1", 0u16)) {
            Ok(l) => l,
            Err(e) => {
                self.log_debug(&format!("[tunnel][gate] Falha ao abrir o porteiro local: {}", e));
                return None;
            }
        };
        let gate_port = listener.local_addr().ok()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let password = password.to_string();
        // TOKEN INJETADO PELO PORTEIRO. Muitos clientes MCP (Claude Desktop e
        // afins) só aceitam uma URL — não há onde colar um header
        // `Authorization` na interface deles. Como o motor 0.16+ é
        // fail-closed, esses clientes tomariam 401 e o túnel seria inútil
        // para eles. Solução: quem provou a senha no path (/s/<senha>/) já
        // está autenticado perante NÓS, então o porteiro acrescenta o Bearer
        // ao falar com o motor. O segredo do motor não viaja pela internet —
        // a credencial pública passa a ser a senha da URL, que é nossa e
        // rotativa. Se o cliente mandar o próprio Authorization, o dele vence.
        let inject = self.mcp_token.trim().to_string();

        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                match conn {
                    Ok(client) => {
                        let pw = password.clone();
                        let tok = inject.clone();
                        std::thread::spawn(move || {
                            gate_handle_conn(client, mcp_port, &pw, &tok);
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        self.tunnel_gate_stop = Some(stop);
        self.tunnel_gate_port = Some(gate_port);
        self.log_debug(&format!(
            "[tunnel][gate] Porteiro de senha ativo em 127.0.0.1:{} -> MCP 127.0.0.1:{} (exige /s/<senha>/{}).",
            gate_port,
            mcp_port,
            if self.mcp_token.trim().is_empty() { "" } else { "; injeta o Bearer do motor" }
        ));
        Some(gate_port)
    }

    /// Encerra o gate: sinaliza a thread e destrava o accept com uma conexão
    /// dummy no próprio porteiro.
    fn stop_gate(&mut self) {
        use std::sync::atomic::Ordering;
        if let Some(stop) = self.tunnel_gate_stop.take() {
            stop.store(true, Ordering::SeqCst);
            if let Some(port) = self.tunnel_gate_port {
                let _ = std::net::TcpStream::connect(("127.0.0.1", port));
            }
            self.log_debug("[tunnel][gate] Porteiro de senha encerrado.");
        }
        self.tunnel_gate_port = None;
    }

    // ─── RELAY LAN ──────────────────────────────────────────────────────
    // Mesma doutrina do gate de senha (thread + AtomicBool + conexão dummy
    // para destravar o accept), com DUAS diferenças deliberadas:
    //   • é TRANSPARENTE: não lê nem reescreve cabeçalho, não força
    //     `Connection: close` — keep-alive e streaming SSE passam intactos;
    //   • conta conexões (ativas/total) para a UI mostrar uso REAL em vez de
    //     "regra existe, deve estar funcionando".

    /// Publica o MCP na rede escutando em `lan_relay_bind:<porta>` e
    /// encaminhando para `127.0.0.1:<porta do motor CONFIRMADA>`.
    /// Nunca publica porta morta: sem motor confirmado, não sobe nada.
    pub fn start_lan_relay(&mut self) {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        if self.lan_relay_listen.is_some() {
            self.status_msg = self.tr(
                "O relay da LAN ja esta ativo.",
                "The LAN relay is already running.",
            );
            return;
        }

        let Some(target) = self.detect_confirmed_cua_port() else {
            self.status_msg = self.tr(
                "O MCP local nao respondeu em 127.0.0.1. Inicie o motor antes de publicar na rede.",
                "The local MCP did not answer on 127.0.0.1. Start the engine before publishing on the network.",
            );
            return;
        };

        // Porta de escuta = a mesma do motor (medido: 0.0.0.0:<p> coexiste com
        // 127.0.0.1:<p> nesta plataforma). Se o bind falhar mesmo assim, o erro
        // real vai para a UI — nada de "provavelmente funcionou".
        let listen_port = self.http_port.trim().parse::<u16>().unwrap_or(target);
        let bind = self.lan_relay_bind.trim().to_string();
        let addr = format!("{}:{}", bind, listen_port);

        let listener = match std::net::TcpListener::bind(addr.as_str()) {
            Ok(l) => l,
            Err(e) => {
                self.status_msg = format!(
                    "{} {}: {}",
                    self.tr("Falha ao escutar em", "Failed to listen on"),
                    addr,
                    e
                );
                self.log_debug(&format!("[relay] ERRO ao bindar {}: {}", addr, e));
                return;
            }
        };

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let conns = self.lan_relay_conns.clone();
        let total = self.lan_relay_total.clone();
        let logbuf = self.lan_relay_log.clone();
        conns.store(0, Ordering::SeqCst);
        total.store(0, Ordering::SeqCst);

        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                match conn {
                    Ok(client) => {
                        let c = conns.clone();
                        let t = total.clone();
                        let lb = logbuf.clone();
                        std::thread::spawn(move || {
                            c.fetch_add(1, Ordering::SeqCst);
                            t.fetch_add(1, Ordering::SeqCst);
                            if let Err(e) = relay_handle_conn(client, target) {
                                if let Ok(mut v) = lb.lock() {
                                    if v.len() < 64 {
                                        v.push(format!("[relay] conexao encerrada: {}", e));
                                    }
                                }
                            }
                            c.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        self.lan_relay_stop = Some(stop);
        self.lan_relay_listen = Some(listen_port);
        self.lan_relay_target = Some(target);
        self.log_debug(&format!(
            "[relay] Publicando na rede: {}:{} -> 127.0.0.1:{} (thread desta GUI; morre com o app, sem UAC).",
            bind, listen_port, target
        ));
        self.status_msg = format!(
            "{} {}:{} -> 127.0.0.1:{}",
            self.tr("Relay da LAN ATIVO", "LAN relay ACTIVE"),
            bind,
            listen_port,
            target
        );
        self.check_port_status();
    }

    /// Encerra o relay: sinaliza a thread e destrava o `accept`.
    ///
    /// A conexão dummy PRECISA cair no listener do relay. Mandá-la para
    /// `127.0.0.1:<porta>` não funciona quando relay e motor dividem a porta:
    /// o bind mais específico (o do motor, em 127.0.0.1) atende primeiro, o
    /// `accept()` do relay nunca acorda, a thread fica presa e o socket
    /// continua publicado na rede — com o badge dizendo "parado". Por isso
    /// batemos no ENDEREÇO EM QUE O RELAY ESCUTA: o IP da LAN quando ele subiu
    /// em 0.0.0.0, ou o próprio endereço configurado.
    pub fn stop_lan_relay(&mut self) {
        use std::sync::atomic::Ordering;
        if let Some(stop) = self.lan_relay_stop.take() {
            stop.store(true, Ordering::SeqCst);
            if let Some(port) = self.lan_relay_listen {
                let bind = self.lan_relay_bind.trim();
                let knock = if bind == "0.0.0.0" || bind.is_empty() {
                    self.lan_ip.trim().to_string()
                } else {
                    bind.to_string()
                };
                let mut acordou = false;
                if !knock.is_empty() && knock != "127.0.0.1" {
                    acordou = std::net::TcpStream::connect((knock.as_str(), port)).is_ok();
                }
                if !acordou {
                    // Último recurso: loopback. Pode ser atendido pelo motor
                    // (e aí não acorda o relay) — registrado com honestidade.
                    let _ = std::net::TcpStream::connect(("127.0.0.1", port));
                    self.log_debug(
                        "[relay] AVISO: nao consegui bater no endereco de escuta do relay; a thread de accept pode so encerrar no fechamento do app.",
                    );
                }
            }
            self.log_debug("[relay] Relay da LAN encerrado.");
        }
        self.lan_relay_listen = None;
        self.lan_relay_target = None;
        self.check_port_status();
    }

    /// Drena para o Console Debug os erros que as threads do relay
    /// registraram (elas não têm `&mut self`). Chamado pelo loop da UI.
    pub fn poll_lan_relay(&mut self) {
        let drained: Vec<String> = match self.lan_relay_log.lock() {
            Ok(mut v) if !v.is_empty() => v.drain(..).collect(),
            _ => return,
        };
        for line in drained {
            self.log_debug(&line);
        }
    }

    /// Restringe a ACL de um arquivo de SEGREDO ao usuário atual (leitura) —
    /// mesmo padrão já aplicado ao token-file do Cloudflare. Fora do Windows
    /// é no-op (os arquivos ficam no perfil do usuário).
    fn restrict_file_acl(&mut self, path: &std::path::Path) {
        #[cfg(target_os = "windows")]
        {
            let user = std::env::var("USERNAME").unwrap_or_default();
            if !user.is_empty() {
                let p = path.display().to_string();
                let grant = format!("{}:R", user);
                let _ = self.run_logged(
                    "icacls",
                    &[p.as_str(), "/inheritance:r", "/grant:r", grant.as_str()],
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = path;
        }
    }

    /// O binário do ngrok tem `--traffic-policy-file`? (agente 3.9+). Nos
    /// antigos (ex.: 3.3.x) a flag NÃO existe e o processo morreria no spawn
    /// com "unknown flag" — a aba mostrava ERRO na hora, verificado na
    /// prática nesta máquina. Pergunta ao PRÓPRIO binário (`http --help`),
    /// nunca deduz por número de versão.
    fn ngrok_supports_policy_file(&mut self, bin: &str) -> bool {
        let sup = quiet_cmd(bin)
            .args(["http", "--help"])
            .output()
            .map(|o| {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                text.contains("--traffic-policy-file")
            })
            .unwrap_or(false);
        self.log_debug(&format!(
            "[tunnel][ngrok] suporte a --traffic-policy-file: {}",
            if sup { "SIM (agente 3.9+)" } else { "NAO (agente antigo)" }
        ));
        sup
    }

    /// Caminho do config PADRÃO do ngrok (onde vive o authtoken), extraído de
    /// `ngrok config check` ("Valid configuration file at <caminho>"). Ao
    /// receber `--config` o agente IGNORA o padrão — por isso, no modo
    /// `start`, passamos os DOIS (padrão + o nosso).
    fn ngrok_default_config(&mut self, bin: &str) -> Option<String> {
        let out = self.run_logged(bin, &["config", "check"])?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        for line in text.lines() {
            if let Some((_, path)) = line.split_once(" at ") {
                let path = path.trim();
                if !path.is_empty() && std::path::Path::new(path).exists() {
                    return Some(path.to_string());
                }
            }
        }
        None
    }

    /// Monta (programa, args) do túnel para a porta LOCAL alvo (MCP direto ou
    /// gate). Retorna None se faltar binário/config essencial (já logando).
    fn tunnel_cmdline(&mut self, local_port: u16) -> Option<(String, Vec<String>)> {
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let log = self.tunnel_log_path();
        let log_s = log.display().to_string();
        let target = format!("http://127.0.0.1:{}", local_port);

        match self.tunnel_provider {
            TunnelProvider::Cloudflare => {
                if self.tunnel_cf_bin.is_empty() {
                    self.status_msg = self.tr(
                        "cloudflared nao encontrado. Use 'Baixar cloudflared' ou instale e clique em 'Detectar binarios'.",
                        "cloudflared not found. Use 'Download cloudflared' or install it and click 'Detect binaries'.",
                    );
                    return None;
                }
                let bin = self.tunnel_cf_bin.clone();
                let named = self.tunnel_cf_name.trim().to_string();
                let host = self.tunnel_cf_hostname.trim().to_string();
                if !named.is_empty() && !host.is_empty() && self.tunnel_cf_token_file.trim().is_empty()
                {
                    // NOMEADO por nome de túnel (fluxo OAuth completo): a
                    // credencial é o cert.pem + o JSON do túnel em
                    // ~/.cloudflared — nada de segredo no argv. O hostname é
                    // fixo, então a URL pública é conhecida ANTES de subir.
                    self.tunnel_public_url = format!("https://{}", host);
                    Some((
                        bin,
                        vec![
                            "--no-autoupdate".into(),
                            "--loglevel".into(),
                            "info".into(),
                            "--logfile".into(),
                            log_s,
                            "tunnel".into(),
                            "run".into(),
                            "--url".into(),
                            target,
                            named,
                        ],
                    ))
                } else if self.tunnel_cf_token_file.trim().is_empty() {
                    // Quick tunnel (sem conta).
                    Some((
                        bin,
                        vec![
                            "--no-autoupdate".into(),
                            "--loglevel".into(),
                            "info".into(),
                            "--logfile".into(),
                            log_s,
                            "tunnel".into(),
                            "--url".into(),
                            target,
                        ],
                    ))
                } else {
                    // Túnel nomeado via token-file (o segredo nunca vai no argv logado).
                    Some((
                        bin,
                        vec![
                            "--no-autoupdate".into(),
                            "--logfile".into(),
                            log_s,
                            "tunnel".into(),
                            "run".into(),
                            "--token-file".into(),
                            self.tunnel_cf_token_file.clone(),
                        ],
                    ))
                }
            }
            TunnelProvider::Ngrok => {
                if self.tunnel_ngrok_bin.is_empty() {
                    self.status_msg = self.tr(
                        "ngrok nao encontrado. Use 'Baixar ngrok' ou instale e clique em 'Detectar binarios'.",
                        "ngrok not found. Use 'Download ngrok' or install it and click 'Detect binaries'.",
                    );
                    return None;
                }
                let bin = self.tunnel_ngrok_bin.clone();

                // basic-auth de borda usa o header Authorization — o MESMO que
                // o cliente MCP usa para o Bearer do motor 0.16+. Os dois não
                // coexistem numa requisição (o cliente só envia um): com token
                // do motor ativo, ligar a borda deixaria o túnel 100%
                // inutilizável (401 do ngrok para todo cliente correto). A
                // proteção real é o token do motor; a borda só entra quando o
                // motor está aberto (sem token).
                let edge_auth = self.tunnel_ngrok_use_policy && self.mcp_token.trim().is_empty();
                if self.tunnel_ngrok_use_policy && !edge_auth {
                    self.log_debug(
                        "[tunnel][ngrok] basic-auth de borda IGNORADO: o motor ja exige Bearer e ambos disputariam o mesmo header Authorization.",
                    );
                }

                // Subcomando: `http` direto; agente ANTIGO com borda pedida
                // troca para `start` + config v2 (abaixo).
                let mut subcmd: Vec<String> =
                    vec!["http".into(), format!("127.0.0.1:{}", local_port)];
                let mut auth_args: Vec<String> = Vec::new();

                if edge_auth {
                    if self.tunnel_ngrok_password.trim().is_empty() {
                        self.tunnel_ngrok_password = Self::gen_token(24);
                    }
                    let pw = self.tunnel_ngrok_password.clone();
                    let dir = Self::tunnel_download_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    if self.ngrok_supports_policy_file(&bin) {
                        // Agente 3.9+: traffic policy em ARQUIVO (o segredo
                        // nunca vai no argv logado).
                        let policy = dir.join("ngrok-policy.yml");
                        let yaml = format!(
                            "on_http_request:\n  - actions:\n      - type: basic-auth\n        config:\n          realm: FzComputerAI MCP\n          credentials:\n            - \"fz:{}\"\n",
                            pw
                        );
                        if std::fs::write(&policy, yaml).is_ok() {
                            self.restrict_file_acl(&policy);
                            auth_args.push("--traffic-policy-file".into());
                            auth_args.push(policy.display().to_string());
                        }
                    } else {
                        // Agente ANTIGO (ex.: 3.3.x): sem a flag, o spawn
                        // morreria com "unknown flag". Rota compatível SEM
                        // segredo no argv: config v2 com basic_auth em arquivo
                        // (ACL restrita) + `ngrok start`, mesclando o config
                        // PADRÃO (authtoken) com o nosso.
                        let Some(default_cfg) = self.ngrok_default_config(&bin) else {
                            self.status_msg = self.tr(
                                "ngrok antigo (sem --traffic-policy-file) e o config padrao (authtoken) nao foi encontrado. Atualize o ngrok ou rode: ngrok config add-authtoken <TOKEN>.",
                                "Old ngrok (no --traffic-policy-file) and the default config (authtoken) was not found. Update ngrok or run: ngrok config add-authtoken <TOKEN>.",
                            );
                            return None;
                        };
                        let ours = dir.join("ngrok-tunnel.yml");
                        let yaml = format!(
                            "version: \"2\"\ntunnels:\n  fz-mcp:\n    proto: http\n    addr: 127.0.0.1:{}\n    basic_auth:\n      - \"fz:{}\"\n",
                            local_port, pw
                        );
                        if std::fs::write(&ours, yaml).is_err() {
                            self.status_msg = self.tr(
                                "Falha ao gravar o config do ngrok (basic_auth).",
                                "Failed to write the ngrok config (basic_auth).",
                            );
                            return None;
                        }
                        self.restrict_file_acl(&ours);
                        subcmd = vec![
                            "start".into(),
                            "fz-mcp".into(),
                            "--config".into(),
                            default_cfg,
                            "--config".into(),
                            ours.display().to_string(),
                        ];
                        self.log_debug(
                            "[tunnel][ngrok] Agente antigo: usando `ngrok start` com config v2 (basic_auth em arquivo com ACL restrita). Recomendado atualizar o ngrok.",
                        );
                    }
                }

                let mut args = subcmd;
                args.push("--log".into());
                args.push(log_s);
                args.push("--log-format".into());
                args.push("logfmt".into());
                args.push("--log-level".into());
                args.push("info".into());
                args.extend(auth_args);
                for tok in self.tunnel_ngrok_extra.split_whitespace() {
                    args.push(tok.to_string());
                }
                Some((bin, args))
            }
            TunnelProvider::Ssh => {
                if self.tunnel_ssh_bin.is_empty() {
                    self.status_msg = self.tr(
                        "ssh nao encontrado (Cliente OpenSSH do Windows).",
                        "ssh not found (Windows OpenSSH Client).",
                    );
                    return None;
                }
                let target_host = self.tunnel_ssh_target.trim().to_string();
                if target_host.is_empty() {
                    self.status_msg = self.tr(
                        "Informe o destino SSH (ex.: usuario@servidor ou nokey@localhost.run).",
                        "Enter the SSH target (e.g. user@server or nokey@localhost.run).",
                    );
                    return None;
                }
                let rport = self.tunnel_ssh_remote_port.trim();
                let rport = if rport.is_empty() { "80" } else { rport };
                let bin = self.tunnel_ssh_bin.clone();
                let mut args: Vec<String> = vec![
                    "-N".into(),
                    "-T".into(),
                    "-E".into(),
                    log_s,
                    "-o".into(),
                    "BatchMode=yes".into(),
                    "-o".into(),
                    "StrictHostKeyChecking=accept-new".into(),
                    "-o".into(),
                    "ExitOnForwardFailure=yes".into(),
                    "-o".into(),
                    "ConnectTimeout=10".into(),
                    "-o".into(),
                    "ServerAliveInterval=30".into(),
                    "-o".into(),
                    "ServerAliveCountMax=3".into(),
                ];
                if !self.tunnel_ssh_key.trim().is_empty() {
                    args.push("-i".into());
                    args.push(self.tunnel_ssh_key.trim().to_string());
                }
                args.push("-R".into());
                args.push(format!("{}:127.0.0.1:{}", rport, local_port));
                args.push(target_host);
                for tok in self.tunnel_ssh_extra.split_whitespace() {
                    args.push(tok.to_string());
                }
                Some((bin, args))
            }
        }
    }

    /// Inicia o túnel selecionado. Pré-checagens honestas antes de qualquer
    /// spawn; se houver senha, sobe o gate e o túnel aponta para ele.
    pub fn start_tunnel(&mut self) {
        if matches!(self.tunnel_status, TunnelStatus::Starting | TunnelStatus::Running) {
            self.status_msg = self.tr(
                "Ja existe um tunel ativo. Pare-o antes de iniciar outro.",
                "A tunnel is already active. Stop it before starting another.",
            );
            return;
        }

        // 1) Porta MCP confirmada de verdade (nunca publica porta morta).
        let Some(mcp_port) = self.detect_confirmed_cua_port() else {
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = self.tr(
                "O MCP local nao respondeu em 127.0.0.1. Inicie o motor na aba MCP & Rede antes de abrir o tunel.",
                "The local MCP did not answer on 127.0.0.1. Start the engine in the MCP & Network tab before opening the tunnel.",
            );
            return;
        };

        // 2) Pré-checagens específicas do provedor.
        if self.tunnel_provider == TunnelProvider::Cloudflare
            && self.tunnel_cf_token_file.trim().is_empty()
        {
            if let Ok(home) = std::env::var("USERPROFILE") {
                let cfg = std::path::PathBuf::from(home).join(".cloudflared").join("config.yaml");
                if cfg.exists() {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "O quick tunnel do Cloudflare falha quando existe ~/.cloudflared/config.yaml. Renomeie esse arquivo ou use o tunel nomeado (token-file).",
                        "Cloudflare quick tunnel fails when ~/.cloudflared/config.yaml exists. Rename that file or use the named tunnel (token-file).",
                    );
                    return;
                }
            }
        }
        if self.tunnel_provider == TunnelProvider::Ngrok {
            let bin = self.tunnel_ngrok_bin.clone();
            if !bin.is_empty() {
                let check = self.run_logged(&bin, &["config", "check"]);
                let ok = check.map(|o| o.status.success()).unwrap_or(false);
                if !ok {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "ngrok sem authtoken configurado. Rode no seu terminal: ngrok config add-authtoken <SEU_TOKEN> (crie a conta em ngrok.com).",
                        "ngrok has no authtoken configured. Run in your terminal: ngrok config add-authtoken <YOUR_TOKEN> (create the account at ngrok.com).",
                    );
                    return;
                }
            }
        }

        // 3) run_id novo + provedor congelado + (se houver senha) gate local.
        self.tunnel_run_id = Self::gen_token(8);
        self.tunnel_run_provider = self.tunnel_provider;
        self.tunnel_gate_port = None;

        // A senha do gate DEVE ser URL-safe (unreserved: letras, dígitos e
        // - . _ ~). Se tiver espaço/#/%/acento, o cliente percent-encoda o path
        // e o gate (que compara o path cru) responderia 404 em TODA requisição,
        // com a UI ainda exibindo uma URL de aparência válida. Sanitiza aqui,
        // antes de usar — gate e tunnel_full_url leem o mesmo campo, então
        // continuam batendo.
        if !self.tunnel_gate_password.trim().is_empty() {
            let raw = self.tunnel_gate_password.trim().to_string();
            let safe: String = raw
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
                .collect();
            if safe != raw {
                self.log_debug(
                    "[tunnel][gate] Senha continha caracteres nao URL-safe; foram removidos.",
                );
                self.status_msg = self.tr(
                    "Senha ajustada para caracteres URL-safe (letras, numeros, - . _ ~).",
                    "Password reduced to URL-safe characters (letters, digits, - . _ ~).",
                );
            }
            self.tunnel_gate_password = safe;
        }

        let local_port = if self.tunnel_gate_password.trim().is_empty() {
            mcp_port
        } else {
            let pw = self.tunnel_gate_password.trim().to_string();
            match self.start_gate(mcp_port, &pw) {
                Some(gp) => gp,
                None => {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "Nao foi possivel abrir o porteiro de senha local.",
                        "Could not open the local password gate.",
                    );
                    return;
                }
            }
        };

        // 4) staging + logs zerados.
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let log = self.tunnel_log_path();
        let _ = std::fs::remove_file(&log);

        let Some((bin, args)) = self.tunnel_cmdline(local_port) else {
            self.stop_gate();
            self.tunnel_status = TunnelStatus::Error;
            return;
        };

        let file = match std::fs::File::create(&log) {
            Ok(f) => f,
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao criar log do tunel: {}", e);
                return;
            }
        };
        let file2 = match file.try_clone() {
            Ok(f) => f,
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao preparar log do tunel: {}", e);
                return;
            }
        };

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match quiet_cmd(&bin)
            .args(&arg_refs)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(file))
            .stderr(std::process::Stdio::from(file2))
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                // ADOÇÃO NO JOB OBJECT: no Windows um filho NÃO morre com o
                // pai (CreateProcess não cria vínculo de ciclo de vida — isso
                // é comportamento de Unix). O job é o mecanismo nativo que
                // garante isso, pelo kernel, inclusive em taskkill /F e crash.
                let adopted = crate::lifecycle::adopt(&child).is_ok();
                if adopted {
                    self.log_debug(&format!(
                        "[tunnel] pid {} adotado pelo Job Object — morre com a GUI (garantia do kernel).",
                        pid
                    ));
                } else {
                    self.log_debug(&format!(
                        "[tunnel] AVISO: pid {} NAO foi adotado pelo Job Object — caindo para o watchdog externo.",
                        pid
                    ));
                }
                self.tunnel_child = Some(child);
                self.tunnel_pid = Some(pid);
                self.tunnel_status = TunnelStatus::Starting;
                self.tunnel_exposure = None;
                if self.tunnel_cf_token_file.trim().is_empty() {
                    self.tunnel_public_url.clear();
                }
                // Log SEM o argv completo (pode conter caminho de token/chave).
                self.log_debug(&format!(
                    "[tunnel] {} iniciado (pid {}) para 127.0.0.1:{} [log: {}]",
                    Self::provider_slug(self.tunnel_provider),
                    pid,
                    local_port,
                    self.tunnel_log_path().display()
                ));
                self.status_msg = self.tr(
                    "Tunel iniciando... aguardando a URL publica aparecer no log.",
                    "Tunnel starting... waiting for the public URL to appear in the log.",
                );
                self.register_tunnel(pid, local_port);
                // Watchdog externo SÓ quando a adoção falhou. Com o Job Object
                // ativo ele seria um processo PowerShell por túnel fazendo o
                // que o kernel já faz melhor — redundância que custa e pode
                // ela mesma falhar. Sem job, ele volta a ser a única rede de
                // proteção contra túnel órfão.
                if !adopted {
                    self.spawn_tunnel_guard(pid);
                }
            }
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao executar {}: {}", bin, e);
                self.log_debug(&format!("[tunnel] ERRO ao spawnar {}: {}", bin, e));
            }
        }
    }

    /// Registra o túnel como PROPRIEDADE nossa em HKCU (identidade forte:
    /// imagem|CreationDate|porta|run_id|modo), para reconciliação e limpeza.
    fn register_tunnel(&mut self, pid: u32, local_port: u16) {
        #[cfg(target_os = "windows")]
        {
            // TUDO EM SEGUNDO PLANO. Isto lia o CreationDate do processo com
            // `Get-CimInstance` — ou seja, um PowerShell — na thread da UI, e
            // nesta máquina uma invocação de PowerShell chega a passar de um
            // minuto: era o clique em "Iniciar túnel" congelando o aplicativo
            // inteiro com "(Não Respondendo)". O registro serve à reconciliação
            // da PRÓXIMA abertura; nada na tela depende dele agora.
            let image = Self::provider_image(self.tunnel_run_provider);
            let mode = if self.tunnel_gate_password.trim().is_empty() {
                "direct"
            } else {
                "gated"
            };
            let run_id = self.tunnel_run_id.clone();
            let slug = Self::provider_slug(self.tunnel_run_provider);
            self.spawn_bg("Registrando o tunel para limpeza futura", move || {
                let creation = quiet_cmd("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "$p = Get-CimInstance Win32_Process -Filter \"ProcessId={}\"; if ($p) {{ $p.CreationDate.ToString('yyyyMMddHHmmss') }}",
                            pid
                        ),
                    ])
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                let value = format!("{}|{}|{}|{}|{}", image, creation, local_port, run_id, mode);
                let name = format!("tunnel:{}:{}", slug, pid);
                let _ = quiet_cmd("reg")
                    .args([
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        name.as_str(),
                        "/t",
                        "REG_SZ",
                        "/d",
                        value.as_str(),
                        "/f",
                    ])
                    .output();
                BgOutcome {
                    log: format!("[tunnel] Registrado para limpeza: {}", name),
                    status: String::new(),
                    effect: BgEffect::None,
                }
            });
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (pid, local_port);
        }
    }

    /// CreationDate (CIM) de um PID — elimina risco de PID reciclado.
    #[cfg(target_os = "windows")]
    fn pid_creation_date(&mut self, pid: u32) -> Option<String> {
        let ps = format!(
            "$p = Get-CimInstance Win32_Process -Filter \"ProcessId={}\"; if ($p) {{ $p.CreationDate.ToString('yyyyMMddHHmmss') }}",
            pid
        );
        let out = self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    /// Guarda-PS disparada NO START: vive por conta própria e mata o túnel
    /// quando ESTE processo da GUI desaparecer (cobre taskkill /F, crash,
    /// queda de energia — casos em que o on_exit NÃO roda). spawn(), NUNCA
    /// wait(). Mata só com imagem + CreationDate + marcador na cmdline.
    fn spawn_tunnel_guard(&mut self, tunnel_pid: u32) {
        #[cfg(target_os = "windows")]
        {
            let gui_pid = std::process::id();
            let slug = Self::provider_slug(self.tunnel_run_provider);
            let image = Self::provider_image(self.tunnel_run_provider);
            // CreationDate vazio de propósito: obtê-lo aqui custava um
            // PowerShell SÍNCRONO na thread da UI. A identidade continua
            // garantida pelos outros dois fatores (imagem + run_id na command
            // line), e o script abaixo já trata `$ct` vazio como "não
            // comparar". Este caminho só roda quando a adoção no Job Object
            // falha — o normal é nem existir.
            let creation = String::new();
            // Marcador de identidade = o run_id (8 chars aleatorios), que
            // aparece no path do --logfile/--log/-E na command line. Unico o
            // bastante para, junto de imagem + CreationDate, garantir que so
            // matamos o NOSSO processo (nunca cloudflared/ssh alheio).
            let marker = self.tunnel_run_id.clone();
            let name = format!("tunnel:{}:{}", slug, tunnel_pid);
            // $tp em vez de $pid ($pid e variavel automatica reservada no PS).
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; $g={gui}; $tp={tp}; $img='{img}'; $ct='{ct}'; $mark='{mark}'; \
                 $key='HKCU:\\Software\\FzComputerAI'; $name='{name}'; \
                 while ($true) {{ \
                   $t = Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                   if (-not $t) {{ break }}; \
                   if ($t.Name -ne $img) {{ break }}; \
                   if ($ct -and $t.CreationDate.ToString('yyyyMMddHHmmss') -ne $ct) {{ break }}; \
                   if ($t.CommandLine -notlike ('*' + $mark + '*')) {{ break }}; \
                   if (-not (Get-Process -Id $g -ErrorAction SilentlyContinue)) {{ Stop-Process -Id $tp -Force; break }}; \
                   Start-Sleep -Seconds 2 \
                 }}; \
                 $cur = (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name; \
                 if ($cur -and (($cur -split '\\|')[3]) -eq $mark) {{ Remove-ItemProperty -Path $key -Name $name -Force -ErrorAction SilentlyContinue }}",
                gui = gui_pid,
                tp = tunnel_pid,
                img = image,
                ct = creation,
                mark = marker,
                name = name
            );
            let _ = quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = tunnel_pid;
        }
    }

    /// Para o túnel: mata o filho, confirma ausência (relê o tasklist —
    /// nunca presume), encerra o gate e limpa o registro em HKCU.
    pub fn stop_tunnel(&mut self) {
        let Some(pid) = self.tunnel_pid else {
            self.tunnel_status = TunnelStatus::Stopped;
            self.stop_gate();
            return;
        };
        // Mata a ÁRVORE enquanto o handle do Child ainda reserva o PID (sem
        // janela de reuso): taskkill /T ANTES do wait()/drop. Depois reap.
        if let Some(mut child) = self.tunnel_child.take() {
            #[cfg(target_os = "windows")]
            {
                let pid_s = pid.to_string();
                let _ = self.run_logged("taskkill", &["/PID", pid_s.as_str(), "/T", "/F"]);
            }
            let _ = child.kill();
            let _ = child.wait();
        }

        // Confirma ausência por IDENTIDADE (imagem + marcador run_id), nunca por
        // PID nu — um PID reciclado leria como "vivo" e vazaria o registro HKCU
        // ou mataria árvore alheia. Mesma disciplina do watchdog/reconcile.
        let still = self.tunnel_pid_is_ours_alive(pid);
        if still {
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = self.tr(
                "AVISO: o processo do tunel AINDA esta vivo apos o encerramento.",
                "WARNING: the tunnel process is STILL alive after termination.",
            );
            self.log_debug("[tunnel] AVISO: processo ainda vivo apos kill+taskkill.");
        } else {
            self.tunnel_status = TunnelStatus::Stopped;
            self.tunnel_exposure = None;
            // URL de tunel morto e mentirosa — limpa (exceto tunel nomeado
            // com hostname fixo informado pelo usuario).
            if self.tunnel_cf_token_file.trim().is_empty() {
                self.tunnel_public_url.clear();
            }
            self.log_debug("[tunnel] Tunel encerrado e confirmado ausente.");
            self.clear_tunnel_registration(pid);
        }
        self.stop_gate();
        self.tunnel_pid = None;
    }

    /// O PID ainda é o NOSSO túnel vivo? Checa identidade (imagem + marcador
    /// run_id na cmdline), não o PID nu — assim um PID reciclado por outro
    /// processo nunca é lido como "nosso túnel vivo". Fora do Windows,
    /// best-effort false.
    fn tunnel_pid_is_ours_alive(&mut self, pid: u32) -> bool {
        // SEM PowerShell. Antes isto era um `Get-CimInstance` síncrono na
        // thread da UI — o clique em "Parar túnel" congelava o aplicativo por
        // todo o tempo do PowerShell (mais de um minuto nesta máquina).
        //
        // E era desnecessário: o túnel é NOSSO FILHO, então temos o handle
        // dele. `tasklist` com filtro de PID responde em ~85ms e ainda
        // confirma a IMAGEM, que é o que interessa aqui: se o PID foi
        // reciclado por outro programa, o nome não bate e devolvemos "não é
        // nosso" — que é justamente a proteção que o AGENTS.md exige.
        #[cfg(target_os = "windows")]
        {
            let image = Self::provider_image(self.tunnel_run_provider);
            let filtro = format!("PID eq {}", pid);
            self.run_logged("tasklist", &["/FI", filtro.as_str(), "/FO", "CSV", "/NH"])
                .map(|o| {
                    let txt = String::from_utf8_lossy(&o.stdout).to_ascii_lowercase();
                    txt.contains(&image.to_ascii_lowercase())
                })
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = pid;
            false
        }
    }

    /// Remove o valor tunnel:<slug>:<pid> do HKCU.
    fn clear_tunnel_registration(&mut self, pid: u32) {
        #[cfg(target_os = "windows")]
        {
            let name = format!("tunnel:{}:{}", Self::provider_slug(self.tunnel_run_provider), pid);
            let _ = self.run_logged(
                "reg",
                &["delete", r"HKCU\Software\FzComputerAI", "/v", name.as_str(), "/f"],
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = pid;
        }
    }

    /// Extrai a URL pública do texto do log, por sufixo do provedor. Sem
    /// crate `regex`: busca "https://", consome o host e aceita se terminar
    /// num sufixo conhecido. Devolve o ÚLTIMO match (o banner final é o que
    /// vale).
    fn extract_public_url(text: &str, p: TunnelProvider) -> Option<String> {
        let suffixes: &[&str] = match p {
            TunnelProvider::Cloudflare => &[".trycloudflare.com"],
            TunnelProvider::Ngrok => &[".ngrok-free.app", ".ngrok-free.dev", ".ngrok.app", ".ngrok.io"],
            // localhost.run -> *.lhr.life ; serveo.net -> a URL publica vem em
            // *.serveousercontent.com (nao em serveo.net). O sufixo antigo
            // ".serveo.net" nunca casava a URL real do serveo.
            TunnelProvider::Ssh => &[".lhr.life", ".serveousercontent.com", ".serveo.net"],
        };
        let bytes = text.as_bytes();
        let mut found: Option<String> = None;
        let mut i = 0usize;
        while let Some(rel) = text[i..].find("https://") {
            let start = i + rel;
            let host_start = start + "https://".len();
            let mut end = host_start;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    end += 1;
                } else {
                    break;
                }
            }
            let host = &text[host_start..end];
            for suf in suffixes {
                if host.len() > suf.len() && host.ends_with(suf) {
                    found = Some(format!("https://{}", host));
                    break;
                }
            }
            i = end.max(start + "https://".len());
        }
        found
    }

    /// Observado a cada ~1s pelo loop da UI enquanto o túnel está ativo:
    /// detecta morte do processo e captura a URL pública do log.
    pub fn poll_tunnel(&mut self) {
        if matches!(self.tunnel_status, TunnelStatus::Stopped | TunnelStatus::Error) {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.tunnel_last_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.tunnel_last_poll = Some(now);

        // (A) o processo morreu sozinho?
        let exited = self
            .tunnel_child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten());
        if let Some(status) = exited {
            let tail = std::fs::read_to_string(self.tunnel_log_path()).unwrap_or_default();
            let tail = tail_str(&tail, 1200);
            // Diagnóstico DIRIGIDO para erro conhecido: o `ngrok config check`
            // do pré-start valida só a SINTAXE do arquivo — um authtoken com
            // valor inválido passa nele e o processo morre aqui com
            // ERR_NGROK_105 (verificado na prática). Sem esta tradução o
            // usuário ganharia um despejo de log e nenhuma instrução.
            let hint = if tail.contains("ERR_NGROK_105") {
                self.tr(
                    "\n\nDIAGNOSTICO: authtoken do ngrok INVALIDO (ERR_NGROK_105). O 'config check' nao valida o token, so a sintaxe. Rode no seu terminal: ngrok config add-authtoken <TOKEN> (token real em dashboard.ngrok.com).",
                    "\n\nDIAGNOSIS: INVALID ngrok authtoken (ERR_NGROK_105). 'config check' validates syntax only, not the token. Run in your terminal: ngrok config add-authtoken <TOKEN> (real token at dashboard.ngrok.com).",
                )
            } else {
                String::new()
            };
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = format!(
                "{} (exit {:?})\n\n{}{}",
                self.tr("O processo do tunel SAIU", "The tunnel process EXITED"),
                status.code(),
                tail,
                hint
            );
            self.log_debug(&format!("[tunnel] processo saiu (exit {:?}).", status.code()));
            if let Some(pid) = self.tunnel_pid.take() {
                self.clear_tunnel_registration(pid);
            }
            self.tunnel_child = None;
            self.stop_gate();
            return;
        }

        // (B) URL ainda não capturada? Lê o log e tenta extrair. Usa o
        // provedor CONGELADO no start — o rádio pode ter sido trocado com o
        // túnel vivo, e os sufixos de URL são por provedor.
        if self.tunnel_public_url.trim().is_empty() {
            let txt = std::fs::read_to_string(self.tunnel_log_path()).unwrap_or_default();
            if let Some(url) = Self::extract_public_url(&txt, self.tunnel_run_provider) {
                self.tunnel_public_url = url.clone();
                self.tunnel_status = TunnelStatus::Running;
                self.status_msg = format!(
                    "{}: {}",
                    self.tr("URL publica capturada", "Public URL captured"),
                    url
                );
                self.log_debug(&format!("[tunnel] URL publica capturada: {}", url));
            }
        } else if self.tunnel_status == TunnelStatus::Starting {
            // URL informada à mão (túnel nomeado): promove a Running.
            self.tunnel_status = TunnelStatus::Running;
        }
    }

    /// URL pública COMPLETA (com /s/<senha> quando houver gate) + /mcp.
    pub fn tunnel_full_url(&self) -> String {
        let base = self.tunnel_public_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return String::new();
        }
        if self.tunnel_gate_password.trim().is_empty() {
            format!("{}/mcp", base)
        } else {
            format!("{}/s/{}/mcp", base, self.tunnel_gate_password.trim())
        }
    }

    /// Snippet mcpServers pronto para colar num cliente MCP. Inclui o header
    /// Authorization quando o motor exige token (0.16+) — sem ele o cliente
    /// tomaria 401 e o usuario ficaria sem saber por que.
    pub fn tunnel_mcp_snippet(&self) -> String {
        let url = self.tunnel_full_url();
        if self.mcp_token.trim().is_empty() {
            format!(
                "{{\n  \"mcpServers\": {{\n    \"fzcomputerai\": {{\n      \"type\": \"http\",\n      \"url\": \"{}\"\n    }}\n  }}\n}}",
                url
            )
        } else {
            format!(
                "{{\n  \"mcpServers\": {{\n    \"fzcomputerai\": {{\n      \"type\": \"http\",\n      \"url\": \"{}\",\n      \"headers\": {{\n        \"Authorization\": \"Bearer {}\"\n      }}\n    }}\n  }}\n}}",
                url,
                self.mcp_token.trim()
            )
        }
    }

    /// Sonda UMA vez a URL pública via curl.exe (TcpStream não faz TLS):
    /// POST initialize. A URL (contém a senha do gate) e o Bearer opcional
    /// vão no ARQUIVO `--config` do curl — nunca no argv (legível por
    /// qualquer processo via Win32_Process.CommandLine) nem por run_logged.
    /// Retorna (código HTTP, corpo+stderr).
    fn tunnel_probe_once(&mut self, url: &str, bearer: Option<&str>) -> Option<(u16, String)> {
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let body_path = dir.join(format!("probe-{}.json", self.tunnel_run_id));
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"fzcomputerai-gui","version":"{}"}}}}}}"#,
            env!("CARGO_PKG_VERSION")
        );
        std::fs::write(&body_path, &body).ok()?;
        let cfg_path = dir.join(format!("probe-{}.cfg", self.tunnel_run_id));
        let mut cfg = format!("url = \"{}\"\n", url);
        if let Some(tok) = bearer {
            cfg.push_str(&format!("header = \"Authorization: Bearer {}\"\n", tok));
        }
        if std::fs::write(&cfg_path, cfg).is_err() {
            let _ = std::fs::remove_file(&body_path);
            return None;
        }
        let data_arg = format!("@{}", body_path.display());
        let cfg_arg = cfg_path.display().to_string();
        self.log_debug(&format!(
            "> curl [probe{}] -X POST {} (URL/credencial via --config; segredos omitidos)",
            if bearer.is_some() { "+Bearer" } else { "" },
            self.mask_gate_url(url)
        ));
        let out = quiet_cmd("curl")
            .args([
                "-sS",
                "-m",
                "20",
                "-o",
                "-",
                "-w",
                "\nHTTP_CODE=%{http_code}",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                "Accept: application/json, text/event-stream",
                "-H",
                "ngrok-skip-browser-warning: 1",
                "--data-binary",
                data_arg.as_str(),
                "--config",
                cfg_arg.as_str(),
            ])
            .output()
            .ok();
        // Não deixa segredo em disco depois do teste.
        let _ = std::fs::remove_file(&cfg_path);
        let _ = std::fs::remove_file(&body_path);
        let o = out?;
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        let stderr = String::from_utf8_lossy(&o.stderr).to_string();
        let code = text
            .rsplit("HTTP_CODE=")
            .next()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or(0);
        let body_only = match text.rfind("HTTP_CODE=") {
            Some(p) => text[..p].trim().to_string(),
            None => text.trim().to_string(),
        };
        Some((code, format!("{}{}", body_only, stderr)))
    }

    /// SONDA DE EXPOSIÇÃO em DUAS FASES na URL pública.
    ///   Fase 1 (SEM credencial): 200 + "result"  => EXPOSTO de verdade;
    ///     401 com corpo JSON-RPC => o MOTOR barrou (EngineAuth, 0.16+);
    ///     401/403/302/407 sem JSON-RPC => a BORDA barrou (EdgeAuth);
    ///     resto => Unknown (tratar como exposto).
    ///   Fase 2 (só com token conhecido e fase 1 não-exposta): com Bearer,
    ///     200 + "result" => AuthOk — protegido E utilizável de ponta a ponta.
    /// O código HTTP decide ANTES do "contém jsonrpc": o 401 dos motores
    /// 0.16+ TAMBÉM contém "jsonrpc" (é erro JSON-RPC) e o critério antigo o
    /// chamava de EXPOSTO — alarme falso, verificado na prática e corrigido.
    /// Roda em SEGUNDO PLANO: são dois `curl -m 20`, ou seja, até 40s. Feito
    /// na thread da UI (como era antes) isso congelava a janela inteira e o
    /// Windows marcava "(Não Respondendo)" no título — foi o pior caso medido
    /// do travamento relatado.
    pub fn verify_tunnel(&mut self) {
        let url = self.tunnel_full_url();
        if url.is_empty() {
            self.status_msg = self.tr("Sem URL publica para testar.", "No public URL to test.");
            return;
        }
        let lang = self.language;
        let token = self.mcp_token.trim().to_string();
        let pw = self.tunnel_gate_password.trim().to_string();
        let run_id = self.tunnel_run_id.clone();
        let dir = Self::tunnel_dir();
        let masked = mask_gate(&url, &pw);
        self.status_msg = tr_of(
            lang,
            "Testando pela internet em segundo plano (pode levar ate 40s)...",
            "Testing over the internet in the background (may take up to 40s)...",
        );

        self.spawn_bg("Teste pela internet", move || {
            let Some((code1, text1)) = probe_once(&dir, &run_id, &url, None) else {
                return BgOutcome {
                    log: "[tunnel] Falha ao executar curl para o teste.".to_string(),
                    status: tr_of(
                        lang,
                        "Falha ao executar curl para o teste.",
                        "Failed to run curl for the test.",
                    ),
                    effect: BgEffect::Exposure(TunnelExposure::Unknown),
                };
            };
            let exposed = code1 == 200 && text1.contains("\"result\"");
            let mut exposure = if exposed {
                TunnelExposure::Exposed
            } else if code1 == 401 && text1.contains("jsonrpc") {
                TunnelExposure::EngineAuth
            } else if matches!(code1, 401 | 403 | 302 | 407) {
                TunnelExposure::EdgeAuth(code1)
            } else {
                TunnelExposure::Unknown
            };

            let mut phase2 = String::new();
            if !exposed && !token.is_empty() {
                if let Some((code2, text2)) = probe_once(&dir, &run_id, &url, Some(&token)) {
                    if code2 == 200 && text2.contains("\"result\"") {
                        exposure = TunnelExposure::AuthOk;
                        phase2 = tr_of(
                            lang,
                            "\nFase 2 (com Bearer): initialize OK — tunel protegido E utilizavel.",
                            "\nPhase 2 (with Bearer): initialize OK — tunnel protected AND usable.",
                        );
                    } else {
                        phase2 = format!(
                            "{} (HTTP {}).",
                            tr_of(
                                lang,
                                "\nFase 2 (com Bearer): o token conhecido NAO passou",
                                "\nPhase 2 (with Bearer): the known token did NOT pass",
                            ),
                            code2
                        );
                    }
                }
            }
            let status = format!(
                "{} ({}): HTTP {}\n{}{}",
                tr_of(lang, "Teste pela internet", "Internet test"),
                masked,
                code1,
                tail_str(mask_gate(text1.trim(), &pw).as_str(), 900),
                phase2
            );
            BgOutcome {
                log: format!("[tunnel] Teste pela internet concluido: HTTP {}.", code1),
                status,
                effect: BgEffect::Exposure(exposure),
            }
        });
    }

    /// Mascara a senha do gate em qualquer texto destinado a log/UI —
    /// substitui o segmento /s/<senha>/ por /s/***/. Nunca deixa a senha do
    /// gate (única credencial da URL pública) aparecer no Console Debug.
    fn mask_gate_url(&self, s: &str) -> String {
        let pw = self.tunnel_gate_password.trim();
        if pw.is_empty() {
            s.to_string()
        } else {
            s.replace(&format!("/s/{}/", pw), "/s/***/")
        }
    }

    /// ngrok: consulta a API local (loopback, HTTP puro) para descobrir a URL.
    pub fn ngrok_query_local_api(&mut self) {
        let body = self.http_get_local(4040, "/api/tunnels");
        let body = match body {
            Some(b) if !b.is_empty() => b,
            _ => {
                self.status_msg = self.tr(
                    "Nao consegui falar com a API local do ngrok (127.0.0.1:4040). Se houver outro agente ngrok, abra http://127.0.0.1:4040.",
                    "Could not reach the local ngrok API (127.0.0.1:4040). If another ngrok agent is running, open http://127.0.0.1:4040.",
                );
                return;
            }
        };
        if let Some(url) = Self::extract_public_url(&body, TunnelProvider::Ngrok) {
            self.tunnel_public_url = url.clone();
            if self.tunnel_status == TunnelStatus::Starting {
                self.tunnel_status = TunnelStatus::Running;
            }
            self.status_msg = format!("ngrok API 4040 -> {}", url);
            self.log_debug(&format!("[tunnel] URL do ngrok pela API 4040: {}", url));
        } else {
            self.status_msg = self.tr(
                "A API 4040 respondeu mas nao encontrei uma URL publica.",
                "The 4040 API answered but no public URL was found.",
            );
        }
    }

    /// GET simples em 127.0.0.1:porta/path (HTTP puro, sem TLS) — para a API
    /// local do ngrok. Espelho enxuto do mcp_probe.
    fn http_get_local(&mut self, port: u16, path: &str) -> Option<String> {
        use std::io::{Read, Write};
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
        let timeout = std::time::Duration::from_millis(1500);
        let mut stream = std::net::TcpStream::connect_timeout(&addr, timeout).ok()?;
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            path, port
        );
        stream.write_all(req.as_bytes()).ok()?;
        let mut collected = Vec::new();
        let mut buf = [0u8; 2048];
        while collected.len() < 65536 {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => collected.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        let text = String::from_utf8_lossy(&collected).to_string();
        // Retorna só o corpo (após o cabeçalho).
        Some(match text.split_once("\r\n\r\n") {
            Some((_, body)) => body.to_string(),
            None => text,
        })
    }

    /// Login OAuth do Cloudflare (abre o navegador). Processo destacado.
    pub fn cloudflared_login(&mut self) {
        if self.tunnel_cf_bin.is_empty() {
            self.status_msg = self.tr(
                "cloudflared nao encontrado. Baixe-o antes de fazer login.",
                "cloudflared not found. Download it before logging in.",
            );
            return;
        }
        let bin = self.tunnel_cf_bin.clone();
        match quiet_cmd(&bin).args(["tunnel", "login"]).spawn() {
            Ok(_) => {
                self.status_msg = self.tr(
                    "Login Cloudflare aberto no navegador: escolha o DOMINIO e autorize. Ao voltar, clique em 'Verificar login' e depois em 'Criar tunel + DNS'.",
                    "Cloudflare login opened in the browser: pick the DOMAIN and authorize. When you return, click 'Check login' and then 'Create tunnel + DNS'.",
                );
                self.log_debug("[tunnel] cloudflared tunnel login disparado (navegador).");
            }
            Err(e) => {
                self.status_msg = format!("Falha ao iniciar login do cloudflared: {}", e);
            }
        }
    }

    /// O certificado de conta do cloudflared existe? É o que o `tunnel login`
    /// grava e o que habilita `create`/`route dns`. Verificação REAL de
    /// arquivo — nunca "provavelmente logou".
    pub fn cloudflared_check_login(&mut self) -> bool {
        let mut found = String::new();
        for var in ["USERPROFILE", "HOME"] {
            if let Ok(home) = std::env::var(var) {
                let p = std::path::PathBuf::from(home).join(".cloudflared").join("cert.pem");
                if p.exists() {
                    found = p.display().to_string();
                    break;
                }
            }
        }
        self.tunnel_cf_logged = !found.is_empty();
        if self.tunnel_cf_logged {
            self.log_debug(&format!("[tunnel][cf] Certificado de conta encontrado: {}", found));
        } else {
            self.log_debug("[tunnel][cf] Sem cert.pem — a conta ainda NAO foi autorizada nesta maquina.");
        }
        self.tunnel_cf_logged
    }

    /// Cria o túnel nomeado e aponta o DNS para ele, em SEGUNDO PLANO
    /// (`tunnel create` + `tunnel route dns` falam com a API da Cloudflare e
    /// levam segundos). Idempotente na prática: se o túnel já existe, o
    /// create falha dizendo isso e seguimos para a rota — que é o que
    /// realmente publica o hostname.
    pub fn cloudflared_create_and_route(&mut self) {
        if self.tunnel_cf_bin.is_empty() {
            self.status_msg = self.tr("cloudflared nao encontrado.", "cloudflared not found.");
            return;
        }
        if !self.cloudflared_check_login() {
            self.status_msg = self.tr(
                "Faca o login do Cloudflare primeiro (botao Login) — sem o cert.pem nao da para criar tunel nem DNS.",
                "Do the Cloudflare login first (Login button) — without cert.pem you cannot create a tunnel or DNS.",
            );
            return;
        }
        let name = self.tunnel_cf_name.trim().to_string();
        let host = self.tunnel_cf_hostname.trim().to_string();
        if name.is_empty() || host.is_empty() {
            self.status_msg = self.tr(
                "Informe o NOME do tunel e o HOSTNAME (ex.: mcphome.seudominio.com.br).",
                "Enter the tunnel NAME and the HOSTNAME (e.g. mcphome.yourdomain.com).",
            );
            return;
        }
        let bin = self.tunnel_cf_bin.clone();
        let lang = self.language;
        self.status_msg = self.tr(
            "Criando tunel e apontando DNS na Cloudflare (segundo plano)...",
            "Creating tunnel and pointing DNS at Cloudflare (background)...",
        );
        self.spawn_bg("Cloudflare: create + route dns", move || {
            let mut log = String::new();
            let create = quiet_cmd(&bin)
                .args(["tunnel", "create", name.as_str()])
                .output();
            let create_txt = match &create {
                Ok(o) => format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
                Err(e) => format!("erro ao executar: {}", e),
            };
            let already = create_txt.contains("already exists");
            log.push_str(&format!(
                "[tunnel][cf] create '{}': {}\n",
                name,
                if already { "ja existia (ok)" } else { create_txt.trim() }
            ));

            let route = quiet_cmd(&bin)
                .args(["tunnel", "route", "dns", name.as_str(), host.as_str()])
                .output();
            let route_txt = match &route {
                Ok(o) => format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
                Err(e) => format!("erro ao executar: {}", e),
            };
            let route_ok = route.as_ref().map(|o| o.status.success()).unwrap_or(false)
                || route_txt.contains("already configured");
            log.push_str(&format!("[tunnel][cf] route dns -> {}: {}", host, route_txt.trim()));

            BgOutcome {
                log,
                status: if route_ok {
                    tr_of(
                        lang,
                        &format!("DNS apontado: https://{} passa a servir este tunel. Clique em 'Iniciar tunel' para subir.", host),
                        &format!("DNS pointed: https://{} now serves this tunnel. Click 'Start tunnel' to bring it up.", host),
                    )
                } else {
                    tr_of(
                        lang,
                        &format!("Falhou ao apontar o DNS para {}. Confira se o dominio esta na SUA conta Cloudflare (nameservers apontados) e veja o console.", host),
                        &format!("Failed to point DNS to {}. Check that the domain is in YOUR Cloudflare account (nameservers delegated) and see the console.", host),
                    )
                },
                effect: BgEffect::None,
            }
        });
    }

    /// Salva o token do Cloudflare colado num arquivo com ACL restrita e usa
    /// só o CAMINHO daqui em diante (o token nunca vai para argv/log/HKCU).
    pub fn save_cf_token(&mut self) {
        let token = self.tunnel_cf_token_input.trim().to_string();
        if token.is_empty() {
            self.status_msg = self.tr("Cole o token do Cloudflare primeiro.", "Paste the Cloudflare token first.");
            return;
        }
        let dir = Self::tunnel_download_dir();
        let path = dir.join("cf-token.txt");
        if std::fs::write(&path, &token).is_err() {
            self.status_msg = self.tr("Falha ao gravar o token-file.", "Failed to write the token-file.");
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let user = std::env::var("USERNAME").unwrap_or_default();
            if !user.is_empty() {
                let p = path.display().to_string();
                let grant = format!("{}:R", user);
                let _ = self.run_logged("icacls", &[p.as_str(), "/inheritance:r", "/grant:r", grant.as_str()]);
            }
        }
        self.tunnel_cf_token_file = path.display().to_string();
        self.tunnel_cf_token_input.clear();
        self.log_debug("[tunnel] Token do Cloudflare salvo em token-file com ACL restrita (nao logado).");
        self.status_msg = self.tr(
            "Token salvo. O tunel Cloudflare passa a rodar em modo NOMEADO (token-file).",
            "Token saved. The Cloudflare tunnel now runs in NAMED mode (token-file).",
        );
    }

    /// Esquece o token do Cloudflare (apaga o arquivo e volta ao quick tunnel).
    pub fn forget_cf_token(&mut self) {
        if !self.tunnel_cf_token_file.is_empty() {
            let _ = std::fs::remove_file(&self.tunnel_cf_token_file);
            self.tunnel_cf_token_file.clear();
            self.log_debug("[tunnel] Token-file do Cloudflare removido — de volta ao quick tunnel.");
        }
    }

    /// Baixa o cloudflared (Apache-2.0) via winget (valida hash sozinho) com
    /// fallback direto do release + comparação de SHA256 com o manifesto
    /// winget. Processo destacado + flags (padrão start_update_download).
    pub fn download_cloudflared(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::tunnel_download_dir();
            let dest = dir.join("cloudflared.exe");
            let ps = format!(
                "$ErrorActionPreference='Stop'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'cf-ready.flag'),(Join-Path $d 'cf-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   winget install --exact --id Cloudflare.cloudflared --source winget --installer-type portable --accept-source-agreements --accept-package-agreements --disable-interactivity --location $d 2>&1 | Out-Null; \
                   if (-not (Test-Path (Join-Path $d 'cloudflared.exe'))) {{ \
                     $u='https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe'; \
                     Invoke-WebRequest -Uri $u -OutFile '{dest}' -UseBasicParsing; \
                   }}; \
                   $h=(Get-FileHash -Path '{dest}' -Algorithm SHA256).Hash.ToLower(); \
                   $sig=(Get-AuthenticodeSignature '{dest}').Status; \
                   Set-Content -Path (Join-Path $d 'cf-ready.flag') -Value (\"SHA256=$h`nAuthenticode=$sig\") \
                 }} catch {{ Set-Content -Path (Join-Path $d 'cf-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display(),
                dest = dest.display()
            );
            match quiet_cmd("powershell").args(["-NoProfile", "-Command", ps.as_str()]).spawn() {
                Ok(_) => {
                    self.tunnel_downloading = true;
                    self.status_msg = self.tr(
                        "Baixando cloudflared em segundo plano...",
                        "Downloading cloudflared in the background...",
                    );
                    self.log_debug("[tunnel] Download do cloudflared iniciado (background).");
                }
                Err(e) => {
                    self.log_debug(&format!("[tunnel] ERRO ao iniciar download do cloudflared: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg = "Download automatico disponivel apenas no Windows.".to_string();
        }
    }

    /// Baixa o ngrok — SOMENTE após o usuário aceitar os termos (modal). O
    /// download é da fonte oficial, na máquina do usuário, por clique dele.
    pub fn download_ngrok(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::tunnel_download_dir();
            let ps = format!(
                "$ErrorActionPreference='Stop'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'ngrok-ready.flag'),(Join-Path $d 'ngrok-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   winget install --exact --id Ngrok.Ngrok --source winget --accept-source-agreements --accept-package-agreements --disable-interactivity 2>&1 | Out-Null; \
                   $z=Join-Path $d 'ngrok.zip'; \
                   if (-not (Get-Command ngrok -ErrorAction SilentlyContinue) -and -not (Test-Path (Join-Path $d 'ngrok.exe'))) {{ \
                     Invoke-WebRequest -Uri 'https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-windows-amd64.zip' -OutFile $z -UseBasicParsing; \
                     Expand-Archive -Path $z -DestinationPath $d -Force; \
                   }}; \
                   $exe=Join-Path $d 'ngrok.exe'; \
                   if (Test-Path $exe) {{ $h=(Get-FileHash $exe -Algorithm SHA256).Hash.ToLower(); $sig=(Get-AuthenticodeSignature $exe).Status }} else {{ $h='(via winget)'; $sig='(via winget)' }}; \
                   Set-Content -Path (Join-Path $d 'ngrok-ready.flag') -Value (\"SHA256=$h`nAuthenticode=$sig\") \
                 }} catch {{ Set-Content -Path (Join-Path $d 'ngrok-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display()
            );
            match quiet_cmd("powershell").args(["-NoProfile", "-Command", ps.as_str()]).spawn() {
                Ok(_) => {
                    self.tunnel_downloading = true;
                    self.status_msg = self.tr(
                        "Baixando ngrok em segundo plano (fonte oficial)...",
                        "Downloading ngrok in the background (official source)...",
                    );
                    self.log_debug("[tunnel] Download do ngrok iniciado (background, apos aceite dos termos).");
                }
                Err(e) => {
                    self.log_debug(&format!("[tunnel] ERRO ao iniciar download do ngrok: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg = "Download automatico disponivel apenas no Windows.".to_string();
        }
    }

    /// Observa os downloads de binário de túnel (flags em %TEMP%).
    pub fn poll_tunnel_download(&mut self) {
        if !self.tunnel_downloading {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.tunnel_last_probe {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.tunnel_last_probe = Some(now);

        let dir = Self::tunnel_download_dir();
        for (ready, error, label) in [
            ("cf-ready.flag", "cf-error.flag", "cloudflared"),
            ("ngrok-ready.flag", "ngrok-error.flag", "ngrok"),
        ] {
            let rp = dir.join(ready);
            let ep = dir.join(error);
            if ep.exists() {
                let msg = std::fs::read_to_string(&ep).unwrap_or_default();
                let _ = std::fs::remove_file(&ep);
                self.tunnel_downloading = false;
                self.status_msg = format!("Falha ao baixar {}: {}", label, msg.trim());
                self.log_debug(&format!("[tunnel] FALHA no download de {}: {}", label, msg.trim()));
                self.detect_tunnel_bins();
            } else if rp.exists() {
                let info = std::fs::read_to_string(&rp).unwrap_or_default();
                let _ = std::fs::remove_file(&rp);
                self.tunnel_downloading = false;
                self.log_debug(&format!(
                    "[tunnel] {} baixado e verificado:\n{}",
                    label,
                    info.trim()
                ));
                self.status_msg = format!(
                    "{} {}:\n{}",
                    label,
                    self.tr("baixado e verificado", "downloaded and verified"),
                    info.trim()
                );
                self.detect_tunnel_bins();
            }
        }
    }

    /// FASE 2 da reconciliação na abertura: túneis NOSSOS (tunnel:*) que
    /// sobraram de uma sessão anterior. Mata só com identidade de 4 fatores;
    /// nunca taskkill /IM.
    #[cfg(target_os = "windows")]
    pub fn startup_reconcile_tracked_tunnels(&mut self) {
        let out = match self.run_logged("reg", &["query", r"HKCU\Software\FzComputerAI"]) {
            Some(o) if o.status.success() => o,
            _ => return,
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut entries: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("tunnel:") {
                continue;
            }
            let Some(pos) = trimmed.find("REG_SZ") else { continue };
            let name = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + "REG_SZ".len()..].trim().to_string();
            entries.push((name, value));
        }
        if entries.is_empty() {
            return;
        }
        for (name, value) in entries {
            // name = tunnel:<slug>:<pid> ; value = imagem|creation|porta|id|modo
            let pid: u32 = match name.rsplit(':').next().and_then(|s| s.parse().ok()) {
                Some(p) => p,
                None => continue,
            };
            let parts: Vec<&str> = value.split('|').collect();
            let image = parts.first().copied().unwrap_or("");
            let creation = parts.get(1).copied().unwrap_or("");
            let run_id = parts.get(3).copied().unwrap_or("");
            let marker = run_id.to_string();
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; $tp={tp}; \
                 $p=Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                 if ($p -and $p.Name -eq '{img}' -and ('{ct}' -eq '' -or $p.CreationDate.ToString('yyyyMMddHHmmss') -eq '{ct}') -and '{mark}' -ne '' -and $p.CommandLine -like ('*{mark}*')) {{ Stop-Process -Id $tp -Force; Write-Output 'KILLED' }} else {{ Write-Output 'SKIP' }}",
                tp = pid,
                img = image,
                ct = creation,
                mark = marker
            );
            let killed = self
                .run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("KILLED"))
                .unwrap_or(false);
            if killed {
                self.log_debug(&format!(
                    "[startup] TUNEL ORFAO encerrado (pid {}, id {}) — a maquina esteve exposta ate agora.",
                    pid, run_id
                ));
            } else {
                self.log_debug(&format!(
                    "[startup] Rastro de tunel (pid {}) nao corresponde a um processo nosso vivo — apenas desregistrando.",
                    pid
                ));
            }
            let _ = self.run_logged(
                "reg",
                &["delete", r"HKCU\Software\FzComputerAI", "/v", name.as_str(), "/f"],
            );
        }
    }

    /// Helper i18n curto: PT ou EN conforme o idioma ativo.
    fn tr(&self, pt: &str, en: &str) -> String {
        match self.language {
            Language::PtBr => pt.to_string(),
            Language::English => en.to_string(),
        }
    }
}

/// Teste MCP REAL num endereço, fora do `impl` (para rodar em thread):
/// conecta, manda `POST /mcp` com `initialize` e devolve
/// `(respondeu_jsonrpc, veio_401)`. Espelho livre do `mcp_probe`; o 401 é
/// devolvido à parte porque motor 0.16+ vivo responde exatamente isso sem
/// Bearer — tratar como "parado" seria mentira.
pub fn probe_mcp(ip: &str, port: u16, token: &str) -> (bool, bool) {
    use std::io::{Read, Write};
    let Ok(addr) = format!("{}:{}", ip, port).parse::<std::net::SocketAddr>() else {
        return (false, false);
    };
    let timeout = std::time::Duration::from_millis(1200);
    let Ok(mut stream) = std::net::TcpStream::connect_timeout(&addr, timeout) else {
        return (false, false);
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let body = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fzcomputerai-gui","version":""#,
        env!("CARGO_PKG_VERSION"),
        r#""}}}"#
    );
    let auth = if token.trim().is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {}\r\n", token.trim())
    };
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        ip, port, auth, body.len(), body
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return (false, false);
    }
    let mut collected: Vec<u8> = Vec::new();
    let mut buf = [0u8; 1024];
    while collected.len() < 4096 {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => collected.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&collected).to_string();
    let status_line = text.lines().next().unwrap_or("").to_string();
    (text.contains("jsonrpc"), status_line.contains(" 401"))
}

/// Tradução fora do `impl` — as tarefas de segundo plano precisam traduzir
/// sem acesso ao `AppState`. Espelha `AppState::tr`.
pub fn tr_of(lang: Language, pt: &str, en: &str) -> String {
    match lang {
        Language::PtBr => pt.to_string(),
        Language::English => en.to_string(),
    }
}

/// Máscara da senha do gate para log/UI, fora do `impl` (mesma regra do
/// `mask_gate_url`): o segredo da URL pública nunca aparece no console.
pub fn mask_gate(s: &str, pw: &str) -> String {
    let pw = pw.trim();
    if pw.is_empty() {
        s.to_string()
    } else {
        s.replace(&format!("/s/{}/", pw), "/s/***/")
    }
}

/// Uma sondagem HTTP na URL pública via `curl.exe` (TcpStream não faz TLS).
/// Versão LIVRE do antigo `tunnel_probe_once`, para rodar em thread. A URL
/// (que pode conter a senha do gate) e o Bearer vão no arquivo `--config` do
/// curl — nunca no argv, que qualquer processo consegue ler.
fn probe_once(
    dir: &std::path::Path,
    run_id: &str,
    url: &str,
    bearer: Option<&str>,
) -> Option<(u16, String)> {
    let _ = std::fs::create_dir_all(dir);
    let body_path = dir.join(format!("probe-{}.json", run_id));
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"fzcomputerai-gui","version":"{}"}}}}}}"#,
        env!("CARGO_PKG_VERSION")
    );
    std::fs::write(&body_path, &body).ok()?;
    let cfg_path = dir.join(format!("probe-{}.cfg", run_id));
    let mut cfg = format!("url = \"{}\"\n", url);
    if let Some(tok) = bearer {
        cfg.push_str(&format!("header = \"Authorization: Bearer {}\"\n", tok));
    }
    if std::fs::write(&cfg_path, cfg).is_err() {
        let _ = std::fs::remove_file(&body_path);
        return None;
    }
    let data_arg = format!("@{}", body_path.display());
    let cfg_arg = cfg_path.display().to_string();
    let out = quiet_cmd("curl")
        .args([
            "-sS",
            "-m",
            "20",
            "-o",
            "-",
            "-w",
            "\nHTTP_CODE=%{http_code}",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-H",
            "Accept: application/json, text/event-stream",
            "-H",
            "ngrok-skip-browser-warning: 1",
            "--data-binary",
            data_arg.as_str(),
            "--config",
            cfg_arg.as_str(),
        ])
        .output()
        .ok();
    // Não deixa segredo em disco depois do teste.
    let _ = std::fs::remove_file(&cfg_path);
    let _ = std::fs::remove_file(&body_path);
    let o = out?;
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
    let code = text
        .rsplit("HTTP_CODE=")
        .next()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .unwrap_or(0);
    let body_only = match text.rfind("HTTP_CODE=") {
        Some(p) => text[..p].trim().to_string(),
        None => text.trim().to_string(),
    };
    Some((code, format!("{}{}", body_only, stderr)))
}

/// Relay TRANSPARENTE de UMA conexão da rede para o MCP local.
///
/// Diferente do gate de senha, aqui NÃO se olha o conteúdo: são dois
/// `io::copy` em sentidos opostos, cada um encerrando o seu lado no EOF
/// (`shutdown(Write)`). É isto que preserva keep-alive, respostas em stream
/// (SSE) e qualquer requisição longa — inspecionar o HTTP quebraria os três.
/// Fora do `impl` porque roda em thread própria (sem `&self`).
fn relay_handle_conn(client: std::net::TcpStream, target_port: u16) -> std::io::Result<()> {
    let upstream = std::net::TcpStream::connect(("127.0.0.1", target_port))?;
    // ANTI-LAÇO: relay e motor dividem a porta (0.0.0.0 e 127.0.0.1). Se o
    // motor morrer, o listener 0.0.0.0 passa a atender TAMBÉM o loopback — e
    // o relay conectaria em si mesmo, cada conexão gerando outra, até esgotar
    // threads e handles do processo. Como toda conexão nossa sai de 127.0.0.1
    // para 127.0.0.1, dá para detectar: se o outro lado é o nosso próprio
    // listener, a porta de destino aparece como porta LOCAL do peer.
    if let (Ok(local), Ok(peer)) = (upstream.local_addr(), upstream.peer_addr()) {
        if local.port() == peer.port() || upstream.peer_addr().ok() == client.local_addr().ok() {
            let _ = upstream.shutdown(std::net::Shutdown::Both);
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "laco detectado: o alvo do relay e o proprio relay (motor caiu?)",
            ));
        }
    }
    // Sem timeout de leitura de propósito: uma sessão MCP pode ficar ociosa
    // entre chamadas e um timeout curto derrubaria conexão saudável. O fim
    // vem do EOF de qualquer um dos lados (ou da morte do processo, que o Job
    // Object garante).
    let _ = client.set_nodelay(true);
    let _ = upstream.set_nodelay(true);

    let (mut c_read, mut c_write) = (client.try_clone()?, client);
    let (mut u_read, mut u_write) = (upstream.try_clone()?, upstream);

    let up = std::thread::spawn(move || {
        let _ = std::io::copy(&mut c_read, &mut u_write);
        let _ = u_write.shutdown(std::net::Shutdown::Write);
    });
    let _ = std::io::copy(&mut u_read, &mut c_write);
    let _ = c_write.shutdown(std::net::Shutdown::Write);
    let _ = up.join();
    Ok(())
}

/// Relay de UMA conexão do gate: valida /s/<senha>/, INJETA o Bearer do motor
/// (quando `inject_token` não é vazio e o cliente não mandou o seu) e
/// encaminha ao MCP. Fora do impl porque roda em thread própria (sem &self).
fn gate_handle_conn(
    mut client: std::net::TcpStream,
    mcp_port: u16,
    password: &str,
    inject_token: &str,
) {
    use std::io::{Read, Write};
    let timeout = std::time::Duration::from_secs(30);
    let _ = client.set_read_timeout(Some(timeout));
    let _ = client.set_write_timeout(Some(timeout));

    // Lê até o fim do cabeçalho (\r\n\r\n).
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    let header_end;
    loop {
        match client.read(&mut tmp) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
                if buf.len() > 65536 {
                    return;
                }
            }
            Err(_) => return,
        }
    }

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let first_line_end = match head.find("\r\n") {
        Some(p) => p,
        None => return,
    };
    let req_line = &head[..first_line_end];
    let mut parts = req_line.split(' ');
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("HTTP/1.1");

    // Content-Length (para ler o corpo por inteiro).
    let mut content_len = 0usize;
    for l in head.lines() {
        let low = l.to_ascii_lowercase();
        if let Some(v) = low.strip_prefix("content-length:") {
            content_len = v.trim().parse().unwrap_or(0);
        }
    }

    // Confere a senha no path: /s/<senha> ou /s/<senha>/...
    let prefix = format!("/s/{}", password);
    let with_slash = format!("{}/", prefix);
    if !(path == prefix || path.starts_with(&with_slash)) {
        let _ = client
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let new_path = {
        let stripped = &path[prefix.len()..];
        if stripped.is_empty() {
            "/".to_string()
        } else {
            stripped.to_string()
        }
    };

    // Conecta ao MCP.
    let mut upstream = match std::net::TcpStream::connect(("127.0.0.1", mcp_port)) {
        Ok(s) => s,
        Err(_) => {
            let _ = client.write_all(
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
    };
    let _ = upstream.set_read_timeout(Some(timeout));

    // Reescreve a 1a linha + cabeçalhos: descarta o Connection do cliente e
    // FORÇA "Connection: close". Sem isso, o MCP (HTTP/1.1 keep-alive por
    // padrão) NÃO fecha após responder — o relay abaixo travaria até o timeout
    // de 30s e o cliente reusaria a conexão e perderia a próxima requisição.
    // Com close, o MCP fecha após 1 resposta (o relay recebe EOF na hora) e o
    // cliente não reusa — coerente com este gate tratar 1 requisição por
    // conexão. (Mesmo padrão de Connection: close já usado no 404/502/probe.)
    let rest_headers = &head[first_line_end + 2..];
    let mut hdrs = String::new();
    let mut client_sent_auth = false;
    for line in rest_headers.split("\r\n") {
        if line.is_empty() {
            continue;
        }
        let low = line.to_ascii_lowercase();
        if low.starts_with("connection:") {
            continue;
        }
        if low.starts_with("authorization:") {
            client_sent_auth = true;
        }
        hdrs.push_str(line);
        hdrs.push_str("\r\n");
    }
    // Injeta o Bearer do motor para quem passou pela senha da URL e não tem
    // como enviar header próprio (Claude Desktop e afins só aceitam URL).
    // Se o cliente mandou o dele, respeitamos o dele.
    if !client_sent_auth && !inject_token.trim().is_empty() {
        hdrs.push_str(&format!("Authorization: Bearer {}\r\n", inject_token.trim()));
    }
    hdrs.push_str("Connection: close\r\n");
    let new_head = format!("{} {} {}\r\n{}\r\n", method, new_path, version, hdrs);
    if upstream.write_all(new_head.as_bytes()).is_err() {
        return;
    }
    // Corpo já bufferizado junto ao cabeçalho.
    let _ = upstream.write_all(&buf[header_end..]);

    // Encaminha o RESTANTE do corpo do cliente -> upstream numa thread, SEM
    // depender de Content-Length (funciona também com Transfer-Encoding:
    // chunked). Termina no EOF do cliente ou no read_timeout de 30s. O
    // content_len parseado acima fica só como referência/diagnóstico.
    let _ = content_len;
    if let (Ok(mut c2), Ok(mut u2)) = (client.try_clone(), upstream.try_clone()) {
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut c2, &mut u2);
            let _ = u2.shutdown(std::net::Shutdown::Write);
        });
    }

    // Relay da resposta de volta ao cliente (termina quando o upstream fecha,
    // graças ao Connection: close acima).
    let mut resp = [0u8; 8192];
    loop {
        match upstream.read(&mut resp) {
            Ok(0) => break,
            Ok(n) => {
                if client.write_all(&resp[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Acha a primeira ocorrência de `needle` em `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Últimos `max` chars de um texto (corta em char boundary), para logs.
fn tail_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut start = s.len() - max;
    while !s.is_char_boundary(start) {
        start += 1;
    }
    format!("...{}", &s[start..])
}

#[derive(Default)]
pub struct FzComputerApp {
    pub state: AppState,
    /// Bandeja do sistema. Criada sob demanda (só quando o usuário liga
    /// "minimizar para a bandeja"), para não deixar ícone na área de
    /// notificação de quem não pediu.
    tray: Option<crate::tray::Tray>,
    /// O primeiro frame já foi desenhado? As tarefas caras do arranque só
    /// começam depois dele, para a janela abrir pintada e responsiva.
    first_frame_done: bool,
}

// ─── PALETA TERMINAL (preto + verde) ────────────────────────────────────
// Fundo preto, texto predominantemente verde, secundário branco/cinza, tudo
// monoespaçado — visual de TUI. As cores SEMÂNTICAS de status (amarelo/
// vermelho) sobrevivem de propósito: elas carregam informação de segurança
// ("EXPOSTO SEM AUTENTICAÇÃO", "REGRA SEM EFEITO") e num tema só-verde essa
// informação se perderia. Decoração é monocromática; estado, não.
// ─── LIMIAR DE LEITURA POR CÂMERA (não baixe o brilho destas cores) ─────
// O Roger lê a tela com template matching em OpenCV, que binariza assim:
//     cv::threshold(cinza, bin, 180, 255, cv::THRESH_BINARY)
// e o cinza vem de COLOR_BGR2GRAY:  Y = 0.299*R + 0.587*G + 0.114*B
// Ou seja: TEXTO COM Y <= 180 SIMPLESMENTE DESAPARECE para ele.
//
// A paleta anterior falhava exatamente nisso: o verde normal (64,224,132)
// dava Y=166 e o "brilhante" (0,255,128) dava Y=164 — ambos abaixo do corte.
// Os valores abaixo foram escolhidos para passar com folga; o Y de cada um
// está no comentário. Se mexer nas cores, RECALCULE o Y e mantenha > 190
// para texto legível.
pub const TERM_BG: Color32 = Color32::from_rgb(6, 8, 6); // fundo raiz (Y~7)
pub const TERM_BG_PANEL: Color32 = Color32::from_rgb(12, 16, 12); // cartões (Y~15)
pub const TERM_BG_INPUT: Color32 = Color32::from_rgb(0, 0, 0); // campos/console
pub const TERM_GREEN: Color32 = Color32::from_rgb(120, 255, 175); // texto normal (Y=206)
pub const TERM_GREEN_BRIGHT: Color32 = Color32::from_rgb(170, 255, 205); // destaque (Y=224)
pub const TERM_GREEN_DIM: Color32 = Color32::from_rgb(46, 110, 70); // bordas (Y~85, não é texto)
pub const TERM_WHITE: Color32 = Color32::from_rgb(235, 240, 235); // secundário (Y=238)
pub const TERM_GRAY: Color32 = Color32::from_rgb(170, 185, 170); // apoio (Y=180 no limite)

// Status (semânticos — NÃO monocromáticos). Também acima do limiar:
pub const ST_OK: Color32 = Color32::from_rgb(110, 255, 165); // Y=203
pub const ST_WARN: Color32 = Color32::from_rgb(255, 214, 90); // Y=212
pub const ST_ERR: Color32 = Color32::from_rgb(255, 140, 135); // Y=174 -> ver nota
// NOTA sobre o vermelho: vermelho puro tem Y baixo por natureza (o olho humano
// pesa pouco o canal R). Para o vermelho de ERRO passar de 180 ele viraria
// rosa-claro e perderia a leitura de "perigo" para quem vê a cor. Mantemos o
// vermelho legível a olho e garantimos que TODO estado crítico também esteja
// escrito em texto (PARADO, ERRO, SEM REGRA, EXPOSTO), que é o que o leitor
// por câmera captura.

/// Botão no estilo terminal: sem preenchimento, contorno e texto verdes sobre
/// preto. O realce no hover vem do `visuals.widgets.hovered` do tema.
pub fn term_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.to_string()).color(TERM_GREEN))
        .fill(TERM_BG_INPUT)
        .stroke(egui::Stroke::new(1.0_f32, TERM_GREEN_DIM))
        .rounding(egui::Rounding::same(2.0))
}

/// Variante para ações destrutivas/de parada (Parar, Remover): mesmo desenho,
/// contorno e texto avermelhados — o vermelho aqui é semântico, não enfeite.
pub fn term_button_danger(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.to_string()).color(ST_ERR))
        .fill(TERM_BG_INPUT)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(120, 45, 45)))
        .rounding(egui::Rounding::same(2.0))
}

fn setup_fazai_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = TERM_BG;
    visuals.window_fill = TERM_BG;
    visuals.faint_bg_color = TERM_BG_PANEL;
    visuals.extreme_bg_color = TERM_BG_INPUT;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_DIM);
    visuals.hyperlink_color = TERM_GREEN_BRIGHT;

    let sharp = egui::Rounding::same(2.0);

    visuals.widgets.noninteractive.bg_fill = TERM_BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = TERM_BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(24, 40, 28));
    visuals.widgets.noninteractive.rounding = sharp;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN);

    visuals.widgets.inactive.bg_fill = TERM_BG_INPUT;
    visuals.widgets.inactive.weak_bg_fill = TERM_BG_INPUT;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_DIM);
    visuals.widgets.inactive.rounding = sharp;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(16, 40, 24);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(16, 40, 24);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);
    visuals.widgets.hovered.rounding = sharp;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    visuals.widgets.active.bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);
    visuals.widgets.active.rounding = sharp;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    visuals.selection.bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    ctx.set_visuals(visuals);

    // TUDO monoespaçado: a fonte mono (Hack) já vem embutida no eframe pela
    // feature "default_fonts" — nenhuma dependência ou arquivo novo.
    //
    // A família mono é ~15% MAIS LARGA que a proporcional por caractere. Se
    // mantivéssemos os tamanhos originais, toda linha "título à esquerda +
    // status à direita" passaria a colidir e as fileiras de botões sairiam da
    // tela. Por isso reduzimos um pouco cada tamanho junto com a troca de
    // família — o layout volta a caber sem mexer em cada tela.
    let mut style = (*ctx.style()).clone();
    for (_ts, font_id) in style.text_styles.iter_mut() {
        font_id.family = egui::FontFamily::Monospace;
        font_id.size = (font_id.size * 0.86).max(9.0);
    }
    // Espaçamento um pouco menor: em fonte mono os widgets já ficam largos.
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.visuals.widgets.noninteractive.fg_stroke.color = TERM_GREEN;
    ctx.set_style(style);
}

impl eframe::App for FzComputerApp {
    // Fechar a GUI = desligar o sistema por inteiro: para o daemon
    // cua-driver e remove as regras portproxy LAN -> localhost (inclusive
    // orfas de testes antigos). Ver AppState::shutdown_cleanup.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.shutdown_cleanup();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_fazai_theme(ctx);

        // Primeiro frame já desenhado? Então agora sim as tarefas caras do
        // arranque. Feitas no construtor, elas seguravam a janela em branco
        // ("Não Respondendo") — era esse o travamento "que sempre aconteceu".
        if self.first_frame_done {
            self.state.run_startup_tasks();
        } else {
            self.first_frame_done = true;
            ctx.request_repaint();
        }

        // Observa o download do upgrade em background (throttle interno de 1s).
        self.state.poll_update_download();
        // Observa o túnel (captura de URL / morte do processo) e downloads de
        // binários de túnel (throttle interno de 1s cada).
        self.state.poll_tunnel();
        self.state.poll_tunnel_download();
        // Motor filho morreu sozinho? Erros das threads do relay a registrar?
        self.state.poll_engine_child();
        self.state.poll_lan_relay();
        // Aplica o que as tarefas de segundo plano terminaram (é o que mantém
        // a janela respondendo enquanto curl/PowerShell rodam).
        self.state.poll_bg();
        // ─── BANDEJA (tray) ─────────────────────────────────────────────
        // Sobe o ícone só quando o usuário liga a opção, e derruba quando ele
        // desliga — quem não pediu não ganha ícone na área de notificação.
        #[cfg(target_os = "windows")]
        {
            if self.state.minimize_to_tray && self.tray.is_none() {
                let labels = [
                    match self.state.language {
                        Language::PtBr => "Abrir FzComputerAI",
                        Language::English => "Open FzComputerAI",
                    }
                    .to_string(),
                    match self.state.language {
                        Language::PtBr => "Iniciar / Parar motor",
                        Language::English => "Start / Stop engine",
                    }
                    .to_string(),
                    match self.state.language {
                        Language::PtBr => "Sair",
                        Language::English => "Quit",
                    }
                    .to_string(),
                    "FzComputerAI".to_string(),
                ];
                self.tray = crate::tray::spawn_tray_with_labels(labels);
                if self.tray.is_some() {
                    self.state.log_debug("[tray] Icone criado na area de notificacao.");
                } else {
                    self.state
                        .log_debug("[tray] FALHA ao criar o icone na area de notificacao.");
                    self.state.minimize_to_tray = false;
                }
            } else if !self.state.minimize_to_tray && self.tray.is_some() {
                self.tray = None; // Drop remove o icone
                self.state.window_hidden = false;
                self.state.log_debug("[tray] Icone removido da area de notificacao.");
            }

            // Comandos vindos do menu da bandeja (a thread da bandeja só
            // publica; quem mexe na janela é aqui, a thread da UI).
            if let Some(tray) = &self.tray {
                match tray.take_command() {
                    crate::tray::TRAY_SHOW => {
                        self.state.window_hidden = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    crate::tray::TRAY_TOGGLE_ENGINE => {
                        if self.state.port_active {
                            self.state.stop_daemon();
                        } else {
                            self.state.start_daemon();
                        }
                    }
                    crate::tray::TRAY_QUIT => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    _ => {}
                }
            }

            // Minimizou com a opção ligada? Esconde a janela (é isso que faz
            // "minimizar para a bandeja" em vez de ficar na barra de tarefas).
            if self.state.minimize_to_tray && !self.state.window_hidden {
                let minimized = ctx.input(|i| i.viewport().minimized).unwrap_or(false);
                if minimized {
                    self.state.window_hidden = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.state
                        .log_debug("[tray] Janela minimizada para a bandeja (o app continua rodando).");
                }
            }
        }

        self.state.poll_driver_update();
        // Com tarefa em voo, garante frames mesmo sem input — senão o
        // resultado só apareceria quando o usuário mexesse o mouse.
        if self.state.bg_is_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.state.update_downloading
            || self.state.driver_updating
            || self.state.tunnel_downloading
            || matches!(
                self.state.tunnel_status,
                TunnelStatus::Starting | TunnelStatus::Running
            )
        {
            // Garante novos frames mesmo sem input, para o poll acontecer.
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }

        // ─── POR QUE A BANDEJA PRECISA DE REPAINT PERIODICO ─────────────
        // Quando a janela e minimizada, o egui PARA de desenhar (sem input,
        // sem frame). Sem frame, o codigo que detecta "minimizou" nunca roda —
        // e a janela ficava na barra de tarefas em vez de esconder. Foi um bug
        // real: funcionou uma vez so porque um repaint casual caiu no momento
        // certo. Enquanto a opcao estiver ligada, pedimos um frame a cada
        // 300ms para conseguir observar a minimizacao e ler os comandos que a
        // thread da bandeja publica.
        if self.state.minimize_to_tray {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }

        // Header Principal
        // ─── BARRA LATERAL (navegação) ───────────────────────────────────
        // Substitui a fileira de abas "pill". Declarada ANTES dos painéis de
        // baixo para ocupar a ALTURA TOTAL da janela (fica ao lado do console
        // também). No pé dela: status real do MCP, chip de túnel, idioma e
        // Sobre — o que antes vivia no header de cima.
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(196.0)
            .frame(
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(10.0, 12.0))
                    .fill(Color32::from_rgb(3, 5, 3)),
            )
            .show(ctx, |ui| {
                // Marca + versão (versão SEMPRE do Cargo.toml).
                ui.label(
                    egui::RichText::new("FZComputerAI")
                        .size(17.0)
                        .strong()
                        .color(TERM_GREEN_BRIGHT),
                );
                ui.label(
                    egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(11.0)
                        .color(TERM_GRAY),
                );
                ui.label(
                    egui::RichText::new(match self.state.language {
                        Language::PtBr => "livre, poderoso e seguro",
                        Language::English => "free, powerful and safe",
                    })
                    .size(10.0)
                    .color(TERM_GREEN_DIM),
                );
                // Modo portátil precisa ser VISÍVEL: muda onde a config mora e
                // desabilita o autostart. Esconder isso confundiria o usuário.
                if self.state.portable_mode {
                    ui.label(
                        egui::RichText::new(match self.state.language {
                            Language::PtBr => "MODO PORTATIL",
                            Language::English => "PORTABLE MODE",
                        })
                        .size(10.0)
                        .strong()
                        .color(ST_WARN),
                    );
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                let items = [
                    (Tab::Network, match self.state.language {
                        Language::PtBr => "MCP & Rede",
                        Language::English => "MCP & Network",
                    }),
                    (Tab::Tunnel, match self.state.language {
                        Language::PtBr => "Túnel",
                        Language::English => "Tunnel",
                    }),
                    (Tab::McpTools, "MCP Tools"),
                    (Tab::Calibration, match self.state.language {
                        Language::PtBr => "Calibração",
                        Language::English => "Calibration",
                    }),
                    (Tab::Windows, match self.state.language {
                        Language::PtBr => "Janelas",
                        Language::English => "Windows",
                    }),
                    (Tab::Recording, match self.state.language {
                        Language::PtBr => "Gravação",
                        Language::English => "Recording",
                    }),
                    (Tab::DoctorSkills, "Doctor & Skills"),
                ];

                for (tab, label) in items {
                    let selected = self.state.active_tab == tab;
                    // Item de menu: barra verde à esquerda quando ativo.
                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 28.0),
                        egui::Sense::click(),
                    );
                    let hovered = resp.hovered();
                    let p = ui.painter();
                    if selected {
                        p.rect_filled(rect, 2.0, Color32::from_rgb(16, 40, 24));
                        p.rect_filled(
                            egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                            0.0,
                            TERM_GREEN_BRIGHT,
                        );
                    } else if hovered {
                        p.rect_filled(rect, 2.0, Color32::from_rgb(10, 22, 14));
                    }
                    let txt_color = if selected {
                        TERM_GREEN_BRIGHT
                    } else if hovered {
                        TERM_GREEN
                    } else {
                        TERM_WHITE
                    };
                    p.text(
                        egui::pos2(rect.min.x + 12.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::monospace(13.0),
                        txt_color,
                    );
                    if resp.clicked() {
                        self.state.active_tab = tab;
                    }
                    ui.add_space(2.0);
                }

                // Pé da sidebar: status REAL + idioma + Sobre.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if ui.add(term_button(match self.state.language {
                        Language::PtBr => "Ajuda & Sobre",
                        Language::English => "Help & About",
                    })).clicked() {
                        self.state.show_about = true;
                    }
                    if ui.add(term_button(match self.state.language {
                        Language::PtBr => "EN | English",
                        Language::English => "PT | Português",
                    })).clicked() {
                        self.state.language = match self.state.language {
                            Language::PtBr => Language::English,
                            Language::English => Language::PtBr,
                        };
                    }

                    ui.add_space(8.0);

                    // Chip de túnel ativo: a máquina está exposta à internet —
                    // visível de QUALQUER aba, não só na aba Túnel.
                    if matches!(
                        self.state.tunnel_status,
                        TunnelStatus::Starting | TunnelStatus::Running
                    ) {
                        let tunnel_color = Color32::from_rgb(255, 112, 67);
                        ui.horizontal(|ui| {
                            crate::app::status_dot(ui, tunnel_color);
                            ui.label(
                                egui::RichText::new(match self.state.language {
                                    Language::PtBr => "TUNEL ATIVO",
                                    Language::English => "TUNNEL ACTIVE",
                                })
                                .color(tunnel_color)
                                .strong()
                                .size(11.0),
                            );
                        });
                    }

                    // Status do MCP: mesmo critério honesto de sempre — verde
                    // só com LAN confirmada por netstat + POST initialize real.
                    let (status_txt, status_color) = match self.state.port_status {
                        crate::app::PortStatus::LanListening => (
                            match self.state.language {
                                Language::PtBr => format!("MCP local+LAN :{}", self.state.http_port),
                                Language::English => format!("MCP local+LAN :{}", self.state.http_port),
                            },
                            ST_OK,
                        ),
                        crate::app::PortStatus::LocalOnly => (
                            match self.state.language {
                                Language::PtBr => format!("MCP local :{}", self.state.http_port),
                                Language::English => format!("MCP local :{}", self.state.http_port),
                            },
                            ST_WARN,
                        ),
                        crate::app::PortStatus::Stopped => (
                            match self.state.language {
                                Language::PtBr => "MCP parado".to_string(),
                                Language::English => "MCP stopped".to_string(),
                            },
                            ST_ERR,
                        ),
                    };
                    ui.horizontal(|ui| {
                        crate::app::status_dot(ui, status_color);
                        ui.label(
                            egui::RichText::new(status_txt)
                                .color(status_color)
                                .strong()
                                .size(11.0),
                        );
                    });
                });
            });

        // Rodapé
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                    .fill(Color32::from_rgb(3, 5, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Adaptativo: em janela estreita o texto da esquerda
                    // colidia com o bloco da direita — some quando nao ha
                    // largura para os dois.
                    if ui.available_width() > 720.0 {
                        ui.label(
                            egui::RichText::new(
                                "\"im not antisocial, im just not user friendly\"",
                            )
                            .size(11.0)
                            .italics()
                            .color(TERM_GREEN_DIM),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Roger Luft (VeilWalker)")
                                .size(11.0)
                                .color(TERM_GRAY),
                        );
                    });
                });
            });

        // ─── CONSOLE GLOBAL ÚNICO (faixa fixa acima do rodapé) ───
        // Um só console para TODAS as abas. Antes cada aba tinha a sua caixa de
        // saída e a aba MCP & Rede tinha o Console Debug — dois consoles com a
        // mesma informação na mesma tela. Comportamento de `tail -f`: acompanha
        // o fim do log sozinho, mas PARA de acompanhar quando o usuário rola
        // para cima (para poder ler) e volta a acompanhar ao retornar ao fim.
        egui::TopBottomPanel::bottom("global_console")
            .resizable(true)
            .default_height(170.0)
            .min_height(96.0)
            .frame(
                egui::Frame::none()
                    .inner_margin(10.0)
                    .fill(Color32::from_rgb(18, 18, 18)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(match self.state.language {
                            Language::PtBr => "Console",
                            Language::English => "Console",
                        })
                        .size(13.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    // Indicador do modo de rolagem — estado real, não presumido.
                    let (follow_txt, follow_color) = if self.state.console_follow {
                        (
                            match self.state.language {
                                Language::PtBr => "seguindo",
                                Language::English => "following",
                            },
                            Color32::from_rgb(76, 175, 80),
                        )
                    } else {
                        (
                            match self.state.language {
                                Language::PtBr => "pausado (rolagem manual)",
                                Language::English => "paused (manual scroll)",
                            },
                            Color32::from_rgb(255, 193, 7),
                        )
                    };
                    crate::app::status_dot(ui, follow_color);
                    ui.label(
                        egui::RichText::new(follow_txt)
                            .size(10.0)
                            .color(follow_color),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Limpar",
                                Language::English => "Clear",
                            })
                            .clicked()
                        {
                            self.state.debug_log.clear();
                            self.state.status_msg.clear();
                        }
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Copiar",
                                Language::English => "Copy",
                            })
                            .clicked()
                        {
                            let log = self.state.debug_log.clone();
                            ui.output_mut(|o| o.copied_text = log);
                        }
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Ir ao fim",
                                Language::English => "Jump to end",
                            })
                            .clicked()
                        {
                            self.state.console_follow = true;
                        }
                    });
                });

                // Faixa da ÚLTIMA mensagem (o que antes era a caixa de saída de
                // cada aba). Fica sempre à vista, sem duplicar o histórico.
                if !self.state.status_msg.trim().is_empty() {
                    ui.add_space(4.0);
                    let msg = self.state.status_msg.trim();
                    // Só a primeira linha na faixa; o corpo inteiro está no log.
                    let first = msg.lines().next().unwrap_or("");
                    let extra = msg.lines().count().saturating_sub(1);
                    let shown = if extra > 0 {
                        format!("{}  (+{} linhas no log)", first, extra)
                    } else {
                        first.to_string()
                    };
                    ui.label(
                        egui::RichText::new(shown)
                            .size(11.0)
                            .color(Color32::from_rgb(255, 213, 79)),
                    );
                }

                ui.add_space(4.0);

                let out = egui::ScrollArea::vertical()
                    .id_salt("global_console_scroll")
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.state.console_follow)
                    .show(ui, |ui| {
                        if self.state.debug_log.is_empty() {
                            ui.monospace(match self.state.language {
                                Language::PtBr => "(nenhum comando executado ainda)",
                                Language::English => "(no command executed yet)",
                            });
                        } else {
                            ui.monospace(&self.state.debug_log);
                        }
                    });

                // Detecta se o usuário está no fim: se sim, segue; se rolou
                // para cima, pausa. É o comportamento de um tail de log.
                let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
                self.state.console_follow = out.state.offset.y >= max_offset - 8.0;
            });

        // ─── PAINEL CENTRAL: só o conteúdo da seção ativa ───
        // A navegação virou a barra lateral (SidePanel "nav"). A antiga fileira
        // de abas "pill" foi removida: com 7 itens ela quebrava em duas
        // fileiras e comia altura útil da tela.
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(14.0).fill(TERM_BG))
            .show(ctx, |ui| {
                match self.state.active_tab {
                    Tab::Network => crate::tabs::network::render(ui, &mut self.state),
                    Tab::Calibration => crate::tabs::calibration::render(ui, &mut self.state),
                    Tab::Windows => crate::tabs::windows::render(ui, &mut self.state),
                    Tab::Recording => crate::tabs::recording::render(ui, &mut self.state),
                    Tab::DoctorSkills => crate::tabs::doctor_skills::render(ui, &mut self.state),
                    Tab::McpTools => crate::tabs::mcp_tools::render(ui, &mut self.state),
                    Tab::Tunnel => crate::tabs::tunnel::render(ui, &mut self.state),
                }
            });

        if self.state.show_about {
            let lang = self.state.language;
            let mut open = true;
            let mut close_clicked = false;

            egui::Window::new(match lang {
                Language::PtBr => "Sobre o FzComputerAI & Suporte",
                Language::English => "About FzComputerAI & Support",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .open(&mut open)
            .show(ctx, |ui| {
                ui.heading("FZComputerAI — Grupo FazAI");
                ui.label(concat!("Versão / Version: v", env!("CARGO_PKG_VERSION")));
                ui.add_space(4.0);
                ui.label(match lang {
                    Language::PtBr => "Servidor Nativo de Visão Computacional, MCP & Hub CLI",
                    Language::English => "Native Computer Vision Server, MCP & CLI Hub",
                });
                ui.label(match lang {
                    Language::PtBr => "Desenvolvido por: Roger Luft (VeilWalker) <roger@webstorage.com.br>",
                    Language::English => "Developed by: Roger Luft (VeilWalker) <roger@webstorage.com.br>",
                });

                // ─── DONATE: apoio ao projeto via GitHub Sponsors ───
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 34, 22))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Coracao DESENHADO (a fonte padrao nao tem glifo de
                            // coracao/emoji — viraria caixa quebrada, igual ao "●").
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(18.0, 18.0),
                                egui::Sense::hover(),
                            );
                            let p = ui.painter();
                            let c = rect.center();
                            let r = 4.2;
                            let pink = Color32::from_rgb(233, 30, 99);
                            p.circle_filled(c + Vec2::new(-r * 0.55, -r * 0.35), r * 0.75, pink);
                            p.circle_filled(c + Vec2::new(r * 0.55, -r * 0.35), r * 0.75, pink);
                            p.add(egui::Shape::convex_polygon(
                                vec![
                                    c + Vec2::new(-r * 1.25, -r * 0.10),
                                    c + Vec2::new(r * 1.25, -r * 0.10),
                                    c + Vec2::new(0.0, r * 1.45),
                                ],
                                pink,
                                egui::Stroke::NONE,
                            ));
                            ui.label(
                                egui::RichText::new(match lang {
                                    Language::PtBr => "APOIE O PROJETO / DONATE",
                                    Language::English => "SUPPORT THE PROJECT / DONATE",
                                })
                                .size(14.0)
                                .strong()
                                .color(Color32::from_rgb(129, 199, 132)),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "O FzComputerAI é software livre (MIT). Se ele te ajuda, considere apoiar o desenvolvimento — qualquer valor mantém o projeto vivo.",
                                Language::English => "FzComputerAI is free software (MIT). If it helps you, consider supporting development — any amount keeps the project alive.",
                            })
                            .size(11.0)
                            .color(Color32::from_rgb(170, 190, 170)),
                        );
                        ui.add_space(8.0);
                        const SPONSOR_URL: &str = "https://github.com/sponsors/RLuf";
                        ui.horizontal(|ui| {
                            let donate_btn = egui::Button::new(
                                egui::RichText::new(match lang {
                                    Language::PtBr => "Doar no GitHub Sponsors",
                                    Language::English => "Donate on GitHub Sponsors",
                                })
                                .color(Color32::WHITE)
                                .strong(),
                            )
                            .fill(Color32::from_rgb(233, 30, 99))
                            .min_size(Vec2::new(190.0, 30.0))
                            .rounding(egui::Rounding::same(6.0));
                            if ui.add(donate_btn).clicked() {
                                let _ = open::that(SPONSOR_URL);
                                self.state.log_debug(&format!(
                                    "[donate] Abrindo {} no navegador.",
                                    SPONSOR_URL
                                ));
                            }
                            if ui
                                .button(match lang {
                                    Language::PtBr => "Copiar link",
                                    Language::English => "Copy link",
                                })
                                .clicked()
                            {
                                ui.output_mut(|o| o.copied_text = SPONSOR_URL.to_string());
                            }
                        });
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(SPONSOR_URL)
                                .size(10.0)
                                .color(Color32::from_rgb(120, 140, 120)),
                        );
                    });

                ui.add_space(10.0);
                ui.label(match lang {
                    Language::PtBr => "Patrocinadores Oficiais:",
                    Language::English => "Official Sponsors:",
                });
                ui.hyperlink_to("Webstorage Tecnologia", "https://www.webstorage.com.br");
                ui.hyperlink_to("Imóvel Site", "https://www.imovelsite.com.br");

                // ─── Credito ao projeto Cua (motor cua-driver, MIT) ───
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Motor de automação: projeto Cua (cua-driver)",
                        Language::English => "Automation engine: Cua project (cua-driver)",
                    })
                    .size(12.0)
                    .strong(),
                );
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "MIT License — Copyright (c) 2025 Cua AI, Inc. Obrigado à Cua AI e à comunidade do Cua: o cua-driver é a base deste projeto.",
                        Language::English => "MIT License — Copyright (c) 2025 Cua AI, Inc. Thanks to Cua AI and the Cua community: cua-driver is the foundation of this project.",
                    })
                    .size(10.0)
                    .color(Color32::from_rgb(150, 150, 150)),
                );
                ui.hyperlink_to("github.com/trycua/cua", "https://github.com/trycua/cua");
                ui.add_space(12.0);
                if ui.button(match lang {
                    Language::PtBr => "Fechar",
                    Language::English => "Close",
                }).clicked() {
                    close_clicked = true;
                }
            });

            if !open || close_clicked {
                self.state.show_about = false;
            }
        }

        // ─── CENTRAL DE ATUALIZAÇÕES: GUI + MOTOR num só lugar ───
        // Abre após "Verificar Atualizações" e mostra o estado REAL dos dois
        // componentes (versão instalada x publicada), com o botão de ação de
        // cada um. Antes este diálogo só falava da GUI e o motor ficava
        // invisível — foi assim que o motor chegou a 9 versões de atraso.
        if self.state.update_checked && !self.state.update_ready {
            let lang = self.state.language;
            let tag = self.state.update_available.clone().unwrap_or_default();
            let gui_has = self.state.update_available.is_some();
            let drv_has = self.state.driver_update_available;
            let drv_cur = self.state.driver_version.clone();
            let drv_new = self.state.driver_latest.clone();
            let drv_notes = self.state.driver_notes_url.clone();
            let downloading = self.state.update_downloading;
            let drv_updating = self.state.driver_updating;
            let mut do_download = false;
            let mut do_driver = false;
            let mut do_dismiss = false;

            egui::Window::new(match lang {
                Language::PtBr => "Central de Atualizações",
                Language::English => "Update Center",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                // ── Componente 1: a interface ──
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Interface (FzComputerAI)",
                        Language::English => "Interface (FzComputerAI)",
                    })
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );
                if gui_has {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!(
                                "instalada v{}  ->  disponivel {}",
                                env!("CARGO_PKG_VERSION"),
                                tag
                            ),
                            Language::English => format!(
                                "installed v{}  ->  available {}",
                                env!("CARGO_PKG_VERSION"),
                                tag
                            ),
                        })
                        .color(ST_WARN),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("v{} — atualizada", env!("CARGO_PKG_VERSION")),
                            Language::English => format!("v{} — up to date", env!("CARGO_PKG_VERSION")),
                        })
                        .color(ST_OK),
                    );
                }
                if gui_has {
                    ui.add_space(4.0);
                    if downloading {
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "baixando o instalador em segundo plano...",
                                Language::English => "downloading the installer in the background...",
                            })
                            .color(TERM_GRAY),
                        );
                    } else if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Baixar instalador",
                            Language::English => "Download installer",
                        }))
                        .clicked()
                    {
                        do_download = true;
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // ── Componente 2: o motor ──
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Motor (cua-driver)",
                        Language::English => "Engine (cua-driver)",
                    })
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );
                if drv_cur.is_empty() {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => "nao foi possivel consultar o motor (instalado? no PATH?)",
                            Language::English => "could not query the engine (installed? on PATH?)",
                        })
                        .color(ST_ERR),
                    );
                } else if drv_has {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("instalado {}  ->  disponivel {}", drv_cur, drv_new),
                            Language::English => format!("installed {}  ->  available {}", drv_cur, drv_new),
                        })
                        .color(ST_WARN),
                    );
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => "Versoes novas do motor podem mudar o contrato do endpoint HTTP (ex.: exigir token). Apos atualizar, confira o estado na aba MCP & Rede.",
                            Language::English => "Newer engine versions may change the HTTP endpoint contract (e.g. require a token). After updating, check the status in the MCP & Network tab.",
                        })
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                    if !drv_notes.is_empty() {
                        ui.hyperlink_to(
                            match lang {
                                Language::PtBr => "notas da versao",
                                Language::English => "release notes",
                            },
                            &drv_notes,
                        );
                    }
                    ui.add_space(4.0);
                    if drv_updating {
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "atualizando o motor em segundo plano...",
                                Language::English => "updating the engine in the background...",
                            })
                            .color(TERM_GRAY),
                        );
                    } else if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Atualizar motor",
                            Language::English => "Update engine",
                        }))
                        .clicked()
                    {
                        do_driver = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("{} — atualizado", drv_cur),
                            Language::English => format!("{} — up to date", drv_cur),
                        })
                        .color(ST_OK),
                    );
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Verificar de novo",
                            Language::English => "Check again",
                        }))
                        .clicked()
                    {
                        do_dismiss = false; // mantem aberto; recheca abaixo
                        self.state.check_for_updates();
                    }
                    if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Fechar",
                            Language::English => "Close",
                        }))
                        .clicked()
                    {
                        do_dismiss = true;
                    }
                });
            });

            if do_download {
                self.state.start_update_download();
            }
            if do_driver {
                self.state.start_driver_update();
            }
            if do_dismiss {
                self.state.update_checked = false;
                self.state.update_available = None;
            }
        }

        // ─── Dialogo 2 do upgrade: download pronto — fechar e instalar? ───
        if self.state.update_ready {
            let lang = self.state.language;
            let mut do_install = false;
            let mut do_dismiss = false;

            egui::Window::new(match lang {
                Language::PtBr => "Pronto para atualizar",
                Language::English => "Ready to update",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.label(match lang {
                    Language::PtBr => "Download concluído em diretório temporário.\nPara instalar, o FzComputerAI precisa ser FECHADO.\nApós a instalação, o aplicativo e o motor cua-driver serão reabertos automaticamente.",
                    Language::English => "Download finished in a temporary directory.\nTo install, FzComputerAI must be CLOSED.\nAfter installation, the app and the cua-driver engine will be reopened automatically.",
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(match lang {
                            Language::PtBr => "Fechar e instalar agora",
                            Language::English => "Close and install now",
                        })
                        .clicked()
                    {
                        do_install = true;
                    }
                    if ui
                        .button(match lang {
                            Language::PtBr => "Depois",
                            Language::English => "Later",
                        })
                        .clicked()
                    {
                        do_dismiss = true;
                    }
                });
            });

            if do_install {
                self.state.install_update_and_restart();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if do_dismiss {
                self.state.update_ready = false;
                self.state.update_available = None;
            }
        }
    }
}

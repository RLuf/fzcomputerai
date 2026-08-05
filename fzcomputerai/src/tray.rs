//! Ícone na bandeja do sistema (área de notificação do Windows).
//!
//! POR QUE FEITO NA MÃO, SEM CRATE: o `eframe`/`winit` não oferece bandeja, e as
//! crates que oferecem (tray-icon e afins) puxam uma árvore de dependências
//! inteira. O `AGENTS.md` pede para não introduzir dependência desnecessária, e
//! este projeto já fala HTTP na mão pelo mesmo motivo. Aqui declaramos apenas as
//! funções do Win32 que realmente usamos, via `extern "system"`.
//!
//! COMO FUNCIONA: a bandeja precisa de uma janela com laço de mensagens para
//! receber os cliques. A janela do egui é do winit e não podemos assumir o laço
//! dela, então criamos uma janela OCULTA (message-only) numa THREAD PRÓPRIA, com
//! o seu próprio `GetMessage`. Essa thread nunca toca a UI: ela só publica um
//! comando num `AtomicU8` que o `update()` do egui lê a cada frame. Sem canal,
//! sem mutex, sem risco de travar o desenho.
//!
//! Fora do Windows tudo aqui vira no-op (a bandeja é específica de plataforma).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Título EXATO da janela principal. Mora aqui e é usado pelo `main.rs` ao
/// criar a janela, porque a thread da bandeja precisa localizá-la por título
/// (`FindWindowW`) para poder restaurá-la. Se os dois lados divergirem, o
/// clique na bandeja para de funcionar — então há UMA fonte só.
pub const MAIN_WINDOW_TITLE: &str = "FzComputerAI — Computer Vision MCP Manager";

/// Comandos que a bandeja publica para a interface. Lidos e zerados pelo
/// `update()` — quem manda na janela é sempre a thread da UI.
pub const TRAY_NONE: u8 = 0;
pub const TRAY_SHOW: u8 = 1;
pub const TRAY_QUIT: u8 = 2;
pub const TRAY_TOGGLE_ENGINE: u8 = 3;

/// Alça da bandeja. Guardar viva enquanto o app existir; ao dropar, o ícone é
/// removido da área de notificação.
pub struct Tray {
    cmd: Arc<AtomicU8>,
    #[cfg(target_os = "windows")]
    hwnd: usize, // HWND da janela oculta, para pedir o encerramento no drop
}

impl Tray {
    /// Último comando pendente (e zera). Chame no `update()`.
    pub fn take_command(&self) -> u8 {
        self.cmd.swap(TRAY_NONE, Ordering::SeqCst)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn spawn_tray() -> Option<Tray> {
    None
}

#[cfg(target_os = "windows")]
mod win {
    #![allow(non_snake_case, non_camel_case_types)]

    pub type HWND = *mut core::ffi::c_void;
    pub type HICON = *mut core::ffi::c_void;
    pub type HMENU = *mut core::ffi::c_void;
    pub type HINSTANCE = *mut core::ffi::c_void;
    pub type HMODULE = *mut core::ffi::c_void;
    pub type WPARAM = usize;
    pub type LPARAM = isize;
    pub type LRESULT = isize;
    pub type BOOL = i32;
    pub type UINT = u32;
    pub type DWORD = u32;

    pub const WM_DESTROY: UINT = 0x0002;
    pub const WM_CLOSE: UINT = 0x0010;
    pub const WM_COMMAND: UINT = 0x0111;
    pub const WM_APP_TRAY: UINT = 0x8000 + 1; // WM_APP + 1: nossa notificação
    pub const WM_LBUTTONUP: UINT = 0x0202;
    pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
    pub const WM_RBUTTONUP: UINT = 0x0205;

    pub const NIM_ADD: DWORD = 0x0000;
    pub const NIM_DELETE: DWORD = 0x0002;
    pub const NIF_MESSAGE: UINT = 0x0001;
    pub const NIF_ICON: UINT = 0x0002;
    pub const NIF_TIP: UINT = 0x0004;

    pub const IDI_APPLICATION: usize = 32512;
    pub const IMAGE_ICON: UINT = 1;
    pub const LR_DEFAULTSIZE: UINT = 0x0040;
    pub const LR_SHARED: UINT = 0x8000;

    pub const MF_STRING: UINT = 0x0000;
    pub const MF_SEPARATOR: UINT = 0x0800;
    pub const TPM_RIGHTBUTTON: UINT = 0x0002;

    pub const ID_SHOW: usize = 1001;
    pub const ID_ENGINE: usize = 1002;
    pub const ID_QUIT: usize = 1003;

    pub const SW_SHOW: i32 = 5;
    pub const SW_RESTORE: i32 = 9;

    /// Traz a janela principal de volta, DIRETO pelo Win32.
    ///
    /// POR QUE AQUI E NÃO NA THREAD DA UI: com a janela escondida o egui para
    /// de desenhar (sem frame, sem `update()`), então um comando publicado para
    /// a UI ler NUNCA seria lido — a janela ficaria presa escondida. Foi
    /// exatamente o bug observado no teste. A thread da bandeja tem laço de
    /// mensagens próprio e roda sempre, então é ela quem mostra a janela; o
    /// `update()` só sincroniza o estado depois, quando o desenho volta.
    pub unsafe fn restore_main_window() {
        let title = wide(super::MAIN_WINDOW_TITLE);
        let h = FindWindowW(std::ptr::null(), title.as_ptr());
        if !h.is_null() {
            ShowWindow(h, SW_SHOW);
            ShowWindow(h, SW_RESTORE);
            SetForegroundWindow(h);
        }
    }

    #[repr(C)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    pub struct MSG {
        pub hwnd: HWND,
        pub message: UINT,
        pub wParam: WPARAM,
        pub lParam: LPARAM,
        pub time: DWORD,
        pub pt: POINT,
    }

    #[repr(C)]
    pub struct WNDCLASSW {
        pub style: UINT,
        pub lpfnWndProc: Option<
            unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
        >,
        pub cbClsExtra: i32,
        pub cbWndExtra: i32,
        pub hInstance: HINSTANCE,
        pub hIcon: HICON,
        pub hCursor: *mut core::ffi::c_void,
        pub hbrBackground: *mut core::ffi::c_void,
        pub lpszMenuName: *const u16,
        pub lpszClassName: *const u16,
    }

    // NOTIFYICONDATAW: o campo cbSize define a versao da struct para o shell.
    // Declaramos ate szTip (128 wchars) — o suficiente para NIF_MESSAGE|ICON|TIP.
    #[repr(C)]
    pub struct NOTIFYICONDATAW {
        pub cbSize: DWORD,
        pub hWnd: HWND,
        pub uID: UINT,
        pub uFlags: UINT,
        pub uCallbackMessage: UINT,
        pub hIcon: HICON,
        pub szTip: [u16; 128],
        pub dwState: DWORD,
        pub dwStateMask: DWORD,
        pub szInfo: [u16; 256],
        pub uVersionOrTimeout: UINT,
        pub szInfoTitle: [u16; 64],
        pub dwInfoFlags: DWORD,
        pub guidItem: [u8; 16],
        pub hBalloonIcon: HICON,
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn RegisterClassW(c: *const WNDCLASSW) -> u16;
        pub fn CreateWindowExW(
            ex: DWORD,
            class: *const u16,
            name: *const u16,
            style: DWORD,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: HWND,
            menu: HMENU,
            inst: HINSTANCE,
            param: *mut core::ffi::c_void,
        ) -> HWND;
        pub fn DefWindowProcW(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT;
        pub fn GetMessageW(msg: *mut MSG, h: HWND, min: UINT, max: UINT) -> BOOL;
        pub fn TranslateMessage(msg: *const MSG) -> BOOL;
        pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
        pub fn PostQuitMessage(code: i32);
        pub fn PostMessageW(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> BOOL;
        pub fn FindWindowW(class: *const u16, name: *const u16) -> HWND;
        pub fn ShowWindow(h: HWND, cmd: i32) -> BOOL;
        pub fn LoadIconW(inst: HINSTANCE, name: usize) -> HICON;
        pub fn LoadImageW(
            inst: HINSTANCE,
            name: *const u16,
            typ: UINT,
            cx: i32,
            cy: i32,
            load: UINT,
        ) -> HICON;
        pub fn CreatePopupMenu() -> HMENU;
        pub fn AppendMenuW(m: HMENU, flags: UINT, id: usize, item: *const u16) -> BOOL;
        pub fn DestroyMenu(m: HMENU) -> BOOL;
        pub fn TrackPopupMenu(
            m: HMENU,
            flags: UINT,
            x: i32,
            y: i32,
            reserved: i32,
            h: HWND,
            rect: *const core::ffi::c_void,
        ) -> BOOL;
        pub fn GetCursorPos(p: *mut POINT) -> BOOL;
        pub fn SetForegroundWindow(h: HWND) -> BOOL;
    }

    #[link(name = "shell32")]
    extern "system" {
        pub fn Shell_NotifyIconW(msg: DWORD, data: *mut NOTIFYICONDATAW) -> BOOL;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetModuleHandleW(name: *const u16) -> HMODULE;
    }

    /// String Rust -> buffer UTF-16 terminado em zero.
    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn fill_tip(dst: &mut [u16; 128], s: &str) {
        let src: Vec<u16> = s.encode_utf16().take(127).collect();
        dst[..src.len()].copy_from_slice(&src);
        dst[src.len()] = 0;
    }
}

#[cfg(target_os = "windows")]
static TRAY_CMD: std::sync::OnceLock<Arc<AtomicU8>> = std::sync::OnceLock::new();

/// Rótulos do menu, definidos pela UI (bilíngue) antes de subir a bandeja.
#[cfg(target_os = "windows")]
static TRAY_LABELS: std::sync::OnceLock<[String; 4]> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: win::HWND,
    msg: win::UINT,
    wparam: win::WPARAM,
    lparam: win::LPARAM,
) -> win::LRESULT {
    use win::*;
    match msg {
        WM_APP_TRAY => {
            let event = (lparam as u32) & 0xFFFF;
            match event {
                // Clique simples ou duplo: mostrar a janela. A restauração é
                // feita AQUI (Win32 direto) porque com a janela escondida o
                // egui não desenha e não leria comando nenhum. O flag serve só
                // para o `update()` sincronizar `window_hidden` depois.
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    restore_main_window();
                    if let Some(c) = TRAY_CMD.get() {
                        c.store(TRAY_SHOW, Ordering::SeqCst);
                    }
                }
                // Botão direito: menu de contexto.
                WM_RBUTTONUP => {
                    let labels = TRAY_LABELS
                        .get()
                        .cloned()
                        .unwrap_or_else(|| {
                            [
                                "Abrir".to_string(),
                                "Iniciar/Parar motor".to_string(),
                                "Sair".to_string(),
                                "FzComputerAI".to_string(),
                            ]
                        });
                    let menu = CreatePopupMenu();
                    if !menu.is_null() {
                        let l0 = wide(&labels[0]);
                        let l1 = wide(&labels[1]);
                        let l2 = wide(&labels[2]);
                        AppendMenuW(menu, MF_STRING, ID_SHOW, l0.as_ptr());
                        AppendMenuW(menu, MF_STRING, ID_ENGINE, l1.as_ptr());
                        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                        AppendMenuW(menu, MF_STRING, ID_QUIT, l2.as_ptr());
                        let mut pt = POINT { x: 0, y: 0 };
                        GetCursorPos(&mut pt);
                        // SetForegroundWindow antes do TrackPopupMenu: sem isso o
                        // menu nao fecha ao clicar fora (comportamento documentado).
                        SetForegroundWindow(hwnd);
                        TrackPopupMenu(
                            menu,
                            TPM_RIGHTBUTTON,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            std::ptr::null(),
                        );
                        DestroyMenu(menu);
                    }
                }
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            // Restaura a janela ANTES de publicar qualquer comando: escondida,
            // a UI nao desenha e nao processaria "Sair" nem "Iniciar/Parar".
            // Com a janela de volta, o `update()` roda e trata o comando.
            restore_main_window();
            if let Some(c) = TRAY_CMD.get() {
                match id {
                    win::ID_SHOW => c.store(TRAY_SHOW, Ordering::SeqCst),
                    win::ID_ENGINE => c.store(TRAY_TOGGLE_ENGINE, Ordering::SeqCst),
                    win::ID_QUIT => c.store(TRAY_QUIT, Ordering::SeqCst),
                    _ => {}
                }
            }
            0
        }
        WM_CLOSE | WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Sobe a bandeja numa thread própria. `labels` = [Abrir, Motor, Sair, tooltip].
#[cfg(target_os = "windows")]
pub fn spawn_tray_with_labels(labels: [String; 4]) -> Option<Tray> {
    use win::*;

    let cmd = Arc::new(AtomicU8::new(TRAY_NONE));
    let _ = TRAY_CMD.set(cmd.clone());
    let _ = TRAY_LABELS.set(labels.clone());

    // A janela precisa nascer NA thread que roda o laço de mensagens.
    let (tx, rx) = std::sync::mpsc::channel::<usize>();
    std::thread::spawn(move || unsafe {
        let inst = GetModuleHandleW(std::ptr::null());
        let class_name = wide("FzComputerAITrayWnd");

        // Icone: tenta o do proprio executavel (recurso embutido pelo build.rs);
        // se nao houver, cai no icone padrao do Windows.
        let icon_res = wide("1");
        let mut hicon = LoadImageW(
            inst as HINSTANCE,
            icon_res.as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        );
        if hicon.is_null() {
            hicon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        }

        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst as HINSTANCE,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc); // falha se ja registrada — inofensivo

        let title = wide("FzComputerAI Tray");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            0, // sem WS_VISIBLE: janela oculta, so para mensagens
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inst as HINSTANCE,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            let _ = tx.send(0);
            return;
        }

        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_APP_TRAY;
        nid.hIcon = hicon;
        fill_tip(&mut nid.szTip, &labels[3]);
        Shell_NotifyIconW(NIM_ADD, &mut nid);

        let _ = tx.send(hwnd as usize);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Saindo: remove o icone para nao deixar fantasma na bandeja.
        Shell_NotifyIconW(NIM_DELETE, &mut nid);
    });

    let hwnd = rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(0);
    if hwnd == 0 {
        return None;
    }
    Some(Tray { cmd, hwnd })
}

#[cfg(target_os = "windows")]
impl Drop for Tray {
    fn drop(&mut self) {
        // Pede o encerramento do laço; o próprio laço remove o ícone.
        if self.hwnd != 0 {
            unsafe {
                win::PostMessageW(self.hwnd as win::HWND, win::WM_CLOSE, 0, 0);
            }
        }
    }
}

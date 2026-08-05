//! Ciclo de vida dos processos filhos: TUDO morre com a GUI.
//!
//! POR QUE JOB OBJECT E NÃO WATCHDOG: no Windows um processo filho NÃO morre
//! com o pai — `CreateProcess` não cria vínculo. A versão anterior deste
//! projeto resolvia isso com um auxiliar PowerShell que ficava vigiando o PID
//! da GUI, o que tinha três defeitos: (1) dependia de um terceiro processo que
//! podia falhar/ser morto; (2) matava o motor com `taskkill /F /IM
//! cua-driver.exe`, atingindo daemons de OUTROS usos (o `.mcp.json` do Claude
//! Code sobe o seu próprio) — exatamente o que o AGENTS.md proíbe; (3) havia
//! uma janela de tempo entre a morte da GUI e a reação do vigia.
//!
//! O Job Object resolve pelo KERNEL. Criamos um job com
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` e mantemos o handle aberto pela vida
//! inteira do processo. Todo filho que a GUI cria é ADOTADO pelo job. Quando o
//! processo da GUI termina — de qualquer forma, inclusive `taskkill /F`, crash,
//! logoff ou fim de energia do processo — o Windows fecha os handles, o job
//! deixa de existir e o kernel mata TODOS os processos dele. Não há vigia para
//! falhar, não há corrida, e nada além dos NOSSOS filhos é atingido.
//!
//! Feito com `extern "system"` na mão, como já se faz em `tray.rs`: o AGENTS.md
//! pede para não introduzir dependência desnecessária, e são três funções.
//!
//! Fora do Windows tudo aqui é no-op honesto (`adopt` devolve false e quem
//! chama registra isso) — em Unix o encerramento fica a cargo do `kill` direto
//! no `Child`, que o `shutdown_cleanup` já faz.

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
mod win {
    #![allow(non_snake_case, non_camel_case_types)]

    pub type HANDLE = *mut core::ffi::c_void;
    pub type BOOL = i32;
    pub type DWORD = u32;

    /// `JobObjectExtendedLimitInformation` — a classe que carrega LimitFlags.
    pub const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: DWORD = 9;
    /// Mata todo processo do job quando o ÚLTIMO handle do job fecha.
    pub const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x0000_2000;

    #[repr(C)]
    #[derive(Default)]
    pub struct IO_COUNTERS {
        pub ReadOperationCount: u64,
        pub WriteOperationCount: u64,
        pub OtherOperationCount: u64,
        pub ReadTransferCount: u64,
        pub WriteTransferCount: u64,
        pub OtherTransferCount: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        pub PerProcessUserTimeLimit: i64,
        pub PerJobUserTimeLimit: i64,
        pub LimitFlags: DWORD,
        pub MinimumWorkingSetSize: usize,
        pub MaximumWorkingSetSize: usize,
        pub ActiveProcessLimit: DWORD,
        pub Affinity: usize,
        pub PriorityClass: DWORD,
        pub SchedulingClass: DWORD,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        pub BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        pub IoInfo: IO_COUNTERS,
        pub ProcessMemoryLimit: usize,
        pub JobMemoryLimit: usize,
        pub PeakProcessMemoryUsed: usize,
        pub PeakJobMemoryUsed: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateJobObjectW(attrs: *mut core::ffi::c_void, name: *const u16) -> HANDLE;
        pub fn SetInformationJobObject(
            job: HANDLE,
            class: DWORD,
            info: *mut core::ffi::c_void,
            len: DWORD,
        ) -> BOOL;
        pub fn AssignProcessToJobObject(job: HANDLE, process: HANDLE) -> BOOL;
        pub fn GetLastError() -> DWORD;
    }
}

/// Handle do job, guardado para a vida inteira do processo. NUNCA fechar: é o
/// fechamento dele (na morte da GUI) que dispara a matança dos filhos.
#[cfg(target_os = "windows")]
static JOB: OnceLock<usize> = OnceLock::new();

/// Cria o job de "morre comigo". Idempotente; devolve true se há job utilizável.
/// Chame UMA vez, cedo no startup, antes de qualquer spawn de longa duração.
#[cfg(target_os = "windows")]
pub fn init() -> bool {
    if JOB.get().is_some() {
        return true;
    }
    unsafe {
        let job = win::CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if job.is_null() {
            return false;
        }
        let mut info = win::JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = win::JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = win::SetInformationJobObject(
            job,
            win::JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
            &mut info as *mut _ as *mut core::ffi::c_void,
            std::mem::size_of::<win::JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) != 0;
        if !ok {
            // Sem o LimitFlags o job não mata ninguém — melhor não ter job do
            // que ter um job que mente sobre garantir a limpeza.
            return false;
        }
        let _ = JOB.set(job as usize);
        true
    }
}

/// Adota um filho já criado: ele passa a morrer junto com esta GUI.
/// Devolve `Ok(())` ou `Err(código do Windows)` — quem chama DEVE registrar a
/// falha, porque sem adoção não há garantia de limpeza (status honesto).
#[cfg(target_os = "windows")]
pub fn adopt(child: &std::process::Child) -> Result<(), u32> {
    use std::os::windows::io::AsRawHandle;
    let Some(job) = JOB.get() else {
        return Err(0);
    };
    unsafe {
        let h = child.as_raw_handle() as win::HANDLE;
        if win::AssignProcessToJobObject(*job as win::HANDLE, h) != 0 {
            Ok(())
        } else {
            Err(win::GetLastError())
        }
    }
}

/// Há um job ativo? (para a UI dizer a verdade sobre a garantia de limpeza)
#[cfg(target_os = "windows")]
pub fn is_active() -> bool {
    JOB.get().is_some()
}

#[cfg(not(target_os = "windows"))]
pub fn init() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn adopt(_child: &std::process::Child) -> Result<(), u32> {
    Err(0)
}

#[cfg(not(target_os = "windows"))]
pub fn is_active() -> bool {
    false
}

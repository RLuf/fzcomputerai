//! Build script do FzComputerAI.
//!
//! Objetivo unico: embutir o recurso VERSIONINFO do Windows no executavel.
//! Sem isso, o `fzcomputerai.exe` sai com "Detalhes" (Propriedades do arquivo)
//! COMPLETAMENTE EM BRANCO — o cargo nao gera esse recurso por conta propria.
//! A versao aparecia so dentro da janela da GUI (env!("CARGO_PKG_VERSION")),
//! nunca nas propriedades do binario, que e onde o usuario/antivirus/SmartScreen
//! olham.
//!
//! REGRAS QUE ESTE ARQUIVO PRECISA RESPEITAR
//! -----------------------------------------
//! 1. NO-OP FORA DO WINDOWS. O CI compila em ubuntu-latest e macos-latest.
//!    Se este script tentar usar `winresource` la, o job inteiro quebra. Por
//!    isso ha DOIS niveis de guarda (ver `main` abaixo).
//! 2. NADA DE VERSAO HARDCODED. Tudo vem de CARGO_PKG_VERSION, que por sua vez
//!    vem do Cargo.toml (e, no release, do stamp de versao do CI). Editar a
//!    versao em um lugar so continua sendo suficiente.
//! 3. VERSAO PRE-RELEASE PRECISA VIRAR NUMERO. O campo binario FILEVERSION do
//!    VS_FIXEDFILEINFO aceita SOMENTE 4 inteiros de 16 bits. Um tag como
//!    "1.0.3-rc1" precisa ser saneado antes — mesma logica conceitual ja usada
//!    em installer/fzcomputerai.iss (bloco "VersionInfoVersion NUMERICA").
//!
//! FERRAMENTA EXTERNA
//! ------------------
//! O `winresource` gera um .rc e o compila. Com a toolchain MSVC ele usa o
//! `rc.exe` do Windows SDK, que ja acompanha o Build Tools do Visual Studio
//! (o runner windows-latest do GitHub Actions tem o SDK instalado, entao nao ha
//! dependencia extra a declarar no workflow). Com toolchain GNU ele usaria
//! `windres`. Em cross-compile a partir de Linux/macOS este script nem chega a
//! rodar o winresource — ver guarda 2 em `main`.

fn main() {
    // Regravar o recurso quando a versao (Cargo.toml) ou este script mudarem.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // GUARDA 1 (host): `winresource` so e declarado como build-dependency em
    // [target.'cfg(windows)'.build-dependencies]. Esse seletor olha para o
    // HOST que compila o build script. Fora do Windows o crate nem existe,
    // entao o codigo que o referencia nao pode ser compilado.
    #[cfg(windows)]
    embed_windows_version_info();
}

/// Sanea uma versao semver para a forma "x.y.z.w" aceita pelo VERSIONINFO.
///
/// Regra: pula o que vier antes do primeiro digito (tolera "v1.0.3") e copia
/// digitos e pontos ate o primeiro caractere invalido. Depois normaliza para
/// exatamente 4 componentes numericos.
///
///   "1.0.2"         -> "1.0.2.0"
///   "1.0.3-rc1"     -> "1.0.3.0"
///   "2.10.0-beta.4" -> "2.10.0.0"
///   "1.0.3+build7"  -> "1.0.3.0"
///   "v1.0.3.7"      -> "1.0.3.7"
///   "nightly"       -> "0.0.0.0"  (rede de seguranca: nunca aborta o build)
#[cfg(windows)]
fn sanitize_version(raw: &str) -> [u16; 4] {
    let numeric: String = raw
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    let mut parts = [0u16; 4];
    for (i, piece) in numeric.split('.').take(4).enumerate() {
        // Componente vazio (ex.: "1..2") ou maior que 65535 vira 0 em vez de
        // derrubar o build.
        parts[i] = piece.parse::<u16>().unwrap_or(0);
    }
    parts
}

#[cfg(windows)]
fn embed_windows_version_info() {
    // GUARDA 2 (alvo): build scripts sao compilados para o HOST, entao a
    // guarda 1 continua verdadeira quando um host Windows faz cross-compile
    // para Linux/macOS. O alvo REAL vem desta env var, setada pelo cargo.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let pkg_version =
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let [major, minor, patch, build] = sanitize_version(&pkg_version);
    let numeric_version = format!("{}.{}.{}.{}", major, minor, patch, build);

    // VS_FIXEDFILEINFO empacota os 4 componentes em um u64.
    let packed: u64 = ((major as u64) << 48)
        | ((minor as u64) << 32)
        | ((patch as u64) << 16)
        | (build as u64);

    let mut res = winresource::WindowsResource::new();

    // StringFileInfo — o que o Explorer mostra na aba "Detalhes" e o que o
    // PowerShell le em (Get-Item exe).VersionInfo.
    res.set("FileVersion", &numeric_version);
    res.set("ProductVersion", &numeric_version);
    res.set("ProductName", "FzComputerAI");
    res.set("FileDescription", "FzComputerAI - Computer Vision, MCP & CLI Hub");
    res.set("CompanyName", "Webstorage Tecnologia");
    res.set(
        "LegalCopyright",
        "Roger Luft / Webstorage Tecnologia - CC BY 4.0",
    );
    res.set("OriginalFilename", "fzcomputerai.exe");
    // Versao "de marketing" completa, COM sufixo de pre-release. Campo livre:
    // e o unico lugar do recurso onde "1.0.3-rc1" cabe sem truncar.
    res.set("InternalName", "fzcomputerai");
    if numeric_version != pkg_version {
        res.set("Comments", &format!("Versao completa: {}", pkg_version));
    }

    // VS_FIXEDFILEINFO — parte BINARIA (a que instaladores e o proprio Windows
    // comparam para decidir "arquivo mais novo"). Precisa casar com as strings.
    res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
    res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);

    // Icone: OPCIONAL. Hoje o repositorio nao tem nenhum .ico (assets/img so
    // tem PNG). Mesmo caminho que installer/fzcomputerai.iss ja documenta no
    // seu bloco #ifexist — basta largar o arquivo la e recompilar.
    let icon = std::path::Path::new("../installer/fzcomputerai.ico");
    println!("cargo:rerun-if-changed=../installer/fzcomputerai.ico");
    if icon.is_file() {
        res.set_icon(icon.to_str().unwrap_or_default());
    }

    // Falha aqui NAO pode derrubar o build: sem rc.exe/windres o binario ainda
    // e perfeitamente funcional, so volta a ficar sem VERSIONINFO. Emitimos um
    // aviso visivel no log em vez de um erro fatal.
    if let Err(e) = res.compile() {
        println!(
            "cargo:warning=VERSIONINFO NAO embutido ({}). O .exe ficara sem \
             versao nas Propriedades. Verifique se o Windows SDK (rc.exe) esta \
             instalado.",
            e
        );
    }
}

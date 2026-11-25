//! Instalação por usuário, sem precisar de administrador.

use crate::{claude, config, versao};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::{env, fs, process::Command};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

/// Chave em "Aplicativos" do Windows; o perfil de teste usa uma chave à parte.
fn registro() -> String {
    let sufixo = config::perfil().map(|p| format!("-{p}")).unwrap_or_default();
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}{sufixo}", config::NOME)
}

const SEM_JANELA: u32 = 0x0800_0000;

pub fn pasta_programa() -> Result<PathBuf, String> {
    Ok(config::pasta("LOCALAPPDATA")?.join("Programs").join(config::NOME))
}

pub fn exe_instalado() -> Result<PathBuf, String> {
    Ok(pasta_programa()?.join("premosienge.exe"))
}

fn atalho() -> Result<PathBuf, String> {
    Ok(config::pasta("APPDATA")?
        .join(r"Microsoft\Windows\Start Menu\Programs")
        .join("MCP PremoSienge.lnk"))
}

/// O mesmo executável vira instalador quando o nome começa com "Instalador".
pub fn modo_instalador() -> bool {
    env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|n| n.to_string_lossy().to_lowercase().starts_with("instalador")))
        .unwrap_or(false)
}

pub type Passo = Result<String, String>;

fn copiar() -> Passo {
    let origem = env::current_exe().map_err(|e| e.to_string())?;
    let destino = exe_instalado()?;
    if origem == destino {
        return Ok("Programa: já está na pasta de instalação.".into());
    }
    let dir = pasta_programa()?;
    let erro = |e: std::io::Error| format!("Programa: não foi possível copiar para {} ({e}).", dir.display());
    fs::create_dir_all(&dir).map_err(erro)?;
    // Sobra de atualização anterior; se o Claude ainda a usa, fica para a próxima
    for i in fs::read_dir(&dir).map_err(erro)?.flatten() {
        if i.path().extension().is_some_and(|e| e == "antigo") {
            let _ = fs::remove_file(i.path());
        }
    }
    let novo = dir.join("premosienge.novo");
    fs::copy(&origem, &novo).map_err(erro)?;
    if destino.exists() {
        // O Windows deixa renomear um executável em uso, mas não sobrescrever
        let antigo = dir.join(format!("premosienge-{}.antigo", std::process::id()));
        fs::rename(&destino, &antigo).map_err(erro)?;
        if let Err(e) = fs::rename(&novo, &destino) {
            let _ = fs::rename(&antigo, &destino);
            return Err(erro(e));
        }
        let _ = fs::remove_file(&antigo);
    } else {
        fs::rename(&novo, &destino).map_err(erro)?;
    }
    Ok("Programa: instalado.".into())
}

fn criar_atalho() -> Passo {
    let f = atalho()?;
    if let Some(dir) = f.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("Atalho: {e}"))?;
    }
    mslnk::ShellLink::new(exe_instalado()?)
        .and_then(|l| l.create_lnk(&f))
        .map_err(|e| format!("Atalho: não foi possível criar ({e})."))?;
    Ok("Atalho: criado no menu Iniciar.".into())
}

fn registrar() -> Passo {
    let exe = exe_instalado()?;
    let gravar = || -> std::io::Result<()> {
        let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(registro())?;
        k.set_value("DisplayName", &"MCP PremoSienge")?;
        k.set_value("DisplayVersion", &versao::numero())?;
        k.set_value("Publisher", &"PremoSienge")?;
        k.set_value("InstallLocation", &exe.parent().unwrap_or(&exe).as_os_str())?;
        k.set_value("DisplayIcon", &exe.as_os_str())?;
        k.set_value("UninstallString", &format!("\"{}\" desinstalar", exe.display()))?;
        k.set_value("NoModify", &1u32)?;
        k.set_value("NoRepair", &1u32)
    };
    gravar().map_err(|e| format!("Aplicativos do Windows: {e}"))?;
    Ok("Aplicativos do Windows: registrado para desinstalar.".into())
}

pub fn instalar() -> Vec<Passo> {
    let mut passos = vec![copiar()];
    if passos[0].is_err() {
        return passos;
    }
    passos.push(criar_atalho());
    passos.push(registrar());
    match exe_instalado() {
        Ok(exe) => {
            passos.push(claude::configurar_desktop(Some(&exe)));
            passos.push(claude::configurar_code(Some(&exe)));
        }
        Err(e) => passos.push(Err(e)),
    }
    passos
}

pub fn desinstalar() -> Vec<Passo> {
    let mut passos = vec![
        claude::configurar_desktop(None),
        claude::configurar_code(None),
        config::apagar_credencial().map(|_| "Login: apagado do cofre do Windows.".into()),
    ];
    if let Ok(dir) = config::pasta_config() {
        let _ = fs::remove_file(dir.join("config.json"));
        let _ = fs::remove_dir(dir); // só remove se estiver vazia
    }
    if let Ok(f) = atalho() {
        let _ = fs::remove_file(f);
    }
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(registro());
    // Um executável não apaga a si mesmo: o cmd apaga os arquivos depois que ele fecha
    if let Ok(dir) = pasta_programa() {
        if dir.is_dir() {
            let r = Command::new("cmd")
                .raw_arg("/D /C \"ping -n 4 127.0.0.1 >nul & del /Q premosienge*.* & cd .. & rmdir MCP-PremoSienge\"")
                .current_dir(&dir)
                .creation_flags(SEM_JANELA)
                .spawn();
            passos.push(match r {
                Ok(_) => Ok("Programa: será apagado ao fechar.".into()),
                Err(e) => Err(format!("Programa: apague a pasta {} ({e}).", dir.display())),
            });
        }
    }
    passos
}

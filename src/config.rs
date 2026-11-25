//! Configuração local e cofre de credenciais.
//! O subdomínio fica em arquivo; usuário e senha, só no Gerenciador de Credenciais do Windows.

use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

pub const NOME: &str = "MCP-PremoSienge";

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Config {
    pub subdominio: String,
}

// Sem Debug de propósito: a senha não pode aparecer em log.
#[derive(Serialize, Deserialize, Clone)]
pub struct Credencial {
    pub usuario: String,
    pub senha: String,
}

/// Pasta vinda de variável de ambiente do Windows (APPDATA, LOCALAPPDATA...).
pub fn pasta(variavel: &str) -> Result<PathBuf, String> {
    env::var_os(variavel)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("Variável {variavel} não encontrada no Windows."))
}

pub fn pasta_config() -> Result<PathBuf, String> {
    Ok(pasta("APPDATA")?.join(NOME))
}

/// Perfil isolado, usado nos testes para não tocar nas credenciais reais.
pub fn perfil() -> Option<String> {
    env::var("PREMOSIENGE_PERFIL").ok().filter(|p| !p.is_empty())
}

pub fn carregar() -> Config {
    pasta_config()
        .ok()
        .and_then(|p| fs::read_to_string(p.join("config.json")).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn salvar(config: &Config) -> Result<(), String> {
    let dir = pasta_config()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar {}: {e}", dir.display()))?;
    let texto = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(dir.join("config.json"), texto).map_err(|e| format!("Não foi possível salvar: {e}"))
}

/// Aceita "empresa", "empresa.sienge.com.br" ou a URL completa.
pub fn normalizar_subdominio(entrada: &str) -> Result<String, String> {
    let t = entrada.trim().to_lowercase();
    let t = t.trim_start_matches("https://").trim_start_matches("http://");
    let host = t.split(['/', '?', '#', ':']).next().unwrap_or("");
    let sub = host.strip_suffix(".sienge.com.br").unwrap_or(host);
    let valido = !sub.is_empty()
        && sub.len() <= 63
        && sub.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !sub.starts_with('-')
        && !sub.ends_with('-');
    if !valido {
        return Err("Informe só o subdomínio. Em https://minhaempresa.sienge.com.br, é \"minhaempresa\".".into());
    }
    if sub == "api" || sub == "www" {
        return Err("Informe o subdomínio da sua empresa no Sienge.".into());
    }
    Ok(sub.to_string())
}

fn cofre() -> Result<keyring::Entry, String> {
    let servico = match perfil() {
        Some(p) => format!("{NOME}-{p}"),
        None => NOME.to_string(),
    };
    keyring::Entry::new(&servico, "sienge").map_err(|e| format!("Cofre do Windows indisponível: {e}"))
}

pub fn ler_credencial() -> Result<Option<Credencial>, String> {
    match cofre()?.get_password() {
        Ok(t) => serde_json::from_str(&t).map(Some).map_err(|_| "Credencial salva inválida. Entre novamente.".into()),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Não foi possível ler o cofre do Windows: {e}")),
    }
}

pub fn salvar_credencial(c: &Credencial) -> Result<(), String> {
    let texto = serde_json::to_string(c).map_err(|e| e.to_string())?;
    cofre()?.set_password(&texto).map_err(|e| format!("Não foi possível salvar no cofre do Windows: {e}"))
}

pub fn apagar_credencial() -> Result<(), String> {
    match cofre()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Não foi possível apagar a credencial: {e}")),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn subdominio() {
        for t in ["minhaempresa", " MinhaEmpresa ", "minhaempresa.sienge.com.br", "https://minhaempresa.sienge.com.br/sienge/8/index.html"] {
            assert_eq!(normalizar_subdominio(t).unwrap(), "minhaempresa");
        }
        for t in ["", "a..b.c", "evil.com", "a@b", "api", "https://", "emp resa", "../x"] {
            assert!(normalizar_subdominio(t).is_err(), "{t}");
        }
    }
}

//! Configuração automática do MCP no Claude Desktop e no Claude Code.
//! Só mexe na entrada "premosienge"; os demais servidores são preservados.

use crate::config::pasta;
use serde_json::{json, Map, Value};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::{env, fs, process::Command};

const CHAVE: &str = "premosienge";
const SEM_JANELA: u32 = 0x0800_0000; // CREATE_NO_WINDOW

#[derive(Clone, PartialEq)]
pub enum Estado {
    NaoInstalado,
    SemMcp,
    OutroCaminho,
    Configurado,
}

impl Estado {
    pub fn texto(&self) -> &'static str {
        match self {
            Estado::NaoInstalado => "não encontrado neste computador",
            Estado::SemMcp => "MCP não configurado",
            Estado::OutroCaminho => "configurado para outra instalação",
            Estado::Configurado => "configurado",
        }
    }
}

fn arquivos_desktop() -> Vec<PathBuf> {
    arquivos_em(pasta("LOCALAPPDATA").ok().as_deref(), pasta("APPDATA").ok().as_deref())
}

/// Subpastas cujo nome começa com "claude_" (família do pacote MSIX).
fn pastas_claude(dir: &Path) -> Vec<PathBuf> {
    let Ok(itens) = fs::read_dir(dir) else { return vec![] };
    itens
        .flatten()
        .filter(|i| i.file_name().to_string_lossy().to_lowercase().starts_with("claude_"))
        .map(|i| i.path())
        .filter(|p| p.is_dir())
        .collect()
}

/// Arquivos de configuração do Claude Desktop. O primeiro é o que o Claude usa.
fn arquivos_em(local: Option<&Path>, roaming: Option<&Path>) -> Vec<PathBuf> {
    let nome = "claude_desktop_config.json";
    let mut lista = vec![];
    let (mut tem_perfil, mut tem_pacote) = (false, false);
    if let Some(local) = local {
        // Versões MSIX recentes: %LOCALAPPDATA%\Claude-Data\claude_<pacote>
        for p in pastas_claude(&local.join("Claude-Data")) {
            tem_perfil = true;
            lista.push(p.join(nome));
        }
        // MSIX anteriores: um arquivo aqui tem prioridade sobre o de %APPDATA%
        for p in pastas_claude(&local.join("Packages")) {
            tem_pacote = true;
            let f = p.join("LocalCache").join("Roaming").join("Claude").join(nome);
            if f.exists() {
                lista.push(f);
            }
        }
    }
    if let Some(roaming) = roaming {
        let dir = roaming.join("Claude");
        if dir.is_dir() || (tem_pacote && !tem_perfil) {
            lista.push(dir.join(nome));
        }
    }
    lista
}

fn ler_json(arquivo: &Path) -> Result<Value, String> {
    if !arquivo.exists() {
        return Ok(json!({}));
    }
    let t = fs::read_to_string(arquivo).map_err(|e| format!("Não foi possível ler {}: {e}", arquivo.display()))?;
    let t = t.trim_start_matches('\u{feff}').trim();
    if t.is_empty() {
        return Ok(json!({}));
    }
    let v: Value = serde_json::from_str(t)
        .map_err(|e| format!("{} não é um JSON válido ({e}). Nada foi alterado.", arquivo.display()))?;
    if !v.is_object() {
        return Err(format!("{} tem formato inesperado. Nada foi alterado.", arquivo.display()));
    }
    Ok(v)
}

/// Grava com cópia de segurança e troca atômica.
fn gravar_json(arquivo: &Path, v: &Value) -> Result<(), String> {
    let erro = |e: std::io::Error| format!("Não foi possível gravar {}: {e}", arquivo.display());
    if let Some(dir) = arquivo.parent() {
        fs::create_dir_all(dir).map_err(erro)?;
    }
    if arquivo.exists() {
        fs::copy(arquivo, arquivo.with_extension("json.premosienge.bak")).map_err(erro)?;
    }
    let tmp = arquivo.with_extension("json.premosienge.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(v).map_err(|e| e.to_string())? + "\n").map_err(erro)?;
    fs::rename(&tmp, arquivo).map_err(erro)
}

/// Com `exe`, inclui ou atualiza a entrada; sem, remove. Devolve se houve mudança.
fn editar(arquivo: &Path, exe: Option<&Path>) -> Result<bool, String> {
    let mut v = ler_json(arquivo)?;
    let antes = v.clone();
    let raiz = v.as_object_mut().ok_or("formato inesperado")?;
    match exe {
        Some(exe) => {
            let servidores = raiz.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
            let servidores = servidores
                .as_object_mut()
                .ok_or_else(|| format!("\"mcpServers\" tem formato inesperado em {}. Nada foi alterado.", arquivo.display()))?;
            servidores.insert(CHAVE.into(), json!({"command": exe.to_string_lossy(), "args": ["mcp"]}));
        }
        None => {
            if let Some(s) = raiz.get_mut("mcpServers").and_then(Value::as_object_mut) {
                s.shift_remove(CHAVE);
            }
        }
    }
    if v == antes {
        return Ok(false);
    }
    gravar_json(arquivo, &v)?;
    Ok(true)
}

fn estado_de(v: &Value, exe: &Path) -> Estado {
    match v["mcpServers"][CHAVE]["command"].as_str() {
        None => Estado::SemMcp,
        Some(c) if Path::new(c) == exe => Estado::Configurado,
        Some(_) => Estado::OutroCaminho,
    }
}

pub fn estado_desktop(exe: &Path) -> Estado {
    // O primeiro da lista é o que o Claude usa
    match arquivos_desktop().first() {
        None => Estado::NaoInstalado,
        Some(f) => ler_json(f).map(|v| estado_de(&v, exe)).unwrap_or(Estado::SemMcp),
    }
}

pub fn configurar_desktop(exe: Option<&Path>) -> Result<String, String> {
    let arquivos = arquivos_desktop();
    if arquivos.is_empty() {
        return Ok("Claude Desktop: não encontrado neste computador.".into());
    }
    let mut mudou = false;
    for f in &arquivos {
        if exe.is_some() || f.exists() {
            mudou |= editar(f, exe)?;
        }
    }
    Ok(match (exe.is_some(), mudou) {
        (true, true) => "Claude Desktop: configurado. Feche o Claude por completo e abra de novo.",
        (true, false) => "Claude Desktop: já estava configurado.",
        (false, true) => "Claude Desktop: MCP removido. Feche e abra o Claude.",
        (false, false) => "Claude Desktop: nada a remover.",
    }
    .into())
}

fn cli() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = env::var_os("PATH").map(|p| env::split_paths(&p).collect()).unwrap_or_default();
    if let Ok(casa) = pasta("USERPROFILE") {
        dirs.push(casa.join(".local").join("bin"));
    }
    dirs.iter()
        .filter(|d| d.is_absolute())
        .flat_map(|d| ["claude.exe", "claude.cmd"].map(|n| d.join(n)))
        .find(|f| f.is_file())
}

fn rodar(cli: &Path, args: &[&str]) -> Result<(), String> {
    let s = Command::new(cli)
        .args(args)
        .creation_flags(SEM_JANELA)
        .output()
        .map_err(|e| format!("Claude Code: não foi possível executar ({e})."))?;
    if s.status.success() {
        return Ok(());
    }
    let msg = String::from_utf8_lossy(if s.stderr.is_empty() { &s.stdout } else { &s.stderr }).trim().to_string();
    Err(format!("Claude Code: {msg}"))
}

pub fn estado_code(exe: &Path) -> Estado {
    if cli().is_none() {
        return Estado::NaoInstalado;
    }
    pasta("USERPROFILE")
        .and_then(|c| ler_json(&c.join(".claude.json")))
        .map(|v| estado_de(&v, exe))
        .unwrap_or(Estado::SemMcp)
}

/// O Claude Code grava o próprio arquivo o tempo todo, por isso usa o comando oficial.
pub fn configurar_code(exe: Option<&Path>) -> Result<String, String> {
    let Some(cli) = cli() else {
        return Ok("Claude Code: não encontrado neste computador.".into());
    };
    let atual = estado_code(exe.unwrap_or(Path::new("")));
    if let (Some(_), Estado::Configurado) = (exe, &atual) {
        return Ok("Claude Code: já estava configurado.".into());
    }
    if atual != Estado::SemMcp {
        rodar(&cli, &["mcp", "remove", "--scope", "user", CHAVE])?;
    } else if exe.is_none() {
        return Ok("Claude Code: nada a remover.".into());
    }
    match exe {
        Some(exe) => {
            let caminho = exe.to_string_lossy();
            rodar(&cli, &["mcp", "add", "--scope", "user", CHAVE, "--", &caminho, "mcp"])?;
            Ok("Claude Code: configurado. Vale para as próximas sessões.".into())
        }
        None => Ok("Claude Code: MCP removido.".into()),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn arquivo(nome: &str, conteudo: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("premosienge-teste-{}-{nome}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("claude_desktop_config.json");
        fs::write(&f, conteudo).unwrap();
        f
    }

    #[test]
    fn preserva_outros_servidores() {
        let f = arquivo("a", "\u{feff}{\"tema\":\"x\",\"mcpServers\":{\"boxeffect\":{\"command\":\"b\"}}}");
        let exe = Path::new(r"C:\Pasta com espaço\premosienge.exe");
        assert!(editar(&f, Some(exe)).unwrap());
        assert!(!editar(&f, Some(exe)).unwrap()); // repetir não muda nada
        let v = ler_json(&f).unwrap();
        assert_eq!(v["mcpServers"]["boxeffect"]["command"], "b");
        assert_eq!(v["tema"], "x");
        assert!(estado_de(&v, exe) == Estado::Configurado);
        assert!(editar(&f, None).unwrap());
        let v = ler_json(&f).unwrap();
        assert!(v["mcpServers"].get(CHAVE).is_none());
        assert_eq!(v["mcpServers"]["boxeffect"]["command"], "b");
        fs::remove_dir_all(f.parent().unwrap()).unwrap();
    }

    #[test]
    fn acha_a_pasta_que_o_claude_usa() {
        let raiz = env::temp_dir().join(format!("premosienge-teste-{}-pastas", std::process::id()));
        let (local, roaming) = (raiz.join("Local"), raiz.join("Roaming"));
        let perfil = local.join("Claude-Data").join("claude_x");
        let antiga = roaming.join("Claude");
        let achar = || arquivos_em(Some(&local), Some(&roaming));

        fs::create_dir_all(&raiz).unwrap();
        assert!(achar().is_empty()); // sem Claude
        fs::create_dir_all(local.join("Packages").join("Claude_x")).unwrap();
        assert!(achar()[0].starts_with(&antiga)); // MSIX anterior
        fs::create_dir_all(&perfil).unwrap();
        assert_eq!(achar().len(), 1); // MSIX recente, sem criar a pasta antiga
        assert!(achar()[0].starts_with(&perfil));
        fs::create_dir_all(&antiga).unwrap();
        assert!(achar()[0].starts_with(&perfil)); // a recente vem primeiro
        assert!(achar()[1].starts_with(&antiga));
        fs::remove_dir_all(&raiz).unwrap();
    }

    #[test]
    fn nao_toca_em_json_invalido() {
        for (n, ruim) in [("b", "{ quebrado"), ("c", "[]"), ("d", "{\"mcpServers\":[]}")] {
            let f = arquivo(n, ruim);
            assert!(editar(&f, Some(Path::new("x"))).is_err());
            assert_eq!(fs::read_to_string(&f).unwrap(), ruim);
            fs::remove_dir_all(f.parent().unwrap()).unwrap();
        }
    }
}

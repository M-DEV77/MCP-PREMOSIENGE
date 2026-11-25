//! Cliente somente leitura da API pública do Sienge.
//! Só existe GET, e só para caminhos que estão no catálogo.

use crate::config::{self, Credencial};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const API: &str = "https://api.sienge.com.br";
const MAX_CORPO: u64 = 50 * 1024 * 1024;

#[derive(Deserialize)]
pub struct Parametro {
    pub nome: String,
    pub em: String,
    pub tipo: String,
    #[serde(default)]
    pub obrigatorio: bool,
    #[serde(default)]
    pub repetir: bool,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub valores: Vec<Value>,
}

#[derive(Deserialize)]
pub struct Recurso {
    pub caminho: String,
    pub grupo: String,
    pub tipo: String,
    pub resumo: String,
    pub parametros: Vec<Parametro>,
}

pub fn catalogo() -> &'static [Recurso] {
    static CATALOGO: OnceLock<Vec<Recurso>> = OnceLock::new();
    CATALOGO.get_or_init(|| {
        serde_json::from_str(include_str!("../recursos/catalogo.json")).expect("catalogo.json inválido")
    })
}

/// Acha o recurso do catálogo e os valores de caminho.
/// Aceita o modelo (/enterprises/{enterpriseId}) ou o caminho já preenchido (/enterprises/12).
pub fn localizar(caminho: &str) -> Option<(&'static Recurso, BTreeMap<String, String>)> {
    let limpo = caminho.trim().trim_matches('/');
    let completo = if limpo.starts_with("public/api/") {
        limpo.to_string()
    } else if limpo.starts_with("bulk-data/") || limpo.starts_with("v1/") {
        format!("public/api/{limpo}")
    } else {
        format!("public/api/v1/{limpo}")
    };
    let partes: Vec<&str> = completo.split('/').collect();
    let mut melhor: Option<(usize, &Recurso, BTreeMap<String, String>)> = None;
    for r in catalogo() {
        let modelo: Vec<&str> = r.caminho.trim_matches('/').split('/').collect();
        if modelo.len() != partes.len() {
            continue;
        }
        let (mut exatas, mut valores, mut serve) = (0, BTreeMap::new(), true);
        for (m, p) in modelo.iter().zip(&partes) {
            if m == p {
                exatas += 1;
            } else if m.starts_with('{') && !p.is_empty() && !p.starts_with('{') {
                valores.insert(m.trim_matches(['{', '}']).to_string(), p.to_string());
            } else {
                serve = false;
                break;
            }
        }
        // Prefere o que casa mais segmentos literais: /units/situations antes de /units/{unitId}
        if serve && melhor.as_ref().is_none_or(|(n, ..)| exatas > *n) {
            melhor = Some((exatas, r, valores));
        }
    }
    melhor.map(|(_, r, v)| (r, v))
}

fn como_textos(v: &Value) -> Result<Vec<String>, String> {
    Ok(match v {
        Value::String(s) => vec![s.clone()],
        Value::Number(n) => vec![n.to_string()],
        Value::Bool(b) => vec![b.to_string()],
        Value::Array(a) => {
            let mut saida = vec![];
            for i in a {
                if i.is_array() || i.is_object() {
                    return Err("Listas aceitam apenas valores simples.".into());
                }
                saida.extend(como_textos(i)?);
            }
            saida
        }
        Value::Null => vec![],
        Value::Object(_) => return Err("Parâmetros não aceitam objetos.".into()),
    })
}

/// Codifica um segmento de caminho, sem deixar sair da rota do catálogo.
fn segmento(v: &str) -> Result<String, String> {
    if v.is_empty() || v == "." || v == ".." {
        return Err("Valor de caminho inválido.".into());
    }
    Ok(v.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect())
}

pub struct Pedido {
    pub caminho: String,
    pub consulta: Vec<(String, String)>,
    pub bulk: bool,
}

/// Valida a consulta contra o catálogo e monta o pedido.
pub fn preparar(caminho: &str, parametros: &Map<String, Value>) -> Result<Pedido, String> {
    let (r, mut no_caminho) = localizar(caminho).ok_or_else(|| {
        format!("Caminho fora do catálogo: {caminho}. Use sienge_listar_recursos para ver os disponíveis.")
    })?;
    let mut consulta = vec![];
    for (nome, valor) in parametros {
        let p = r.parametros.iter().find(|p| p.nome == *nome).ok_or_else(|| {
            let validos: Vec<&str> = r.parametros.iter().map(|p| p.nome.as_str()).collect();
            format!("Parâmetro desconhecido: {nome}. Válidos: {}", validos.join(", "))
        })?;
        let textos = como_textos(valor)?;
        if textos.is_empty() {
            continue;
        }
        if p.em == "path" {
            no_caminho.insert(nome.clone(), textos.join(","));
        } else if p.repetir {
            consulta.extend(textos.into_iter().map(|t| (nome.clone(), t)));
        } else {
            consulta.push((nome.clone(), textos.join(",")));
        }
    }
    let faltando: Vec<&str> = r
        .parametros
        .iter()
        .filter(|p| p.obrigatorio)
        .filter(|p| if p.em == "path" { !no_caminho.contains_key(&p.nome) } else { !consulta.iter().any(|(n, _)| *n == p.nome) })
        .map(|p| p.nome.as_str())
        .collect();
    if !faltando.is_empty() {
        return Err(format!("Parâmetros obrigatórios ausentes: {}", faltando.join(", ")));
    }
    let mut partes = vec![];
    for m in r.caminho.trim_matches('/').split('/') {
        if m.starts_with('{') {
            let nome = m.trim_matches(['{', '}']);
            let v = no_caminho.get(nome).ok_or_else(|| format!("Parâmetro obrigatório ausente: {nome}"))?;
            partes.push(segmento(v)?);
        } else {
            partes.push(m.to_string());
        }
    }
    Ok(Pedido { caminho: format!("/{}", partes.join("/")), consulta, bulk: r.tipo == "BULK" })
}

/// Endereço da API. O desvio para testes só aceita a própria máquina.
fn base() -> String {
    if let Ok(t) = std::env::var("PREMOSIENGE_API") {
        if let Some(porta) = t.strip_prefix("http://127.0.0.1:") {
            if porta.parse::<u16>().is_ok() {
                return t;
            }
        }
    }
    API.to_string()
}

/// Limites de segurança do Sienge por minuto: 200 REST e 20 BULK.
fn aguardar_limite(bulk: bool) -> Result<(), String> {
    static JANELAS: Mutex<[VecDeque<Instant>; 2]> = Mutex::new([VecDeque::new(), VecDeque::new()]);
    let mut janelas = JANELAS.lock().unwrap_or_else(|e| e.into_inner());
    let fila = &mut janelas[bulk as usize];
    let agora = Instant::now();
    while fila.front().is_some_and(|t| agora.duration_since(*t) > Duration::from_secs(60)) {
        fila.pop_front();
    }
    if fila.len() >= if bulk { 20 } else { 200 } {
        return Err("Limite de consultas por minuto do Sienge atingido. Aguarde um minuto.".into());
    }
    fila.push_back(agora);
    Ok(())
}

pub struct Resposta {
    pub status: u16,
    pub tipo: String,
    pub corpo: String,
}

fn agente() -> &'static ureq::Agent {
    static AGENTE: OnceLock<ureq::Agent> = OnceLock::new();
    AGENTE.get_or_init(|| {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0) // não leva a credencial para outro endereço
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(180)))
            .tls_config(
                // Certificados do Windows, para funcionar também em rede corporativa
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::NativeTls)
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .new_agent()
    })
}

fn enviar(subdominio: &str, c: &Credencial, pedido: &Pedido) -> Result<Resposta, String> {
    aguardar_limite(pedido.bulk)?;
    let url = format!("{}/{}{}", base(), subdominio, pedido.caminho);
    let token = STANDARD.encode(format!("{}:{}", c.usuario, c.senha));
    let mut req = agente().get(&url).header("Authorization", format!("Basic {token}")).header("Accept", "application/json");
    for (n, v) in &pedido.consulta {
        req = req.query(n, v);
    }
    let mut r = req.call().map_err(|e| format!("Sem conexão com o Sienge: {e}"))?;
    let status = r.status().as_u16();
    let tipo = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let corpo = r
        .body_mut()
        .with_config()
        .limit(MAX_CORPO)
        .read_to_vec()
        .map_err(|e| format!("Falha ao ler a resposta do Sienge: {e}"))?;
    Ok(Resposta { status, tipo, corpo: String::from_utf8_lossy(&corpo).into_owned() })
}

/// Sessão atual, lida a cada chamada: sair na tela corta o acesso do Claude na hora.
pub fn sessao() -> Result<(String, Credencial), String> {
    let sub = config::normalizar_subdominio(&config::carregar().subdominio)
        .map_err(|_| "API não configurada. Abra o MCP PremoSienge e preencha a tela API.".to_string())?;
    let c = config::ler_credencial()?
        .ok_or("Nenhum usuário conectado. Abra o MCP PremoSienge e entre na tela Login.")?;
    Ok((sub, c))
}

pub fn explicar(status: u16) -> &'static str {
    match status {
        400 => "Consulta inválida. Confira os parâmetros.",
        401 => "Não autorizado: usuário ou senha de API incorretos, senha expirada, usuário inativo ou recurso não liberado no Sienge.",
        403 => "Sem permissão: este usuário não tem acesso a este recurso no Sienge. O administrador libera em Integrações > APIs > Usuários de APIs.",
        404 => "Registro não encontrado, ou subdomínio incorreto.",
        422 => "Os dados informados não atendem às regras do Sienge.",
        429 => "Limite de consultas atingido (por minuto ou a cota diária do pacote). Aguarde antes de tentar de novo.",
        503 => "Sienge indisponível ou em manutenção (00:00 às 06:30).",
        300..=399 => "O Sienge respondeu com redirecionamento, que não é seguido por segurança.",
        _ => "O Sienge retornou um erro.",
    }
}

pub fn consultar(caminho: &str, parametros: &Map<String, Value>) -> Result<Resposta, String> {
    let pedido = preparar(caminho, parametros)?;
    let (sub, c) = sessao()?;
    enviar(&sub, &c, &pedido)
}

/// Confirma a credencial. Um 403 também confirma: o usuário existe, só não tem aquele recurso.
/// Um 401 isolado pode ser recurso não liberado, por isso tenta mais de um.
pub fn validar_login(subdominio: &str, c: &Credencial) -> Result<String, String> {
    let sub = config::normalizar_subdominio(subdominio)?;
    let mut ultimo = 0;
    for caminho in ["enterprises", "companies", "customers"] {
        let mut p = preparar(caminho, &Map::new())?;
        if localizar(caminho).is_some_and(|(r, _)| r.parametros.iter().any(|x| x.nome == "limit")) {
            p.consulta.push(("limit".into(), "1".into()));
        }
        let r = enviar(&sub, c, &p)?;
        match r.status {
            200 => return Ok("Conectado ao Sienge.".into()),
            400 | 403 | 422 => return Ok("Conectado ao Sienge, com acesso restrito pelas permissões do usuário.".into()),
            401 => ultimo = 401,
            s => return Err(format!("{} (HTTP {s})", explicar(s))),
        }
    }
    Err(explicar(ultimo).to_string())
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn prep(c: &str, p: Value) -> Result<Pedido, String> {
        preparar(c, p.as_object().unwrap())
    }

    #[test]
    fn catalogo_so_tem_caminhos_da_api() {
        assert!(catalogo().len() > 150);
        for r in catalogo() {
            assert!(r.caminho.starts_with("/public/api/"), "{}", r.caminho);
            assert!(r.parametros.iter().all(|p| !p.nome.starts_with("_async")));
            for s in r.caminho.split('/').filter(|s| s.starts_with('{')) {
                let nome = s.trim_matches(['{', '}']);
                assert!(r.parametros.iter().any(|p| p.nome == nome && p.em == "path"), "{} sem {nome}", r.caminho);
            }
        }
    }

    #[test]
    fn caminho_modelo_e_preenchido() {
        assert_eq!(prep("/enterprises/{enterpriseId}", json!({"enterpriseId": 12})).unwrap().caminho, "/public/api/v1/enterprises/12");
        assert_eq!(prep("enterprises/12", json!({})).unwrap().caminho, "/public/api/v1/enterprises/12");
        assert_eq!(prep("/units/situations", json!({})).unwrap().caminho, "/public/api/v1/units/situations");
    }

    #[test]
    fn recusa_o_que_nao_esta_no_catalogo() {
        assert!(prep("/hooks", json!({})).is_err());
        assert!(prep("/../../outro", json!({})).is_err());
        assert!(prep("/enterprises/..", json!({})).is_err());
        assert!(prep("/enterprises", json!({"_asyncWebHookUrl": "http://x"})).is_err());
        assert!(prep("/enterprises/{enterpriseId}", json!({})).is_err());
        assert!(prep("/enterprises", json!({"companyId": {"a": 1}})).is_err());
    }

    #[test]
    fn valor_de_caminho_nao_muda_a_rota() {
        let p = prep("/enterprises/{enterpriseId}", json!({"enterpriseId": "1/../../hooks?x=1"})).unwrap();
        assert_eq!(p.caminho, "/public/api/v1/enterprises/1%2F..%2F..%2Fhooks%3Fx%3D1");
    }

    #[test]
    fn listas_e_bulk() {
        let p = prep("bulk-data/v1/income", json!({"startDate": "2026-01-01", "endDate": "2026-01-31", "selectionType": "D", "costCentersId": [1, 2]})).unwrap();
        assert!(p.bulk);
        assert!(p.consulta.contains(&("costCentersId".into(), "1,2".into())));
    }

    /// Só rede e certificado, sem credencial: cargo test -- --ignored
    #[test]
    #[ignore]
    fn tls_com_o_servidor_real() {
        let r = agente().get("https://api.sienge.com.br/docs/").call().unwrap();
        assert_eq!(r.status().as_u16(), 200);
    }
}

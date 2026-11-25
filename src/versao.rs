//! Versão e indicador, sempre a partir de versao.json (gravado por versionar.py).

use serde_json::Value;
use std::sync::OnceLock;

fn dados() -> &'static Value {
    static DADOS: OnceLock<Value> = OnceLock::new();
    DADOS.get_or_init(|| serde_json::from_str(include_str!("../versao.json")).expect("versao.json inválido"))
}

pub fn indicador() -> &'static str {
    dados()["indicador"].as_str().unwrap_or("ver ?")
}

pub fn numero() -> &'static str {
    dados()["versao"].as_str().unwrap_or("0.00")
}

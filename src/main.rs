//! MCP PremoSienge: um único executável.
//!   premosienge              abre a interface
//!   premosienge mcp          servidor MCP (chamado pelo Claude)
//!   premosienge instalar     instala sem janela
//!   premosienge desinstalar  desinstala sem janela
#![windows_subsystem = "windows"]

mod claude;
mod config;
mod instalador;
mod mcp;
mod sienge;
mod tela;
mod versao;

use std::process::ExitCode;

fn mostrar(passos: Vec<instalador::Passo>) -> bool {
    let mut ok = true;
    for p in passos {
        match p {
            Ok(t) => println!("{t}"),
            Err(t) => {
                ok = false;
                println!("ERRO: {t}");
            }
        }
    }
    ok
}

fn main() -> ExitCode {
    let ok = match std::env::args().nth(1).as_deref() {
        Some("mcp") => mcp::servir().is_ok(),
        Some("instalar") => mostrar(instalador::instalar()),
        Some("desinstalar") => mostrar(instalador::desinstalar()),
        _ => tela::abrir(instalador::modo_instalador()).is_ok(),
    };
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

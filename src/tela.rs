//! Interface: API, Login e Claude. No modo instalador, mostra só a instalação.

use crate::claude::{self, Estado};
use crate::config::{self, Config, Credencial};
use crate::instalador::{self, Passo};
use crate::{sienge, versao};
use eframe::egui::{self, Color32, RichText};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};

// Vermelho da logo
const MARCA: Color32 = Color32::from_rgb(0xCF, 0x30, 0x34);
const VERDE: Color32 = Color32::from_rgb(22, 120, 70);
const VERMELHO: Color32 = Color32::from_rgb(180, 40, 40);

#[derive(PartialEq, Clone, Copy)]
enum Aba {
    Api,
    Login,
    Claude,
}

enum Evento {
    Login(Result<String, String>),
    Passos(Vec<Passo>),
}

struct App {
    instalador: bool,
    aba: Aba,
    subdominio: String,
    salvo: Config,
    usuario: String,
    senha: String,
    conectado: Option<String>,
    aviso: Option<Result<String, String>>,
    passos: Vec<Passo>,
    confirmar_remocao: bool,
    fim: bool,
    estado_desktop: Estado,
    estado_code: Estado,
    tarefa: Option<Receiver<Evento>>,
    logo: Option<egui::TextureHandle>,
}

fn imagem_logo() -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../recursos/logo.png")).ok()
}

/// Botão da ação principal da tela.
fn botao_principal(texto: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(texto).color(Color32::WHITE).strong()).fill(MARCA).min_size(egui::vec2(0.0, 26.0))
}

pub fn abrir(instalador: bool) -> Result<(), String> {
    let titulo = if instalador { "Instalador MCP PremoSienge" } else { "MCP PremoSienge" };
    let mut janela = egui::ViewportBuilder::default().with_inner_size([600.0, 520.0]).with_min_inner_size([520.0, 480.0]);
    if let Some(i) = imagem_logo() {
        janela = janela.with_icon(i);
    }
    let opcoes = eframe::NativeOptions {
        viewport: janela,
        ..Default::default()
    };
    eframe::run_native(
        titulo,
        opcoes,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(egui::Theme::Light);
            cc.egui_ctx.set_zoom_factor(1.15);
            let mut visual = egui::Visuals::light();
            visual.override_text_color = Some(Color32::from_rgb(30, 30, 30));
            visual.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_gray(150));
            visual.selection.bg_fill = MARCA;
            cc.egui_ctx.set_visuals_of(egui::Theme::Light, visual);
            let mut app = App::novo(instalador);
            app.logo = imagem_logo().map(|i| {
                let img = egui::ColorImage::from_rgba_unmultiplied([i.width as usize, i.height as usize], &i.rgba);
                cc.egui_ctx.load_texture("logo", img, egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)))
            });
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())
}

impl App {
    fn novo(instalador: bool) -> Self {
        let salvo = config::carregar();
        let (conectado, aviso) = match config::ler_credencial() {
            Ok(c) => (c.map(|c| c.usuario), None),
            Err(e) => (None, Some(Err(e))),
        };
        // Abre na primeira etapa que falta
        let aba = if salvo.subdominio.is_empty() {
            Aba::Api
        } else if conectado.is_none() {
            Aba::Login
        } else {
            Aba::Claude
        };
        let mut app = App {
            instalador,
            aba,
            subdominio: salvo.subdominio.clone(),
            salvo,
            usuario: String::new(),
            senha: String::new(),
            conectado,
            aviso,
            passos: vec![],
            confirmar_remocao: false,
            fim: false,
            estado_desktop: Estado::SemMcp,
            estado_code: Estado::SemMcp,
            tarefa: None,
            logo: None,
        };
        app.atualizar_estados();
        app
    }

    fn atualizar_estados(&mut self) {
        if let Ok(exe) = instalador::exe_instalado() {
            self.estado_desktop = claude::estado_desktop(&exe);
            self.estado_code = claude::estado_code(&exe);
        }
    }

    /// Roda fora da tela para ela não travar.
    fn iniciar(&mut self, ctx: &egui::Context, f: impl FnOnce() -> Evento + Send + 'static) {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        self.aviso = None;
        self.tarefa = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }

    fn receber(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let Some(rx) = &self.tarefa else { return };
        match rx.try_recv() {
            Ok(Evento::Login(r)) => {
                if r.is_ok() {
                    self.conectado = Some(self.usuario.trim().to_string());
                    self.senha.clear();
                }
                self.aviso = Some(r);
                self.tarefa = None;
            }
            Ok(Evento::Passos(p)) => {
                self.passos = p;
                self.tarefa = None;
                self.conectado = config::ler_credencial().ok().flatten().map(|c| c.usuario);
                self.salvo = config::carregar();
                self.atualizar_estados();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.tarefa = None;
                self.aviso = Some(Err("A operação foi interrompida.".into()));
            }
        }
    }

    fn mostrar_aviso(&self, ui: &mut egui::Ui) {
        if self.tarefa.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Aguarde...");
            });
        }
        match &self.aviso {
            Some(Ok(t)) => ui.colored_label(VERDE, t),
            Some(Err(t)) => ui.colored_label(VERMELHO, t),
            None => return,
        };
    }

    fn mostrar_passos(&self, ui: &mut egui::Ui) {
        for p in &self.passos {
            match p {
                Ok(t) => ui.label(t),
                Err(t) => ui.colored_label(VERMELHO, t),
            };
        }
    }

    fn tela_api(&mut self, ui: &mut egui::Ui) {
        ui.heading("API do Sienge");
        ui.label("Subdomínio da empresa. Em https://minhaempresa.sienge.com.br, é \"minhaempresa\".");
        ui.add_space(8.0);
        let rotulo = ui.label("Subdomínio");
        ui.add(egui::TextEdit::singleline(&mut self.subdominio).desired_width(300.0)).labelled_by(rotulo.id);
        ui.add_space(8.0);
        if ui.add(botao_principal("Salvar")).clicked() {
            let resultado = config::normalizar_subdominio(&self.subdominio).and_then(|s| {
                let novo = Config { subdominio: s.clone() };
                if novo != self.salvo && self.conectado.is_some() {
                    // A credencial pertence à empresa anterior
                    config::apagar_credencial()?;
                    self.conectado = None;
                }
                config::salvar(&novo)?;
                self.salvo = novo;
                self.subdominio = s;
                Ok("API salva. Agora entre na tela Login.".to_string())
            });
            if resultado.is_ok() && self.conectado.is_none() {
                self.aba = Aba::Login;
            }
            self.aviso = Some(resultado);
        }
    }

    fn tela_login(&mut self, ui: &mut egui::Ui) {
        ui.heading("Login no Sienge");
        if self.salvo.subdominio.is_empty() {
            ui.label("Preencha primeiro a tela API.");
            return;
        }
        ui.label(format!("Empresa: {}", self.salvo.subdominio));
        ui.add_space(8.0);
        if let Some(u) = self.conectado.clone() {
            ui.colored_label(VERDE, format!("Conectado como {u}"));
            ui.label("O Claude acessa apenas o que este usuário tem liberado no Sienge.");
            ui.add_space(8.0);
            if ui.button("Sair").clicked() {
                self.aviso = Some(config::apagar_credencial().map(|_| {
                    self.conectado = None;
                    self.usuario.clear();
                    "Sessão encerrada. O Claude não tem mais acesso.".into()
                }));
            }
            return;
        }
        ui.label("Use o seu usuário de API, criado pelo administrador no Sienge em Integrações > APIs > Usuários de APIs. A API não aceita o login comum da plataforma.");
        ui.add_space(8.0);
        let livre = self.tarefa.is_none();
        let mut enter = false;
        ui.add_enabled_ui(livre, |ui| {
            let r = ui.label("Usuário de API");
            ui.add(egui::TextEdit::singleline(&mut self.usuario).desired_width(300.0)).labelled_by(r.id);
            let r = ui.label("Senha");
            let campo = ui.add(egui::TextEdit::singleline(&mut self.senha).password(true).desired_width(300.0)).labelled_by(r.id);
            enter = campo.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        });
        ui.add_space(8.0);
        let pronto = livre && !self.usuario.trim().is_empty() && !self.senha.is_empty();
        let clicou = ui.add_enabled(pronto, botao_principal("Entrar")).clicked();
        if pronto && (clicou || enter) {
            let sub = self.salvo.subdominio.clone();
            let c = Credencial { usuario: self.usuario.trim().to_string(), senha: self.senha.clone() };
            if c.usuario.contains(':') {
                self.aviso = Some(Err("O usuário não pode conter dois-pontos.".into()));
                return;
            }
            let ctx = ui.ctx().clone();
            self.iniciar(&ctx, move || {
                Evento::Login(sienge::validar_login(&sub, &c).and_then(|m| config::salvar_credencial(&c).map(|_| m)))
            });
        }
    }

    fn tela_claude(&mut self, ui: &mut egui::Ui) {
        ui.heading("Claude");
        ui.label(format!("Claude Desktop: {}", self.estado_desktop.texto()));
        ui.label(format!("Claude Code: {}", self.estado_code.texto()));
        ui.add_space(8.0);
        let livre = self.tarefa.is_none();
        ui.add_enabled_ui(livre, |ui| {
            ui.horizontal(|ui| {
                if ui.add(botao_principal("Configurar automaticamente")).clicked() {
                    self.confirmar_remocao = false;
                    let ctx = ui.ctx().clone();
                    self.iniciar(&ctx, || Evento::Passos(instalador::instalar()));
                }
                if ui.button("Remover do Claude").clicked() {
                    let ctx = ui.ctx().clone();
                    self.iniciar(&ctx, || Evento::Passos(vec![claude::configurar_desktop(None), claude::configurar_code(None)]));
                }
            });
        });
        ui.add_space(8.0);
        self.mostrar_passos(ui);
        ui.add_space(8.0);
        ui.label("Depois de configurar, feche e abra o Claude e pergunte, por exemplo: \"liste as obras do Sienge\".");
        ui.add_space(8.0);
        if self.fim {
            return;
        }
        if !self.confirmar_remocao {
            if ui.add_enabled(livre, egui::Button::new("Desinstalar tudo")).clicked() {
                self.confirmar_remocao = true;
            }
        } else {
            ui.colored_label(VERMELHO, "Remove o programa, o login e o MCP do Claude. Confirma?");
            ui.horizontal(|ui| {
                if ui.button("Sim, desinstalar").clicked() {
                    self.fim = true;
                    let ctx = ui.ctx().clone();
                    self.iniciar(&ctx, || Evento::Passos(instalador::desinstalar()));
                }
                if ui.button("Cancelar").clicked() {
                    self.confirmar_remocao = false;
                }
            });
        }
    }

    fn tela_instalador(&mut self, ui: &mut egui::Ui) {
        ui.heading("Instalação");
        ui.label("Instala o programa neste usuário do Windows e configura o MCP no Claude Desktop e no Claude Code. Não precisa de administrador.");
        ui.add_space(12.0);
        if self.passos.is_empty() {
            if ui.add_enabled(self.tarefa.is_none(), botao_principal("Instalar")).clicked() {
                let ctx = ui.ctx().clone();
                self.iniciar(&ctx, || Evento::Passos(instalador::instalar()));
            }
            return;
        }
        self.mostrar_passos(ui);
        ui.add_space(12.0);
        // O primeiro passo é a cópia do programa: sem ela não há o que abrir
        if self.passos[0].is_ok() && ui.add(botao_principal("Abrir e conectar ao Sienge")).clicked() {
            match instalador::exe_instalado().and_then(|e| Command::new(e).spawn().map_err(|e| e.to_string())) {
                Ok(_) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                Err(e) => self.aviso = Some(Err(e)),
            }
        } else if self.passos[0].is_err() && ui.button("Tentar de novo").clicked() {
            self.passos.clear();
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receber();

        // Indicador da aplicação: DATA DETALHES ver 0.00
        egui::Panel::bottom("indicador").show(ui, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(versao::indicador()).small());
            ui.add_space(4.0);
        });

        egui::Panel::top("topo").frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(10.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(l) = &self.logo {
                    ui.add(egui::Image::new((l.id(), egui::vec2(52.0, 52.0))).alt_text("Logo"));
                }
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("MCP PremoSienge").size(20.0).strong());
                    ui.label("Dados do Sienge no Claude");
                });
            });
        });

        if !self.instalador {
            egui::Panel::top("abas").show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for (aba, nome) in [(Aba::Api, "API"), (Aba::Login, "Login"), (Aba::Claude, "Claude")] {
                        let cor = if self.aba == aba { Color32::WHITE } else { Color32::from_gray(40) };
                        if ui.selectable_label(self.aba == aba, RichText::new(nome).size(16.0).color(cor)).clicked() && self.aba != aba {
                            self.aba = aba;
                            self.aviso = None;
                            self.confirmar_remocao = false;
                        }
                    }
                });
                ui.add_space(6.0);
            });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                if self.instalador {
                    self.tela_instalador(ui);
                } else {
                    match self.aba {
                        Aba::Api => self.tela_api(ui),
                        Aba::Login => self.tela_login(ui),
                        Aba::Claude => self.tela_claude(ui),
                    }
                }
                ui.add_space(10.0);
                self.mostrar_aviso(ui);
            });
        });
    }
}

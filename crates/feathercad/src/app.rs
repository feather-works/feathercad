//! A fiação (o antigo `src/ui/app.ts`): as abas de desenho (cada uma com o próprio motor e o próprio Ctrl+Z), as janelas
//! (caixas, camadas, propriedades, configurações, Monitor) e o caminho de cada gesto até o motor. A tela está em `telas.rs`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use feather_ui::janelas::Janelas;
use feather_ui::monitor::{self, Monitor, Nivel};
use feather_ui::tema::*;
use feathercad_io::Formato;
use pelicano::{Event, Image, Key, Modifiers, NamedKey, Point, ResizeDirection, Size, Subscription, Task, Theme, WindowCmd, WindowId};
use serde_json::json;

use crate::area::EventoArea;
use crate::caixas::{Caixa, Depois, MsgCaixa, Saida};
use crate::camadas::{Camadas, MsgCamadas};
use crate::config::Config;
use crate::configuracoes::{self, Configuracoes, MsgConfig};
use crate::doc::{Doc, FATOR_DO_ZOOM};
use crate::faixa::Pedido;
use crate::idioma::{t, texto_com, tv};
use crate::inicio::{Inicio, MsgInicio};
use crate::mcp_vivo::{self, PedidoMcp, Vivo};
use crate::motor::ItemDeMenu;
use crate::propriedades::{MsgProps, Propriedades};
use crate::arquivos;

pub const ID_COMANDO: &str = "linha-de-comando";
/// A IA conta como "trabalhando" por este tempo depois da última ferramenta.
pub const IA_ATIVA_POR: Duration = Duration::from_secs(4);
/// Quanto tempo um aviso fica na tela.
const AVISO_POR: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Janela {
    Monitor,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Janela(WindowCmd),
    Puxar(ResizeDirection),
    Maximizada(bool),
    Tamanho(Size),
    Fechar,
    FecharAuxiliar(WindowId),
    JanelaAux(WindowId, WindowCmd),
    Area(EventoArea),
    Faixa(Pedido),
    Digitando(String),
    Enviar,
    /// Um pedido sem corpo que devolve o estado (`desfazer`, `refazer`, `cancelar`).
    Simples(&'static str),
    Interruptor(&'static str),
    Fundo(&'static str),
    OrtoAngulo(String),
    SnapModo(String, bool),
    AlternarSnapModos,
    MenuItem(String),
    FecharMenu,
    Tecla(Key, Modifiers),
    Mods(Modifiers),
    PedirNovo,
    Abrir(Option<PathBuf>),
    Salvar { como: bool, formato: Option<Formato> },
    Destino(Option<(PathBuf, Formato)>),
    PdfDestino(Option<PathBuf>),
    Aba(usize),
    FecharAba(usize),
    Inicio,
    Caixa(MsgCaixa),
    Camadas(MsgCamadas),
    Props(MsgProps),
    Config(MsgConfig),
    Pagina(MsgInicio),
    Mcp(PedidoMcp),
    Monitor(monitor::Evento),
    Tique,
    /// O arrasto da vista tem quadro atrasado (ver `Doc::em_dia`).
    QuadroDoArrasto,
    Imagem(String, Option<(u32, u32, Vec<u8>)>),
    Tema,
    Vista(f64, f64),
    Zoom(f32),
    Ajustar,
    Espaco(String),
    NovoLayout,
    Fita(MsgProps),
    CamadaAtual(String),
    CamadaChave(&'static str),
}

impl From<EventoArea> for Msg {
    fn from(e: EventoArea) -> Msg {
        Msg::Area(e)
    }
}

pub struct App {
    pub config: Config,
    pub docs: Vec<Doc>,
    /// A aba de desenho ativa; `None` = a página inicial.
    pub ativa: Option<usize>,
    /// O desenho sem aba (página inicial): é nele que a IA começa quando não há nenhum aberto.
    pub vazio: Doc,
    contador: usize,
    pub inicio: Inicio,
    pub caixa: Option<Caixa>,
    pub camadas: Option<Camadas>,
    pub props: Option<Propriedades>,
    pub configuracoes: Option<Configuracoes>,
    pub menu: Option<(Point, Vec<ItemDeMenu>)>,
    pub snap_modos_aberto: bool,
    pub comando: String,
    pub aba_faixa: usize,
    pub maximizada: bool,
    pub tamanho_janela: Size,
    mods: Modifiers,
    vivo: Vivo,
    pub ia_em: Option<Instant>,
    pub avisos: Vec<(String, bool, Instant)>,
    pub monitor: Monitor,
    pub janelas: Janelas<Janela>,
    pub imagens: HashMap<String, Option<Image>>,
    /// O que continua depois de salvar (fechar aba ou sair, vindo do "Salvar alterações?").
    pendente: Option<Depois>,
    /// A aba (`Doc::id`) cuja caixa "Salvar como" está aberta: o arquivo escolhido é dela, não da aba ativa.
    salvando: Option<u64>,
    pdf: Option<Vec<u8>>,
    pub claro: bool,
    pub angulos: Vec<String>,
    /// O resumo das propriedades para as listas da faixa (cor, espessura, tipo), sempre em dia.
    pub fita: Propriedades,
    /// As camadas para a lista "camada atual" da faixa.
    pub camadas_fita: Vec<crate::camadas::Linha>,
    pub nomes_camadas: Vec<String>,
    versao_fita: Option<(usize, u64)>,
}

impl App {
    pub fn novo(arquivos: Vec<String>) -> (App, Task<Msg>) {
        let config = Config::carregar();
        crate::idioma::escolher(config.idioma.as_deref());
        let claro = config.claro();
        definir_tema(!claro, config.acento);
        let vazio = Doc::novo(crate::idioma::para_o_motor(&config.apelidos), &config, t("app.semTitulo"), claro);
        let mut monitor = Monitor::novo("FeatherCAD");
        monitor.registrar(Nivel::Info, "FeatherCAD", &format!("FeatherCAD {} iniciado", env!("CARGO_PKG_VERSION")));
        let app = App {
            angulos: configuracoes::ANGULOS_DO_ORTO.iter().map(|g| format!("{g}°")).collect(),
            config,
            docs: Vec::new(),
            ativa: None,
            vazio,
            contador: 1,
            inicio: Inicio::novo(),
            caixa: None,
            camadas: None,
            props: None,
            configuracoes: None,
            menu: None,
            snap_modos_aberto: false,
            comando: String::new(),
            aba_faixa: 0,
            maximizada: false,
            tamanho_janela: Size::new(1360.0, 860.0),
            mods: Modifiers::empty(),
            vivo: Vivo::default(),
            ia_em: None,
            avisos: Vec::new(),
            monitor,
            janelas: {
                let mut j = Janelas::default();
                j.principal = Some(WindowId::MAIN);
                j
            },
            imagens: HashMap::new(),
            pendente: None,
            salvando: None,
            pdf: None,
            claro,
            fita: Propriedades::vazia(),
            camadas_fita: Vec::new(),
            nomes_camadas: Vec::new(),
            versao_fita: None,
        };
        let abrir = arquivos.into_iter().map(|a| Task::done(Msg::Abrir(Some(a.into()))));
        let mut app = app;
        let miniaturas = app.pedir_miniaturas();
        (app, Task::batch(abrir.chain([miniaturas])))
    }

    // ---------------------------------------------------------------- abas

    pub fn doc(&self) -> Option<&Doc> {
        self.ativa.and_then(|i| self.docs.get(i))
    }

    pub fn doc_mut(&mut self) -> Option<&mut Doc> {
        self.ativa.and_then(|i| self.docs.get_mut(i))
    }

    fn doc_novo(&mut self) -> Doc {
        let nome = format!("{} {}", t("app.semTitulo"), self.contador);
        self.contador += 1;
        Doc::novo(crate::idioma::para_o_motor(&self.config.apelidos), &self.config, nome, self.claro)
    }

    fn ativar(&mut self, i: Option<usize>) {
        self.ativa = i.filter(|i| *i < self.docs.len());
        self.menu = None;
        self.camadas = None;
        if self.ativa.is_none() {
            self.props = None;
            self.inicio.recarregar();
        }
        if let (Some(p), Some(d)) = (self.props.as_mut(), self.ativa.and_then(|i| self.docs.get_mut(i))) {
            p.recarregar(d);
        }
    }

    fn acrescentar(&mut self, d: Doc) {
        self.docs.push(d);
        self.ativar(Some(self.docs.len() - 1));
    }

    fn remover_aba(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        self.docs.remove(i);
        let proxima = match self.ativa {
            Some(a) if a == i => (!self.docs.is_empty()).then(|| i.min(self.docs.len() - 1)),
            Some(a) if a > i => Some(a - 1),
            outra => outra,
        };
        self.ativar(proxima);
    }

    /// Fecha a aba; se o desenho tem alterações, pergunta antes.
    fn pedir_fechar_aba(&mut self, i: usize) {
        match self.docs.get(i) {
            Some(d) if d.estado.sujo => {
                self.ativar(Some(i));
                self.caixa = Some(Caixa::Confirmar { nome: self.docs[i].nome(), depois: Depois::FecharAba(self.docs[i].id) });
            }
            Some(_) => self.remover_aba(i),
            None => {}
        }
    }

    /// Sair: pergunta por cada desenho com alterações, um de cada vez; sem nenhum, fecha.
    fn continuar_saida(&mut self) -> Task<Msg> {
        match self.docs.iter().position(|d| d.estado.sujo) {
            Some(i) => {
                self.ativar(Some(i));
                self.caixa = Some(Caixa::Confirmar { nome: self.docs[i].nome(), depois: Depois::Sair(self.docs[i].id) });
                Task::none()
            }
            None => {
                let _ = self.config.gravar();
                mcp_vivo::encerrar();
                Task::window(WindowCmd::Close)
            }
        }
    }

    fn indice_do(&self, id: u64) -> Option<usize> {
        self.docs.iter().position(|d| d.id == id)
    }

    fn confirmado(&mut self, depois: Depois, salvar: bool) -> Task<Msg> {
        self.caixa = None;
        let (Depois::FecharAba(id) | Depois::Sair(id)) = depois;
        let Some(i) = self.indice_do(id) else { return Task::none() };
        if salvar {
            self.pendente = Some(depois);
            return self.salvar_aba(i, false, None);
        }
        // Descartar fecha a aba na hora: só marcar como salva deixava a aba aberta (se a saída fosse cancelada
        // na aba seguinte) pronta para fechar depois sem perguntar.
        self.remover_aba(i);
        match depois {
            Depois::FecharAba(_) => Task::none(),
            Depois::Sair(_) => self.continuar_saida(),
        }
    }

    fn depois_de_salvar(&mut self) -> Task<Msg> {
        match self.pendente.take() {
            Some(Depois::FecharAba(id)) => {
                if let Some(i) = self.indice_do(id) {
                    self.remover_aba(i);
                }
                Task::none()
            }
            Some(Depois::Sair(_)) => self.continuar_saida(),
            None => Task::none(),
        }
    }

    // ---------------------------------------------------------------- avisos e registro

    pub fn avisar(&mut self, texto: String, erro: bool) {
        self.monitor.registrar(if erro { Nivel::Erro } else { Nivel::Info }, "FeatherCAD", &texto);
        self.avisos.push((texto, erro, Instant::now()));
        if self.avisos.len() > 4 {
            self.avisos.remove(0);
        }
    }

    /// Erros que o motor deixou no desenho ativo viram aviso.
    fn recolher_erros(&mut self) {
        let erro = self.doc_mut().and_then(|d| d.erro.take()).or_else(|| self.vazio.erro.take());
        if let Some(e) = erro {
            self.avisar(e, true);
        }
    }

    // ---------------------------------------------------------------- arquivos

    fn abrir(&mut self, caminho: PathBuf) -> Task<Msg> {
        // Já aberto: só vai para a aba dele.
        let alvo = caminho.canonicalize().unwrap_or_else(|_| caminho.clone());
        if let Some(i) = self.docs.iter().position(|d| d.estado.arquivo.as_ref().is_some_and(|a| PathBuf::from(&a.caminho).canonicalize().unwrap_or_default() == alvo)) {
            self.ativar(Some(i));
            return Task::none();
        }
        let mut d = self.doc_novo();
        match arquivos::abrir(&mut d.motor, &caminho) {
            Ok(e) => {
                d.visto = std::fs::metadata(&caminho).and_then(|m| m.modified()).ok();
                d.dxf_perderia = arquivos::formato_do_caminho(&caminho) == Some(Formato::Dxf) && (!e.avisos_do_arquivo.is_empty() || arquivos::dxf_mais_novo_que_r12(&caminho));
                let avisos: Vec<String> = e.avisos_do_arquivo.iter().map(|m| texto_com(&m.chave, &m.vars)).collect();
                d.aplicar(e);
                d.ajustar();
                // Uma aba "Sem título" intocada é trocada pelo arquivo aberto (como no TypeScript).
                let trocar = self.docs.len() == 1 && self.docs[0].estado.arquivo.is_none() && !self.docs[0].estado.sujo && !self.docs[0].estado.pode_desfazer;
                if trocar {
                    self.docs.clear();
                }
                self.acrescentar(d);
                if !avisos.is_empty() {
                    self.avisar(avisos.join(" "), false);
                }
                if self.ativa.is_some_and(|_| self.doc().is_some_and(|d| d.estado.unidade.is_none())) {
                    self.caixa = Some(Caixa::Unidade { trocar: true });
                }
            }
            Err(e) => self.avisar(tv("erro.abrir", &[("motivo", &e)]), true),
        }
        self.inicio.recarregar();
        self.carregar_imagens()
    }

    fn salvar(&mut self, como: bool, formato: Option<Formato>) -> Task<Msg> {
        match self.ativa {
            Some(i) => self.salvar_aba(i, como, formato),
            None => {
                self.pendente = None;
                Task::none()
            }
        }
    }

    fn salvar_aba(&mut self, i: usize, como: bool, formato: Option<Formato>) -> Task<Msg> {
        let padrao_config = Formato::da_extensao(&self.config.formato).unwrap_or(Formato::Dxf);
        let Some(d) = self.docs.get(i) else {
            self.pendente = None;
            return Task::none();
        };
        let atual = d.estado.arquivo.clone();
        let padrao = formato.or_else(|| atual.as_ref().and_then(|a| Formato::da_extensao(&a.formato))).unwrap_or(padrao_config);
        // Ctrl+S sobre um DXF que não é R12 (ou que perdeu algo ao abrir) vira "Salvar como": gravar R12 por cima
        // jogaria fora para sempre o que o app não lê.
        if let (false, Some(a), false) = (como, &atual, d.dxf_perderia) {
            let caminho = PathBuf::from(&a.caminho);
            let f = arquivos::formato_do_caminho(&caminho).unwrap_or(padrao);
            return self.gravar_em(i, caminho, f);
        }
        self.salvando = Some(d.id);
        let base = atual.as_ref().map_or_else(|| d.nome(), |a| std::path::Path::new(&a.nome).file_stem().map_or_else(|| "desenho".into(), |s| s.to_string_lossy().into_owned()));
        let nome = format!("{base}.{}", padrao.extensao());
        let pasta = atual.map(|a| PathBuf::from(a.caminho)).or_else(|| arquivos::pasta_padrao().map(|p| p.join("x")));
        Task::perform(arquivos::escolher_para_salvar(pasta, nome, padrao), Msg::Destino)
    }

    fn gravar_em(&mut self, i: usize, caminho: PathBuf, formato: Formato) -> Task<Msg> {
        let Some(d) = self.docs.get_mut(i) else { return Task::none() };
        match arquivos::gravar(&mut d.motor, &caminho, formato) {
            Ok(e) => {
                d.visto = std::fs::metadata(&caminho).and_then(|m| m.modified()).ok();
                d.avisou_conflito = false;
                d.dxf_perderia = false;
                let avisos: Vec<String> = e.avisos_do_arquivo.iter().map(|m| texto_com(&m.chave, &m.vars)).collect();
                d.aplicar(e);
                if !avisos.is_empty() {
                    self.avisar(avisos.join(" "), false);
                }
                self.inicio.recarregar();
                self.depois_de_salvar()
            }
            Err(e) => {
                self.pendente = None;
                self.avisar(tv("erro.salvar", &[("motivo", &e)]), true);
                Task::none()
            }
        }
    }

    /// O arquivo do desenho ativo mudou no disco (outro programa, o MCP fora do app)? Sem alterações aqui, recarrega;
    /// com alterações, nada se perde: avisa uma vez.
    fn vigiar_disco(&mut self) {
        let Some(d) = self.doc_mut() else { return };
        if d.estado.ferramenta != "nenhuma" || d.estado.arrastando {
            return;
        }
        let Some(a) = d.estado.arquivo.clone() else { return };
        let caminho = PathBuf::from(&a.caminho);
        let Some(agora) = std::fs::metadata(&caminho).and_then(|m| m.modified()).ok() else { return };
        if d.visto.is_none() || d.visto == Some(agora) {
            d.visto = d.visto.or(Some(agora));
            return;
        }
        if d.estado.sujo {
            if !d.avisou_conflito {
                d.avisou_conflito = true;
                self.avisar(tv("erro.mudouNoDisco", &[("nome", &a.nome)]), true);
            }
            return;
        }
        d.visto = Some(agora);
        d.avisou_conflito = false;
        let r = std::fs::read(&caminho).map_err(|e| e.to_string()).and_then(|b| d.motor.abrir(&b)).and_then(|_| {
            d.motor.estado_de("documento_arquivo_definir", json!({ "caminho": a.caminho, "formato": a.formato, "salvo": false }))
        });
        match r {
            Ok(_) => d.invalidar(),
            Err(e) => self.avisar(e, true),
        }
    }

    // ---------------------------------------------------------------- imagens e miniaturas

    /// Pede em segundo plano as imagens do quadro que ainda não foram lidas.
    fn carregar_imagens(&mut self) -> Task<Msg> {
        let Some(d) = self.doc() else { return Task::none() };
        let faltam: Vec<String> = d.cena.imagens.iter().map(|i| i.caminho.clone()).filter(|c| !self.imagens.contains_key(c)).collect();
        for c in &faltam {
            self.imagens.insert(c.clone(), None);
        }
        Task::batch(faltam.into_iter().map(|c| {
            Task::perform(
                async move {
                    let img = std::fs::read(&c).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                        let i = i.to_rgba8();
                        (i.width(), i.height(), i.into_raw())
                    });
                    (c, img)
                },
                |(c, img)| Msg::Imagem(c, img),
            )
        }))
    }

    fn pedir_miniaturas(&mut self) -> Task<Msg> {
        if self.config.visao_inicio != "grade" {
            return Task::none();
        }
        Task::batch(self.inicio.faltando().into_iter().map(|c| Task::perform(async move { (c.clone(), arquivos::miniatura(std::path::Path::new(&c))) }, |(c, s)| Msg::Pagina(MsgInicio::Miniatura(c, s)))))
    }

    // ---------------------------------------------------------------- MCP

    fn mcp(&mut self, p: PedidoMcp) -> Task<Msg> {
        let claro = self.claro;
        let sem_aba = self.ativa.is_none();
        let d = match self.ativa {
            Some(i) => &mut self.docs[i],
            None => &mut self.vazio,
        };
        let r = self.vivo.atender(&mut d.motor, &p, claro);
        if let Some(novo) = &r.arquivo {
            match novo.as_deref().and_then(|c| arquivos::formato_do_caminho(c).map(|f| (c, f))) {
                Some((c, f)) => {
                    let _ = d.motor.estado_de("documento_arquivo_definir", json!({ "caminho": c.to_string_lossy(), "formato": f.extensao(), "salvo": true }));
                    d.visto = std::fs::metadata(c).and_then(|m| m.modified()).ok();
                    arquivos::lembrar(c);
                }
                None => d.motor.ponte.esquecer_arquivo(),
            }
        }
        let mexeu = r.mudou_o_desenho || r.arquivo.is_some();
        if mexeu {
            self.ia_em = Some(Instant::now());
            d.invalidar();
            // A IA mexeu sem aba aberta: o desenho dela ganha uma aba (sem recarregar: o Ctrl+Z dela fica).
            if sem_aba {
                let novo = self.doc_novo();
                let mut d = std::mem::replace(&mut self.vazio, novo);
                d.ajustar();
                self.acrescentar(d);
            }
        }
        let abrir = r.abrir.into_iter().map(|a| Task::done(Msg::Abrir(Some(a.into()))));
        Task::batch(abrir.chain([if mexeu { self.carregar_imagens() } else { Task::none() }]))
    }

    // ---------------------------------------------------------------- teclado e menu

    /// O texto de atalho que o motor entende (`Ctrl+G`, `F8`, `Esc`), ou `None` para tecla solta comum.
    fn tecla_do_evento(key: &Key, m: Modifiers) -> Option<String> {
        let nome = match key {
            Key::Named(NamedKey::Escape) => "Esc".to_string(),
            Key::Named(NamedKey::Delete) => "Delete".to_string(),
            Key::Named(n) => {
                let s = format!("{n:?}");
                let tecla_f = s.starts_with('F') && s[1..].parse::<u8>().is_ok();
                if !tecla_f && !m.control_key() && !m.alt_key() {
                    return None;
                }
                s
            }
            Key::Character(c) if m.control_key() || m.alt_key() || m.super_key() => c.to_uppercase(),
            _ => return None,
        };
        let partes = [(m.control_key() || m.super_key()).then_some("Ctrl"), m.alt_key().then_some("Alt"), m.shift_key().then_some("Shift")];
        Some(partes.into_iter().flatten().chain([nome.as_str()]).collect::<Vec<_>>().join("+"))
    }

    /// As teclas da janela (não do motor): abas, painéis, Configurações, Monitor. `None` = não é delas.
    fn tecla_da_janela(&mut self, key: &Key, m: Modifiers) -> Option<Task<Msg>> {
        let c = |s: &str| matches!(key, Key::Character(x) if x.eq_ignore_ascii_case(s));
        let ctrl = m.control_key() || m.super_key();
        Some(match key {
            Key::Named(NamedKey::Escape) if self.caixa.is_some() || self.configuracoes.is_some() || self.camadas.is_some() || self.menu.is_some() || self.snap_modos_aberto => {
                self.caixa = None;
                self.configuracoes = None;
                self.camadas = None;
                self.menu = None;
                self.snap_modos_aberto = false;
                self.pendente = None;
                Task::none()
            }
            Key::Named(NamedKey::F12) => self.janelas.alternar(Janela::Monitor, Size::new(980.0, 640.0), true),
            Key::Named(NamedKey::Tab) if ctrl && !self.docs.is_empty() => {
                let n = self.docs.len();
                let i = self.ativa.map_or(0, |a| if m.shift_key() { (a + n - 1) % n } else { (a + 1) % n });
                self.ativar(Some(i));
                Task::none()
            }
            _ if ctrl && c(",") => Task::done(Msg::Config(MsgConfig::Secao("aparencia"))),
            _ if ctrl && c("1") && self.ativa.is_some() => Task::done(Msg::Faixa(Pedido::Acao("propriedades"))),
            _ if ctrl && (c("w") || matches!(key, Key::Named(NamedKey::F4))) => {
                if let Some(i) = self.ativa {
                    self.pedir_fechar_aba(i);
                }
                Task::none()
            }
            _ if ctrl && m.shift_key() && c("f") => {
                self.config.fps = !self.config.fps;
                let _ = self.config.gravar();
                Task::none()
            }
            _ if ctrl && c("n") && self.ativa.is_none() => Task::done(Msg::PedirNovo),
            _ if ctrl && c("o") && self.ativa.is_none() => Task::done(Msg::Abrir(None)),
            _ => return None,
        })
    }

    fn tecla(&mut self, key: Key, m: Modifiers) -> Task<Msg> {
        if let Some(t) = self.tecla_da_janela(&key, m) {
            return t;
        }
        // Com uma caixa aberta, as teclas são dela (1/2/3 na escolha da unidade).
        if let Some(Caixa::Unidade { .. }) = self.caixa {
            if let Key::Character(c) = &key {
                if let Some(u) = c.parse::<usize>().ok().and_then(|n| crate::caixas::UNIDADES.get(n.wrapping_sub(1))) {
                    return Task::done(Msg::Caixa(MsgCaixa::Unidade(u)));
                }
            }
        }
        if self.caixa.is_some() || self.configuracoes.is_some() || self.camadas.is_some() || self.ativa.is_none() {
            return Task::none();
        }
        let Some(d) = self.doc_mut() else { return Task::none() };
        if let Some(atalho) = Self::tecla_do_evento(&key, m) {
            let r: Result<crate::motor::RespostaAtalho, String> = d.motor.json("atalho", json!({ "tecla": atalho }));
            return match r {
                Ok(r) if r.tratado => {
                    d.aplicar(r.estado);
                    match r.janela.as_deref() {
                        Some("novo") => Task::done(Msg::PedirNovo),
                        Some("abrir") => Task::done(Msg::Abrir(None)),
                        Some("salvar") => Task::done(Msg::Salvar { como: false, formato: None }),
                        Some("salvar_como") => Task::done(Msg::Salvar { como: true, formato: None }),
                        _ => Task::none(),
                    }
                }
                Ok(_) => Task::none(),
                Err(e) => {
                    d.erro = Some(e);
                    Task::none()
                }
            };
        }
        match key {
            // Enter (e espaço, como no AutoCAD) com a caixa vazia repete/confirma.
            Key::Named(NamedKey::Enter | NamedKey::Space) => {
                d.pedir("digitar", json!({ "texto": "" }));
                Task::none()
            }
            // Tecla solta cai na linha de comando, como no AutoCAD.
            Key::Character(c) => {
                self.comando.push_str(&c);
                Task::focus(ID_COMANDO)
            }
            _ => Task::none(),
        }
    }

    fn item_do_menu(&mut self, id: &str) -> Task<Msg> {
        self.menu = None;
        let fechar = crate::idioma::para_o_motor(&self.config.apelidos)["opcoes"]["fechar"][0].as_str().unwrap_or("").to_string();
        let Some(d) = self.doc_mut() else { return Task::none() };
        match id.split_once(':') {
            _ if id == "enter" => d.pedir("digitar", json!({ "texto": "" })),
            _ if id == "cancelar" => d.pedir("cancelar", serde_json::Value::Null),
            _ if id == "fechar" => d.pedir("digitar", json!({ "texto": fechar })),
            Some(("interruptor", nome)) => {
                let ligado = interruptor_ligado(d, nome);
                d.pedir("interruptor", json!({ "nome": nome, "ligado": !ligado }));
            }
            Some(("janela", "zoom_ajustar")) => d.ajustar(),
            Some(("janela", "propriedades")) => return Task::done(Msg::Faixa(Pedido::Acao("propriedades"))),
            Some(("janela", _)) => return Task::done(Msg::Config(MsgConfig::Secao("aparencia"))),
            _ => d.pedir("executar", json!({ "comando": id })),
        }
        Task::none()
    }

    fn faixa(&mut self, p: Pedido) -> Task<Msg> {
        match p {
            Pedido::Aba(i) => self.aba_faixa = i,
            Pedido::Comando(c) => {
                if let Some(d) = self.doc_mut() {
                    d.pedir("executar", json!({ "comando": c }));
                }
            }
            Pedido::Acao(a) => return self.acao(a),
        }
        Task::none()
    }

    /// As ações da interface (botões da faixa sem comando do motor).
    fn acao(&mut self, a: &str) -> Task<Msg> {
        let abrir_caixa = |app: &mut App, f: fn(&mut Doc) -> Result<Caixa, String>| {
            if let Some(d) = app.doc_mut() {
                match f(d) {
                    Ok(c) => app.caixa = Some(c),
                    Err(e) => app.avisar(tv("erro.folha", &[("motivo", &e)]), true),
                }
            }
        };
        match a {
            "camadas" => {
                self.camadas = match (self.camadas.take(), self.doc_mut()) {
                    (None, Some(d)) => Some(Camadas::abrir(d)),
                    _ => None,
                }
            }
            "propriedades" => {
                self.props = match (self.props.take(), self.doc_mut()) {
                    (None, Some(d)) => Some(Propriedades::abrir(d)),
                    _ => None,
                }
            }
            "hachura_padrao" => abrir_caixa(self, Caixa::hachura),
            "folha_inserir" => abrir_caixa(self, Caixa::folha_nova),
            "folha_escala" => abrir_caixa(self, Caixa::folha_escala),
            "folha_carimbo" => abrir_caixa(self, Caixa::carimbo),
            "exportar_pdf" => abrir_caixa(self, Caixa::exportar),
            "unidades" if self.ativa.is_some() => self.caixa = Some(Caixa::Unidade { trocar: true }),
            "vista_ajustar" => return Task::done(Msg::Ajustar),
            "zoom_mais" => return Task::done(Msg::Zoom(FATOR_DO_ZOOM)),
            "zoom_menos" => return Task::done(Msg::Zoom(1.0 / FATOR_DO_ZOOM)),
            "alternar_tema" => return Task::done(Msg::Tema),
            "salvar_dxf" => return self.salvar(true, Some(Formato::Dxf)),
            "salvar_fcdz" => return self.salvar(true, Some(Formato::Fcdz)),
            "monitor" => return self.janelas.alternar(Janela::Monitor, Size::new(980.0, 640.0), true),
            "configuracoes" => return Task::done(Msg::Config(MsgConfig::Secao("aparencia"))),
            "mcp" => return Task::done(Msg::Config(MsgConfig::Secao("mcp"))),
            "sobre" => return Task::done(Msg::Config(MsgConfig::Secao("sobre"))),
            "apelidos" => return Task::done(Msg::Config(MsgConfig::Secao("apelidos"))),
            _ => {}
        }
        Task::none()
    }

    // ---------------------------------------------------------------- preferências

    /// O `Config` mudou: grava e espalha pelo tema e por todos os desenhos abertos.
    fn config_mudou(&mut self, antes: &Config) {
        let _ = self.config.gravar();
        let c = self.config.clone();
        if c.tema != antes.tema || c.acento != antes.acento {
            self.claro = c.claro();
            definir_tema(!self.claro, c.acento);
        }
        let claro = self.claro;
        for d in self.docs.iter_mut().chain(std::iter::once(&mut self.vazio)) {
            if d.claro != claro {
                d.claro = claro;
                d.invalidar();
            }
            if c.fundo != antes.fundo {
                d.pedir("fundo_definir", json!({ "modo": c.fundo }));
            }
            if c.cota != antes.cota {
                d.pedir("cota_tamanho_definir", json!({ "tamanho": c.cota }));
            }
            if c.orto_angulo != antes.orto_angulo {
                d.pedir("orto_angulo_definir", json!({ "graus": c.orto_angulo }));
            }
            if c.apelidos != antes.apelidos {
                d.pedir("idioma_definir", crate::idioma::para_o_motor(&c.apelidos));
            }
        }
        if !self.angulos.contains(&format!("{}°", c.orto_angulo)) {
            self.angulos.push(format!("{}°", c.orto_angulo));
        }
    }

    fn configuracoes(&mut self, m: MsgConfig) -> Task<Msg> {
        let cfg = self.configuracoes.get_or_insert_with(|| Configuracoes::abrir("aparencia", &self.config));
        if let MsgConfig::Secao(s) = &m {
            cfg.secao = s;
        }
        let antes = self.config.clone();
        match cfg.update(m, &mut self.config) {
            configuracoes::Pedido::Fechar => self.configuracoes = None,
            configuracoes::Pedido::Mudou => self.config_mudou(&antes),
            configuracoes::Pedido::Interruptor(n, v) => {
                if let Some(d) = self.doc_mut() {
                    d.pedir("interruptor", json!({ "nome": n, "ligado": v }));
                }
            }
            configuracoes::Pedido::Salvar(como) => return self.salvar(como, None),
            configuracoes::Pedido::Copiar(s) => {
                self.avisar(t("config.mcp.copiado"), false);
                return Task::write_clipboard(s);
            }
            configuracoes::Pedido::Testar => return Task::perform(async { configuracoes::testar_mcp() }, |r| Msg::Config(MsgConfig::Testado(r))),
            configuracoes::Pedido::Monitor => return self.janelas.alternar(Janela::Monitor, Size::new(980.0, 640.0), true),
            configuracoes::Pedido::Nada => {}
        }
        Task::none()
    }

    fn caixa(&mut self, m: MsgCaixa) -> Task<Msg> {
        let Some(c) = self.caixa.as_mut() else { return Task::none() };
        let doc = match self.ativa {
            Some(i) => self.docs.get_mut(i),
            None => None,
        };
        match c.update(m, doc) {
            Saida::Nada => {}
            Saida::Fechar => {
                self.caixa = None;
                self.pendente = None;
            }
            Saida::Erro(e) => self.avisar(e, true),
            Saida::NovoDesenho(u) => {
                self.caixa = None;
                self.config.unidade = u.into();
                let _ = self.config.gravar();
                let mut d = self.doc_novo();
                d.pedir("documento_novo", json!({ "unidade": u }));
                let v = d.motor.vista("vista_inicial", json!({ "largura": d.vista.largura, "altura": d.vista.altura }));
                d.vista_mudou(v);
                self.acrescentar(d);
            }
            Saida::Confirmado(depois, salvar) => return self.confirmado(depois, salvar),
            Saida::ExportarPdf(bytes) => {
                self.caixa = None;
                let nome = self.doc().map_or_else(|| "desenho".into(), |d| std::path::Path::new(&d.nome()).file_stem().map_or_else(|| "desenho".into(), |s| s.to_string_lossy().into_owned()));
                self.pdf = Some(bytes);
                return Task::perform(
                    async move { rfd::AsyncFileDialog::new().set_file_name(format!("{nome}.pdf")).add_filter("PDF", &["pdf"]).save_file().await.map(|f| f.path().to_path_buf()) },
                    Msg::PdfDestino,
                );
            }
        }
        Task::none()
    }

    fn inicio_msg(&mut self, m: MsgInicio) -> Task<Msg> {
        match m {
            MsgInicio::Navegar(onde) => {
                self.inicio.onde = onde;
                self.inicio.recarregar();
                return self.pedir_miniaturas();
            }
            MsgInicio::Visao(v) => {
                self.config.visao_inicio = v.into();
                let _ = self.config.gravar();
                return self.pedir_miniaturas();
            }
            MsgInicio::Abrir(c) => return self.abrir(c),
            MsgInicio::TirarRecente(c) => {
                arquivos::alterar(|p| p.remover_recente(&c));
                self.inicio.recarregar();
            }
            MsgInicio::AdicionarProjeto => return Task::perform(arquivos::escolher_pasta(), |p| Msg::Pagina(MsgInicio::ProjetoEscolhido(p))),
            MsgInicio::ProjetoEscolhido(Some(p)) => {
                arquivos::alterar(|x| x.adicionar_projeto(&p.to_string_lossy()));
                self.inicio.onde = Some(p.to_string_lossy().into_owned());
                self.inicio.recarregar();
                return self.pedir_miniaturas();
            }
            MsgInicio::ProjetoEscolhido(None) => {}
            MsgInicio::TirarProjeto(p) => {
                arquivos::alterar(|x| x.remover_projeto(&p));
                if self.inicio.onde.as_deref() == Some(p.as_str()) {
                    self.inicio.onde = None;
                }
                self.inicio.recarregar();
            }
            MsgInicio::Miniatura(c, s) => {
                self.inicio.miniaturas.insert(c, s.and_then(|s| pelicano::Svg::from_bytes(s.as_bytes())));
            }
        }
        Task::none()
    }

    fn tique(&mut self) {
        self.avisos.retain(|(_, _, quando)| quando.elapsed() < AVISO_POR);
        self.vigiar_disco();
        if self.janelas.aberta(Janela::Monitor) {
            self.monitor.amostrar();
        }
    }
}

impl App {
    /// A faixa mostra a camada atual e as propriedades da seleção: relidas quando o desenho ou a seleção mudam.
    fn recarregar_fita(&mut self) {
        let Some(i) = self.ativa else { return };
        let d = &mut self.docs[i];
        let chave = (i, d.estado.versao ^ (d.estado.selecao_rev << 32));
        if self.versao_fita == Some(chave) {
            return;
        }
        self.versao_fita = Some(chave);
        self.fita.recarregar(d);
        if let Ok(tab) = d.motor.json::<crate::camadas::Tabela>("camadas", json!({})) {
            self.nomes_camadas = tab.linhas.iter().map(|l| l.nome.clone()).collect();
            self.camadas_fita = tab.linhas;
        }
    }

    fn camada_fita(&mut self, pedido: serde_json::Value) {
        let Some(d) = self.doc_mut() else { return };
        #[derive(serde::Deserialize)]
        struct R {
            erro: Option<String>,
            estado: crate::motor::Estado,
        }
        match d.motor.json::<R>("camada_acao", pedido) {
            Ok(r) => {
                d.aplicar(r.estado);
                if let Some(e) = r.erro {
                    self.avisar(t(&e), true);
                }
            }
            Err(e) => self.avisar(e, true),
        }
    }
}

pub fn interruptor_ligado(d: &Doc, nome: &str) -> bool {
    match nome {
        "snap" => d.estado.snap,
        "orto" => d.estado.orto,
        "osnap" => d.estado.osnap,
        _ => d.estado.fundo == "grade",
    }
}

impl pelicano::App for App {
    type Msg = Msg;

    fn theme(&self) -> Theme {
        tema()
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        let tarefa = self.despachar(msg);
        self.recolher_erros();
        if let (Some(p), Some(i)) = (self.props.as_mut(), self.ativa) {
            p.recarregar(&mut self.docs[i]);
        }
        self.recarregar_fita();
        let imagens = self.carregar_imagens();
        Task::batch([tarefa, imagens])
    }

    fn subscription(&self) -> Subscription<Msg> {
        let mut s = vec![mcp_vivo::assinatura(Msg::Mcp), Subscription::every(Duration::from_secs(1)).map(|_| Msg::Tique)];
        if self.doc().is_some_and(|d| d.pan_pendente) {
            s.push(Subscription::every(crate::doc::QUADRO_DO_ARRASTO).map(|_| Msg::QuadroDoArrasto));
        }
        Subscription::batch(s)
    }

    fn on_event(&self, ev: &Event) -> Option<Msg> {
        match ev {
            Event::CloseRequested => Some(Msg::Fechar),
            Event::Maximized(m) => Some(Msg::Maximizada(*m)),
            Event::Resized(s) => Some(Msg::Tamanho(*s)),
            Event::ModifiersChanged(m) => Some(Msg::Mods(*m)),
            _ => None,
        }
    }

    fn on_window_event(&self, id: WindowId, ev: &Event) -> Option<Msg> {
        if id == WindowId::MAIN {
            return self.on_event(ev);
        }
        matches!(ev, Event::CloseRequested).then_some(Msg::FecharAuxiliar(id))
    }

    fn on_unhandled_event(&self, id: WindowId, ev: &Event) -> Option<Msg> {
        match ev {
            Event::Key { key: Key::Named(NamedKey::F12), pressed: true, .. } => Some(Msg::Tecla(Key::Named(NamedKey::F12), Modifiers::empty())),
            Event::Key { key, modifiers, pressed: true, .. } if id == WindowId::MAIN => Some(Msg::Tecla(key.clone(), *modifiers)),
            Event::FileDropped(c) if id == WindowId::MAIN => Some(Msg::Abrir(Some(c.clone()))),
            _ => None,
        }
    }

    fn view(&self) -> pelicano::Element<'_, Msg> {
        self.view_principal()
    }

    fn view_window(&self, id: WindowId) -> pelicano::Element<'_, Msg> {
        match self.janelas.tipo(id) {
            Some(Janela::Monitor) => self.view_monitor(id),
            None => self.view_principal(),
        }
    }
}

impl App {
    fn despachar(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Janela(cmd) => return Task::window(cmd),
            Msg::JanelaAux(id, cmd) => return Task::window_of(id, cmd),
            Msg::Puxar(d) => return Task::window(WindowCmd::DragResize(d)),
            Msg::Maximizada(m) => self.maximizada = m,
            Msg::Tamanho(s) => self.tamanho_janela = s,
            Msg::Mods(m) => self.mods = m,
            Msg::Fechar => return self.continuar_saida(),
            Msg::FecharAuxiliar(id) => {
                self.janelas.retirar(id);
                return Task::window_of(id, WindowCmd::Close);
            }
            Msg::Area(ev) => {
                let Some(d) = self.doc_mut() else { return Task::none() };
                if let Some(m) = d.area(ev) {
                    self.menu = Some((m.canto, m.itens));
                }
            }
            Msg::Faixa(p) => return self.faixa(p),
            Msg::Digitando(s) => self.comando = s,
            Msg::Enviar => {
                let texto = std::mem::take(&mut self.comando);
                if let Some(d) = self.doc_mut() {
                    d.pedir("digitar", json!({ "texto": texto }));
                }
            }
            Msg::Simples(c) => {
                if let Some(d) = self.doc_mut() {
                    d.pedir(c, serde_json::Value::Null);
                }
            }
            Msg::Interruptor(nome) => {
                if let Some(d) = self.doc_mut() {
                    let ligado = interruptor_ligado(d, nome);
                    d.pedir("interruptor", json!({ "nome": nome, "ligado": !ligado }));
                }
            }
            Msg::Fundo(modo) => {
                let antes = self.config.clone();
                self.config.fundo = modo.into();
                self.config_mudou(&antes);
            }
            Msg::OrtoAngulo(s) => {
                let antes = self.config.clone();
                self.config.orto_angulo = s.trim_end_matches('°').parse().unwrap_or(90.0);
                self.config_mudou(&antes);
            }
            Msg::SnapModo(tipo, v) => {
                self.config.snap_modos.insert(tipo.clone(), v);
                let _ = self.config.gravar();
                for d in self.docs.iter_mut().chain(std::iter::once(&mut self.vazio)) {
                    d.pedir("osnap_modos_definir", json!({ "modos": { tipo.clone(): v } }));
                }
            }
            Msg::AlternarSnapModos => self.snap_modos_aberto = !self.snap_modos_aberto,
            Msg::MenuItem(id) => return self.item_do_menu(&id),
            Msg::FecharMenu => {
                self.menu = None;
                self.snap_modos_aberto = false;
            }
            Msg::Tecla(k, m) => return self.tecla(k, m),
            Msg::PedirNovo => self.caixa = Some(Caixa::Unidade { trocar: false }),
            Msg::Abrir(Some(c)) => return self.abrir(c),
            Msg::Abrir(None) => {
                let pasta = self.doc().and_then(|d| d.estado.arquivo.as_ref().map(|a| PathBuf::from(&a.caminho))).and_then(|c| c.parent().map(PathBuf::from)).or_else(arquivos::pasta_padrao);
                return Task::perform(arquivos::escolher_para_abrir(pasta), |c| match c {
                    Some(c) => Msg::Abrir(Some(c)),
                    None => Msg::FecharMenu,
                });
            }
            Msg::Salvar { como, formato } => return self.salvar(como, formato),
            Msg::Destino(Some((c, f))) => match self.salvando.take().and_then(|id| self.indice_do(id)) {
                Some(i) => return self.gravar_em(i, c, f),
                None => self.pendente = None,
            },
            Msg::Destino(None) => {
                self.pendente = None;
                self.salvando = None;
            }
            Msg::PdfDestino(destino) => {
                if let (Some(mut c), Some(bytes)) = (destino, self.pdf.take()) {
                    if c.extension().is_none() {
                        c.set_extension("pdf");
                    }
                    match std::fs::write(&c, bytes) {
                        Ok(()) => self.avisar(tv("aviso.pdfGravado", &[("caminho", &c.display().to_string())]), false),
                        Err(e) => self.avisar(tv("erro.exportar", &[("motivo", &e.to_string())]), true),
                    }
                }
            }
            outra => return self.despachar_resto(outra),
        }
        Task::none()
    }
}

impl App {
    /// A segunda metade do `despachar` (abas, janelas, vista, faixa).
    fn despachar_resto(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Aba(i) => self.ativar(Some(i)),
            Msg::FecharAba(i) => self.pedir_fechar_aba(i),
            Msg::Inicio => {
                self.ativar(None);
                return self.pedir_miniaturas();
            }
            Msg::Caixa(m) => return self.caixa(m),
            Msg::Camadas(m) => {
                let mods = self.mods;
                if let (Some(c), Some(i)) = (self.camadas.as_mut(), self.ativa) {
                    if c.update(m, &mut self.docs[i], mods) {
                        self.camadas = None;
                    }
                }
            }
            Msg::Props(m) => {
                if let (Some(p), Some(i)) = (self.props.as_mut(), self.ativa) {
                    if p.update(m, &mut self.docs[i]) {
                        self.props = None;
                    }
                }
            }
            Msg::Config(m) => return self.configuracoes(m),
            Msg::Pagina(m) => return self.inicio_msg(m),
            Msg::Mcp(p) => return self.mcp(p),
            Msg::Monitor(e) => return self.monitor.atualizar(e),
            Msg::Tique => self.tique(),
            Msg::QuadroDoArrasto => {
                if let Some(d) = self.doc_mut() {
                    d.em_dia();
                }
            }
            Msg::Imagem(c, img) => {
                let img = img.and_then(|(w, h, rgba)| Image::from_rgba(w, h, rgba));
                if img.is_none() {
                    self.monitor.registrar(Nivel::Aviso, "FeatherCAD", &format!("imagem não carregou: {c}"));
                }
                self.imagens.insert(c, img);
            }
            Msg::Tema => {
                let antes = self.config.clone();
                self.config.tema = if self.claro { "escuro".into() } else { "claro".into() };
                self.config_mudou(&antes);
            }
            Msg::Vista(rx, rz) => {
                if let Some(d) = self.doc_mut() {
                    d.girar_para(rx, rz);
                }
            }
            Msg::Zoom(f) => {
                if let Some(d) = self.doc_mut() {
                    d.zoom_no_centro(f);
                }
            }
            Msg::Ajustar => {
                if let Some(d) = self.doc_mut() {
                    d.ajustar();
                }
            }
            Msg::Espaco(nome) => {
                if let Some(d) = self.doc_mut() {
                    d.pedir("layout_alternar", json!({ "nome": nome }));
                    d.ajustar();
                }
            }
            Msg::Fita(m) => {
                if let Some(i) = self.ativa {
                    self.fita.update(m, &mut self.docs[i]);
                }
            }
            Msg::CamadaAtual(nome) => {
                if let Some(l) = self.camadas_fita.iter().find(|l| l.nome == nome) {
                    let i = l.indice;
                    self.camada_fita(json!({ "acao": "atual", "indices": [i] }));
                }
            }
            Msg::CamadaChave(prop) => {
                if let Some(l) = self.camadas_fita.iter().find(|l| l.atual) {
                    let valor = !match prop {
                        "ligada" => l.ligada,
                        "congelada" => l.congelada,
                        _ => l.bloqueada,
                    };
                    let i = l.indice;
                    self.camada_fita(json!({ "acao": "propriedade", "indices": [i], "propriedade": prop, "valor": valor }));
                }
            }
            Msg::NovoLayout => {
                if let Some(d) = self.doc_mut() {
                    let nome = tv("rodape.layout", &[("n", &(d.estado.layouts.len() + 1).to_string())]);
                    d.pedir("layout_criar", json!({ "nome": nome, "papel": "A4", "largura_mm": 297.0, "altura_mm": 210.0 }));
                }
            }
            _ => {}
        }
        Task::none()
    }
}

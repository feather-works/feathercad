//! As Configurações (Ctrl+,; o antigo `configuracoes.ts`): aparência, desenho, salvamento, idioma, apelidos de comando,
//! MCP e sobre. O que muda vai para o `Config` (gravado na hora) e, quando é do motor, para todos os desenhos abertos.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use feather_ui::dialogo::{self, Dialogo};
use feather_ui::tema::*;
use pelicano::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Align, Element, Font, Length, Style};

use crate::config::Config;
use crate::idioma::{t, tv};

pub const SECOES: [&str; 7] = ["aparencia", "desenho", "salvamento", "idioma", "apelidos", "mcp", "sobre"];
pub const ANGULOS_DO_ORTO: [f64; 6] = [90.0, 60.0, 45.0, 30.0, 15.0, 5.0];
const CORES: [&str; 6] = ["azul", "verde", "roxo", "laranja", "rosa", "grafite"];
const NOME_MCP: &str = if cfg!(windows) { "feathercad-mcp.exe" } else { "feathercad-mcp" };

#[derive(Debug, Clone)]
pub enum MsgConfig {
    Secao(&'static str),
    Tema(&'static str),
    Acento(usize),
    Fundo(&'static str),
    Interruptor(&'static str, bool),
    Cota(String),
    Orto(String),
    Formato(&'static str),
    SalvarAgora(bool),
    Idioma(String),
    Apelido(String, String),
    Restaurar,
    Copiar(String),
    Testar,
    Testado(Result<usize, String>),
    Monitor,
    Fps(bool),
    Fechar,
}

/// O que as Configurações pedem ao app.
pub enum Pedido {
    Nada,
    Fechar,
    /// O `Config` mudou: gravar e reaplicar (tema, motor…).
    Mudou,
    Interruptor(&'static str, bool),
    Salvar(bool),
    Copiar(String),
    Testar,
    Monitor,
}

pub struct Configuracoes {
    pub secao: &'static str,
    cota: String,
    apelidos: BTreeMap<String, String>,
    teste: Option<Result<usize, String>>,
    testando: bool,
    linguas: Vec<String>,
    nomes_cores: Vec<String>,
    angulos: Vec<String>,
}

/// Onde está o programa do MCP: ao lado do app ou na pasta de compilação do repositório.
pub fn caminho_do_mcp() -> (PathBuf, bool) {
    let mut v = Vec::new();
    if let Some(p) = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from)) {
        v.push(p.join(NOME_MCP));
    }
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../feathercad-mcp/target/release").join(NOME_MCP));
    match v.iter().find(|c| c.is_file()) {
        Some(c) => (c.canonicalize().unwrap_or_else(|_| c.clone()), true),
        None => (v.pop().unwrap_or_default(), false),
    }
}

/// Abre o programa do MCP, pede `initialize` e `tools/list` e conta as ferramentas (5 s no máximo).
pub fn testar_mcp() -> Result<usize, String> {
    let (programa, existe) = caminho_do_mcp();
    if !existe {
        return Err("programa não encontrado".into());
    }
    let mut filho = Command::new(&programa).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).env("FEATHERCAD_VIVO", "0").spawn().map_err(|e| format!("não consegui abrir o programa: {e}"))?;
    let (Some(mut entrada), Some(saida)) = (filho.stdin.take(), filho.stdout.take()) else { return Err("sem stdin/stdout".into()) };
    for p in [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"feathercad-config","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    ] {
        writeln!(entrada, "{p}").map_err(|_| "o programa fechou sem responder")?;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut linhas = BufReader::new(saida).lines();
        let _ = tx.send((linhas.next(), linhas.next()));
    });
    let resposta = rx.recv_timeout(Duration::from_secs(5));
    let _ = filho.kill();
    let _ = filho.wait();
    let Ok((Some(Ok(_)), Some(Ok(b)))) = resposta else { return Err("o programa não respondeu a tempo".into()) };
    let lista: serde_json::Value = serde_json::from_str(&b).map_err(|_| "resposta que não é JSON")?;
    match lista["result"]["tools"].as_array().map_or(0, Vec::len) {
        0 => Err("o programa respondeu, mas sem ferramentas".into()),
        n => Ok(n),
    }
}

impl Configuracoes {
    pub fn abrir(secao: &'static str, c: &Config) -> Configuracoes {
        let mut apelidos: BTreeMap<String, String> = crate::idioma::apelidos_padrao().into_keys().map(|k| (k, String::new())).collect();
        for (k, v) in &c.apelidos {
            apelidos.insert(k.clone(), v.join(", "));
        }
        let mut angulos: Vec<String> = ANGULOS_DO_ORTO.iter().map(|g| format!("{g}°")).collect();
        if !ANGULOS_DO_ORTO.contains(&c.orto_angulo) {
            angulos.push(format!("{}°", c.orto_angulo));
        }
        Configuracoes {
            secao,
            cota: c.cota.map(|v| v.to_string()).unwrap_or_default(),
            apelidos,
            teste: None,
            testando: false,
            linguas: crate::idioma::linguas().into_iter().map(|(_, n)| n).collect(),
            nomes_cores: CORES.iter().map(|c| t(&format!("config.cores.{c}"))).collect(),
            angulos,
        }
    }

    pub fn update(&mut self, msg: MsgConfig, c: &mut Config) -> Pedido {
        match msg {
            MsgConfig::Fechar => return Pedido::Fechar,
            MsgConfig::Secao(s) => self.secao = s,
            MsgConfig::Tema(m) => c.tema = m.into(),
            MsgConfig::Acento(i) => c.acento = i,
            MsgConfig::Fundo(f) => c.fundo = f.into(),
            MsgConfig::Interruptor(n, v) => return Pedido::Interruptor(n, v),
            MsgConfig::Cota(s) => {
                self.cota = s;
                let v = self.cota.trim().replace(',', ".");
                c.cota = if v.is_empty() { None } else { v.parse::<f64>().ok().filter(|x| *x > 0.0) };
                if !v.is_empty() && c.cota.is_none() {
                    return Pedido::Nada;
                }
            }
            MsgConfig::Orto(s) => c.orto_angulo = s.trim_end_matches('°').parse().unwrap_or(90.0),
            MsgConfig::Formato(f) => c.formato = f.into(),
            MsgConfig::SalvarAgora(como) => return Pedido::Salvar(como),
            MsgConfig::Idioma(nome) => c.idioma = crate::idioma::linguas().into_iter().find(|(_, n)| *n == nome).map(|(k, _)| k.to_string()),
            MsgConfig::Apelido(cmd, v) => {
                let lista: Vec<String> = v.split(',').map(|a| a.trim().to_uppercase()).filter(|a| !a.is_empty()).collect();
                self.apelidos.insert(cmd.clone(), v);
                if lista.is_empty() {
                    c.apelidos.remove(&cmd);
                } else {
                    c.apelidos.insert(cmd, lista);
                }
            }
            MsgConfig::Restaurar => {
                c.apelidos.clear();
                self.apelidos.values_mut().for_each(String::clear);
            }
            MsgConfig::Copiar(s) => return Pedido::Copiar(s),
            MsgConfig::Testar => {
                self.testando = true;
                return Pedido::Testar;
            }
            MsgConfig::Testado(r) => {
                self.testando = false;
                self.teste = Some(r);
                return Pedido::Nada;
            }
            MsgConfig::Monitor => return Pedido::Monitor,
            MsgConfig::Fps(v) => c.fps = v,
        }
        Pedido::Mudou
    }

    pub fn view<'a, M: Clone + 'static>(&'a self, c: &'a Config, estado: &'a crate::motor::Estado, em_desenho: bool, f: impl Fn(MsgConfig) -> M + Copy + 'a, base: Element<'a, M>, altura: f32) -> Element<'a, M> {
        let menu = column(SECOES.iter().map(|s| button(text(t(&format!("config.secoes.{s}"))).size(13)).width(Length::FILL).padding([7u16, 12]).style(estilo_botao_pequeno(self.secao == *s)).on_press(f(MsgConfig::Secao(s))).into())).spacing(2).width(180);
        let corpo: Element<'a, M> = match self.secao {
            "aparencia" => self.aparencia(c, f),
            "desenho" => self.desenho(c, estado, em_desenho, f),
            "salvamento" => salvamento(c, estado, em_desenho, f),
            "idioma" => self.idioma(c, f),
            "apelidos" => self.apelidos(f),
            "mcp" => self.mcp(f),
            _ => sobre(c, f),
        };
        let tudo = row![menu, container(scrollable(container(corpo).padding([0u16, 12]))).width(Length::FILL).height(460)].spacing(14);
        Dialogo::new(t("config.titulo"), tudo).largura(820.0).altura_janela(altura).sem_rolagem().ao_fechar(f(MsgConfig::Fechar)).secundario(t("config.fechar"), Some(f(MsgConfig::Fechar))).sobre(base)
    }

    fn aparencia<'a, M: Clone + 'static>(&'a self, c: &'a Config, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
        let temas = row(["auto", "claro", "escuro"].into_iter().map(|m| button(text(t(&format!("config.tema.{m}"))).size(12)).style(estilo_botao_pequeno(c.tema == m)).on_press(f(MsgConfig::Tema(m))).into())).spacing(4);
        let cores = pick_list(&self.nomes_cores, self.nomes_cores.get(c.acento).cloned(), move |s| f(MsgConfig::Acento(CORES.iter().position(|c| t(&format!("config.cores.{c}")) == s).unwrap_or(0)))).width(200);
        column![item("config.tema.rotulo", "config.tema.dica", temas.into()), item("config.destaque.rotulo", "config.destaque.dica", cores.into())].spacing(16).into()
    }

    fn desenho<'a, M: Clone + 'static>(&'a self, c: &'a Config, e: &'a crate::motor::Estado, em_desenho: bool, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
        let fundos = row(["nada", "pontos", "grade"].into_iter().map(|m| button(text(t(&format!("config.fundo.{m}"))).size(12)).style(estilo_botao_pequeno(c.fundo == m)).on_press(f(MsgConfig::Fundo(m))).into())).spacing(4);
        let mut col = column![item("config.fundo.rotulo", "config.fundo.dica", fundos.into())].spacing(16);
        if em_desenho {
            for (nome, ligado) in [("snap", e.snap), ("orto", e.orto), ("osnap", e.osnap)] {
                col = col.push(item(&format!("config.desenho.{nome}.rotulo"), &format!("config.desenho.{nome}.dica"), checkbox("", ligado).on_toggle(move |v| f(MsgConfig::Interruptor(nome, v))).into()));
            }
        }
        let unidade = e.unidade.clone().unwrap_or_else(|| "mm".into());
        let cota = text_input(t("config.cota.automatico"), &self.cota).width(140).on_input(move |s| f(MsgConfig::Cota(s)));
        col = col.push(
            column![text(t("config.cota.rotulo")).size(13).font(Font::SEMIBOLD), text(tv("config.cota.dica", &[("unidade", &t(&format!("unidade.nomes.{unidade}")))])).size(11).color(COR_TEXTO_MUTED()), cota].spacing(4),
        );
        let orto = pick_list(&self.angulos, Some(format!("{}°", c.orto_angulo)), move |s| f(MsgConfig::Orto(s))).width(140);
        col.push(item("config.orto.rotulo", "config.orto.dica", orto.into())).into()
    }

    fn idioma<'a, M: Clone + 'static>(&'a self, c: &'a Config, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
        let atual = crate::idioma::linguas().into_iter().find(|(k, _)| Some(*k) == c.idioma.as_deref().or(Some(crate::idioma::codigo()))).map(|(_, n)| n);
        let lista = pick_list(&self.linguas, atual, move |s| f(MsgConfig::Idioma(s))).width(240);
        let dica = tv("config.idioma.dica", &[("decimal", &t("idioma.decimal")), ("separador", &t("idioma.separadorDeCoordenadas")), ("casas", &t("idioma.casas"))]);
        column![text(t("config.idioma.rotulo")).size(13).font(Font::SEMIBOLD), text(dica).size(11).color(COR_TEXTO_MUTED()), lista, text(t("config.idioma.nota")).size(11).color(COR_TEXTO_MUTED())].spacing(6).into()
    }

    fn apelidos<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
        let padrao = crate::idioma::apelidos_padrao();
        // Um apelido que já é de outro comando: avisa (o motor usaria o primeiro).
        let dono = |a: &str, exceto: &str| padrao.iter().find(|(k, v)| k.as_str() != exceto && v.iter().any(|x| x.eq_ignore_ascii_case(a))).map(|(k, _)| k.clone());
        let linhas = column(self.apelidos.iter().map(|(cmd, valor)| {
            let ja = padrao.get(cmd).map(|v| v.join(", ")).unwrap_or_default();
            let conflito = valor.split(',').map(str::trim).filter(|a| !a.is_empty()).find_map(|a| dono(a, cmd).map(|d| tv("config.apelidos.conflito", &[("apelido", &a.to_uppercase()), ("comando", &d)])));
            let mut l = row![
                text(t(&format!("menu.item.{cmd}")).replace("menu.item.", "")).size(12).width(150),
                text_input(t("config.apelidos.exemplo"), valor.as_str()).size(12).width(140).on_input(move |s| f(MsgConfig::Apelido(cmd.clone(), s))),
                text(tv("config.apelidos.padrao", &[("lista", &ja)])).size(11).color(COR_TEXTO_MUTED()),
            ]
            .spacing(8)
            .align(Align::Center);
            if let Some(c) = conflito {
                l = l.push(text(c).size(11).color(COR_ERRO()));
            }
            l.into()
        }))
        .spacing(4);
        column![
            text(t("config.apelidos.rotulo")).size(13).font(Font::SEMIBOLD),
            text(t("config.apelidos.dica")).size(11).color(COR_TEXTO_MUTED()),
            button(text(t("config.apelidos.restaurar")).size(12)).style(estilo_botao_secundario).on_press(f(MsgConfig::Restaurar)),
            linhas
        ]
        .spacing(8)
        .into()
    }

    fn mcp<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
        let (caminho, existe) = caminho_do_mcp();
        let caminho = caminho.display().to_string();
        let comando = format!("claude mcp add feathercad -- \"{caminho}\"");
        let json = serde_json::to_string_pretty(&serde_json::json!({ "mcpServers": { "feathercad": { "command": caminho, "args": [] } } })).unwrap_or_default();
        let copiar = |s: String| button(text(t("config.mcp.copiar")).size(11)).style(estilo_botao_pequeno(false)).on_press(f(MsgConfig::Copiar(s)));
        let bloco = |s: String| container(text(s.clone()).size(11).font(Font::MONO)).padding(8).width(Length::FILL).style(|_| Style::default().background(COR_PAINEL_FAIXA()));
        let estado = if existe { t("config.mcp.estadoOk") } else { t("config.mcp.estadoFalta") };
        let teste: Element<'a, M> = match (&self.teste, self.testando) {
            (_, true) => text(t("config.mcp.testando")).size(12).into(),
            (Some(Ok(n)), _) => text(tv("config.mcp.testeOk", &[("n", &n.to_string())])).size(12).color(COR_AZUL_ACENTO()).into(),
            (Some(Err(e)), _) => text(tv("config.mcp.testeErro", &[("motivo", e)])).size(12).color(COR_ERRO()).into(),
            _ => text("").into(),
        };
        column![
            text(t("config.mcp.texto")).size(12),
            row![text(t("config.mcp.programa")).size(13).font(Font::SEMIBOLD), text(estado).size(12).color(if existe { COR_AZUL_ACENTO() } else { COR_ERRO() })].spacing(8),
            bloco(caminho.clone()),
            text(t(if existe { "config.mcp.encontrado" } else { "config.mcp.naoEncontrado" })).size(11).color(COR_TEXTO_MUTED()),
            row![button(text(t("config.mcp.testar")).size(12)).style(estilo_botao_secundario).on_press_maybe(existe.then(|| f(MsgConfig::Testar))), teste].spacing(10).align(Align::Center),
            row![text(t("config.mcp.claude")).size(13).font(Font::SEMIBOLD), copiar(comando.clone())].spacing(8).align(Align::Center),
            text(t("config.mcp.claudeDica")).size(11).color(COR_TEXTO_MUTED()),
            bloco(comando),
            row![text(t("config.mcp.outros")).size(13).font(Font::SEMIBOLD), copiar(json.clone())].spacing(8).align(Align::Center),
            text(t("config.mcp.outrosDica")).size(11).color(COR_TEXTO_MUTED()),
            bloco(json),
            text(t("config.mcp.ferramentas")).size(13).font(Font::SEMIBOLD),
            text(t("config.mcp.ferramentasLista")).size(12),
            text(t("config.mcp.privacidade")).size(11).color(COR_TEXTO_MUTED()),
        ]
        .spacing(8)
        .into()
    }
}

fn item<'a, M: 'a>(rotulo: &str, dica: &str, controle: Element<'a, M>) -> Element<'a, M> {
    column![text(t(rotulo)).size(13).font(Font::SEMIBOLD), text(t(dica)).size(11).color(COR_TEXTO_MUTED()), controle].spacing(4).into()
}

fn salvamento<'a, M: Clone + 'static>(c: &'a Config, e: &'a crate::motor::Estado, em_desenho: bool, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
    let aberto = match (&e.arquivo, em_desenho) {
        (_, false) => t("config.salvamento.nenhum"),
        (Some(a), true) => tv("config.salvamento.aberto", &[("nome", &a.nome)]),
        (None, true) => tv("config.salvamento.aberto", &[("nome", &t("app.semTitulo"))]),
    };
    let formatos = row(["dxf", "fcdz"].into_iter().map(|m| button(text(format!(".{m}")).size(12)).style(estilo_botao_pequeno(c.formato == m)).on_press(f(MsgConfig::Formato(m))).into())).spacing(4);
    column![
        text(t("config.salvamento.rotulo")).size(13).font(Font::SEMIBOLD),
        text(aberto).size(12),
        row![dialogo::botao_secundario(t("config.salvamento.agora"), em_desenho.then(|| f(MsgConfig::SalvarAgora(false)))), dialogo::botao_secundario(t("config.salvamento.como"), em_desenho.then(|| f(MsgConfig::SalvarAgora(true))))].spacing(8),
        item("config.salvamento.formato.rotulo", "config.salvamento.formato.dica", formatos.into()),
        text(t("config.salvamento.nota")).size(11).color(COR_TEXTO_MUTED()),
    ]
    .spacing(10)
    .into()
}

fn sobre<'a, M: Clone + 'static>(c: &'a Config, f: impl Fn(MsgConfig) -> M + Copy + 'a) -> Element<'a, M> {
    column![
        text(format!("FeatherCAD {}", env!("CARGO_PKG_VERSION"))).size(15).font(Font::SEMIBOLD),
        text(t("config.sobre.texto")).size(12),
        item("config.sobre.monitor", "config.sobre.monitorDica", dialogo::botao_secundario(t("config.sobre.abrirMonitor"), Some(f(MsgConfig::Monitor)))),
        item("config.sobre.fps.rotulo", "config.sobre.fps.dica", checkbox("", c.fps).on_toggle(move |v| f(MsgConfig::Fps(v))).into()),
    ]
    .spacing(14)
    .into()
}

//! O gerenciador de propriedades de camadas (o antigo `camadas-painel.ts`), no layout do AutoCAD: filtros, busca,
//! colunas que ordenam, várias linhas com Ctrl/Shift. **Filtrar, buscar, ordenar, validar e aplicar são do motor**
//! (`camadas` e `camada_acao`); aqui só se monta o pedido e se mostra a tabela que volta.

use feather_ui::dialogo::Dialogo;
use feather_ui::seletor_cor;
use feather_ui::tema::*;
use pelicano::{
    button, checkbox, column, container, mouse_area, pick_list, row, scrollable, text, text_input, Align, Border, Color, Element, Length, Modifiers,
    Style,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::doc::Doc;
use crate::idioma::{t, tv};

/// Os tipos de linha: o nome que a tabela devolve (serde do motor) e o do DXF (o que se manda de volta).
pub const TIPOS: [(&str, &str); 6] = [("continua", "CONTINUOUS"), ("tracejada", "DASHED"), ("oculta", "HIDDEN"), ("eixo", "CENTER"), ("pontilhada", "DOT"), ("fantasma", "PHANTOM")];
/// As espessuras de linha padrão do AutoCAD, em mm.
pub const ESPESSURAS: [f32; 24] = [0.0, 0.05, 0.09, 0.13, 0.15, 0.18, 0.2, 0.25, 0.3, 0.35, 0.4, 0.5, 0.53, 0.6, 0.7, 0.8, 0.9, 1.0, 1.06, 1.2, 1.4, 1.58, 2.0, 2.11];
const COLUNAS: [(&str, &str); 10] = [
    ("status", "status"),
    ("nome", "nome"),
    ("ligada", "ligada"),
    ("congelada", "congelada"),
    ("bloqueada", "bloqueada"),
    ("plotar", "plotar"),
    ("cor", "cor"),
    ("tipo_linha", "tipoLinha"),
    ("espessura", "espessura"),
    ("transparencia", "transparencia"),
];
const LARGURAS: [f32; 10] = [52.0, 190.0, 64.0, 70.0, 66.0, 54.0, 76.0, 120.0, 110.0, 64.0];

#[derive(Debug, Clone, Deserialize)]
pub struct Linha {
    pub indice: usize,
    pub nome: String,
    pub atual: bool,
    pub usada: bool,
    pub ligada: bool,
    pub congelada: bool,
    pub bloqueada: bool,
    pub plotar: bool,
    pub cor: String,
    pub tipo_linha: String,
    pub espessura: Option<f32>,
    pub transparencia: u8,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Tabela {
    pub linhas: Vec<Linha>,
    pub exibidas: usize,
    pub totais: usize,
    pub atual: String,
}

#[derive(Debug, Clone)]
pub enum MsgCamadas {
    Filtro(&'static str),
    Busca(String),
    Ordenar(&'static str),
    Escolher(usize),
    Renomear(usize),
    Nome(String),
    ConfirmarNome,
    Nova,
    Excluir,
    TornarAtual,
    Propriedade(usize, &'static str, Value),
    AbrirCor(usize),
    Cor(seletor_cor::Evento),
    Fechar,
}

#[derive(Default)]
pub struct Camadas {
    filtro: &'static str,
    busca: String,
    ordem: &'static str,
    crescente: bool,
    pub tabela: Tabela,
    /// As camadas escolhidas, pelo índice no documento.
    escolhidas: Vec<usize>,
    /// A última clicada (Shift escolhe o intervalo até ela).
    ancora: Option<usize>,
    renomeando: Option<(usize, String)>,
    cor: Option<(usize, [f32; 4], String)>,
    erro: Option<String>,
    /// Os rótulos das listas (o `pick_list` empresta a lista por toda a vida da tela).
    nomes_tipos: Vec<String>,
    nomes_espessuras: Vec<String>,
}

impl Camadas {
    pub fn abrir(doc: &mut Doc) -> Camadas {
        let mut c = Camadas {
            filtro: "todas",
            ordem: "nome",
            crescente: true,
            nomes_tipos: TIPOS.iter().map(|(_, d)| t(&format!("camadas.tipos.{d}"))).collect(),
            nomes_espessuras: std::iter::once(t("camadas.padrao")).chain(ESPESSURAS.iter().map(|e| format!("{e:.2} mm"))).collect(),
            ..Camadas::default()
        };
        c.recarregar(doc);
        c
    }

    fn consulta(&self) -> Value {
        json!({ "filtro": self.filtro, "busca": self.busca, "ordem": self.ordem, "crescente": self.crescente })
    }

    pub fn recarregar(&mut self, doc: &mut Doc) {
        match doc.motor.json::<Tabela>("camadas", self.consulta()) {
            Ok(t) => self.tabela = t,
            Err(e) => self.erro = Some(e),
        }
    }

    fn acao(&mut self, doc: &mut Doc, mut pedido: Value) {
        pedido["consulta"] = self.consulta();
        #[derive(Deserialize)]
        struct R {
            erro: Option<String>,
            tabela: Tabela,
            estado: crate::motor::Estado,
        }
        match doc.motor.json::<R>("camada_acao", pedido) {
            Ok(r) => {
                self.erro = r.erro.map(|c| t(&c));
                self.tabela = r.tabela;
                doc.aplicar(r.estado);
            }
            Err(e) => self.erro = Some(e),
        }
    }

    /// As linhas que uma mudança atinge: as escolhidas, se a clicada está entre elas; senão só a clicada.
    fn alvo(&self, indice: usize) -> Vec<usize> {
        if self.escolhidas.contains(&indice) {
            self.escolhidas.clone()
        } else {
            vec![indice]
        }
    }

    /// `true` = fechar o gerenciador.
    pub fn update(&mut self, msg: MsgCamadas, doc: &mut Doc, mods: Modifiers) -> bool {
        match msg {
            MsgCamadas::Fechar => return true,
            MsgCamadas::Filtro(f) => {
                self.filtro = f;
                self.recarregar(doc);
            }
            MsgCamadas::Busca(b) => {
                self.busca = b;
                self.recarregar(doc);
            }
            MsgCamadas::Ordenar(o) => {
                self.crescente = if self.ordem == o { !self.crescente } else { true };
                self.ordem = o;
                self.recarregar(doc);
            }
            MsgCamadas::Escolher(i) => self.escolher(i, mods),
            MsgCamadas::Renomear(i) => {
                let nome = self.tabela.linhas.iter().find(|l| l.indice == i).map(|l| l.nome.clone()).unwrap_or_default();
                self.renomeando = Some((i, nome));
            }
            MsgCamadas::Nome(n) => {
                if let Some((_, nome)) = &mut self.renomeando {
                    *nome = n;
                }
            }
            MsgCamadas::ConfirmarNome => {
                if let Some((i, nome)) = self.renomeando.take() {
                    self.acao(doc, json!({ "acao": "renomear", "indices": [i], "nome": nome }));
                }
            }
            MsgCamadas::Nova => {
                let base: Vec<usize> = self.escolhidas.first().copied().into_iter().collect();
                self.acao(doc, json!({ "acao": "nova", "indices": base }));
            }
            MsgCamadas::Excluir => {
                let alvo = std::mem::take(&mut self.escolhidas);
                self.acao(doc, json!({ "acao": "excluir", "indices": alvo }));
            }
            MsgCamadas::TornarAtual => {
                if let Some(&i) = self.escolhidas.first() {
                    self.acao(doc, json!({ "acao": "atual", "indices": [i] }));
                }
            }
            MsgCamadas::Propriedade(i, prop, valor) => {
                let alvo = self.alvo(i);
                self.acao(doc, json!({ "acao": "propriedade", "indices": alvo, "propriedade": prop, "valor": valor }));
            }
            MsgCamadas::AbrirCor(i) => {
                let atual = self.tabela.linhas.iter().find(|l| l.indice == i).map(|l| l.cor.clone()).unwrap_or_default();
                self.cor = Some((i, seletor_cor::rgba_para_hsva(seletor_cor::rgba_de(&atual), 0.0), atual));
            }
            MsgCamadas::Cor(ev) => {
                let fechar = matches!(ev, seletor_cor::Evento::Fechar);
                if let Some((i, mut hsva, mut hex)) = self.cor.take() {
                    if let Some(nova) = seletor_cor::aplicar(ev, &mut hsva, &mut hex).filter(|c| !c.is_empty()) {
                        let alvo = self.alvo(i);
                        self.acao(doc, json!({ "acao": "propriedade", "indices": alvo, "propriedade": "cor", "valor": &nova[..7] }));
                    }
                    if !fechar {
                        self.cor = Some((i, hsva, hex));
                    }
                }
            }
        }
        false
    }

    fn escolher(&mut self, i: usize, mods: Modifiers) {
        if mods.shift_key() {
            let pos = |x: usize| self.tabela.linhas.iter().position(|l| l.indice == x);
            if let (Some(a), Some(b)) = (self.ancora.and_then(pos), pos(i)) {
                let (a, b) = (a.min(b), a.max(b));
                self.escolhidas = self.tabela.linhas[a..=b].iter().map(|l| l.indice).collect();
                return;
            }
        }
        if mods.control_key() {
            match self.escolhidas.iter().position(|&x| x == i) {
                Some(p) => {
                    self.escolhidas.remove(p);
                }
                None => self.escolhidas.push(i),
            }
        } else {
            self.escolhidas = vec![i];
        }
        self.ancora = Some(i);
    }

    pub fn view<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgCamadas) -> M + Copy + 'a, base: Element<'a, M>, altura: f32) -> Element<'a, M> {
        let filtros = column![
            text(t("camadas.filtros")).size(12).font(pelicano::Font::SEMIBOLD),
            botao_filtro(t("camadas.filtroTodas"), self.filtro == "todas", f(MsgCamadas::Filtro("todas"))),
            botao_filtro(t("camadas.filtroUsadas"), self.filtro == "usadas", f(MsgCamadas::Filtro("usadas"))),
        ]
        .spacing(4)
        .width(150);
        let alguma = !self.escolhidas.is_empty();
        let barra = row![
            button(text(format!("＋ {}", t("camadas.nova"))).size(12)).style(estilo_botao_secundario).on_press(f(MsgCamadas::Nova)),
            button(text(format!("✕ {}", t("camadas.excluir"))).size(12)).style(estilo_botao_secundario).on_press_maybe(alguma.then(|| f(MsgCamadas::Excluir))),
            button(text(format!("✔ {}", t("camadas.tornarAtual"))).size(12)).style(estilo_botao_secundario).on_press_maybe(alguma.then(|| f(MsgCamadas::TornarAtual))),
            text_input(t("camadas.buscar"), &self.busca).size(12).width(200).on_input(move |s| f(MsgCamadas::Busca(s))),
        ]
        .spacing(6)
        .align(Align::Center);
        let cabecalho = row(COLUNAS.iter().zip(LARGURAS).map(|((ordem, chave), w)| {
            let seta = if self.ordem == *ordem { if self.crescente { " ▲" } else { " ▼" } } else { "" };
            button(text(format!("{}{seta}", t(&format!("camadas.colunas.{chave}")))).size(11).font(pelicano::Font::SEMIBOLD))
                .width(w)
                .padding([4u16, 4])
                .style(estilo_botao_pequeno(false))
                .on_press(f(MsgCamadas::Ordenar(ordem)))
                .into()
        }));
        let linhas = column(self.tabela.linhas.iter().map(|l| self.linha(l, f))).spacing(1);
        let contagem = if self.filtro == "todas" && self.busca.is_empty() { "camadas.contagem" } else { "camadas.contagemFiltrada" };
        let mut rodape = row![text(tv(contagem, &[("exibidas", &self.tabela.exibidas.to_string()), ("totais", &self.tabela.totais.to_string())])).size(11).color(COR_TEXTO_MUTED())].spacing(12);
        if let Some(e) = &self.erro {
            rodape = rodape.push(text(e.clone()).size(11).color(COR_ERRO()));
        }
        let tabela = column![cabecalho, scrollable(linhas).height(Length::FILL)].spacing(2);
        let corpo = column![
            text(tv("camadas.atual", &[("nome", &self.tabela.atual)])).size(12),
            barra,
            row![filtros, container(scrollable(tabela).direction(pelicano::Direction::Horizontal)).width(Length::FILL).height(420)].spacing(10),
            rodape,
        ]
        .spacing(10);
        let mut d = Dialogo::new(t("camadas.titulo"), corpo).ao_fechar(f(MsgCamadas::Fechar)).largura(1080.0).altura_janela(altura).sem_rolagem().secundario(t("camadas.fechar"), Some(f(MsgCamadas::Fechar)));
        if let Some((_, hsva, hex)) = &self.cor {
            let atual = hex.clone();
            let seletor = seletor_cor::painel_embutido("Cor", &atual, *hsva, hex).map(move |e| f(MsgCamadas::Cor(e)));
            let ok = button(text(t("camadas.fechar")).size(12)).style(estilo_botao_secundario).on_press(f(MsgCamadas::Cor(seletor_cor::Evento::Fechar)));
            d = d.rodape_esquerda(column![seletor, ok].spacing(6).width(280));
        }
        d.sobre(base)
    }

    fn linha<'a, M: Clone + 'static>(&'a self, l: &'a Linha, f: impl Fn(MsgCamadas) -> M + Copy + 'a) -> Element<'a, M> {
        let i = l.indice;
        let w = LARGURAS;
        let status = if l.atual { "✔" } else if l.usada { "●" } else { "○" };
        let dica_status = if l.atual { "camadas.dicaAtual" } else if l.usada { "camadas.dicaUsada" } else { "camadas.dicaVazia" };
        let nome: Element<'a, M> = match &self.renomeando {
            Some((r, n)) if *r == i => text_input("", n.as_str()).size(12).width(w[1]).on_input(move |s| f(MsgCamadas::Nome(s))).on_submit(f(MsgCamadas::ConfirmarNome)).into(),
            _ => mouse_area(text(l.nome.clone()).size(12).width(w[1])).on_double_click(f(MsgCamadas::Renomear(i))).into(),
        };
        let caixa = |w: f32, ligado: bool, prop: &'static str| -> Element<'a, M> {
            container(checkbox("", ligado).on_toggle(move |v| f(MsgCamadas::Propriedade(i, prop, Value::Bool(v))))).width(w).into()
        };
        let cor = Color::hex(&l.cor).unwrap_or(Color::BLACK);
        let amostra = button(row![container(text("")).width(14).height(14).style(move |_| Style::default().background(cor).border(Border::default().color(COR_BORDA(), 1.0))), text(l.cor.clone()).size(10)].spacing(4).align(Align::Center))
            .padding([2u16, 2])
            .width(w[6])
            .style(estilo_botao_pequeno(false))
            .on_press(f(MsgCamadas::AbrirCor(i)));
        let tipos = &self.nomes_tipos;
        let tipo_atual = TIPOS.iter().position(|(n, _)| *n == l.tipo_linha).map(|p| tipos[p].clone());
        let tipo = pick_list(tipos, tipo_atual, move |s: String| {
            let dxf = tipos.iter().position(|n| *n == s).map_or("CONTINUOUS", |p| TIPOS[p].1);
            f(MsgCamadas::Propriedade(i, "tipo_linha", Value::String(dxf.into())))
        })
        .size(11)
        .width(w[7]);
        let esp_atual = Some(l.espessura.map_or_else(|| t("camadas.padrao"), |e| format!("{e:.2} mm")));
        let espessura = pick_list(&self.nomes_espessuras, esp_atual, move |s: String| {
            let v = s.trim_end_matches(" mm").parse::<f64>().map_or(Value::Null, Value::from);
            f(MsgCamadas::Propriedade(i, "espessura", v))
        })
        .size(11)
        .width(w[8]);
        let transp = text_input("0", l.transparencia.to_string()).size(11).width(w[9]).on_input(move |s| f(MsgCamadas::Propriedade(i, "transparencia", s.trim().parse::<u64>().map_or(Value::Null, Value::from))));
        let escolhida = self.escolhidas.contains(&i);
        let conteudo = row![
            pelicano::tooltip(text(status).size(13).width(w[0]), t(dica_status), pelicano::TooltipPosition::Bottom),
            nome,
            caixa(w[2], l.ligada, "ligada"),
            caixa(w[3], l.congelada, "congelada"),
            caixa(w[4], l.bloqueada, "bloqueada"),
            caixa(w[5], l.plotar, "plotar"),
            amostra,
            tipo,
            espessura,
            transp,
        ]
        .align(Align::Center)
        .spacing(0);
        mouse_area(container(conteudo).padding([2u16, 4]).style(move |_| if escolhida { Style::default().background(COR_AZUL_FUNDO()) } else { Style::default() })).on_press(f(MsgCamadas::Escolher(i))).into()
    }
}

fn botao_filtro<'a, M: Clone + 'static>(rotulo: String, ativo: bool, msg: M) -> Element<'a, M> {
    button(text(rotulo).size(12)).width(Length::FILL).style(estilo_botao_pequeno(ativo)).on_press(msg).into()
}

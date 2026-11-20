//! A página inicial (o antigo `pagina-inicial.ts`): Novo, Abrir, e à esquerda Recentes ou Projetos; os desenhos em lista
//! (nome, aberto em, tipo, local) ou em miniaturas. Tudo local: recentes e pastas ficam no `inicio.json`.

use std::collections::HashMap;
use std::path::PathBuf;

use feather_ui::tema::*;
use feathercad_io::inicio::ItemDeArquivo;
use pelicano::{button, column, container, horizontal_space, mouse_area, row, scrollable, svg, text, Align, Border, Element, Font, Length, Style, Svg};

use crate::arquivos;
use crate::idioma::t;

#[derive(Debug, Clone)]
pub enum MsgInicio {
    Navegar(Option<String>),
    Visao(&'static str),
    Abrir(PathBuf),
    TirarRecente(String),
    AdicionarProjeto,
    ProjetoEscolhido(Option<PathBuf>),
    TirarProjeto(String),
    Miniatura(String, Option<String>),
}

pub struct Inicio {
    /// `None` = Recentes; `Some(pasta)` = um projeto.
    pub onde: Option<String>,
    pub recentes: Vec<(ItemDeArquivo, u64)>,
    pub projetos: Vec<String>,
    pub da_pasta: Vec<ItemDeArquivo>,
    pub erro: Option<String>,
    /// Miniaturas já feitas (`None` = sem o que mostrar) e as pedidas.
    pub miniaturas: HashMap<String, Option<Svg>>,
}

impl Inicio {
    pub fn novo() -> Inicio {
        let mut i = Inicio { onde: None, recentes: Vec::new(), projetos: Vec::new(), da_pasta: Vec::new(), erro: None, miniaturas: HashMap::new() };
        i.recarregar();
        i
    }

    pub fn recarregar(&mut self) {
        self.recentes = arquivos::recentes();
        self.projetos = arquivos::preferencias().projetos;
        self.da_pasta = match &self.onde {
            Some(p) => feathercad_io::inicio::listar_pasta(std::path::Path::new(p)).unwrap_or_else(|e| {
                self.erro = Some(e);
                Vec::new()
            }),
            None => Vec::new(),
        };
    }

    fn itens(&self) -> Vec<(&ItemDeArquivo, Option<u64>)> {
        match &self.onde {
            None => self.recentes.iter().map(|(i, em)| (i, Some(*em))).collect(),
            Some(_) => self.da_pasta.iter().map(|i| (i, None)).collect(),
        }
    }

    /// As miniaturas que faltam (para o app pedir em segundo plano), marcadas como pedidas.
    pub fn faltando(&mut self) -> Vec<String> {
        let faltam: Vec<String> = self.itens().iter().map(|(i, _)| i.caminho.clone()).filter(|c| !self.miniaturas.contains_key(c)).collect();
        for c in &faltam {
            self.miniaturas.insert(c.clone(), None);
        }
        faltam
    }

    pub fn view<'a, M: Clone + 'static>(&'a self, visao: &'a str, f: impl Fn(MsgInicio) -> M + Copy + 'a, novo: M, abrir: M, aviso: Option<&'a str>) -> Element<'a, M> {
        let nav = |rotulo: String, ativo: bool, m: MsgInicio| button(text(rotulo).size(13)).width(Length::FILL).padding([7u16, 12]).style(estilo_botao_pequeno(ativo)).on_press(f(m));
        let mut lateral = column![
            button(text(format!("＋ {}", t("inicio.novo"))).size(13)).width(Length::FILL).padding([9u16, 12]).style(estilo_botao_primario).on_press(novo),
            button(text(t("inicio.abrir")).size(13)).width(Length::FILL).padding([9u16, 12]).style(estilo_botao_secundario).on_press(abrir),
            pelicano::Space::new(0, 12),
            nav(t("inicio.navegacao.recentes"), self.onde.is_none(), MsgInicio::Navegar(None)),
            text(t("inicio.navegacao.projetos")).size(11).color(COR_TEXTO_MUTED()),
        ]
        .spacing(6)
        .width(230);
        for p in &self.projetos {
            let nome = std::path::Path::new(p).file_name().map_or_else(|| p.clone(), |n| n.to_string_lossy().into_owned());
            let tirar = button(text("✕").size(11)).style(estilo_botao_pequeno(false)).on_press(f(MsgInicio::TirarProjeto(p.clone())));
            lateral = lateral.push(row![nav(nome, self.onde.as_deref() == Some(p), MsgInicio::Navegar(Some(p.clone()))), pelicano::tooltip(tirar, t("inicio.projetos.remover"), pelicano::TooltipPosition::Right)].spacing(2).align(Align::Center));
        }
        lateral = lateral.push(button(text(t("inicio.projetos.adicionar")).size(12)).width(Length::FILL).style(estilo_botao_pequeno(false)).on_press(f(MsgInicio::AdicionarProjeto)));
        let titulo = match &self.onde {
            None => t("inicio.recentes.titulo"),
            Some(p) => p.clone(),
        };
        let visoes = row(["lista", "grade"].into_iter().map(|v| button(text(t(&format!("inicio.visao.{v}"))).size(12)).style(estilo_botao_pequeno(visao == v)).on_press(f(MsgInicio::Visao(v))).into())).spacing(2);
        let itens = self.itens();
        let corpo: Element<'a, M> = if itens.is_empty() {
            let vazio = match &self.onde {
                None => "inicio.recentes.vazio",
                Some(_) => "inicio.projetos.semDesenhos",
            };
            text(t(vazio)).size(13).color(COR_TEXTO_MUTED()).into()
        } else if visao == "grade" {
            self.grade(&itens, f)
        } else {
            self.lista(&itens, f)
        };
        let mut direita = column![row![text(titulo).size(17).font(Font::SEMIBOLD), horizontal_space(), visoes].align(Align::Center), scrollable(corpo).height(Length::FILL)].spacing(14);
        if let Some(a) = aviso.or(self.erro.as_deref()) {
            direita = direita.push(text(a.to_string()).size(12).color(COR_ERRO()));
        }
        container(row![lateral, container(direita).width(Length::FILL)].spacing(24).padding(28)).width(Length::FILL).height(Length::FILL).style(|_| Style::default().background(COR_FUNDO_JANELA())).into()
    }

    fn lista<'a, M: Clone + 'static>(&'a self, itens: &[(&'a ItemDeArquivo, Option<u64>)], f: impl Fn(MsgInicio) -> M + Copy + 'a) -> Element<'a, M> {
        let col = |s: String, w: f32| text(s).size(11).font(Font::SEMIBOLD).color(COR_TEXTO_MUTED()).width(w);
        let quando = if self.onde.is_none() { t("inicio.colunas.aberto") } else { t("inicio.colunas.criadoEm") };
        let cab = row![col(t("inicio.colunas.nome"), 280.0), col(quando, 150.0), col(t("inicio.colunas.tipo"), 170.0), col(t("inicio.colunas.local"), 300.0)].spacing(8);
        let linhas = itens.iter().map(|(i, em)| {
            let data = em.or(i.criado_ms).map_or_else(String::new, arquivos::data);
            let mut r = row![
                text(i.nome.clone()).size(13).width(280),
                text(data).size(12).color(COR_TEXTO_MUTED()).width(150),
                text(t(&format!("inicio.tipos.{}", i.tipo))).size(12).color(COR_TEXTO_MUTED()).width(170),
                text(i.pasta.clone()).size(12).color(COR_TEXTO_MUTED()).width(300),
            ]
            .spacing(8)
            .align(Align::Center);
            if self.onde.is_none() {
                r = r.push(button(text("✕").size(11)).style(estilo_botao_pequeno(false)).on_press(f(MsgInicio::TirarRecente(i.caminho.clone()))));
            }
            button(r).width(Length::FILL).padding([7u16, 10]).style(estilo_botao_pequeno(false)).on_press(f(MsgInicio::Abrir(PathBuf::from(&i.caminho)))).into()
        });
        column(std::iter::once(cab.into()).chain(linhas)).spacing(2).into()
    }

    fn grade<'a, M: Clone + 'static>(&'a self, itens: &[(&'a ItemDeArquivo, Option<u64>)], f: impl Fn(MsgInicio) -> M + Copy + 'a) -> Element<'a, M> {
        let cartoes: Vec<Element<'a, M>> = itens
            .iter()
            .map(|(i, _)| {
                let imagem: Element<'a, M> = match self.miniaturas.get(&i.caminho) {
                    Some(Some(s)) => svg(s.clone()).width(220).height(138).into(),
                    _ => container(text("⬚").size(32).color(COR_TEXTO_MUTED())).width(220).height(138).center().into(),
                };
                let cartao = column![
                    container(imagem).style(|_| Style::default().background(COR_PAINEL()).border(Border::rounded(6.0).color(COR_BORDA(), 1.0))),
                    text(i.nome.clone()).size(12),
                    text(t(&format!("inicio.tipos.{}", i.tipo))).size(11).color(COR_TEXTO_MUTED()),
                ]
                .spacing(4)
                .width(220);
                mouse_area(cartao).on_press(f(MsgInicio::Abrir(PathBuf::from(&i.caminho)))).into()
            })
            .collect();
        let mut linhas = column![].spacing(16);
        let mut it = cartoes.into_iter().peekable();
        while it.peek().is_some() {
            linhas = linhas.push(row(it.by_ref().take(4)).spacing(16));
        }
        linhas.into()
    }
}

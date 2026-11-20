//! As caixas de diálogo (os antigos `caixa-*.ts`): unidade, alterações não salvas, padrão de hachura, folha (inserir,
//! escala, carimbo) e exportar PDF. Cada uma só junta a escolha num pedido; quem valida e faz é o motor.

use feather_ui::dialogo::{self, Dialogo};
use feather_ui::tema::*;
use pelicano::{button, checkbox, column, pick_list, row, svg, text, text_input, Align, Element, Font, Length, Svg};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::doc::Doc;
use crate::idioma::{t, tv};

pub const UNIDADES: [&str; 3] = ["mm", "cm", "m"];
const PAPEIS_PDF: [&str; 10] = ["A4", "A3", "A2", "A1", "A0", "2A0", "4A0", "8A0", "16A0", "FOLHA"];
const ORIENTACOES: [&str; 3] = ["automatica", "horizontal", "vertical"];
/// Os campos do carimbo: (nome no motor, chave do rótulo).
const CAMPOS: [(&str, &str); 10] = [
    ("cliente", "cliente"),
    ("fase", "fase"),
    ("disciplina", "disciplina"),
    ("responsavel", "responsavel"),
    ("titulo", "tituloDesenho"),
    ("subtitulo", "subtitulo"),
    ("numero", "numero"),
    ("revisao", "revisao"),
    ("revisao_descricao", "revisaoDescricao"),
    ("folha", "numeroDaFolha"),
];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InfoFolha {
    pub indice: usize,
    pub papel: String,
    pub escala: f64,
    pub escala_texto: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Folhas {
    pub folhas: Vec<InfoFolha>,
    pub papeis: Vec<String>,
    pub escalas: Vec<(f64, String)>,
}

impl Folhas {
    fn rotulo(&self, f: &InfoFolha) -> String {
        tv("folha.item", &[("n", &(f.indice + 1).to_string()), ("papel", &f.papel), ("escala", &f.escala_texto)])
    }
}

/// O que fazer depois do "Salvar alterações?", com o `Doc::id` da aba perguntada (não o índice: a pessoa pode
/// trocar ou fechar abas enquanto a caixa de salvar está aberta).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Depois {
    FecharAba(u64),
    Sair(u64),
}

pub enum Caixa {
    /// Desenho novo (`None`) ou trocar a unidade do aberto (`Some(atual)`).
    Unidade { trocar: bool },
    Confirmar { nome: String, depois: Depois },
    Hachura { padroes: Vec<String>, nomes: Vec<String>, padrao: String, escalas: Vec<(f64, String)>, rotulos: Vec<String>, escala: f64, previa: Option<Svg> },
    FolhaNova { folhas: Folhas, papel: String, escala: Option<f64>, rotulos: Vec<String>, campos: Value, cobrir: bool, com_selecao: bool },
    FolhaEscala { folhas: Folhas, rotulos: Vec<String>, indice: usize, escala: Option<f64>, rotulos_escala: Vec<String> },
    Carimbo { folhas: Folhas, rotulos: Vec<String>, indice: usize, campos: Value },
    Exportar { paginas: usize, papel: String, papeis: Vec<String>, orientacao: String, orientacoes: Vec<String>, cores: bool, escolhidas: Vec<bool> },
}

#[derive(Debug, Clone)]
pub enum MsgCaixa {
    Fechar,
    Unidade(&'static str),
    Salvar,
    Descartar,
    Padrao(String),
    Escala(String),
    Papel(String),
    Orientacao(String),
    Folha(String),
    Campo(&'static str, String),
    Divisoria(bool),
    Cobrir(bool),
    Cores(bool),
    Pagina(usize, bool),
    Todas(bool),
    Aplicar,
    Ajustar,
}

/// O que a caixa pede ao app quando termina.
pub enum Saida {
    Nada,
    Fechar,
    NovoDesenho(&'static str),
    Confirmado(Depois, bool),
    ExportarPdf(Vec<u8>),
    Erro(String),
}

fn rotulos_escala(f: &Folhas) -> Vec<String> {
    std::iter::once(t("folha.inserir.automatica")).chain(f.escalas.iter().map(|(_, s)| s.clone())).collect()
}

impl Caixa {
    pub fn hachura(doc: &mut Doc) -> Result<Caixa, String> {
        #[derive(Deserialize)]
        struct O {
            padroes: Vec<String>,
            atual: String,
            escala: f64,
            escalas: Vec<(f64, String)>,
        }
        let o: O = doc.motor.json("hachura_opcoes", Value::Null)?;
        let nomes = o.padroes.iter().map(|p| t(&format!("hachura.padroes.{p}"))).map(|n| n.trim_start_matches("hachura.padroes.").to_string()).collect();
        let rotulos = o.escalas.iter().map(|(_, s)| s.clone()).collect();
        let mut c = Caixa::Hachura { padroes: o.padroes, nomes, padrao: o.atual, escalas: o.escalas, rotulos, escala: o.escala, previa: None };
        c.previa(doc);
        Ok(c)
    }

    fn previa(&mut self, doc: &mut Doc) {
        if let Caixa::Hachura { padrao, escala, previa, .. } = self {
            *previa = doc.motor.json::<String>("hachura_previa", json!({ "padrao": padrao, "escala": escala })).ok().and_then(|s| Svg::from_bytes(s.as_bytes()));
        }
    }

    pub fn folhas(doc: &mut Doc) -> Result<Folhas, String> {
        doc.motor.json("folha_listar", Value::Null)
    }

    pub fn folha_nova(doc: &mut Doc) -> Result<Caixa, String> {
        let folhas = Self::folhas(doc)?;
        let papel = folhas.papeis.iter().find(|p| p.as_str() == "A1").or(folhas.papeis.first()).cloned().unwrap_or_default();
        let com_selecao = doc.estado.selecionados > 0;
        Ok(Caixa::FolhaNova { rotulos: rotulos_escala(&folhas), folhas, papel, escala: None, campos: json!({}), cobrir: com_selecao, com_selecao })
    }

    pub fn folha_escala(doc: &mut Doc) -> Result<Caixa, String> {
        let folhas = Self::folhas(doc)?;
        let rotulos = folhas.folhas.iter().map(|f| folhas.rotulo(f)).collect();
        let rotulos_escala = folhas.escalas.iter().map(|(_, s)| s.clone()).collect();
        let escala = folhas.folhas.first().map(|f| f.escala);
        Ok(Caixa::FolhaEscala { folhas, rotulos, indice: 0, escala, rotulos_escala })
    }

    pub fn carimbo(doc: &mut Doc) -> Result<Caixa, String> {
        let folhas = Self::folhas(doc)?;
        let rotulos = folhas.folhas.iter().map(|f| folhas.rotulo(f)).collect();
        let campos = if folhas.folhas.is_empty() { json!({}) } else { doc.motor.json("folha_campos", json!({ "indice": 0 }))? };
        Ok(Caixa::Carimbo { folhas, rotulos, indice: 0, campos })
    }

    pub fn exportar(doc: &mut Doc) -> Result<Caixa, String> {
        let paginas = doc.motor.json::<Vec<(f64, f64)>>("documento_folhas", Value::Null)?.len();
        Ok(Caixa::Exportar {
            paginas,
            papel: "A3".into(),
            papeis: PAPEIS_PDF.iter().map(|s| s.to_string()).collect(),
            orientacao: "automatica".into(),
            orientacoes: ORIENTACOES.iter().map(|o| t(&format!("exportar.orientacoes.{o}"))).collect(),
            cores: false,
            escolhidas: vec![true; paginas],
        })
    }

    pub fn update(&mut self, msg: MsgCaixa, doc: Option<&mut Doc>) -> Saida {
        if let MsgCaixa::Fechar = msg {
            return Saida::Fechar;
        }
        match (self, msg) {
            (Caixa::Unidade { trocar: false }, MsgCaixa::Unidade(u)) => Saida::NovoDesenho(u),
            (Caixa::Unidade { trocar: true }, MsgCaixa::Unidade(u)) => {
                if let Some(d) = doc {
                    d.pedir("documento_unidade_definir", json!({ "unidade": u }));
                }
                Saida::Fechar
            }
            (Caixa::Confirmar { depois, .. }, MsgCaixa::Salvar) => Saida::Confirmado(*depois, true),
            (Caixa::Confirmar { depois, .. }, MsgCaixa::Descartar) => Saida::Confirmado(*depois, false),
            (c @ Caixa::Hachura { .. }, m) => c.update_hachura(m, doc),
            (c @ (Caixa::FolhaNova { .. } | Caixa::FolhaEscala { .. } | Caixa::Carimbo { .. }), m) => match doc {
                Some(d) => c.update_folha(m, d),
                None => Saida::Fechar,
            },
            (c @ Caixa::Exportar { .. }, m) => match doc {
                Some(d) => c.update_exportar(m, d),
                None => Saida::Fechar,
            },
            _ => Saida::Nada,
        }
    }

    fn update_hachura(&mut self, msg: MsgCaixa, doc: Option<&mut Doc>) -> Saida {
        let Some(d) = doc else { return Saida::Fechar };
        let Caixa::Hachura { padroes, nomes, padrao, escalas, escala, .. } = self else { return Saida::Nada };
        match msg {
            MsgCaixa::Padrao(n) => {
                if let Some(i) = nomes.iter().position(|x| *x == n) {
                    *padrao = padroes[i].clone();
                }
            }
            MsgCaixa::Escala(s) => {
                if let Some((v, _)) = escalas.iter().find(|(_, r)| *r == s) {
                    *escala = *v;
                }
            }
            MsgCaixa::Aplicar => {
                return match d.motor.json::<Value>("hachura_definir", json!({ "padrao": padrao, "escala": escala })) {
                    Ok(_) => Saida::Fechar,
                    Err(e) => Saida::Erro(e),
                };
            }
            _ => return Saida::Nada,
        }
        self.previa(d);
        Saida::Nada
    }

    fn update_folha(&mut self, msg: MsgCaixa, d: &mut Doc) -> Saida {
        #[derive(Deserialize)]
        struct R {
            estado: crate::motor::Estado,
        }
        let mut aplicar = |pedido: &str, corpo: Value| match d.motor.json::<R>(pedido, corpo) {
            Ok(r) => {
                d.aplicar(r.estado);
                Saida::Fechar
            }
            Err(e) => Saida::Erro(e),
        };
        match (self, msg) {
            (Caixa::FolhaNova { papel, .. }, MsgCaixa::Papel(p)) => *papel = p,
            (Caixa::FolhaNova { folhas, escala, .. }, MsgCaixa::Escala(s)) => *escala = folhas.escalas.iter().find(|(_, r)| *r == s).map(|(v, _)| *v),
            (Caixa::FolhaNova { campos, .. } | Caixa::Carimbo { campos, .. }, MsgCaixa::Campo(c, v)) => campos[c] = Value::String(v),
            (Caixa::FolhaNova { campos, .. } | Caixa::Carimbo { campos, .. }, MsgCaixa::Divisoria(v)) => campos["divisoria"] = Value::Bool(v),
            (Caixa::FolhaNova { cobrir, .. }, MsgCaixa::Cobrir(v)) => *cobrir = v,
            (Caixa::FolhaNova { papel, escala, campos, cobrir, .. }, MsgCaixa::Aplicar) => {
                let mut p = json!({ "papel": papel, "campos": campos, "cobrir_selecao": *cobrir });
                if let Some(e) = escala {
                    p["escala"] = json!(e);
                }
                let s = aplicar("folha_inserir", p);
                d.ajustar();
                return s;
            }
            (Caixa::FolhaEscala { folhas, rotulos, indice, escala, .. }, MsgCaixa::Folha(f)) => {
                *indice = rotulos.iter().position(|r| *r == f).unwrap_or(0);
                *escala = folhas.folhas.get(*indice).map(|f| f.escala);
            }
            (Caixa::FolhaEscala { folhas, escala, .. }, MsgCaixa::Escala(s)) => *escala = folhas.escalas.iter().find(|(_, r)| *r == s).map(|(v, _)| *v),
            (Caixa::FolhaEscala { indice, escala: Some(e), .. }, MsgCaixa::Aplicar) => {
                let s = aplicar("folha_escala", json!({ "indice": indice, "escala": e }));
                d.ajustar();
                return s;
            }
            (Caixa::FolhaEscala { indice, .. }, MsgCaixa::Ajustar) => {
                let s = aplicar("folha_ajustar", json!({ "indice": indice }));
                d.ajustar();
                return s;
            }
            (Caixa::Carimbo { rotulos, indice, campos, .. }, MsgCaixa::Folha(f)) => {
                *indice = rotulos.iter().position(|r| *r == f).unwrap_or(0);
                *campos = d.motor.json("folha_campos", json!({ "indice": *indice })).unwrap_or_else(|_| json!({}));
            }
            (Caixa::Carimbo { indice, campos, .. }, MsgCaixa::Aplicar) => return aplicar("folha_campos_definir", json!({ "indice": indice, "campos": campos })),
            _ => {}
        }
        Saida::Nada
    }

    fn update_exportar(&mut self, msg: MsgCaixa, d: &mut Doc) -> Saida {
        let Caixa::Exportar { papel, orientacao, orientacoes, cores, escolhidas, .. } = self else { return Saida::Nada };
        match msg {
            MsgCaixa::Papel(p) => *papel = p,
            MsgCaixa::Orientacao(o) => *orientacao = orientacoes.iter().position(|x| *x == o).map_or("automatica", |i| ORIENTACOES[i]).to_string(),
            MsgCaixa::Cores(v) => *cores = v,
            MsgCaixa::Pagina(i, v) => {
                if let Some(e) = escolhidas.get_mut(i) {
                    *e = v;
                }
            }
            MsgCaixa::Todas(v) => escolhidas.iter_mut().for_each(|e| *e = v),
            MsgCaixa::Aplicar => {
                let mut p = json!({ "papel": papel, "orientacao": orientacao, "cores": cores });
                if !escolhidas.is_empty() {
                    p["folhas"] = json!(escolhidas.iter().enumerate().filter(|(_, e)| **e).map(|(i, _)| i).collect::<Vec<_>>());
                }
                return match d.motor.ponte.chamar("documento_exportar_pdf", p.to_string().as_bytes()) {
                    Ok(b) => Saida::ExportarPdf(b),
                    Err(e) => Saida::Erro(e),
                };
            }
            _ => {}
        }
        Saida::Nada
    }

    pub fn view<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgCaixa) -> M + Copy + 'a, base: Element<'a, M>, altura: f32) -> Element<'a, M> {
        let fechar = f(MsgCaixa::Fechar);
        let d = match self {
            Caixa::Unidade { trocar } => {
                let botoes = column(UNIDADES.iter().enumerate().map(|(i, u)| {
                    button(
                        row![
                            text(format!("{}", i + 1)).size(12).color(COR_TEXTO_MUTED()).width(16),
                            column![text(t(&format!("unidade.nomes.{u}"))).size(13).font(Font::SEMIBOLD), text(t(&format!("unidade.uso.{u}"))).size(11).color(COR_TEXTO_MUTED())].spacing(2),
                            pelicano::horizontal_space(),
                            text(t(&format!("unidade.regua.{u}"))).size(11).color(COR_TEXTO_MUTED()),
                        ]
                        .spacing(10)
                        .align(Align::Center),
                    )
                    .width(Length::FILL)
                    .padding([10u16, 12])
                    .style(estilo_botao_secundario)
                    .on_press(f(MsgCaixa::Unidade(u)))
                    .into()
                }))
                .spacing(6);
                let corpo = column![text(t(if *trocar { "unidade.dica" } else { "unidade.corpo" })).size(12), botoes, text(t("unidade.trocarDepois")).size(11).color(COR_TEXTO_MUTED())].spacing(10);
                Dialogo::new(t("unidade.titulo"), corpo).secundario(t("unidade.cancelar"), Some(fechar.clone()))
            }
            Caixa::Confirmar { nome, .. } => Dialogo::new(t("confirmar.titulo"), text(tv("confirmar.corpo", &[("nome", nome)])).size(13))
                .secundario(t("confirmar.cancelar"), Some(fechar.clone()))
                .botao(dialogo::botao_secundario(t("confirmar.descartar"), Some(f(MsgCaixa::Descartar))))
                .primario(t("confirmar.salvar"), Some(f(MsgCaixa::Salvar))),
            Caixa::Hachura { nomes, padroes, padrao, rotulos, escalas, escala, previa, .. } => {
                let atual = padroes.iter().position(|p| p == padrao).map(|i| nomes[i].clone());
                let esc = escalas.iter().position(|(v, _)| (v - escala).abs() < 1e-9).map(|i| rotulos[i].clone());
                let mut corpo = column![text(t("hachura.escolha.corpo")).size(12)].spacing(10);
                if let Some(s) = previa {
                    corpo = corpo.push(svg(s.clone()).width(Length::FILL).height(120));
                }
                corpo = corpo
                    .push(campo(t("hachura.escolha.padrao"), pick_list(nomes, atual, move |s| f(MsgCaixa::Padrao(s))).width(Length::FILL).into()))
                    .push(campo(t("hachura.escolha.escala"), pick_list(rotulos, esc, move |s| f(MsgCaixa::Escala(s))).width(Length::FILL).into()));
                Dialogo::new(t("hachura.escolha.titulo"), corpo).secundario(t("hachura.escolha.cancelar"), Some(fechar.clone())).primario(t("hachura.escolha.aplicar"), Some(f(MsgCaixa::Aplicar)))
            }
            outra => outra.dialogo_folha(f),
        };
        d.ao_fechar(fechar).altura_janela(altura).sobre(base)
    }
}

impl Caixa {
    /// Os diálogos de folha e de PDF (a outra metade do `view`).
    fn dialogo_folha<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgCaixa) -> M + Copy + 'a) -> Dialogo<'a, M> {
        let fechar = f(MsgCaixa::Fechar);
        match self {
            Caixa::FolhaNova { folhas, papel, escala, rotulos, campos, cobrir, com_selecao } => {
                let esc = escala.and_then(|e| folhas.escalas.iter().find(|(v, _)| (v - e).abs() < 1e-9)).map_or_else(|| rotulos[0].clone(), |(_, s)| s.clone());
                let mut corpo = column![
                    text(t("folha.inserir.corpo")).size(12),
                    row![
                        campo(t("folha.inserir.papel"), pick_list(&folhas.papeis, Some(papel.clone()), move |s| f(MsgCaixa::Papel(s))).width(Length::FILL).into()),
                        campo(t("folha.inserir.escala"), pick_list(rotulos, Some(esc), move |s| f(MsgCaixa::Escala(s))).width(Length::FILL).into()),
                    ]
                    .spacing(10),
                    text(t("folha.inserir.carimbo")).size(12).font(Font::SEMIBOLD),
                    campos_do_carimbo(campos, f),
                ]
                .spacing(10);
                if *com_selecao {
                    corpo = corpo.push(checkbox(t("folha.inserir.cobrirSelecao"), *cobrir).on_toggle(move |v| f(MsgCaixa::Cobrir(v))));
                }
                Dialogo::new(t("folha.inserir.titulo"), corpo).largura(620.0).secundario(t("folha.inserir.cancelar"), Some(fechar.clone())).primario(t("folha.inserir.inserir"), Some(f(MsgCaixa::Aplicar)))
            }
            Caixa::FolhaEscala { folhas, rotulos, indice, escala, rotulos_escala } => {
                let corpo: Element<'a, M> = if folhas.folhas.is_empty() {
                    text(t("folha.escala.semFolhas")).size(12).into()
                } else {
                    let esc = escala.and_then(|e| folhas.escalas.iter().find(|(v, _)| (v - e).abs() < 1e-9)).map(|(_, s)| s.clone());
                    column![
                        text(t("folha.escala.corpo")).size(12),
                        campo(t("folha.escala.qual"), pick_list(rotulos, rotulos.get(*indice).cloned(), move |s| f(MsgCaixa::Folha(s))).width(Length::FILL).into()),
                        campo(t("folha.escala.escala"), pick_list(rotulos_escala, esc, move |s| f(MsgCaixa::Escala(s))).width(Length::FILL).into()),
                    ]
                    .spacing(10)
                    .into()
                };
                let tem = !folhas.folhas.is_empty();
                Dialogo::new(t("folha.escala.titulo"), corpo)
                    .secundario(t("folha.escala.fechar"), Some(fechar.clone()))
                    .botao(dialogo::botao_secundario(t("folha.escala.ajustar"), tem.then(|| f(MsgCaixa::Ajustar))))
                    .primario(t("folha.escala.aplicar"), tem.then(|| f(MsgCaixa::Aplicar)))
            }
            Caixa::Carimbo { folhas, rotulos, indice, campos } => {
                let corpo: Element<'a, M> = if folhas.folhas.is_empty() {
                    text(t("folha.escala.semFolhas")).size(12).into()
                } else {
                    column![
                        text(t("folha.carimbo.corpo")).size(12),
                        campo(t("folha.escala.qual"), pick_list(rotulos, rotulos.get(*indice).cloned(), move |s| f(MsgCaixa::Folha(s))).width(Length::FILL).into()),
                        campos_do_carimbo(campos, f),
                    ]
                    .spacing(10)
                    .into()
                };
                Dialogo::new(t("folha.carimbo.titulo"), corpo)
                    .largura(620.0)
                    .secundario(t("folha.carimbo.cancelar"), Some(fechar.clone()))
                    .primario(t("folha.carimbo.salvar"), (!folhas.folhas.is_empty()).then(|| f(MsgCaixa::Aplicar)))
            }
            Caixa::Exportar { paginas, papel, papeis, orientacao, orientacoes, cores, escolhidas } => {
                let ori = ORIENTACOES.iter().position(|o| o == orientacao).map(|i| orientacoes[i].clone());
                let mut corpo = column![
                    text(t("exportar.corpo")).size(12),
                    row![
                        campo(t("exportar.papel"), pick_list(papeis, Some(papel.clone()), move |s| f(MsgCaixa::Papel(s))).width(Length::FILL).into()),
                        campo(t("exportar.orientacao"), pick_list(orientacoes, ori, move |s| f(MsgCaixa::Orientacao(s))).width(Length::FILL).into()),
                    ]
                    .spacing(10),
                ]
                .spacing(10);
                if *paginas == 0 {
                    corpo = corpo.push(text(t("exportar.semFolhas")).size(12).color(COR_TEXTO_MUTED()));
                } else {
                    let todas = row![
                        text(t("exportar.folhas")).size(12).font(Font::SEMIBOLD),
                        button(text(t("exportar.todas")).size(11)).style(estilo_botao_pequeno(false)).on_press(f(MsgCaixa::Todas(true))),
                        button(text(t("exportar.nenhuma")).size(11)).style(estilo_botao_pequeno(false)).on_press(f(MsgCaixa::Todas(false))),
                    ]
                    .spacing(8)
                    .align(Align::Center);
                    let lista = row(escolhidas.iter().enumerate().map(|(i, e)| checkbox(tv("exportar.folha", &[("n", &(i + 1).to_string())]), *e).on_toggle(move |v| f(MsgCaixa::Pagina(i, v))).into())).spacing(12);
                    corpo = corpo.push(todas).push(pelicano::scrollable(lista).direction(pelicano::Direction::Horizontal).height(34));
                }
                corpo = corpo.push(checkbox(t("exportar.cores"), *cores).on_toggle(move |v| f(MsgCaixa::Cores(v))));
                let alguma = *paginas == 0 || escolhidas.iter().any(|e| *e);
                Dialogo::new(t("exportar.titulo"), corpo).largura(560.0).secundario(t("exportar.cancelar"), Some(fechar.clone())).primario(t("exportar.exportar"), alguma.then(|| f(MsgCaixa::Aplicar)))
            }
            _ => Dialogo::new(String::new(), text("")),
        }
    }
}

fn campo<'a, M: 'a>(rotulo: String, controle: Element<'a, M>) -> Element<'a, M> {
    column![text(rotulo).size(11).color(COR_TEXTO_MUTED()), controle].spacing(3).width(Length::FILL).into()
}

fn campos_do_carimbo<'a, M: Clone + 'static>(campos: &'a Value, f: impl Fn(MsgCaixa) -> M + Copy + 'a) -> Element<'a, M> {
    let caixa = |nome: &'static str, chave: &str| -> Element<'a, M> {
        let valor = campos[nome].as_str().unwrap_or("");
        campo(t(&format!("folha.inserir.{chave}")), text_input("", valor).size(12).on_input(move |s| f(MsgCaixa::Campo(nome, s))).into())
    };
    let pares = CAMPOS.chunks(2).map(|par| row(par.iter().map(|(n, c)| caixa(n, c))).spacing(10).into());
    let divisoria = checkbox(t("folha.inserir.divisoria"), campos["divisoria"].as_bool().unwrap_or(false)).on_toggle(move |v| f(MsgCaixa::Divisoria(v)));
    column(pares).push(divisoria).spacing(8).into()
}

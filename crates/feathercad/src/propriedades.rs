//! A paleta de propriedades dos objetos selecionados (Ctrl+1; o antigo `propriedades-painel.ts`): o que eles têm em comum
//! ou *VARIADO*, e mudar de uma vez. O resumo e a validação são do motor (`propriedades_selecao`/`propriedades_definir`).

use std::collections::BTreeMap;

use feather_ui::seletor_cor;
use feather_ui::tema::*;
use pelicano::{button, column, container, pick_list, row, scrollable, text, text_input, Align, Border, Color, Element, Font, Length, Style};
use serde::Deserialize;
use serde_json::json;

use crate::camadas::{ESPESSURAS, TIPOS};
use crate::doc::Doc;
use crate::idioma::{t, tv};

const POR_CAMADA: &str = "porcamada";
const NOMES_DAS_CORES: [&str; 7] = ["vermelho", "amarelo", "verde", "ciano", "azul", "magenta", "branco"];
/// Os campos de texto editáveis: (nome no motor, chave do rótulo).
const GEOMETRIA: [(&str, &str); 9] = [
    ("posicao_x", "posicaoX"),
    ("posicao_y", "posicaoY"),
    ("comprimento", "comprimento"),
    ("raio", "raio"),
    ("diametro", "diametro"),
    ("area", "area"),
    ("tamanho_geral", "tamanhoGeral"),
    ("rotacao", "rotacao"),
    ("conteudo", "conteudo"),
];

/// Um campo do resumo: sem objetos, o mesmo em todos, ou variado.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(tag = "estado", content = "valor", rename_all = "snake_case")]
pub enum Campo {
    #[default]
    Vazio,
    Igual(String),
    Variado,
}

impl Campo {
    fn texto(&self) -> String {
        match self {
            Campo::Igual(v) => v.clone(),
            Campo::Variado => t("propriedades.variado"),
            Campo::Vazio => String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Resumo {
    pub n: usize,
    pub camada: Campo,
    pub cor: Campo,
    pub cor_efetiva: Campo,
    pub tipo_linha: Campo,
    pub espessura: Campo,
    pub cores_basicas: Vec<String>,
    #[serde(flatten)]
    pub resto: BTreeMap<String, serde_json::Value>,
}

impl Resumo {
    fn campo(&self, nome: &str) -> Campo {
        self.resto.get(nome).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum MsgProps {
    Definir(&'static str, String),
    Editar(&'static str, String),
    Confirmar(&'static str),
    AbrirCor,
    Cor(seletor_cor::Evento),
    Fechar,
}

#[derive(Default)]
pub struct Propriedades {
    pub resumo: Resumo,
    rev: Option<(u64, u64)>,
    editando: BTreeMap<&'static str, String>,
    camadas: Vec<String>,
    cores: Vec<String>,
    tipos: Vec<String>,
    espessuras: Vec<String>,
    cor: Option<([f32; 4], String)>,
    erro: Option<String>,
}

impl Propriedades {
    /// Sem desenho ainda (as listas prontas, o resumo vazio).
    pub fn vazia() -> Propriedades {
        Propriedades {
            tipos: std::iter::once(t("propriedades.porCamada")).chain(TIPOS.iter().map(|(_, d)| t(&format!("camadas.tipos.{d}")))).collect(),
            espessuras: std::iter::once(t("propriedades.porCamada")).chain(ESPESSURAS.iter().map(|e| format!("{e:.2} mm"))).collect(),
            ..Propriedades::default()
        }
    }

    pub fn abrir(doc: &mut Doc) -> Propriedades {
        let mut p = Propriedades::vazia();
        p.recarregar(doc);
        p
    }

    /// Relê o resumo se a seleção ou o desenho mudaram.
    pub fn recarregar(&mut self, doc: &mut Doc) {
        let rev = (doc.estado.versao, doc.estado.selecao_rev);
        if self.rev == Some(rev) {
            return;
        }
        self.rev = Some(rev);
        self.editando.clear();
        if let Ok(r) = doc.motor.json::<Resumo>("propriedades_selecao", serde_json::Value::Null) {
            self.resumo = r;
        }
        if let Ok(tab) = doc.motor.json::<crate::camadas::Tabela>("camadas", json!({})) {
            self.camadas = tab.linhas.into_iter().map(|l| l.nome).collect();
        }
        self.cores = std::iter::once(t("propriedades.porCamada"))
            .chain(NOMES_DAS_CORES.iter().map(|c| t(&format!("propriedades.cores.{c}"))))
            .chain(std::iter::once(t("propriedades.cores.personalizada")))
            .collect();
    }

    pub fn update(&mut self, msg: MsgProps, doc: &mut Doc) -> bool {
        match msg {
            MsgProps::Fechar => return true,
            MsgProps::Editar(campo, v) => {
                self.editando.insert(campo, v);
            }
            MsgProps::Confirmar(campo) => {
                if let Some(v) = self.editando.remove(campo) {
                    self.definir(doc, campo, v);
                }
            }
            MsgProps::Definir(campo, v) => self.definir(doc, campo, v),
            MsgProps::AbrirCor => {
                let atual = match &self.resumo.cor_efetiva {
                    Campo::Igual(c) => c.clone(),
                    _ => "#ffffff".into(),
                };
                self.cor = Some((seletor_cor::rgba_para_hsva(seletor_cor::rgba_de(&atual), 0.0), atual));
            }
            MsgProps::Cor(ev) => {
                let fechar = matches!(ev, seletor_cor::Evento::Fechar);
                if let Some((mut hsva, mut hex)) = self.cor.take() {
                    if let Some(nova) = seletor_cor::aplicar(ev, &mut hsva, &mut hex).filter(|c| c.len() >= 7) {
                        self.definir(doc, "cor", nova[..7].to_string());
                    }
                    if !fechar {
                        self.cor = Some((hsva, hex));
                    }
                }
            }
        }
        false
    }

    fn definir(&mut self, doc: &mut Doc, campo: &str, valor: String) {
        match doc.motor.estado_de("propriedades_definir", json!({ "campo": campo, "valor": valor })) {
            Ok(e) => {
                self.erro = None;
                doc.aplicar(e);
                self.rev = None;
                self.recarregar(doc);
            }
            // O motor devolve a chave da mensagem (`propriedades.corInvalida`…).
            Err(e) => self.erro = Some(t(&e)),
        }
    }

    pub fn view<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgProps) -> M + Copy + 'a) -> Element<'a, M> {
        let r = &self.resumo;
        let titulo = row![text(t("propriedades.titulo")).size(13).font(Font::SEMIBOLD), pelicano::horizontal_space(), feather_ui::dialogo::botao_fechar(f(MsgProps::Fechar))].align(Align::Center);
        let n = if r.n == 0 { t("propriedades.nenhum") } else { tv("propriedades.objetos", &[("n", &r.n.to_string())]) };
        let mut c = column![titulo, text(n).size(12).color(COR_TEXTO_MUTED())].spacing(6);
        if r.n > 0 {
            c = c.push(secao(t("propriedades.geral")));
            c = c.push(linha(t("propriedades.cor"), self.controle_cor(f)));
            let camada = match &r.camada {
                Campo::Igual(n) => Some(n.clone()),
                _ => None,
            };
            c = c.push(linha(t("propriedades.camada"), pick_list(&self.camadas, camada, move |s| f(MsgProps::Definir("camada", s))).placeholder(r.camada.texto()).size(12).width(Length::FILL).into()));
            c = c.push(linha(t("propriedades.tipoLinha"), self.controle_tipo(f)));
            c = c.push(linha(t("propriedades.espessura"), self.controle_espessura(f)));
            c = c.push(secao(t("propriedades.geometria")));
            for (campo, chave) in GEOMETRIA {
                let valor = r.campo(campo);
                if valor == Campo::Vazio {
                    continue;
                }
                if campo == "conteudo" {
                    c = c.push(secao(t("propriedades.texto")));
                }
                let mostrado = self.editando.get(campo).cloned().unwrap_or_else(|| valor.texto());
                let campo_txt = text_input("", mostrado).size(12).on_input(move |s| f(MsgProps::Editar(campo, s))).on_submit(f(MsgProps::Confirmar(campo)));
                c = c.push(linha(t(&format!("propriedades.{chave}")), campo_txt.into()));
            }
        }
        if let Some(e) = &self.erro {
            c = c.push(text(e.clone()).size(11).color(COR_ERRO()));
        }
        if let Some((hsva, hex)) = &self.cor {
            c = c.push(seletor_cor::painel_embutido("Cor", hex, *hsva, hex).map(move |e| f(MsgProps::Cor(e))));
        }
        container(scrollable(c.padding(10)))
            .width(300)
            .height(Length::FILL)
            .style(|_| Style::default().background(COR_PAINEL()).border(Border::default().color(COR_BORDA(), 1.0)))
            .into()
    }

    pub fn controle_tipo<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgProps) -> M + Copy + 'a) -> Element<'a, M> {
        let r = &self.resumo;
        let tipo = match &r.tipo_linha {
            Campo::Igual(v) if v == POR_CAMADA => Some(self.tipos[0].clone()),
            Campo::Igual(v) => TIPOS.iter().position(|(_, d)| d.eq_ignore_ascii_case(v)).map(|i| self.tipos[i + 1].clone()),
            _ => None,
        };
        let tipos = &self.tipos;
        pick_list(tipos, tipo, move |s| {
            let v = tipos.iter().position(|x| *x == s).map_or(POR_CAMADA.to_string(), |i| if i == 0 { POR_CAMADA.into() } else { TIPOS[i - 1].1.into() });
            f(MsgProps::Definir("tipo_linha", v))
        })
        .placeholder(r.tipo_linha.texto())
        .size(12)
        .width(Length::FILL)
        .into()
    }

    pub fn controle_espessura<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgProps) -> M + Copy + 'a) -> Element<'a, M> {
        let r = &self.resumo;
        let esp = match &r.espessura {
            Campo::Igual(v) if v == POR_CAMADA => Some(self.espessuras[0].clone()),
            Campo::Igual(v) => v.parse::<f32>().ok().map(|e| format!("{e:.2} mm")),
            _ => None,
        };
        pick_list(&self.espessuras, esp, move |s: String| {
            let v = s.strip_suffix(" mm").map_or(POR_CAMADA.to_string(), |n| n.to_string());
            f(MsgProps::Definir("espessura", v))
        })
        .placeholder(r.espessura.texto())
        .size(12)
        .width(Length::FILL)
        .into()
    }

    pub fn controle_cor<'a, M: Clone + 'static>(&'a self, f: impl Fn(MsgProps) -> M + Copy + 'a) -> Element<'a, M> {
        let r = &self.resumo;
        let basicas = &r.cores_basicas;
        let cores = &self.cores;
        let atual = match &r.cor {
            Campo::Igual(v) if v == POR_CAMADA => Some(cores[0].clone()),
            Campo::Igual(v) => basicas.iter().position(|b| b.eq_ignore_ascii_case(v)).map(|i| cores[i + 1].clone()),
            _ => None,
        };
        let lista = pick_list(cores, atual, move |s| {
            let i = cores.iter().position(|x| *x == s).unwrap_or(0);
            match i {
                0 => f(MsgProps::Definir("cor", POR_CAMADA.into())),
                i if i <= basicas.len() => f(MsgProps::Definir("cor", basicas[i - 1].clone())),
                _ => f(MsgProps::AbrirCor),
            }
        })
        .placeholder(r.cor.texto())
        .size(12)
        .width(Length::FILL);
        let efetiva = match &r.cor_efetiva {
            Campo::Igual(c) => Color::hex(c).unwrap_or(Color::TRANSPARENT),
            _ => Color::TRANSPARENT,
        };
        let amostra = button(container(text("")).width(16).height(16).style(move |_| Style::default().background(efetiva).border(Border::default().color(COR_BORDA(), 1.0))))
            .padding(2)
            .style(estilo_botao_pequeno(false))
            .on_press(f(MsgProps::AbrirCor));
        row![amostra, lista].spacing(4).align(Align::Center).into()
    }
}

fn secao<'a, M: 'a>(titulo: String) -> Element<'a, M> {
    container(text(titulo).size(12).font(Font::SEMIBOLD)).padding([6u16, 0]).into()
}

fn linha<'a, M: 'a>(rotulo: String, controle: Element<'a, M>) -> Element<'a, M> {
    row![text(rotulo).size(12).width(110).color(COR_TEXTO_MUTED()), controle].spacing(6).align(Align::Center).into()
}

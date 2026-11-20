//! A faixa (Ribbon) como **dado**: abas → painéis → botões (o antigo `src/ui/ribbon-dados.ts`). Aba nova entra aqui sem
//! tocar nas outras. `comando` = nome interno no motor; `acao` = coisa da interface; nenhum dos dois = "em breve"
//! (desabilitado). Textos: `ribbon.abas.<id>`, `ribbon.paineis.<id>`, `ribbon.botoes.<id>.rotulo/dica`.

use feather_ui::tema::{estilo_aba_faixa, estilo_botao_grande, estilo_botao_pequeno, estilo_dica, COR_BORDA, COR_PAINEL_FAIXA, COR_TEXTO_MUTED};
use pelicano::{button, column, container, row, text, tooltip, vertical_rule, Align, Element, Length, Style, TooltipPosition};

use crate::idioma::t;

pub struct Botao {
    pub id: &'static str,
    pub comando: Option<&'static str>,
    pub simbolo: &'static str,
    pub pequeno: bool,
    pub acao: Option<&'static str>,
    /// Não é botão: é um controle da faixa (camada atual, cor…), que o app fabrica.
    pub widget: Option<&'static str>,
}

pub struct Painel {
    pub id: &'static str,
    pub botoes: &'static [Botao],
    /// A ação do "lançador" (↘) no canto do painel.
    pub lancador: Option<&'static str>,
}

pub struct Aba {
    pub id: &'static str,
    pub paineis: &'static [Painel],
}

const fn g(id: &'static str, comando: &'static str, simbolo: &'static str) -> Botao {
    Botao { id, comando: Some(comando), simbolo, pequeno: false, acao: None, widget: None }
}
const fn p(id: &'static str, comando: &'static str, simbolo: &'static str) -> Botao {
    Botao { id, comando: Some(comando), simbolo, pequeno: true, acao: None, widget: None }
}
/// Ação da interface, botão grande (`pequeno` = falso) ou pequeno.
const fn a(id: &'static str, acao: &'static str, simbolo: &'static str, pequeno: bool) -> Botao {
    Botao { id, comando: None, simbolo, pequeno, acao: Some(acao), widget: None }
}
/// Ainda não existe (desabilitado, com a dica "em breve").
const fn e(id: &'static str, simbolo: &'static str, pequeno: bool) -> Botao {
    Botao { id, comando: None, simbolo, pequeno, acao: None, widget: None }
}
const fn w(id: &'static str) -> Botao {
    Botao { id, comando: None, simbolo: "", pequeno: true, acao: None, widget: Some(id) }
}
const fn painel(id: &'static str, botoes: &'static [Botao]) -> Painel {
    Painel { id, botoes, lancador: None }
}

pub const ABAS: &[Aba] = &[
    Aba {
        id: "inicio",
        paineis: &[
            painel(
                "desenhar",
                &[
                    g("linha", "linha", "╱"),
                    g("polilinha", "polilinha", "⌇"),
                    g("circulo", "circulo", "◯"),
                    g("arco", "arco", "◠"),
                    p("retangulo", "retangulo", "▭"),
                    p("elipse", "elipse", "⬭"),
                    p("hachura", "hachura", "▨"),
                    a("hachura_padrao", "hachura_padrao", "▩", true),
                    p("3dface", "3dface", "⬡"),
                    p("caixa3d", "caixa3d", "⬚"),
                    p("elevacao", "elevacao", "⇡"),
                ],
            ),
            painel(
                "modificar",
                &[
                    p("mover", "mover", "✥"),
                    p("copiar", "copiar", "⧉"),
                    p("esticar", "esticar", "⇔"),
                    p("girar", "girar", "↻"),
                    p("espelhar", "espelhar", "⇋"),
                    p("escala", "escala", "⤡"),
                    p("aparar", "aparar", "✂"),
                    p("deslocar", "deslocar", "⫽"),
                    p("estender", "estender", "⇥"),
                    p("concordancia", "concordancia", "⌒"),
                    p("matriz", "matriz", "⁙"),
                    p("apagar", "apagar", "✕"),
                ],
            ),
            painel(
                "anotacao",
                &[
                    g("texto", "texto", "A"),
                    g("cota", "cota", "⟷"),
                    p("cota_alinhada", "cota_alinhada", "⤢"),
                    p("cota_raio", "cota_raio", "R"),
                    p("cota_diametro", "cota_diametro", "Ø"),
                    p("chamada", "chamada", "↗"),
                    p("tabela", "tabela", "▦"),
                ],
            ),
            painel("camadas", &[a("propriedades_camada", "camadas", "≣", false), w("camadaAtual"), p("tornar_atual", "tornar_atual", "✔"), p("igualar_camada", "igualar_camada", "⇉")]),
            painel("bloco", &[g("inserir", "inserir", "⧉"), p("criar_bloco", "criar_bloco", "⊞"), e("editar_bloco", "✎", true)]),
            Painel { id: "propriedades", botoes: &[g("igualar_propriedades", "igualar_propriedades", "⇶"), w("propCor"), w("propEspessura"), w("propTipo")], lancador: Some("propriedades") },
            painel("grupos", &[g("agrupar", "agrupar", "▣")]),
            painel("utilitarios", &[g("medir", "medir", "📏"), g("contar", "contar", "#")]),
            painel("area_transferencia", &[g("colar", "colar", "📋"), p("copiar_area", "copiar_area", "⎘"), p("recortar", "recortar", "✂")]),
        ],
    },
    Aba { id: "inserir", paineis: &[painel("folha", &[a("folha_inserir", "folha_inserir", "▭", false), a("folha_escala", "folha_escala", "⤡", false), a("folha_carimbo", "folha_carimbo", "✎", false)])] },
    Aba {
        id: "anotar",
        paineis: &[
            painel("texto", &[g("anot_texto", "texto", "A")]),
            painel("cotas", &[g("anot_cota", "cota", "⟷"), p("anot_cota_alinhada", "cota_alinhada", "⤢"), p("anot_cota_raio", "cota_raio", "R"), p("anot_cota_diametro", "cota_diametro", "Ø")]),
            painel("chamada", &[g("anot_chamada", "chamada", "↗")]),
            painel("tabela", &[g("anot_tabela", "tabela", "▦")]),
            painel("marcacao", &[g("anot_medir", "medir", "📏"), g("anot_contar", "contar", "#")]),
        ],
    },
    Aba {
        id: "parametrico",
        paineis: &[
            painel(
                "geometria",
                &[
                    e("param_coincidente", "⊕", false),
                    e("param_colinear", "⫽", true),
                    e("param_concentrico", "◎", true),
                    e("param_paralelo", "∥", true),
                    e("param_perpendicular", "⟂", true),
                    e("param_tangente", "⟒", true),
                    e("param_horizontal", "⬌", true),
                    e("param_vertical", "⬍", true),
                ],
            ),
            painel("dimensional", &[e("param_linear", "⟷", false), e("param_raio", "R", true), e("param_angulo", "∠", true)]),
        ],
    },
    Aba {
        id: "modelo3d",
        paineis: &[
            painel(
                "primitivas3d",
                &[
                    g("m3d_caixa3d", "caixa3d", "⬚"),
                    p("m3d_cilindro3d", "cilindro3d", "◯"),
                    p("m3d_esfera3d", "esfera3d", "●"),
                    p("m3d_cone3d", "cone3d", "▲"),
                    p("m3d_toro3d", "toro3d", "◎"),
                    p("m3d_cunha3d", "cunha3d", "⊿"),
                ],
            ),
            painel(
                "operacoes3d",
                &[g("m3d_extrusao", "extrusao", "⇡"), p("m3d_revolucao", "revolucao", "⟳"), p("m3d_varredura", "varredura", "⌇"), p("m3d_loft", "loft", "⤯"), p("m3d_presspull", "presspull", "⇕")],
            ),
            painel("booleanas3d", &[g("m3d_uniao3d", "uniao3d", "⋃"), p("m3d_subtracao3d", "subtracao3d", "⋂"), p("m3d_intersecao3d", "intersecao3d", "⨅")]),
            painel("gizmos3d", &[g("m3d_mover3d", "mover3d", "✥"), p("m3d_rotacionar3d", "rotacionar3d", "↻"), p("m3d_escalar3d", "escalar3d", "⤡")]),
        ],
    },
    Aba {
        id: "exibir",
        paineis: &[
            painel("vistas", &[a("exib_enquadrar", "vista_ajustar", "⛶", false), a("exib_zoom_mais", "zoom_mais", "+", true), a("exib_zoom_menos", "zoom_menos", "−", true)]),
            painel("interface", &[a("exib_propriedades", "propriedades", "☰", false), a("exib_camadas", "camadas", "≣", true), a("exib_monitor", "monitor", "📈", true)]),
            painel("tema", &[a("exib_tema", "alternar_tema", "◑", false)]),
        ],
    },
    Aba {
        id: "gerenciar",
        paineis: &[
            painel("personalizacao", &[a("geren_configuracoes", "configuracoes", "⚙", false), a("geren_mcp", "mcp", "🤖", true)]),
            painel("desenho_geren", &[a("geren_unidades", "unidades", "📐", false), e("geren_limpar", "🧹", true)]),
        ],
    },
    Aba {
        id: "saida",
        paineis: &[
            painel("exportar", &[a("saida_pdf", "exportar_pdf", "📄", false), a("saida_dxf", "salvar_dxf", "💾", true), a("saida_fcdz", "salvar_fcdz", "💾", true)]),
            painel("impressao", &[a("saida_folhas", "folha_inserir", "▭", false)]),
        ],
    },
    Aba {
        id: "colaborar",
        paineis: &[
            painel("nuvem", &[a("colab_mcp", "mcp", "🤖", false), e("colab_ia", "✨", true)]),
            painel("compartilhar", &[a("colab_exportar", "exportar_pdf", "📄", false), a("colab_sobre", "sobre", "ℹ", true)]),
        ],
    },
];

/// As mensagens que a faixa manda (o app as traduz para as suas).
#[derive(Debug, Clone)]
pub enum Pedido {
    Aba(usize),
    Comando(&'static str),
    Acao(&'static str),
}

fn botao<'a, M: Clone + 'a>(b: &'static Botao, ativo: bool, f: &impl Fn(Pedido) -> M) -> Element<'a, M> {
    let rotulo = t(&format!("ribbon.botoes.{}.rotulo", b.id));
    let dica = match (b.comando, b.acao) {
        (None, None) => t("ribbon.embreve"),
        _ => t(&format!("ribbon.botoes.{}.dica", b.id)),
    };
    let msg = b.comando.map(Pedido::Comando).or(b.acao.map(Pedido::Acao)).map(f);
    let corpo: Element<'a, M> = if b.pequeno {
        row![crate::icones::icone(b.id, 16.0, b.simbolo), text(rotulo).size(11).no_wrap()].spacing(5).align(Align::Center).into()
    } else {
        column![crate::icones::icone(b.id, 24.0, b.simbolo), text(rotulo).size(11).no_wrap()].spacing(3).align(Align::Center).into()
    };
    let controle = button(corpo).padding(if b.pequeno { [2u16, 6] } else { [4u16, 8] }).on_press_maybe(msg);
    let controle = if b.pequeno { controle.style(estilo_botao_pequeno(ativo)) } else { controle.style(estilo_botao_grande(ativo)) };
    tooltip(controle, dica, TooltipPosition::Bottom).padding([4, 6]).style(estilo_dica).into()
}

fn painel_view<'a, M: Clone + 'a>(p: &'static Painel, ferramenta: &str, f: &impl Fn(Pedido) -> M, widget: &impl Fn(&'static str) -> Element<'a, M>) -> Element<'a, M> {
    let ativo = |b: &Botao| b.comando == Some(ferramenta);
    let mut itens = row![].spacing(2).align(Align::Center);
    let mut pilha = column![].spacing(1);
    let mut na_pilha = 0;
    for b in p.botoes {
        let el = match b.widget {
            Some(id) => widget(id),
            None => botao(b, ativo(b), f),
        };
        if b.pequeno {
            pilha = pilha.push(el);
            na_pilha += 1;
            if na_pilha == 3 {
                itens = itens.push(std::mem::replace(&mut pilha, column![].spacing(1)));
                na_pilha = 0;
            }
        } else {
            itens = itens.push(el);
        }
    }
    if na_pilha > 0 {
        itens = itens.push(pilha);
    }
    let mut rotulo = row![text(t(&format!("ribbon.paineis.{}", p.id))).size(11).no_wrap().color(COR_TEXTO_MUTED())].spacing(4).align(Align::Center);
    if let Some(l) = p.lancador {
        rotulo = rotulo.push(button(text("↘").size(10)).padding([0u16, 3]).style(estilo_botao_pequeno(false)).on_press(f(Pedido::Acao(l))));
    }
    row![
        container(column![container(itens).height(Length::FILL).align_y(Align::Center), rotulo].align(Align::Center).spacing(2)).padding([4u16, 10]),
        vertical_rule(1.0).color(COR_BORDA()),
    ]
    .into()
}

/// As abas e os painéis da aba escolhida. `ferramenta` = o comando em andamento (o botão dele fica aceso); `widget`
/// fabrica os controles que não são botão (camada atual, cor…).
pub fn faixa<'a, M: Clone + 'a>(aba: usize, ferramenta: &str, f: impl Fn(Pedido) -> M, widget: impl Fn(&'static str) -> Element<'a, M>) -> Element<'a, M> {
    let abas = row(ABAS.iter().enumerate().map(|(i, a)| {
        button(text(t(&format!("ribbon.abas.{}", a.id))).size(12)).padding([5u16, 12]).style(estilo_aba_faixa(i == aba)).on_press(f(Pedido::Aba(i))).into()
    }))
    .spacing(2);
    let aba_dados = ABAS.get(aba);
    let paineis: Element<'a, M> = match aba_dados.map(|a| a.paineis) {
        Some(ps) if !ps.is_empty() => row(ps.iter().map(|p| painel_view(p, ferramenta, &f, &widget))).height(92).into(),
        _ => container(text(t("ribbon.embreve")).size(12).color(COR_TEXTO_MUTED())).padding(12).height(92).into(),
    };
    column![
        container(abas).padding([2u16, 8]),
        container(pelicano::scrollable(paineis).direction(pelicano::Direction::Horizontal).height(100).scrollbar_width(4)).width(Length::FILL).style(|_| Style::default().background(COR_PAINEL_FAIXA())),
    ]
    .into()
}

#[cfg(test)]
mod testes {
    #[test]
    fn todo_comando_da_faixa_existe_no_motor_e_tem_texto() {
        for aba in super::ABAS {
            assert_ne!(crate::idioma::t(&format!("ribbon.abas.{}", aba.id)), format!("ribbon.abas.{}", aba.id));
            for p in aba.paineis {
                for b in p.botoes {
                    if let Some(c) = b.comando {
                        assert!(feathercad_core::comandos::CONHECIDOS.contains(&c), "o motor não tem `{c}`");
                    }
                    if b.widget.is_none() {
                        let chave = format!("ribbon.botoes.{}.rotulo", b.id);
                        assert_ne!(crate::idioma::t(&chave), chave, "falta o texto de `{}`", b.id);
                    }
                }
            }
        }
    }
}

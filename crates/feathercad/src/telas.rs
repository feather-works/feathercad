//! O que se vê: barra de título, abas de desenho, faixa, área (com o cubo de vista e a barra de navegação), paleta de
//! propriedades, linha de comando, rodapé, avisos e as janelas por cima. Nada aqui muda estado: só mensagens.

use feather_ui::cromo::{self, AcoesJanela};
use feather_ui::tema::*;
use pelicano::{
    button, column, container, context_menu, horizontal_space, menu_item, menu_separator, pick_list, program, row, stack, text, text_input,
    tooltip, Align, Border, Element, Font, Length, MenuItem, Point, Space, Style, TooltipPosition, WindowCmd, WindowId,
};

use crate::app::{interruptor_ligado, App, Msg, IA_ATIVA_POR, ID_COMANDO};
use crate::area::{Area, Cores};
use crate::doc::{Doc, FATOR_DO_ZOOM};
use crate::idioma::{t, texto_com, tv};
use crate::motor::{Entrada, ItemDeMenu};

const TIPOS_DE_SNAP: [&str; 9] = ["extremidade", "meio", "centro", "quadrante", "intersecao", "perpendicular", "tangente", "no", "proximo"];

fn botao_barra<'a>(id: &str, rotulo: &str, dica: String, msg: Option<Msg>) -> Element<'a, Msg> {
    let b = button(crate::icones::icone(id, 17.0, rotulo)).padding([4u16, 8]).style(estilo_botao_pequeno(false)).on_press_maybe(msg);
    tooltip(b, dica, TooltipPosition::Bottom).padding([4, 6]).style(estilo_dica).into()
}

impl App {
    pub fn view_principal(&self) -> Element<'_, Msg> {
        let corpo: Element<'_, Msg> = match self.doc() {
            Some(d) => self.view_desenho(d),
            None => self.inicio.view(&self.config.visao_inicio, Msg::Pagina, Msg::PedirNovo, Msg::Abrir(None), None),
        };
        let tudo = column![self.view_barra(), self.view_abas(), corpo].height(Length::FILL);
        let mut janela = cromo::moldura_redimensionavel_com(tudo.into(), self.maximizada, Msg::Puxar);
        if !self.avisos.is_empty() {
            janela = stack![janela, self.view_avisos()].into();
        }
        let altura = self.tamanho_janela.height;
        if let Some(c) = &self.configuracoes {
            let estado = self.doc().map_or(&self.vazio.estado, |d| &d.estado);
            return c.view(&self.config, estado, self.doc().is_some(), Msg::Config, janela, altura);
        }
        if let Some(c) = &self.caixa {
            return c.view(Msg::Caixa, janela, altura);
        }
        if let Some(c) = &self.camadas {
            return c.view(Msg::Camadas, janela, altura);
        }
        if let Some((canto, itens)) = &self.menu {
            return stack![janela, context_menu(itens_do_menu(itens), *canto, Msg::FecharMenu)].into();
        }
        if self.snap_modos_aberto {
            if let Some(d) = self.doc() {
                let itens = TIPOS_DE_SNAP
                    .iter()
                    .map(|tipo| {
                        let ligado = d.estado.osnap_modos.get(*tipo).copied().unwrap_or(false);
                        menu_item(t(&format!("rodape.snapTipos.{tipo}")), Msg::SnapModo(tipo.to_string(), !ligado)).checked(ligado)
                    })
                    .collect::<Vec<_>>();
                let itens = std::iter::once(menu_separator(t("rodape.osnapModos"))).chain(itens).collect();
                return stack![janela, context_menu(itens, Point::new(560.0, self.tamanho_janela.height - 340.0), Msg::FecharMenu)].into();
            }
        }
        janela
    }

    fn view_barra(&self) -> Element<'_, Msg> {
        let nome = match self.doc() {
            Some(d) => format!("{}{} — {}", d.nome(), if d.estado.sujo { " •" } else { "" }, t("app.nome")),
            None => t("app.nome"),
        };
        let d = self.doc();
        let em_desenho = d.is_some();
        let mut barra = row![
            cromo::emblema(),
            text(nome).size(13).font(Font::SEMIBOLD),
            Space::new(12, 0),
            botao_barra("novo", "＋", t("titulo.novo"), Some(Msg::PedirNovo)),
            botao_barra("abrir", "📂", t("titulo.abrir"), Some(Msg::Abrir(None))),
            botao_barra("salvar", "💾", t("titulo.salvar"), em_desenho.then_some(Msg::Salvar { como: false, formato: None })),
            botao_barra("exportar", "📄", t("titulo.exportar"), em_desenho.then_some(Msg::Faixa(crate::faixa::Pedido::Acao("exportar_pdf")))),
            botao_barra("desfazer", "↶", t("titulo.desfazer"), d.filter(|d| d.estado.pode_desfazer).map(|_| Msg::Simples("desfazer"))),
            botao_barra("refazer", "↷", t("titulo.refazer"), d.filter(|d| d.estado.pode_refazer).map(|_| Msg::Simples("refazer"))),
            horizontal_space(),
        ]
        .spacing(6)
        .align(Align::Center);
        if self.ia_em.is_some_and(|i| i.elapsed() < IA_ATIVA_POR) {
            barra = barra.push(text(format!("✨ {}", t("titulo.iaTrabalhando"))).size(12).color(COR_AZUL_ACENTO()));
        }
        let modo = if self.config.tema == "auto" { "auto" } else if self.claro { "light" } else { "dark" };
        barra = barra
            .push(botao_barra("tema", "◑", tv("tema.dica", &[("modo", &t(&format!("tema.modos.{modo}")))]), Some(Msg::Tema)))
            .push(botao_barra("configuracoes", "⚙", t("config.abrir"), Some(Msg::Config(crate::configuracoes::MsgConfig::Secao("aparencia")))));
        let barra = container(barra).padding([0, 14]).align_y(Align::Center).width(Length::FILL).height(cromo::BARRA_ALTURA).style(estilo_container_topo);
        cromo::barra_integrada(
            barra.into(),
            AcoesJanela { arrastar: Msg::Janela(WindowCmd::Drag), minimizar: Msg::Janela(WindowCmd::Minimize), maximizar: Msg::Janela(WindowCmd::ToggleMaximize), fechar: Msg::Fechar },
        )
    }

    /// As abas de desenho (o antigo `abas-arquivo.ts`): Início, um por desenho (com • se tem alterações e ✕), e ＋.
    fn view_abas(&self) -> Element<'_, Msg> {
        let inicio = button(text(format!("⌂ {}", t("abas.inicio"))).size(12)).padding([5u16, 12]).style(estilo_aba_faixa(self.ativa.is_none())).on_press(Msg::Inicio);
        let abas = self.docs.iter().enumerate().map(|(i, d)| {
            let rotulo = format!("{}{}", d.nome(), if d.estado.sujo { " •" } else { "" });
            let fechar = button(text("✕").size(10)).padding([1u16, 4]).style(estilo_botao_pequeno(false)).on_press(Msg::FecharAba(i));
            let corpo = row![text(rotulo).size(12), tooltip(fechar, t("abas.fechar"), TooltipPosition::Bottom)].spacing(6).align(Align::Center);
            let aba = button(corpo).padding([3u16, 10]).style(estilo_aba_faixa(self.ativa == Some(i))).on_press(Msg::Aba(i));
            match &d.estado.arquivo {
                Some(a) => tooltip(aba, a.caminho.clone(), TooltipPosition::Bottom).padding([4, 6]).style(estilo_dica).into(),
                None => aba.into(),
            }
        });
        let novo = tooltip(button(text("＋").size(12)).padding([3u16, 10]).style(estilo_botao_pequeno(false)).on_press(Msg::PedirNovo), t("abas.novo"), TooltipPosition::Bottom);
        let linha = row(std::iter::once(inicio.into()).chain(abas).chain([novo.into()])).spacing(2).align(Align::Center);
        container(pelicano::scrollable(linha).direction(pelicano::Direction::Horizontal).height(32).scrollbar_width(3)).padding([2u16, 8]).width(Length::FILL).style(estilo_container_topo).into()
    }

    fn view_desenho<'a>(&'a self, d: &'a Doc) -> Element<'a, Msg> {
        let faixa = crate::faixa::faixa(self.aba_faixa, &d.estado.ferramenta, Msg::Faixa, |id| self.widget(id));
        let area = program(Area {
            cena: &d.cena,
            imagens: &self.imagens,
            arrastados: d.arrastados.as_ref(),
            cursor: d.cursor.as_ref(),
            cores: Cores::do_tema(self.claro),
            tamanho: d.tamanho(),
            textos: [t("entrada.distancia"), t("entrada.angulo")],
        });
        let mut camadas_area = vec![Element::from(area), self.view_navegacao(d)];
        if self.config.fps {
            let fps = container(text(format!("{} {:.1} ms · {}", t("fps.motor"), d.ms_quadro, d.zoom)).size(11).font(Font::MONO).color(COR_TEXTO()))
                .padding([2u16, 6])
                .style(|_| Style::default().background(COR_PAINEL().alpha(0.85)));
            camadas_area.push(container(fps).padding(8).into());
        }
        let mut meio = row![container(stack(camadas_area)).width(Length::FILL).height(Length::FILL)];
        if let Some(p) = &self.props {
            meio = meio.push(p.view(Msg::Props));
        }
        column![faixa, meio.height(Length::FILL), self.view_comando(d), self.view_rodape(d)].into()
    }

    /// Os controles da faixa que não são botão (o antigo `ribbon-widgets.ts`).
    fn widget(&self, id: &'static str) -> Element<'_, Msg> {
        match id {
            "camadaAtual" => {
                let atual = self.camadas_fita.iter().find(|l| l.atual);
                let lista = pick_list(&self.nomes_camadas, atual.map(|l| l.nome.clone()), Msg::CamadaAtual).size(11).width(150);
                let chave = |prop: &'static str, ligado: bool, sim: &str, nao: &str, dica: &str| {
                    tooltip(button(text(if ligado { sim.to_string() } else { nao.to_string() }).size(12)).padding([1u16, 4]).style(estilo_botao_pequeno(false)).on_press(Msg::CamadaChave(prop)), t(dica), TooltipPosition::Bottom)
                        .padding([4, 6])
                        .style(estilo_dica)
                };
                let mut r = row![lista].spacing(2).align(Align::Center);
                if let Some(l) = atual {
                    r = r.push(chave("ligada", l.ligada, "💡", "◌", "camadas.ligada")).push(chave("congelada", l.congelada, "❄", "☀", "camadas.congelada")).push(chave("bloqueada", l.bloqueada, "🔒", "🔓", "camadas.bloqueada"));
                }
                r.into()
            }
            "propCor" => container(self.fita.controle_cor(Msg::Fita)).width(170).into(),
            "propEspessura" => container(self.fita.controle_espessura(Msg::Fita)).width(170).into(),
            _ => container(self.fita.controle_tipo(Msg::Fita)).width(170).into(),
        }
    }

    /// O cubo de vista e a barra de navegação, no canto da área (os antigos `vista-cubo.ts` e `barra-navegacao.ts`).
    fn view_navegacao<'a>(&'a self, d: &'a Doc) -> Element<'a, Msg> {
        let rotulos = ["topo", "baixo", "frente", "tras", "direita", "esquerda"].map(|k| t(&format!("vistaCubo.{k}")));
        let bussola = ["norte", "sul", "leste", "oeste"].map(|k| t(&format!("vistaCubo.{k}")));
        let cubo = program(crate::cubo::Cubo { rx: d.vista.rotacao_x, rz: d.vista.rotacao_z, claro: self.claro, acento: COR_AZUL_ACENTO(), rotulos, bussola, msg: |(rx, rz)| Msg::Vista(rx, rz) }).width(124).height(136);
        let nav = |simbolo: &str, chave: &str, msg: Msg| -> Element<'a, Msg> {
            tooltip(button(crate::icones::icone(simbolo, 14.0, simbolo)).padding([2u16, 7]).style(estilo_botao_pequeno(false)).on_press(msg), t(chave), TooltipPosition::Bottom).padding([4, 6]).style(estilo_dica).into()
        };
        let barra = container(
            row![
                nav("zoom_mais", "navegacao.maisZoom", Msg::Zoom(FATOR_DO_ZOOM)),
                nav("zoom_menos", "navegacao.menosZoom", Msg::Zoom(1.0 / FATOR_DO_ZOOM)),
                nav("enquadrar", "navegacao.ajustar", Msg::Ajustar),
                nav("⟳", "navegacao.orbita", Msg::Vista(d.vista.rotacao_x + 15.0, d.vista.rotacao_z + 15.0)),
            ]
            .spacing(1),
        )
        .padding(2)
        .style(|_| Style::default().background(COR_PAINEL().alpha(0.85)).border(Border::rounded(6.0).color(COR_BORDA(), 1.0)));
        container(column![cubo, barra].spacing(4).align(Align::Center)).width(Length::FILL).align_x(Align::End).padding(10).into()
    }

    fn view_comando<'a>(&'a self, d: &'a Doc) -> Element<'a, Msg> {
        let linhas = d.estado.historico.iter().rev().take(3).collect::<Vec<_>>();
        let historico = column(linhas.into_iter().rev().map(|h| {
            let s = match h {
                Entrada::Digitado(d) => format!("> {d}"),
                Entrada::Mensagem(m) => texto_com(&m.chave, &m.vars),
            };
            text(s).size(12).font(Font::MONO).color(COR_TEXTO_MUTED()).into()
        }));
        let blocos = row(d.estado.blocos.iter().map(|b| button(text(b.clone()).size(12)).padding([1u16, 12]).style(estilo_botao_secundario).on_press(Msg::Digitando(b.clone())).into())).spacing(6);
        let prompt = texto_com(&d.estado.prompt.chave, &d.estado.prompt.vars);
        let entrada = row![
            text("❯").size(13).color(COR_AZUL_ACENTO()),
            text(prompt).size(12).font(Font::MONO).color(COR_TEXTO_MUTED()),
            text_input(t("comando.placeholder"), &self.comando).id(ID_COMANDO).size(12).font(Font::MONO).on_input(Msg::Digitando).on_submit(Msg::Enviar),
        ]
        .spacing(8)
        .align(Align::Center);
        let mut c = column![historico].spacing(4).padding([6u16, 10]);
        if !d.estado.blocos.is_empty() {
            c = c.push(pelicano::scrollable(blocos).direction(pelicano::Direction::Horizontal).height(30));
        }
        container(c.push(entrada)).width(Length::FILL).style(|_| Style::default().background(COR_PAINEL_FAIXA()).border(Border::default().color(COR_BORDA(), 1.0))).into()
    }

    fn view_rodape<'a>(&'a self, d: &'a Doc) -> Element<'a, Msg> {
        let e = &d.estado;
        let coords = d.cursor.as_ref().map_or_else(String::new, |c| format!("X {}   Y {}", c.texto_x, c.texto_y));
        let liga = |nome: &'static str, chave: &str| -> Element<'a, Msg> {
            button(text(t(chave)).size(11)).padding([2u16, 8]).style(estilo_botao_pequeno(interruptor_ligado(d, nome))).on_press(Msg::Interruptor(nome)).into()
        };
        let fundos = [("nada", "rodape.fundoNada"), ("pontos", "rodape.fundoPontos"), ("grade", "rodape.fundoGrade")];
        let fundo = row(fundos.into_iter().map(|(m, chave)| button(text(t(chave)).size(11)).padding([2u16, 8]).style(estilo_botao_pequeno(e.fundo == m)).on_press(Msg::Fundo(m)).into())).spacing(2);
        // As abas de espaço: Modelo, os layouts e ＋ (o antigo `abas-layout.ts`).
        let modelo = e.espaco_ativo.as_str() == Some("modelo") || e.espaco_ativo.is_null();
        let ativo = e.espaco_ativo.get("layout").and_then(|v| v.as_str());
        let espacos = row(std::iter::once(button(text(t("rodape.modeloAba")).size(11)).padding([2u16, 8]).style(estilo_aba_faixa(modelo)).on_press(Msg::Espaco("modelo".into())).into())
            .chain(e.layouts.iter().map(|l| button(text(l.nome.clone()).size(11)).padding([2u16, 8]).style(estilo_aba_faixa(ativo == Some(l.nome.as_str()))).on_press(Msg::Espaco(l.nome.clone())).into()))
            .chain([tooltip(button(text("＋").size(11)).padding([2u16, 6]).style(estilo_botao_pequeno(false)).on_press(Msg::NovoLayout), t("rodape.novoLayout"), TooltipPosition::Top).into()]))
        .spacing(2);
        let orto = tooltip(pick_list(&self.angulos, Some(format!("{}°", e.orto_angulo)), Msg::OrtoAngulo).size(11).width(70), t("rodape.anguloOrto"), TooltipPosition::Top);
        let modos = tooltip(button(text("▾").size(11)).padding([2u16, 4]).style(estilo_botao_pequeno(self.snap_modos_aberto)).on_press(Msg::AlternarSnapModos), t("rodape.osnapModos"), TooltipPosition::Top);
        let mut r = row![
            espacos,
            text(coords).size(11).font(Font::MONO).width(250),
            text(d.zoom.clone()).size(11).width(56),
            fundo,
            liga("snap", "rodape.snap"),
            liga("orto", "rodape.orto"),
            orto,
            row![liga("osnap", "rodape.osnap"), modos].spacing(0),
            horizontal_space(),
        ]
        .spacing(10)
        .align(Align::Center);
        if let Some(u) = &e.unidade {
            r = r.push(text(tv("unidade.dicaRodape", &[("nome", &t(&format!("unidade.nomes.{u}")))])).size(11).color(COR_TEXTO_MUTED()));
        }
        container(r).padding([3u16, 10]).width(Length::FILL).style(estilo_container_topo).into()
    }

    /// Os avisos flutuantes (o antigo `toast.ts`), no canto de baixo à direita.
    fn view_avisos(&self) -> Element<'_, Msg> {
        let lista = column(self.avisos.iter().map(|(s, erro, _)| {
            let cor = if *erro { COR_ERRO() } else { COR_AZUL_ACENTO() };
            container(text(s.clone()).size(12).color(COR_TEXTO()))
                .padding([8u16, 12])
                .max_width(460.0)
                .style(move |_| Style::default().background(COR_PAINEL()).border(Border::rounded(8.0).color(cor, 1.0)))
                .into()
        }))
        .spacing(6);
        container(lista).width(Length::FILL).height(Length::FILL).align_x(Align::End).align_y(Align::End).padding(pelicano::Padding { top: 0.0, right: 24.0, bottom: 72.0, left: 0.0 }).into()
    }

    pub fn view_monitor(&self, id: WindowId) -> Element<'_, Msg> {
        let d = self.doc();
        let metricas = feather_ui::monitor::Metricas {
            cartoes: vec![
                (t("app.nome"), self.docs.len().to_string(), t("abas.inicio")),
                (t("fps.motor"), d.map_or_else(String::new, |d| format!("{:.1} ms", d.ms_quadro)), t("fps.rotulo")),
            ],
            linhas: d.map_or_else(Vec::new, |d| {
                vec![(t("app.nome"), d.nome()), (t("rodape.fundo"), d.estado.fundo.clone()), (t("camadas.dicaAtual"), d.estado.camada_atual.clone()), ("versão".into(), d.estado.versao.to_string())]
            }),
            ocupado: None,
        };
        cromo::janela_com_barra(
            &tv("monitor.titulo", &[("nome", "FeatherCAD")]),
            self.monitor.view(metricas, Msg::Monitor),
            AcoesJanela { arrastar: Msg::JanelaAux(id, WindowCmd::Drag), minimizar: Msg::JanelaAux(id, WindowCmd::Minimize), maximizar: Msg::JanelaAux(id, WindowCmd::ToggleMaximize), fechar: Msg::FecharAuxiliar(id) },
        )
    }
}

fn itens_do_menu(itens: &[ItemDeMenu]) -> Vec<MenuItem<Msg>> {
    itens
        .iter()
        .map(|i| {
            if i.separador {
                return menu_separator(String::new());
            }
            let rotulo = match &i.repetido {
                Some(nome) => tv("menu.repetir", &[("nome", &t(&format!("menu.item.{nome}")))]),
                None => t(&i.chave),
            };
            let mut m = menu_item(rotulo, Msg::MenuItem(i.id.clone())).shortcut(i.atalho.clone().unwrap_or_default()).checked(i.marcado == Some(true));
            if !i.ativo {
                m = m.disabled();
            }
            m
        })
        .collect()
}

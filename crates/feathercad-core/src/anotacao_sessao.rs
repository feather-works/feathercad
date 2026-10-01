//! As ferramentas de anotação e de utilitário da sessão: texto, cota, chamada, tabela, medir, colar (área de
//! transferência interna), tornar atual, igualar propriedades — mais as ações sobre seleção que fecham a aba
//! Início: copiar/recortar para a prancheta, igualar camada, criar bloco e agrupar. Tudo do Rust.

use crate::anotacao::{altura_padrao, area_e_perimetro, chamada, cota_alinhada, cota_de_raio, cota_linear, tabela};
use crate::aparar::caixa_dos;
use crate::documento::{AlinhaH, AlinhaV, Documento, Geometria, Id, Mudanca, NovaEntidade, Propriedades};
use crate::edicao::{escolher, escolher_com, transformar};
use crate::edicao_sessao::{CmdEdicao, Etapa};
use crate::geom::{Ponto, Transf};
use crate::sessao::{Ferramenta, Mensagem, Sessao};

/// Em que passo está o texto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EtapaTexto {
    /// O ponto de inserção.
    Ponto,
    /// Ponto marcado; pede a altura (Enter = a padrão).
    Altura(Ponto),
    /// Altura escolhida; pede o conteúdo (digitado).
    Conteudo(Ponto, f64),
}

/// O que se copiou (ou recortou): as entidades e o ponto de referência (o canto de baixo à esquerda).
#[derive(Clone, Debug, PartialEq)]
pub struct Prancheta {
    pub base: Ponto,
    pub itens: Vec<NovaEntidade>,
}

/// Mais que isto de vértices numa área medida e a medição para (proteção).
const VERTICES_MAXIMOS_DA_AREA: usize = 10_000;

impl Sessao {
    /// A altura de texto em uso: a última escolhida, ou 1/150 da diagonal do desenho.
    fn altura_do_texto(&self, doc: &Documento) -> f64 {
        self.altura_texto.unwrap_or_else(|| altura_padrao(doc))
    }

    /// O tamanho da cota (altura do texto; a seta e o resto vêm dele): o escolhido em Configurações,
    /// senão o mesmo do texto.
    fn altura_da_cota(&self, doc: &Documento) -> f64 {
        self.altura_cota.unwrap_or_else(|| self.altura_do_texto(doc))
    }

    /// Grava formas novas (na camada atual, cor e propriedades "por camada") como **um** passo de desfazer.
    fn criar_formas(&mut self, doc: &mut Documento, formas: Vec<Geometria>) {
        let camada = doc.camada_atual();
        let espaco = doc.espaco_ativo().clone();
        doc.aplicar_mudancas(formas.into_iter().map(|g| Mudanca::Novo(NovaEntidade { camada, cor: None, geometria: g, props: Propriedades::default(), espaco: espaco.clone() })).collect());
    }

    pub(crate) fn base_da_anotacao(&self, f: &Ferramenta) -> Option<Ponto> {
        match f {
            Ferramenta::Texto { etapa: EtapaTexto::Altura(p) | EtapaTexto::Conteudo(p, _) } => Some(*p),
            Ferramenta::Cota { pontos } | Ferramenta::CotaAlinhada { pontos } | Ferramenta::Chamada { pontos } | Ferramenta::Medir { pontos, .. } => pontos.last().copied(),
            Ferramenta::Tabela { origem } => *origem,
            _ => None,
        }
    }

    pub(crate) fn prompt_da_anotacao(&self, f: &Ferramenta) -> Mensagem {
        let chave = match f {
            Ferramenta::Texto { etapa: EtapaTexto::Ponto } => "texto.ponto",
            Ferramenta::Texto { etapa: EtapaTexto::Altura(_) } => return Mensagem::nova("texto.altura"),
            Ferramenta::Texto { etapa: EtapaTexto::Conteudo(..) } => "texto.conteudo",
            Ferramenta::Cota { pontos } => match pontos.len() {
                0 => "cota.primeiro",
                1 => "cota.segundo",
                _ => "cota.posicao",
            },
            Ferramenta::CotaAlinhada { pontos } => match pontos.len() {
                0 => "cota.primeiro",
                1 => "cota.segundo",
                _ => "cota.posicaoAlinhada",
            },
            Ferramenta::CotaRaio { diametro: false, alvo: None } => "cota.objetoRaio",
            Ferramenta::CotaRaio { diametro: true, alvo: None } => "cota.objetoDiametro",
            Ferramenta::CotaRaio { alvo: Some(_), .. } => "cota.posicaoRaio",
            Ferramenta::Chamada { pontos } => match pontos.len() {
                0 => "chamada.ponta",
                1 => "chamada.pouso",
                _ => "chamada.texto",
            },
            Ferramenta::Tabela { origem: None } => "tabela.ponto",
            Ferramenta::Tabela { origem: Some(_) } => "tabela.dados",
            Ferramenta::Medir { pontos, area: false } => if pontos.is_empty() { "medir.primeiro" } else { "medir.segundo" },
            Ferramenta::Medir { area: true, .. } => "medir.areaPonto",
            Ferramenta::Colar => "colar.ponto",
            Ferramenta::TornarAtual => "tornarAtual.clique",
            Ferramenta::IgualarPropriedades { origem: None } => "igualar.origem",
            Ferramenta::IgualarPropriedades { origem: Some(_) } => "igualar.destino",
            _ => "comando.digite",
        };
        Mensagem::nova(chave)
    }

    /// A prévia: a linha elástica, a cota como ficaria, os itens colados sob o cursor.
    pub(crate) fn previa_da_anotacao(&self, doc: &Documento, f: &Ferramenta, cursor: Ponto) -> Vec<Geometria> {
        let h = self.altura_do_texto(doc);
        match f {
            Ferramenta::Texto { etapa: EtapaTexto::Altura(p) } => vec![Geometria::Linha { a: *p, b: cursor }],
            Ferramenta::Cota { pontos } => match pontos.as_slice() {
                [a] => vec![Geometria::Linha { a: *a, b: cursor }],
                [a, b] => cota_linear(*a, *b, cursor, self.altura_da_cota(doc), &self.regional).unwrap_or_default(),
                _ => Vec::new(),
            },
            Ferramenta::CotaAlinhada { pontos } => match pontos.as_slice() {
                [a] => vec![Geometria::Linha { a: *a, b: cursor }],
                [a, b] => cota_alinhada(*a, *b, cursor, self.altura_da_cota(doc), &self.regional).unwrap_or_default(),
                _ => Vec::new(),
            },
            Ferramenta::CotaRaio { diametro, alvo: Some((centro, raio)) } => cota_de_raio(*centro, *raio, cursor, *diametro, self.altura_da_cota(doc), &self.regional).unwrap_or_default(),
            Ferramenta::Chamada { pontos } => match pontos.as_slice() {
                [a] => chamada(*a, cursor, h, "").unwrap_or_default(),
                [a, b] => chamada(*a, *b, h, "").unwrap_or_default(),
                _ => Vec::new(),
            },
            Ferramenta::Medir { pontos, area } => {
                if pontos.is_empty() {
                    Vec::new()
                } else if *area {
                    let vertices = pontos.iter().chain(std::iter::once(&cursor)).map(|p| crate::geom::Vertice { p: *p, bulge: 0.0 }).collect();
                    vec![Geometria::Polilinha { vertices, fechada: true }]
                } else {
                    vec![Geometria::Linha { a: pontos[0], b: cursor }]
                }
            }
            Ferramenta::Colar => self
                .prancheta
                .iter()
                .flat_map(|p| p.itens.iter().take(300).map(|n| transformar(&n.geometria, &Transf::mover(cursor.x - p.base.x, cursor.y - p.base.y))).collect::<Vec<_>>())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Comandos de anotação e de utilitário. `false` se o nome não é de nenhum deles.
    pub(crate) fn executar_anotacao(&mut self, doc: &mut Documento, comando: &str) -> bool {
        self.janela = None;
        match comando {
            "texto" => self.ferramenta = Ferramenta::Texto { etapa: EtapaTexto::Ponto },
            "cota" => self.ferramenta = Ferramenta::Cota { pontos: Vec::new() },
            "cota_alinhada" => self.ferramenta = Ferramenta::CotaAlinhada { pontos: Vec::new() },
            "cota_raio" => self.ferramenta = Ferramenta::CotaRaio { diametro: false, alvo: None },
            "cota_diametro" => self.ferramenta = Ferramenta::CotaRaio { diametro: true, alvo: None },
            "chamada" => self.ferramenta = Ferramenta::Chamada { pontos: Vec::new() },
            "tabela" => self.ferramenta = Ferramenta::Tabela { origem: None },
            "medir" => self.ferramenta = Ferramenta::Medir { pontos: Vec::new(), area: false },
            "colar" if self.prancheta.is_none() => self.dizer(Mensagem::nova("colar.vazia")),
            "colar" => self.ferramenta = Ferramenta::Colar,
            "igualar_propriedades" => self.ferramenta = Ferramenta::IgualarPropriedades { origem: None },
            "tornar_atual" => match self.selecao.iter().find_map(|id| doc.entidade(*id)).map(|e| e.camada) {
                Some(camada) => self.tornar_atual_de(doc, camada),
                None => self.ferramenta = Ferramenta::TornarAtual,
            },
            _ => return false,
        }
        true
    }

    fn tornar_atual_de(&mut self, doc: &mut Documento, camada: usize) {
        if doc.tornar_camada_atual(camada).is_ok() {
            let nome = doc.camadas().get(camada).map_or_else(String::new, |c| c.nome.clone());
            self.dizer(Mensagem::nova("tornarAtual.feito").com("nome", &nome));
        }
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Clique de uma ferramenta de anotação ou utilitário. `false` se a ferramenta é outra.
    pub(crate) fn clique_de_anotacao(&mut self, doc: &mut Documento, f: Ferramenta, ponto: Ponto, tolerancia: f64) -> bool {
        match f {
            Ferramenta::Texto { etapa: EtapaTexto::Ponto } => self.ferramenta = Ferramenta::Texto { etapa: EtapaTexto::Altura(ponto) },
            Ferramenta::Texto { etapa: EtapaTexto::Altura(p) } => {
                let h = p.distancia(ponto);
                if h > 0.0 {
                    self.ferramenta = Ferramenta::Texto { etapa: EtapaTexto::Conteudo(p, h) };
                }
            }
            Ferramenta::Texto { .. } => {}
            Ferramenta::Cota { mut pontos } => {
                pontos.push(ponto);
                if let [a, b, posicao] = pontos.as_slice() {
                    let h = self.altura_da_cota(doc);
                    match cota_linear(*a, *b, *posicao, h, &self.regional) {
                        Some(formas) => {
                            doc.adicionar_cota(formas, *a);
                            self.ferramenta = Ferramenta::Nenhuma;
                        }
                        None => {
                            self.dizer(Mensagem::nova("cota.nula"));
                            pontos.truncate(1);
                            self.ferramenta = Ferramenta::Cota { pontos };
                        }
                    }
                } else {
                    self.ferramenta = Ferramenta::Cota { pontos };
                }
            }
            Ferramenta::CotaAlinhada { mut pontos } => {
                pontos.push(ponto);
                if let [a, b, posicao] = pontos.as_slice() {
                    match cota_alinhada(*a, *b, *posicao, self.altura_da_cota(doc), &self.regional) {
                        Some(formas) => {
                            doc.adicionar_cota(formas, *a);
                            self.ferramenta = Ferramenta::Nenhuma;
                        }
                        None => {
                            self.dizer(Mensagem::nova("cota.nula"));
                            pontos.truncate(1);
                            self.ferramenta = Ferramenta::CotaAlinhada { pontos };
                        }
                    }
                } else {
                    self.ferramenta = Ferramenta::CotaAlinhada { pontos };
                }
            }
            Ferramenta::CotaRaio { diametro, alvo: None } => {
                let forma = crate::edicao::escolher(doc, ponto, tolerancia).and_then(|id| doc.entidade(id)).map(|e| e.geometria.clone());
                match forma {
                    Some(Geometria::Circulo { centro, raio }) | Some(Geometria::Arco { centro, raio, .. }) => self.ferramenta = Ferramenta::CotaRaio { diametro, alvo: Some((centro, raio)) },
                    Some(_) => self.dizer(Mensagem::nova("cota.soCirculo")),
                    None => {}
                }
            }
            Ferramenta::CotaRaio { diametro, alvo: Some((centro, raio)) } => match cota_de_raio(centro, raio, ponto, diametro, self.altura_da_cota(doc), &self.regional) {
                Some(formas) => {
                    doc.adicionar_cota(formas, centro);
                    self.ferramenta = Ferramenta::Nenhuma;
                }
                None => self.dizer(Mensagem::nova("cota.nula")),
            },
            Ferramenta::Chamada { mut pontos } => {
                if pontos.len() < 2 {
                    pontos.push(ponto);
                    self.ferramenta = Ferramenta::Chamada { pontos };
                }
            }
            Ferramenta::Tabela { origem: None } => self.ferramenta = Ferramenta::Tabela { origem: Some(ponto) },
            Ferramenta::Tabela { .. } => {}
            Ferramenta::Medir { mut pontos, area } => {
                pontos.push(ponto);
                if !area && pontos.len() == 2 {
                    self.dizer_distancia(pontos[0], pontos[1]);
                    pontos.clear();
                } else if area && pontos.len() > VERTICES_MAXIMOS_DA_AREA {
                    self.dizer(Mensagem::nova("medir.demais"));
                    pontos.pop();
                }
                self.ferramenta = Ferramenta::Medir { pontos, area };
            }
            Ferramenta::Colar => self.colar(doc, ponto),
            Ferramenta::TornarAtual => {
                if let Some(e) = escolher_com(doc, ponto, tolerancia, true).and_then(|id| doc.entidade(id)) {
                    let camada = e.camada;
                    self.tornar_atual_de(doc, camada);
                }
            }
            Ferramenta::IgualarPropriedades { origem } => self.clique_igualar(doc, origem, ponto, tolerancia),
            _ => return false,
        }
        true
    }

    fn dizer_distancia(&mut self, a: Ponto, b: Ponto) {
        let r = &self.regional;
        let m = Mensagem::nova("medir.distancia")
            .com("distancia", &r.numero(a.distancia(b)))
            .com("angulo", &r.numero(a.angulo_ate(b)))
            .com("dx", &r.numero(b.x - a.x))
            .com("dy", &r.numero(b.y - a.y));
        self.dizer(m);
    }

    fn dizer_area(&mut self, pontos: &[Ponto]) {
        if pontos.len() < 3 {
            self.dizer(Mensagem::nova("medir.poucosPontos"));
            return;
        }
        let (area, perimetro) = area_e_perimetro(pontos);
        let m = Mensagem::nova("medir.area").com("area", &self.regional.numero(area)).com("perimetro", &self.regional.numero(perimetro));
        self.dizer(m);
    }

    fn clique_igualar(&mut self, doc: &mut Documento, origem: Option<Id>, ponto: Ponto, tolerancia: f64) {
        let Some(origem) = origem else {
            // A origem pode estar em camada bloqueada: só se lêem as propriedades dela.
            if let Some(id) = escolher_com(doc, ponto, tolerancia, true) {
                self.ferramenta = Ferramenta::IgualarPropriedades { origem: Some(id) };
            }
            return;
        };
        let (Some(fonte), Some(destino)) = (doc.entidade(origem).cloned(), escolher(doc, ponto, tolerancia)) else { return };
        if destino != origem {
            doc.aplicar_mudancas(vec![Mudanca::Alterar { id: destino, camada: Some(fonte.camada), cor: Some(fonte.cor), props: Some(Propriedades { grupo: None, ..fonte.props }) }]);
        }
    }

    /// Cola a prancheta com o ponto de referência em `ponto`; a ferramenta continua (Enter termina).
    fn colar(&mut self, doc: &mut Documento, ponto: Ponto) {
        let Some(p) = self.prancheta.clone() else { return };
        let t = Transf::mover(ponto.x - p.base.x, ponto.y - p.base.y);
        let ms: Vec<Mudanca> = p.itens.iter().map(|n| Mudanca::Novo(NovaEntidade { geometria: transformar(&n.geometria, &t), ..n.clone() })).collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova("colar.feito").com("n", &n));
    }

    /// Copia (ou recorta) a seleção para a prancheta interna: nada sai do aplicativo.
    pub(crate) fn para_prancheta(&mut self, doc: &mut Documento, recortar: bool) {
        let ids: Vec<Id> = self.selecao.iter().copied().collect();
        let itens: Vec<NovaEntidade> = ids.iter().filter_map(|id| doc.entidade(*id)).filter(|e| !doc.bloqueada(e)).map(|e| NovaEntidade { camada: e.camada, cor: e.cor, geometria: e.geometria.clone(), props: e.props.clone(), espaco: e.espaco.clone() }).collect();
        if let (false, Some((min, _))) = (itens.is_empty(), caixa_dos(doc, &ids)) {
            let n = itens.len().to_string();
            self.prancheta = Some(Prancheta { base: min, itens });
            if recortar {
                doc.aplicar_mudancas(ids.iter().map(|id| Mudanca::Remover(*id)).collect());
            }
            self.dizer(Mensagem::nova(if recortar { "prancheta.recortados" } else { "prancheta.copiados" }).com("n", &n));
        } else {
            self.dizer(Mensagem::nova("edicao.nadaSelecionado"));
        }
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Agrupa a seleção (um clique em qualquer membro passa a selecionar o grupo todo); se já é um grupo, desfaz.
    pub(crate) fn agrupar(&mut self, doc: &mut Documento) {
        let membros: Vec<_> = self.selecao.iter().filter_map(|id| doc.entidade(*id)).cloned().collect();
        let grupo_comum = membros.first().and_then(|e| e.props.grupo).filter(|g| membros.iter().all(|e| e.props.grupo == Some(*g)));
        let (novo, chave): (Option<u32>, &str) = match grupo_comum {
            Some(_) => (None, "agrupar.desfeito"),
            None => (Some(doc.entidades().iter().filter_map(|e| e.props.grupo).max().unwrap_or(0) + 1), "agrupar.criado"),
        };
        // Desagrupar leva o grupo inteiro (mesmo quem não estava selecionado).
        let alvos: Vec<_> = match grupo_comum {
            Some(g) => doc.entidades().iter().filter(|e| e.props.grupo == Some(g)).cloned().collect(),
            None => membros,
        };
        let n = alvos.len().to_string();
        doc.aplicar_mudancas(alvos.into_iter().map(|e| Mudanca::Alterar { id: e.id, camada: None, cor: None, props: Some(Propriedades { grupo: novo, ..e.props }) }).collect());
        self.dizer(Mensagem::nova(chave).com("n", &n));
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Acrescenta à seleção todos os membros dos grupos que ela toca.
    pub(crate) fn expandir_grupos(&mut self, doc: &Documento) {
        let grupos: std::collections::BTreeSet<u32> = self.selecao.iter().filter_map(|id| doc.entidade(*id)).filter_map(|e| e.props.grupo).collect();
        if grupos.is_empty() {
            return;
        }
        let antes = self.selecao.len();
        self.selecao.extend(doc.entidades().iter().filter(|e| e.props.grupo.is_some_and(|g| grupos.contains(&g)) && doc.visivel(e) && !doc.bloqueada(e)).map(|e| e.id));
        if self.selecao.len() != antes {
            self.rev_selecao += 1;
        }
    }

    /// O clique do "igualar camada": leva a seleção para a camada do objeto clicado.
    pub(crate) fn clique_alvo_da_camada(&mut self, doc: &mut Documento, ponto: Ponto, tolerancia: f64) {
        let Some(camada) = escolher_com(doc, ponto, tolerancia, true).and_then(|id| doc.entidade(id)).map(|e| e.camada) else { return };
        let ms: Vec<Mudanca> = self.selecao.iter().filter_map(|id| doc.entidade(*id)).filter(|e| !doc.bloqueada(e)).map(|e| Mudanca::Alterar { id: e.id, camada: Some(camada), cor: None, props: None }).collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova("edicao.alterados").com("n", &n));
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Cria o bloco da seleção com o nome digitado; o ponto-base é o já clicado.
    pub(crate) fn criar_bloco(&mut self, doc: &mut Documento, base: Ponto, nome: &str) {
        match doc.criar_bloco_da_selecao(nome, base, &self.selecao) {
            Ok(_) => {
                self.dizer(Mensagem::nova("bloco.criado").com("nome", nome.trim()));
                self.limpar_selecao();
                self.ferramenta = Ferramenta::Nenhuma;
            }
            Err(e) => self.dizer(Mensagem::nova(e.chave())),
        }
    }

    /// Texto digitado numa ferramenta de anotação. `false` se a ferramenta é outra.
    pub(crate) fn digitar_de_anotacao(&mut self, doc: &mut Documento, f: &Ferramenta, texto: &str) -> bool {
        match f {
            Ferramenta::Texto { etapa } => self.digitar_no_texto(doc, *etapa, texto),
            Ferramenta::Chamada { pontos } if pontos.len() == 2 => {
                if let Some(formas) = chamada(pontos[0], pontos[1], self.altura_do_texto(doc), texto) {
                    self.criar_formas(doc, formas);
                }
                self.ferramenta = Ferramenta::Nenhuma;
            }
            Ferramenta::Tabela { origem: Some(o) } if !texto.is_empty() => self.digitar_na_tabela(doc, *o, texto),
            Ferramenta::Medir { pontos, area } if texto.is_empty() => {
                if *area {
                    self.dizer_area(pontos);
                }
                self.ferramenta = Ferramenta::Nenhuma;
            }
            Ferramenta::Medir { .. } if self.comandos.resolver_opcao(texto).is_some() => {
                self.ferramenta = Ferramenta::Medir { pontos: Vec::new(), area: self.comandos.resolver_opcao(texto) == Some("area") };
            }
            Ferramenta::Cota { .. } | Ferramenta::CotaAlinhada { .. } | Ferramenta::CotaRaio { .. } | Ferramenta::Chamada { .. } | Ferramenta::Tabela { .. } | Ferramenta::Medir { .. } | Ferramenta::Colar | Ferramenta::TornarAtual | Ferramenta::IgualarPropriedades { .. } => {
                if texto.is_empty() {
                    self.ferramenta = Ferramenta::Nenhuma;
                } else {
                    self.ponto_ou_comando(doc, f, texto);
                }
            }
            _ => return false,
        }
        true
    }

    /// Coordenada digitada vira clique na ferramenta; nome de comando troca de comando; o resto é erro.
    fn ponto_ou_comando(&mut self, doc: &mut Documento, f: &Ferramenta, texto: &str) {
        if let Some(p) = self.ler_ponto(texto) {
            self.clique_de_anotacao(doc, f.clone(), p, 0.0);
        } else if let Some(c) = self.comandos.resolver(texto) {
            self.executar(doc, c);
        } else {
            self.ponto_invalido(texto);
        }
    }

    fn digitar_no_texto(&mut self, doc: &mut Documento, etapa: EtapaTexto, texto: &str) {
        match etapa {
            EtapaTexto::Ponto if texto.is_empty() => self.ferramenta = Ferramenta::Nenhuma,
            EtapaTexto::Ponto => self.ponto_ou_comando(doc, &Ferramenta::Texto { etapa }, texto),
            EtapaTexto::Altura(p) => {
                let altura = if texto.is_empty() { Some(self.altura_do_texto(doc)) } else { self.regional.ler_numero(texto).filter(|h| *h > 0.0) };
                match altura {
                    Some(h) => self.ferramenta = Ferramenta::Texto { etapa: EtapaTexto::Conteudo(p, h) },
                    None => self.dizer(Mensagem::nova("texto.alturaInvalida")),
                }
            }
            EtapaTexto::Conteudo(_, _) if texto.is_empty() => self.ferramenta = Ferramenta::Nenhuma,
            EtapaTexto::Conteudo(ponto, altura) => {
                self.criar_formas(doc, vec![Geometria::Texto { ponto, altura, rotacao: 0.0, conteudo: texto.to_string(), h: AlinhaH::Esquerda, v: AlinhaV::Base }]);
                // A próxima anotação começa com a altura que se acabou de usar.
                self.altura_texto = Some(altura);
                self.ferramenta = Ferramenta::Nenhuma;
            }
        }
    }

    /// `linhas colunas` (com o tamanho da célula pela altura do texto) ou `linhas colunas altura-da-linha largura-da-coluna`.
    fn digitar_na_tabela(&mut self, doc: &mut Documento, origem: Ponto, texto: &str) {
        let numeros: Vec<f64> = texto.split(|c: char| c.is_whitespace() || c == ';').filter(|s| !s.is_empty()).map_while(|s| self.regional.ler_numero(s)).collect();
        let h = self.altura_do_texto(doc);
        let (linhas, colunas, altura, largura) = match numeros.as_slice() {
            [l, c] => (*l, *c, h * 2.0, h * 10.0),
            [l, c, a, w] => (*l, *c, *a, *w),
            _ => {
                self.dizer(Mensagem::nova("tabela.invalida"));
                return;
            }
        };
        if linhas.fract() != 0.0 || colunas.fract() != 0.0 || linhas < 1.0 || colunas < 1.0 {
            self.dizer(Mensagem::nova("tabela.invalida"));
            return;
        }
        match tabela(origem, linhas as usize, colunas as usize, altura, largura) {
            Some(formas) => {
                self.criar_formas(doc, formas);
                self.ferramenta = Ferramenta::Nenhuma;
            }
            None => self.dizer(Mensagem::nova("tabela.invalida")),
        }
    }

    /// Ao concluir a escolha de objetos, os comandos que não pedem mais nada ou pedem etapas próprias.
    pub(crate) fn depois_da_selecao_de_anotacao(&mut self, doc: &mut Documento, cmd: CmdEdicao) -> Option<Etapa> {
        match cmd {
            CmdEdicao::CopiarArea => self.para_prancheta(doc, false),
            CmdEdicao::Recortar => self.para_prancheta(doc, true),
            CmdEdicao::Agrupar => self.agrupar(doc),
            CmdEdicao::IgualarCamada => return Some(Etapa::Alvo),
            _ => return Some(Etapa::Base),
        }
        None
    }
}

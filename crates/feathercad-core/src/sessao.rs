//! A sessão de trabalho: a ferramenta ativa, os interruptores e a linha de
//! comando. Recebe clique, texto digitado e tecla; devolve o **prompt** como
//! chave + variáveis (a interface traduz a chave pelo arquivo de língua).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::atalhos::{acao_da_tecla, Acao, Interruptor};
use crate::comandos::Comandos;
use crate::cursor::Ajudas;
use crate::documento::{Documento, Id, Unidade};
use crate::anotacao_sessao::{EtapaTexto, Prancheta};
use crate::edicao_sessao::{CmdEdicao, Etapa};
use crate::formato::Regional;
use crate::documento::Geometria;
use crate::geom::{arco_por_3_pontos, Ponto, Vertice};

const HISTORICO_MAX: usize = 200;
/// Passo do snap e da grade, em unidades do desenho.
pub const PASSO_DA_GRADE: f64 = 10.0;

/// Uma mensagem para a interface traduzir: `chave` + `{variáveis}`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Mensagem {
    pub chave: String,
    pub vars: BTreeMap<String, String>,
}

impl Mensagem {
    /// Uma mensagem sem variáveis.
    pub fn nova(chave: &str) -> Self {
        Self { chave: chave.into(), vars: BTreeMap::new() }
    }

    /// Acrescenta uma variável `{nome}`.
    pub fn com(mut self, nome: &str, valor: &str) -> Self {
        self.vars.insert(nome.into(), valor.into());
        self
    }
}

/// Uma linha do histórico da linha de comando.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Entrada {
    Digitado(String),
    Mensagem(Mensagem),
}

/// O fundo do desenho: nada, pontos ou linhas de grade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fundo {
    Nada,
    Pontos,
    Grade,
}

/// O resultado de uma tecla: se é de algum comando e se pede ação de janela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultadoTecla {
    pub tratado: bool,
    pub janela: Option<&'static str>,
}

/// A ferramenta em andamento.
#[derive(Clone, Debug, PartialEq)]
pub enum Ferramenta {
    Nenhuma,
    Linha { primeiro: Option<Ponto> },
    /// Os vértices já marcados; vira uma entidade só quando a polilinha termina.
    Polilinha { pontos: Vec<Ponto> },
    /// O centro (depois vem o raio).
    Circulo { centro: Option<Ponto> },
    /// Arco por 3 pontos: início, um ponto no meio e o fim.
    Arco { pontos: Vec<Ponto> },
    /// Inserir bloco: primeiro o nome (digitado), depois o ponto.
    Insercao { bloco: Option<usize> },
    /// Um comando que age sobre a seleção (mover, girar…), na etapa em que está.
    Editar { cmd: CmdEdicao, etapa: Etapa },
    /// Aparar: cada clique apara o trecho clicado (as arestas de corte são as outras formas).
    Aparar,
    /// Concordância de duas linhas: a primeira, já clicada.
    Concordancia { primeira: Option<(Id, Ponto)> },
    /// Retângulo: o primeiro canto.
    Retangulo { canto: Option<Ponto> },
    /// Elipse: centro, ponta do eixo e (no clique final) o outro semi-eixo.
    Elipse { pontos: Vec<Ponto> },
    /// Hachura: cada clique dentro de um contorno fechado.
    Hachura,
    /// Texto de uma linha: ponto, altura e o conteúdo (digitado).
    Texto { etapa: EtapaTexto },
    /// Cota linear: os dois pontos medidos e a posição da linha de cota.
    Cota { pontos: Vec<Ponto> },
    /// Chamada: a ponta da seta, o pouso e o texto (digitado).
    Chamada { pontos: Vec<Ponto> },
    /// Tabela: o canto superior esquerdo; depois se digita linhas, colunas e o tamanho da célula.
    Tabela { origem: Option<Ponto> },
    /// Medir distância (dois cliques) ou área (vários cliques e Enter).
    Medir { pontos: Vec<Ponto>, area: bool },
    /// Colar a prancheta: cada clique cola uma cópia.
    Colar,
    /// Tornar atual a camada do objeto clicado.
    TornarAtual,
    /// Contar iguais: cada clique num objeto seleciona todos os iguais a ele e diz quantos são.
    Contar,
    /// Igualar propriedades: primeiro o objeto de origem, depois os de destino.
    IgualarPropriedades { origem: Option<Id> },
}

impl Ferramenta {
    /// O nome interno da ferramenta (o do comando que a abriu), para a interface destacar o botão.
    pub fn nome(&self) -> &'static str {
        match self {
            Ferramenta::Nenhuma => "nenhuma",
            Ferramenta::Linha { .. } => "linha",
            Ferramenta::Polilinha { .. } => "polilinha",
            Ferramenta::Circulo { .. } => "circulo",
            Ferramenta::Arco { .. } => "arco",
            Ferramenta::Insercao { .. } => "inserir",
            Ferramenta::Editar { cmd, .. } => cmd.nome(),
            Ferramenta::Aparar => "aparar",
            Ferramenta::Concordancia { .. } => "concordancia",
            Ferramenta::Retangulo { .. } => "retangulo",
            Ferramenta::Elipse { .. } => "elipse",
            Ferramenta::Hachura => "hachura",
            Ferramenta::Texto { .. } => "texto",
            Ferramenta::Cota { .. } => "cota",
            Ferramenta::Chamada { .. } => "chamada",
            Ferramenta::Tabela { .. } => "tabela",
            Ferramenta::Medir { .. } => "medir",
            Ferramenta::Contar => "contar",
            Ferramenta::Colar => "colar",
            Ferramenta::TornarAtual => "tornar_atual",
            Ferramenta::IgualarPropriedades { .. } => "igualar_propriedades",
        }
    }
}

/// A sessão.
#[derive(Debug)]
pub struct Sessao {
    pub ferramenta: Ferramenta,
    pub interruptores: Interruptores,
    /// O fundo mostrado (F7 alterna entre `Nada` e o último estilo).
    pub fundo: Fundo,
    ultimo_estilo: Fundo,
    pub comandos: Comandos,
    pub regional: Regional,
    historico: Vec<Entrada>,
    /// Os objetos selecionados (ids); só valem os que ainda existem.
    pub(crate) selecao: BTreeSet<Id>,
    /// Sobe a cada mudança da seleção: o quadro precisa ser refeito para o realce.
    pub(crate) rev_selecao: u64,
    /// O primeiro canto de uma janela de seleção (dois cliques), se já foi marcado.
    pub(crate) janela: Option<Ponto>,
    /// A última janela de seleção (canto mínimo, canto máximo): o `esticar` usa.
    pub(crate) ultima_janela: Option<(Ponto, Ponto)>,
    /// O raio da concordância (0 = só encontrar as linhas).
    pub(crate) raio_concordancia: f64,
    /// Hachura sólida (senão, linhas a 45°).
    pub(crate) hachura_solida: bool,
    /// O padrão da biblioteca (`padroes::NOMES`) e a escala dele para a próxima hachura (a sólida vale `hachura_solida`).
    pub(crate) hachura_padrao: String,
    pub(crate) hachura_escala: f64,
    /// O que se copiou ou recortou (a área de transferência do aplicativo; nada vai para o sistema).
    pub(crate) prancheta: Option<Prancheta>,
    /// A altura do último texto (`None` = 1/150 da diagonal do desenho).
    pub(crate) altura_texto: Option<f64>,
    /// O tamanho da cota escolhido em Configurações (`None` = o do texto).
    pub altura_cota: Option<f64>,
    /// Onde o cursor estava da última vez (já resolvido: snap, orto…); dá a **direção** da distância digitada.
    pub cursor: Option<Ponto>,
    /// De quantos em quantos graus o Orto trava a direção (padrão 90; o usuário escolhe 15, 30, 45, 60…).
    pub angulo_orto: f64,
    /// Que tipos de snap ao objeto estão ligados (Extremidade, Meio, Centro…).
    pub modos_snap: crate::snap::ModosSnap,
    /// `<45` digitado com um ponto anterior: a próxima distância/ponto sai nessa direção (graus, anti-horário a partir do eixo X).
    pub angulo_travado: Option<f64>,
    /// O ponto onde o "grab" começou: o objeto sob o mouse está sendo arrastado (ver `arrasto_iniciar`).
    pub(crate) arrasto: Option<Ponto>,
    /// O último comando que abriu uma ferramenta (o menu do botão direito oferece repetir).
    pub(crate) ultimo_comando: Option<String>,
    /// A unidade do desenho (a ponte atualiza antes de cada texto digitado).
    pub unidade: Option<Unidade>,
}

/// Estado dos botões do rodapé.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interruptores {
    /// Snap à grade (F9).
    pub snap: bool,
    pub orto: bool,
    pub osnap: bool,
}

impl Default for Sessao {
    fn default() -> Self {
        Self {
            ferramenta: Ferramenta::Nenhuma,
            interruptores: Interruptores { snap: false, orto: false, osnap: true },
            fundo: Fundo::Grade,
            ultimo_estilo: Fundo::Grade,
            comandos: Comandos::default(),
            regional: Regional::default(),
            historico: Vec::new(),
            selecao: BTreeSet::new(),
            rev_selecao: 0,
            janela: None,
            ultima_janela: None,
            raio_concordancia: 0.0,
            hachura_solida: false,
            hachura_padrao: crate::padroes::PADRAO_DE_FABRICA.to_string(),
            hachura_escala: 1.0,
            prancheta: None,
            altura_texto: None,
            altura_cota: None,
            cursor: None,
            angulo_orto: 90.0,
            modos_snap: crate::snap::ModosSnap::default(),
            angulo_travado: None,
            arrasto: None,
            ultimo_comando: None,
            unidade: None,
        }
    }
}

impl Sessao {
    /// Os interruptores como o cursor os entende.
    pub fn ajudas(&self) -> Ajudas {
        Ajudas {
            orto: self.interruptores.orto,
            grade: self.interruptores.snap.then_some(PASSO_DA_GRADE),
            osnap: self.interruptores.osnap,
            modos: self.modos_snap,
            angulo: self.angulo_orto,
            direcao: self.angulo_travado,
        }
    }

    /// O ponto anterior da ferramenta (a base do ortogonal e da distância).
    pub fn base(&self) -> Option<Ponto> {
        match &self.ferramenta {
            Ferramenta::Linha { primeiro } => *primeiro,
            Ferramenta::Polilinha { pontos } => pontos.last().copied(),
            Ferramenta::Circulo { centro } => *centro,
            Ferramenta::Arco { pontos } => pontos.last().copied(),
            Ferramenta::Nenhuma => self.arrasto,
            Ferramenta::Insercao { .. } => None,
            outra => self.base_da_edicao(outra),
        }
    }

    /// Os pontos já marcados de uma polilinha em andamento.
    pub fn pontos_da_previa(&self) -> &[Ponto] {
        match &self.ferramenta {
            Ferramenta::Polilinha { pontos } => pontos,
            _ => &[],
        }
    }

    /// O que a ferramenta desenharia se o usuário clicasse em `cursor` agora (a interface só a traça).
    pub fn previa(&self, cursor: Ponto) -> Vec<Geometria> {
        match &self.ferramenta {
            Ferramenta::Linha { primeiro: Some(a) } => vec![Geometria::Linha { a: *a, b: cursor }],
            Ferramenta::Polilinha { pontos } if !pontos.is_empty() => {
                let vertices = pontos.iter().chain(std::iter::once(&cursor)).map(|p| Vertice { p: *p, bulge: 0.0 }).collect();
                vec![Geometria::Polilinha { vertices, fechada: false }]
            }
            Ferramenta::Circulo { centro: Some(c) } if c.distancia(cursor) > 0.0 => vec![Geometria::Circulo { centro: *c, raio: c.distancia(cursor) }],
            Ferramenta::Arco { pontos } => match pontos.as_slice() {
                [p1] => vec![Geometria::Linha { a: *p1, b: cursor }],
                [p1, p2] => match arco_por_3_pontos(*p1, *p2, cursor) {
                    Some(a) => vec![Geometria::Arco { centro: a.centro, raio: a.raio, inicio: a.inicio, fim: a.fim }],
                    None => vec![Geometria::Linha { a: *p2, b: cursor }],
                },
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Todas as formas da prévia: a da ferramenta que desenha e a da edição (objetos movidos, janela…).
    pub fn previa_com(&self, doc: &Documento, cursor: Ponto) -> Vec<Geometria> {
        let mut v = self.previa(cursor);
        v.extend(self.previa_da_edicao(doc, cursor));
        v
    }

    /// O que a linha de comando está pedindo agora.
    pub fn prompt(&self) -> Mensagem {
        match &self.ferramenta {
            Ferramenta::Nenhuma => Mensagem::nova("comando.digite"),
            Ferramenta::Linha { primeiro: None } => Mensagem::nova("linha.primeiroPonto"),
            Ferramenta::Linha { primeiro: Some(_) } => Mensagem::nova("linha.proximoPonto"),
            Ferramenta::Polilinha { pontos } if pontos.is_empty() => Mensagem::nova("polilinha.primeiroPonto"),
            Ferramenta::Polilinha { .. } => Mensagem::nova("polilinha.proximoPonto"),
            Ferramenta::Circulo { centro: None } => Mensagem::nova("circulo.centro"),
            Ferramenta::Circulo { centro: Some(_) } => Mensagem::nova("circulo.raio"),
            Ferramenta::Insercao { bloco: None } => Mensagem::nova("insercao.nome"),
            Ferramenta::Insercao { bloco: Some(_) } => Mensagem::nova("insercao.ponto"),
            Ferramenta::Arco { pontos } => Mensagem::nova(match pontos.len() {
                0 => "arco.primeiroPonto",
                1 => "arco.segundoPonto",
                _ => "arco.pontoFinal",
            }),
            outra => self.prompt_da_edicao(outra),
        }
    }

    /// As últimas linhas do histórico.
    pub fn historico(&self) -> &[Entrada] {
        &self.historico
    }

    pub(crate) fn registrar(&mut self, e: Entrada) {
        self.historico.push(e);
        if self.historico.len() > HISTORICO_MAX {
            self.historico.remove(0);
        }
    }

    /// Escreve uma mensagem no histórico da linha de comando.
    /// Escolhe o padrão de hachura (`SOLID` ou um de `padroes::NOMES`) e a escala dele. Erro se o nome não existe.
    pub fn definir_hachura(&mut self, nome: &str, escala: f64) -> Result<(), String> {
        let canonico = crate::padroes::reconhecer(nome).ok_or_else(|| format!("padrão de hachura `{nome}` não existe"))?;
        if !(escala.is_finite() && escala > 0.0 && escala <= 100.0) {
            return Err(format!("escala de hachura inválida: {escala}"));
        }
        self.hachura_solida = canonico == crate::padroes::SOLIDA;
        if !self.hachura_solida {
            self.hachura_padrao = canonico.to_string();
        }
        self.hachura_escala = escala;
        Ok(())
    }

    /// O padrão escolhido (`SOLID` se a sólida) e a escala.
    pub fn hachura_escolhida(&self) -> (&str, f64) {
        (if self.hachura_solida { crate::padroes::SOLIDA } else { &self.hachura_padrao }, self.hachura_escala)
    }

    pub fn dizer(&mut self, m: Mensagem) {
        self.registrar(Entrada::Mensagem(m));
    }

    /// Esvazia o histórico e cancela a ferramenta (desenho novo ou recém-aberto).
    pub fn recomecar(&mut self) {
        self.ferramenta = Ferramenta::Nenhuma;
        self.historico.clear();
        self.selecao.clear();
        self.janela = None;
        self.rev_selecao += 1;
    }

    /// Escolhe o fundo. Guarda o estilo para o F7 saber a que voltar.
    pub fn definir_fundo(&mut self, f: Fundo) {
        self.fundo = f;
        if f != Fundo::Nada {
            self.ultimo_estilo = f;
        }
    }

    /// Liga ou desliga um interruptor do rodapé.
    pub fn alternar(&mut self, i: Interruptor) {
        let ligado = match i {
            Interruptor::Grade => self.fundo == Fundo::Nada,
            Interruptor::Snap => !self.interruptores.snap,
            Interruptor::Orto => !self.interruptores.orto,
            Interruptor::Osnap => !self.interruptores.osnap,
        };
        self.definir(i, ligado);
    }

    /// Define um interruptor (usado pelos botões do rodapé). Ligar a grade volta
    /// ao último estilo de fundo; desligar é `Nada`.
    pub fn definir(&mut self, i: Interruptor, ligado: bool) {
        match i {
            Interruptor::Grade => self.definir_fundo(if ligado { self.ultimo_estilo } else { Fundo::Nada }),
            Interruptor::Snap => self.interruptores.snap = ligado,
            Interruptor::Orto => self.interruptores.orto = ligado,
            Interruptor::Osnap => self.interruptores.osnap = ligado,
        }
    }

    /// Uma tecla de atalho.
    pub fn tecla(&mut self, doc: &mut Documento, tecla: &str) -> ResultadoTecla {
        let mut r = ResultadoTecla { tratado: true, janela: None };
        match acao_da_tecla(tecla) {
            None => r.tratado = false,
            Some(Acao::Alternar(i)) => self.alternar(i),
            Some(Acao::Comando(c)) => self.executar(doc, c),
            Some(Acao::Cancelar) => self.cancelar(doc),
            Some(Acao::Janela(j)) => r.janela = Some(j),
        }
        r
    }

    /// Esc. Numa polilinha já com dois pontos ou mais, o que foi marcado fica (como no
    /// AutoCAD); em qualquer outro comando, só cancela.
    pub fn cancelar(&mut self, doc: &mut Documento) {
        self.angulo_travado = None;
        // Esc com nada em andamento: primeiro some a janela pela metade, depois a seleção.
        if self.ferramenta == Ferramenta::Nenhuma {
            if self.arrasto.take().is_some() {
                return;
            }
            if self.janela.take().is_none() {
                self.limpar_selecao();
            }
            return;
        }
        self.terminar_polilinha(doc, false);
        self.ferramenta = Ferramenta::Nenhuma;
        self.janela = None;
    }

    /// Termina a polilinha em andamento (se há), criando a entidade se houver 2+ pontos.
    fn terminar_polilinha(&mut self, doc: &mut Documento, fechada: bool) {
        if let Ferramenta::Polilinha { pontos } = &self.ferramenta {
            let vertices: Vec<Vertice> = pontos.iter().map(|p| Vertice { p: *p, bulge: 0.0 }).collect();
            doc.adicionar_polilinha(vertices, fechada);
            self.ferramenta = Ferramenta::Nenhuma;
        }
    }

    /// Clique na área de desenho, já com o ponto resolvido pelo cursor.
    pub fn clique(&mut self, doc: &mut Documento, ponto: Ponto) {
        self.clique_com(doc, ponto, 0.0, false);
    }

    /// Como `clique`, com o alcance da escolha de objetos (`tolerancia`, em unidades do desenho) e
    /// `aditivo` (Shift: acrescenta à seleção em vez de trocá-la).
    pub fn clique_com(&mut self, doc: &mut Documento, ponto: Ponto, tolerancia: f64, aditivo: bool) {
        self.angulo_travado = None;
        if self.clique_de_edicao(doc, ponto, tolerancia, aditivo) {
            return;
        }
        match &mut self.ferramenta {
            Ferramenta::Linha { primeiro } => {
                if let Some(a) = *primeiro {
                    doc.adicionar_linha(a, ponto);
                }
                *primeiro = Some(ponto);
            }
            Ferramenta::Polilinha { pontos } => pontos.push(ponto),
            Ferramenta::Circulo { centro } => match *centro {
                None => *centro = Some(ponto),
                Some(c) => {
                    doc.adicionar_circulo(c, c.distancia(ponto));
                    self.ferramenta = Ferramenta::Nenhuma;
                }
            },
            Ferramenta::Arco { pontos } => {
                pontos.push(ponto);
                if let [p1, p2, p3] = pontos.as_slice() {
                    if let Some(a) = arco_por_3_pontos(*p1, *p2, *p3) {
                        doc.adicionar_arco(a.centro, a.raio, a.inicio, a.fim);
                        self.ferramenta = Ferramenta::Nenhuma;
                    } else {
                        // Alinhados: não há arco. Descarta o último ponto e pede outro.
                        pontos.pop();
                        self.dizer(Mensagem::nova("arco.alinhados"));
                    }
                }
            }
            Ferramenta::Insercao { bloco: Some(b) } => {
                doc.adicionar_insercao(*b, ponto);
                self.ferramenta = Ferramenta::Nenhuma;
            }
            _ => {}
        }
    }

    /// Texto digitado na linha de comando (Enter).
    pub fn digitar(&mut self, doc: &mut Documento, texto: &str) {
        let texto = texto.trim();
        if !texto.is_empty() {
            self.registrar(Entrada::Digitado(texto.to_string()));
        }
        // `<45`: trava a direção (como no AutoCAD); a próxima distância ou clique sai nesse ângulo.
        if let (Some(resto), Some(_)) = (texto.strip_prefix('<'), self.base()) {
            if let Some(graus) = self.regional.ler_numero(resto) {
                self.angulo_travado = Some(graus);
                let g = self.regional.numero_com(graus, 2);
                self.dizer(Mensagem::nova("comando.anguloTravado").com("graus", &g));
                return;
            }
        }
        if self.digitar_de_edicao(doc, texto) {
            return;
        }
        match self.ferramenta {
            Ferramenta::Linha { .. } => self.digitar_na_linha(doc, texto),
            Ferramenta::Polilinha { .. } => self.digitar_na_polilinha(doc, texto),
            Ferramenta::Circulo { .. } => self.digitar_no_circulo(doc, texto),
            Ferramenta::Arco { .. } => self.digitar_no_arco(doc, texto),
            Ferramenta::Insercao { .. } => self.digitar_na_insercao(doc, texto),
            Ferramenta::Nenhuma if !texto.is_empty() => self.comando_digitado(doc, texto),
            _ => {}
        }
    }

    /// Com a ferramenta Linha ativa: Enter termina; coordenada vira ponto.
    fn digitar_na_linha(&mut self, doc: &mut Documento, texto: &str) {
        if texto.is_empty() {
            self.ferramenta = Ferramenta::Nenhuma;
        } else if let Some(p) = self.ler_ponto(texto) {
            self.clique(doc, p);
        } else if self.comandos.resolver(texto).is_some() {
            self.comando_digitado(doc, texto);
        } else {
            self.ponto_invalido(texto);
        }
    }

    /// Com o círculo: sem centro, coordenada; com centro, o **raio** digitado ou um ponto.
    fn digitar_no_circulo(&mut self, doc: &mut Documento, texto: &str) {
        let centro = self.base();
        if texto.is_empty() {
            self.ferramenta = Ferramenta::Nenhuma;
        } else if let (Some(c), Some(raio)) = (centro, self.ler_distancia(texto)) {
            doc.adicionar_circulo(c, raio);
            self.ferramenta = Ferramenta::Nenhuma;
        } else if let Some(p) = self.ler_ponto(texto) {
            self.clique(doc, p);
        } else if self.comandos.resolver(texto).is_some() {
            self.comando_digitado(doc, texto);
        } else {
            self.ponto_invalido(texto);
        }
    }

    /// Com o inserir: sem bloco escolhido, o **nome** do bloco; com ele, coordenadas viram o ponto.
    fn digitar_na_insercao(&mut self, doc: &mut Documento, texto: &str) {
        let escolhido = matches!(self.ferramenta, Ferramenta::Insercao { bloco: Some(_) });
        if texto.is_empty() {
            self.ferramenta = Ferramenta::Nenhuma;
        } else if !escolhido {
            match doc.bloco_por_nome(texto) {
                Some(b) => self.ferramenta = Ferramenta::Insercao { bloco: Some(b) },
                None => self.dizer(Mensagem::nova("insercao.desconhecido").com("nome", texto)),
            }
        } else if let Some(p) = self.ler_ponto(texto) {
            self.clique(doc, p);
        } else {
            self.ponto_invalido(texto);
        }
    }

    /// Com o arco: coordenadas viram pontos; Enter cancela.
    fn digitar_no_arco(&mut self, doc: &mut Documento, texto: &str) {
        if texto.is_empty() {
            self.ferramenta = Ferramenta::Nenhuma;
        } else if let Some(p) = self.ler_ponto(texto) {
            self.clique(doc, p);
        } else if self.comandos.resolver(texto).is_some() {
            self.comando_digitado(doc, texto);
        } else {
            self.ponto_invalido(texto);
        }
    }

    /// Com a ferramenta Polilinha ativa: Enter termina, `Fechar` fecha, coordenada vira vértice.
    fn digitar_na_polilinha(&mut self, doc: &mut Documento, texto: &str) {
        if texto.is_empty() {
            self.terminar_polilinha(doc, false);
        } else if self.comandos.resolver_opcao(texto) == Some("fechar") {
            if self.pontos_da_previa().len() >= 3 {
                self.terminar_polilinha(doc, true);
            } else {
                self.dizer(Mensagem::nova("polilinha.poucosPontos"));
            }
        } else if let Some(p) = self.ler_ponto(texto) {
            self.clique(doc, p);
        } else if self.comandos.resolver(texto).is_some() {
            self.terminar_polilinha(doc, false);
            self.comando_digitado(doc, texto);
        } else {
            self.ponto_invalido(texto);
        }
    }

    fn comando_digitado(&mut self, doc: &mut Documento, texto: &str) {
        match self.comandos.resolver(texto) {
            Some(c) => self.executar(doc, c),
            None => self.dizer(Mensagem::nova("comando.desconhecido").com("nome", texto)),
        }
    }

    /// Executa um comando pelo nome interno.
    pub fn executar(&mut self, doc: &mut Documento, comando: &str) {
        self.angulo_travado = None;
        if !matches!(comando, "desfazer" | "refazer" | "colar") {
            self.ultimo_comando = Some(comando.to_string());
        }
        if self.executar_edicao(doc, comando) {
            return;
        }
        match comando {
            "linha" => self.ferramenta = Ferramenta::Linha { primeiro: None },
            "polilinha" => self.ferramenta = Ferramenta::Polilinha { pontos: Vec::new() },
            "circulo" => self.ferramenta = Ferramenta::Circulo { centro: None },
            "arco" => self.ferramenta = Ferramenta::Arco { pontos: Vec::new() },
            "inserir" if doc.blocos().is_empty() => self.dizer(Mensagem::nova("insercao.semBlocos")),
            "inserir" => self.ferramenta = Ferramenta::Insercao { bloco: None },
            "desfazer" => {
                let chave = if doc.desfazer() { "desfazer.feito" } else { "desfazer.nada" };
                self.dizer(Mensagem::nova(chave));
            }
            "refazer" => {
                let chave = if doc.refazer() { "refazer.feito" } else { "refazer.nada" };
                self.dizer(Mensagem::nova(chave));
            }
            _ => {}
        }
    }

    /// Uma distância digitada, na unidade do desenho: `10` (já na unidade dela) ou com unidade
    /// (`10cm`, `2,5m`, `300mm`), convertida. Sem unidade no desenho, o sufixo não vale.
    pub fn ler_distancia(&self, texto: &str) -> Option<f64> {
        let t = texto.trim().to_lowercase();
        for (sufixo, u) in [("mm", Unidade::Milimetro), ("cm", Unidade::Centimetro), ("m", Unidade::Metro)] {
            if let Some(numero) = t.strip_suffix(sufixo) {
                let valor = self.regional.ler_numero(numero.trim_end())?;
                return Some(valor * u.em_milimetros() / self.unidade?.em_milimetros());
            }
        }
        self.regional.ler_numero(&t)
    }

    /// O ponto que uma distância digitada marca: a partir da base, na direção do cursor (como no
    /// AutoCAD). `None` se não for distância, se não há base ou se o cursor está em cima da base.
    fn ponto_da_distancia(&self, texto: &str) -> Option<Ponto> {
        let d = self.ler_distancia(texto)?;
        let b = self.base()?;
        if let Some(graus) = self.angulo_travado {
            // Direção travada com `<ângulo`: o lado (para a frente ou para trás) é o que o cursor está indicando.
            let (s, c) = graus.to_radians().sin_cos();
            let frente = self.cursor.map_or(true, |m| (m.x - b.x) * c + (m.y - b.y) * s >= 0.0);
            let d = if frente { d } else { -d };
            return Some(Ponto::novo(b.x + d * c, b.y + d * s));
        }
        let c = self.cursor?;
        let total = b.distancia(c);
        (total > 1e-9).then(|| Ponto::novo(b.x + (c.x - b.x) / total * d, b.y + (c.y - b.y) / total * d))
    }

    /// Diz que o texto não é ponto; se era uma distância sem direção, diz o que falta.
    pub(crate) fn ponto_invalido(&mut self, texto: &str) {
        let chave = if self.base().is_some() && self.ler_distancia(texto).is_some() { "comando.semDirecao" } else { "comando.pontoInvalido" };
        self.dizer(Mensagem::nova(chave).com("texto", texto));
    }

    /// `x;y` (absoluto) ou `@x;y` (relativo ao ponto anterior), na língua atual; ou uma distância solta.
    pub fn ler_ponto(&self, texto: &str) -> Option<Ponto> {
        let (relativo, resto) = match texto.strip_prefix('@') {
            Some(r) => (true, r),
            None => (false, texto),
        };
        // Polar, como no AutoCAD: `10<45` (da origem) ou `@10<45` (do ponto anterior); o ângulo é anti-horário a partir do eixo X.
        if let Some((d, a)) = resto.split_once('<') {
            let dist = self.ler_distancia(d)?;
            let rad = self.regional.ler_numero(a)?.to_radians();
            let limpo = |v: f64| if v.abs() < dist.abs() * 1e-12 { 0.0 } else { v };
            let (dx, dy) = (limpo(dist * rad.cos()), limpo(dist * rad.sin()));
            return if relativo { self.base().map(|b| Ponto::novo(b.x + dx, b.y + dy)) } else { Some(Ponto::novo(dx, dy)) };
        }
        // Sem separador de coordenadas: um número solto é distância, na direção do cursor.
        let Some((sx, sy)) = resto.split_once(self.regional.separador_de_coordenadas) else {
            return if relativo { None } else { self.ponto_da_distancia(texto) };
        };
        let (x, y) = (self.regional.ler_numero(sx)?, self.regional.ler_numero(sy)?);
        if relativo {
            let b = self.base()?;
            Some(Ponto::novo(b.x + x, b.y + y))
        } else {
            Some(Ponto::novo(x, y))
        }
    }
}

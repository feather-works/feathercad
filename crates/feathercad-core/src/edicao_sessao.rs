//! A sessão de edição: seleção (clique e janela) e os comandos que agem sobre ela (apagar, mover,
//! copiar, girar, escala, espelhar, esticar, matriz), mais aparar, concordância, retângulo, elipse
//! e hachura. É do Rust: a interface só manda clique, Enter e texto e desenha o que volta.

use std::collections::BTreeSet;

use crate::aparar::{aparar, concordar, hachura_com_padrao, Recusa};
use crate::deslocar::deslocar;
use crate::estender::estender;
use crate::documento::{Documento, Geometria, Id, Mudanca, NovaEntidade};
use crate::edicao::{distancia, elipse, escolher, escolher_na_janela, esticar, matriz, retangulo, transformar};
use crate::geom::{Ponto, Transf};
use crate::sessao::{Ferramenta, Mensagem, Sessao};

/// Os comandos que agem sobre objetos escolhidos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdEdicao {
    Apagar,
    Mover,
    Copiar,
    Girar,
    Escala,
    Espelhar,
    Esticar,
    Matriz,
    /// Copiar para a prancheta (Ctrl+C).
    CopiarArea,
    /// Recortar para a prancheta (Ctrl+X).
    Recortar,
    /// Levar os objetos para a camada de outro objeto.
    IgualarCamada,
    /// Criar um bloco com os objetos.
    CriarBloco,
    /// Agrupar (ou desagrupar).
    Agrupar,
}

impl CmdEdicao {
    /// O nome interno (o mesmo do comando).
    pub fn nome(self) -> &'static str {
        match self {
            CmdEdicao::Apagar => "apagar",
            CmdEdicao::Mover => "mover",
            CmdEdicao::Copiar => "copiar",
            CmdEdicao::Girar => "girar",
            CmdEdicao::Escala => "escala",
            CmdEdicao::Espelhar => "espelhar",
            CmdEdicao::Esticar => "esticar",
            CmdEdicao::Matriz => "matriz",
            CmdEdicao::CopiarArea => "copiar_area",
            CmdEdicao::Recortar => "recortar",
            CmdEdicao::IgualarCamada => "igualar_camada",
            CmdEdicao::CriarBloco => "criar_bloco",
            CmdEdicao::Agrupar => "agrupar",
        }
    }

    fn de_nome(nome: &str) -> Option<Self> {
        [Self::Apagar, Self::Mover, Self::Copiar, Self::Girar, Self::Escala, Self::Espelhar, Self::Esticar, Self::Matriz, Self::CopiarArea, Self::Recortar, Self::IgualarCamada, Self::CriarBloco, Self::Agrupar]
            .into_iter()
            .find(|c| c.nome() == nome)
    }
}

/// Em que ponto do comando a sessão está.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Etapa {
    /// Escolhendo os objetos (clique ou janela); Enter confirma.
    Objetos,
    /// Pedindo o ponto base.
    Base,
    /// Base marcada; pedindo o destino (ou o ângulo, ou o fator).
    Destino(Ponto),
    /// Primeiro ponto da linha de espelho.
    Espelho1,
    /// Segundo ponto da linha de espelho.
    Espelho2(Ponto),
    /// Digitar linhas, colunas e os espaços.
    Matriz,
    /// Clicar num objeto da camada de destino (igualar camada).
    Alvo,
    /// Base marcada; digitar o nome do bloco.
    Nome(Ponto),
}

/// No máximo tantos objetos aparecem na prévia (mover 5 mil objetos não pode travar o cursor).
const PREVIA_MAXIMA: usize = 300;

impl Sessao {
    /// Quantos objetos estão selecionados.
    pub fn selecionados(&self) -> usize {
        self.selecao.len()
    }

    /// Há o primeiro canto de uma janela de seleção marcado?
    pub fn janela_pendente(&self) -> bool {
        self.janela.is_some()
    }

    /// Os ids selecionados.
    pub fn selecao(&self) -> &BTreeSet<Id> {
        &self.selecao
    }

    /// Um número que muda a cada mudança da seleção (o quadro precisa saber para refazer o realce).
    pub fn revisao_da_selecao(&self) -> u64 {
        self.rev_selecao
    }

    pub(crate) fn podar_selecao(&mut self, doc: &Documento) {
        let antes = self.selecao.len();
        self.selecao.retain(|id| doc.entidade(*id).is_some_and(|e| doc.visivel(e) && !doc.bloqueada(e)));
        if self.selecao.len() != antes {
            self.rev_selecao += 1;
        }
    }

    pub(crate) fn limpar_selecao(&mut self) {
        if !self.selecao.is_empty() {
            self.selecao.clear();
            self.rev_selecao += 1;
        }
        self.janela = None;
    }

    fn dizer_selecionados(&mut self) {
        let n = self.selecao.len().to_string();
        self.dizer(Mensagem::nova("edicao.selecionados").com("n", &n));
    }

    /// Clique escolhendo objetos: no objeto (troca a seleção; com Shift alterna), no vazio (abre uma janela;
    /// o clique seguinte a fecha — da esquerda para a direita só pega o que cabe inteiro, da direita para a
    /// esquerda pega o que toca). `so_janela` ignora o objeto sob o cursor (esticar).
    fn selecionar_por_clique(&mut self, doc: &Documento, ponto: Ponto, tolerancia: f64, aditivo: bool, so_janela: bool) {
        if let Some(canto) = self.janela.take() {
            let cruzando = so_janela || ponto.x < canto.x;
            let ids = escolher_na_janela(doc, canto, ponto, cruzando);
            self.ultima_janela = Some((Ponto::novo(canto.x.min(ponto.x), canto.y.min(ponto.y)), Ponto::novo(canto.x.max(ponto.x), canto.y.max(ponto.y))));
            if !aditivo {
                self.selecao.clear();
            }
            self.selecao.extend(ids);
            self.expandir_grupos(doc);
            self.rev_selecao += 1;
            self.dizer_selecionados();
            return;
        }
        if !so_janela {
            if let Some(id) = escolher(doc, ponto, tolerancia) {
                let grupo = doc.entidade(id).and_then(|e| e.props.grupo);
                if aditivo && self.selecao.contains(&id) {
                    // Tirar um membro de um grupo tira o grupo todo.
                    self.selecao.remove(&id);
                    if let Some(g) = grupo {
                        self.selecao.retain(|o| doc.entidade(*o).map_or(true, |e| e.props.grupo != Some(g)));
                    }
                } else {
                    if !aditivo {
                        self.selecao.clear();
                    }
                    self.selecao.insert(id);
                    self.expandir_grupos(doc);
                }
                self.rev_selecao += 1;
                return;
            }
        }
        self.janela = Some(ponto);
    }

    pub(crate) fn base_da_edicao(&self, f: &Ferramenta) -> Option<Ponto> {
        match f {
            Ferramenta::Editar { etapa: Etapa::Destino(b) | Etapa::Espelho2(b), .. } => Some(*b),
            Ferramenta::Retangulo { canto } => *canto,
            Ferramenta::Elipse { pontos } => pontos.last().copied(),
            Ferramenta::Editar { etapa: Etapa::Nome(b), .. } => Some(*b),
            outra => self.base_da_anotacao(outra),
        }
    }

    pub(crate) fn prompt_da_edicao(&self, f: &Ferramenta) -> Mensagem {
        let chave = match f {
            Ferramenta::Editar { cmd, etapa } => match etapa {
                Etapa::Objetos if *cmd == CmdEdicao::Esticar => if self.janela.is_some() { "edicao.janelaSegundo" } else { "edicao.esticarJanela" },
                Etapa::Objetos => if self.janela.is_some() { "edicao.janelaSegundo" } else { "edicao.selecionar" },
                Etapa::Base => "edicao.base",
                Etapa::Destino(_) => match cmd {
                    CmdEdicao::Girar => "edicao.angulo",
                    CmdEdicao::Escala => "edicao.fator",
                    _ => "edicao.destino",
                },
                Etapa::Espelho1 => "edicao.espelho1",
                Etapa::Espelho2(_) => "edicao.espelho2",
                Etapa::Matriz => "edicao.matriz",
                Etapa::Alvo => "edicao.alvoCamada",
                Etapa::Nome(_) => "bloco.nome",
            },
            Ferramenta::Aparar => "aparar.clique",
            Ferramenta::Estender => "estender.clique",
            Ferramenta::Concordancia { primeira: None } => {
                return Mensagem::nova("concordancia.primeira").com("raio", &self.regional.numero(self.raio_concordancia));
            }
            Ferramenta::Concordancia { primeira: Some(_) } => "concordancia.segunda",
            Ferramenta::Deslocar { alvo: None } => {
                return Mensagem::nova("deslocar.objeto").com("distancia", &self.regional.numero(self.distancia_deslocar));
            }
            Ferramenta::Deslocar { alvo: Some(_) } => "deslocar.lado",
            Ferramenta::Retangulo { canto: None } => "retangulo.primeiroCanto",
            Ferramenta::Retangulo { canto: Some(_) } => "retangulo.outroCanto",
            Ferramenta::Elipse { pontos } => match pontos.len() {
                0 => "elipse.centro",
                1 => "elipse.eixo",
                _ => "elipse.outroEixo",
            },
            Ferramenta::Hachura => return Mensagem::nova(if self.hachura_solida { "hachura.pontoSolida" } else { "hachura.pontoPadrao" }),
            Ferramenta::Contar => "contar.clique",
            outra => return self.prompt_da_anotacao(outra),
        };
        Mensagem::nova(chave)
    }

    /// As formas da prévia de edição: a janela sendo marcada, os objetos onde ficariam, a linha elástica.
    pub(crate) fn previa_da_edicao(&self, doc: &Documento, cursor: Ponto) -> Vec<Geometria> {
        let janela = |canto: Ponto| vec![retangulo(canto, cursor)];
        match &self.ferramenta {
            Ferramenta::Nenhuma => match self.arrasto {
                // Durante o arrasto os objetos vêm por `quadro_arrasto` (desenho completo, na posição nova):
                // a prévia só traça a linha da pegada até o cursor.
                Some(b) => vec![Geometria::Linha { a: b, b: cursor }],
                None => self.janela.map(janela).unwrap_or_default(),
            },
            Ferramenta::Editar { etapa: Etapa::Objetos, .. } => self.janela.map(janela).unwrap_or_default(),
            Ferramenta::Editar { cmd, etapa: Etapa::Destino(b) } => {
                let mut v = vec![Geometria::Linha { a: *b, b: cursor }];
                match cmd {
                    CmdEdicao::Esticar => {
                        if let Some(j) = self.ultima_janela {
                            v.extend(self.geometrias_da_selecao(doc).iter().filter_map(|g| esticar(g, j, cursor.x - b.x, cursor.y - b.y)));
                        }
                    }
                    _ => {
                        if let Some(t) = transformacao_do_destino(*cmd, *b, cursor) {
                            v.extend(self.geometrias_da_selecao(doc).iter().map(|g| transformar(g, &t)));
                        }
                    }
                }
                v
            }
            Ferramenta::Editar { etapa: Etapa::Espelho2(a), .. } => {
                let mut v = vec![Geometria::Linha { a: *a, b: cursor }];
                if let Some(t) = Transf::espelhar(*a, cursor) {
                    v.extend(self.geometrias_da_selecao(doc).iter().map(|g| transformar(g, &t)));
                }
                v
            }
            Ferramenta::Deslocar { alvo: Some(id) } => doc
                .entidade(*id)
                .and_then(|e| self.deslocada(doc, &e.geometria, cursor))
                .into_iter()
                .collect(),
            Ferramenta::Retangulo { canto: Some(c) } => vec![retangulo(*c, cursor)],
            Ferramenta::Elipse { pontos } => match pontos.as_slice() {
                [c] => vec![Geometria::Linha { a: *c, b: cursor }],
                [c, ponta] => elipse(*c, *ponta, distancia_ao_eixo(*c, *ponta, cursor)).into_iter().collect(),
                _ => Vec::new(),
            },
            outra => self.previa_da_anotacao(doc, outra, cursor),
        }
    }

    fn geometrias_da_selecao(&self, doc: &Documento) -> Vec<Geometria> {
        self.selecao.iter().take(PREVIA_MAXIMA).filter_map(|id| doc.entidade(*id)).map(|e| e.geometria.clone()).collect()
    }

    /// Um comando de edição ou de desenho de forma pronta. `false` se o nome não é de nenhum deles.
    pub(crate) fn executar_edicao(&mut self, doc: &mut Documento, comando: &str) -> bool {
        if let Some(cmd) = CmdEdicao::de_nome(comando) {
            self.iniciar_edicao(doc, cmd);
            return true;
        }
        self.janela = None;
        match comando {
            "aparar" => self.ferramenta = Ferramenta::Aparar,
            "estender" => self.ferramenta = Ferramenta::Estender,
            "concordancia" => self.ferramenta = Ferramenta::Concordancia { primeira: None },
            "deslocar" => {
                self.limpar_selecao();
                self.ferramenta = Ferramenta::Deslocar { alvo: None };
            }
            "retangulo" => self.ferramenta = Ferramenta::Retangulo { canto: None },
            "elipse" => self.ferramenta = Ferramenta::Elipse { pontos: Vec::new() },
            "hachura" => self.ferramenta = Ferramenta::Hachura,
            "contar" => self.ferramenta = Ferramenta::Contar,
            outro => return self.executar_anotacao(doc, outro),
        }
        true
    }

    /// Começa um comando sobre objetos. Com objetos já selecionados, pula a escolha (como o AutoCAD).
    fn iniciar_edicao(&mut self, doc: &mut Documento, cmd: CmdEdicao) {
        self.podar_selecao(doc);
        self.janela = None;
        // Esticar sempre pede a janela; os outros aproveitam a seleção que já existe.
        if cmd == CmdEdicao::Esticar {
            self.limpar_selecao();
            self.ferramenta = Ferramenta::Editar { cmd, etapa: Etapa::Objetos };
        } else if self.selecao.is_empty() {
            self.ferramenta = Ferramenta::Editar { cmd, etapa: Etapa::Objetos };
        } else {
            self.depois_da_selecao(doc, cmd);
        }
    }

    /// Objetos escolhidos: segue para a próxima etapa do comando (ou apaga, que não pede mais nada).
    fn depois_da_selecao(&mut self, doc: &mut Documento, cmd: CmdEdicao) {
        self.podar_selecao(doc);
        if self.selecao.is_empty() {
            self.dizer(Mensagem::nova("edicao.nadaSelecionado"));
            self.ferramenta = Ferramenta::Nenhuma;
            return;
        }
        if cmd == CmdEdicao::Esticar && self.ultima_janela.is_none() {
            self.dizer(Mensagem::nova("edicao.esticarJanela"));
            return;
        }
        let etapa = match cmd {
            CmdEdicao::Apagar => return self.apagar(doc),
            CmdEdicao::Espelhar => Etapa::Espelho1,
            CmdEdicao::Matriz => Etapa::Matriz,
            CmdEdicao::CopiarArea | CmdEdicao::Recortar | CmdEdicao::Agrupar | CmdEdicao::IgualarCamada => match self.depois_da_selecao_de_anotacao(doc, cmd) {
                Some(e) => e,
                None => return,
            },
            _ => Etapa::Base,
        };
        self.ferramenta = Ferramenta::Editar { cmd, etapa };
    }

    /// O botão do mouse desceu sobre um objeto sem comando ativo: começa o **arrasto** (o "grab").
    /// Se o objeto não estava selecionado, passa a ser a seleção; se estava, leva junto tudo o que
    /// está selecionado. `false` se não há objeto ali (aí vale o clique de sempre: janela de seleção).
    pub fn arrasto_iniciar(&mut self, doc: &Documento, ponto: Ponto, tolerancia: f64) -> bool {
        if self.ferramenta != Ferramenta::Nenhuma {
            return false;
        }
        self.podar_selecao(doc);
        let Some(id) = escolher(doc, ponto, tolerancia) else { return false };
        if !self.selecao.contains(&id) {
            self.selecao.clear();
            self.selecao.insert(id);
            self.rev_selecao += 1;
        }
        self.arrasto = Some(ponto);
        true
    }

    /// Soltou o botão: os objetos vão para onde o mouse está (um passo só de desfazer). A seleção fica.
    pub fn arrasto_fim(&mut self, doc: &mut Documento, ponto: Ponto) {
        let Some(base) = self.arrasto.take() else { return };
        if base.distancia(ponto) > 0.0 {
            self.aplicar_transformacao(doc, &Transf::mover(ponto.x - base.x, ponto.y - base.y), false, false);
        }
    }

    /// Quanto o arrasto já andou até `ponto` (`None` se não há arrasto).
    pub fn deslocamento_do_arrasto(&self, ponto: Ponto) -> Option<(f64, f64)> {
        self.arrasto.map(|b| (ponto.x - b.x, ponto.y - b.y))
    }

    /// A janela de seleção que está aberta e como ela pega: `Some(true)` = **cruzando** (da direita para
    /// a esquerda, ou Esticar: basta tocar), `Some(false)` = **janela** (da esquerda para a direita:
    /// só o que cabe inteiro). `None` se não há primeiro canto marcado.
    pub fn tipo_da_janela(&self, cursor: Ponto) -> Option<bool> {
        let canto = self.janela?;
        let so_janela = matches!(&self.ferramenta, Ferramenta::Editar { cmd: CmdEdicao::Esticar, etapa: Etapa::Objetos });
        Some(so_janela || cursor.x < canto.x)
    }

    /// Há um objeto sendo arrastado?
    pub fn arrastando(&self) -> bool {
        self.arrasto.is_some()
    }

    /// O arrasto virou um clique (o mouse quase não andou): não move nada.
    pub fn arrasto_cancelar(&mut self) {
        self.arrasto = None;
    }

    fn apagar(&mut self, doc: &mut Documento) {
        let ms: Vec<Mudanca> = self.selecao.iter().map(|id| Mudanca::Remover(*id)).collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova("edicao.apagados").com("n", &n));
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Aplica `t` à seleção: troca as formas no lugar, ou (com `copiar`) cria cópias transformadas.
    /// `encerrar` termina o comando e limpa a seleção; senão o comando segue (copiar repete).
    fn aplicar_transformacao(&mut self, doc: &mut Documento, t: &Transf, copiar: bool, encerrar: bool) {
        self.podar_selecao(doc);
        let ms: Vec<Mudanca> = self
            .selecao
            .iter()
            .filter_map(|id| doc.entidade(*id))
            .map(|e| {
                let g = transformar(&e.geometria, t);
                if copiar {
                    Mudanca::Novo(NovaEntidade { camada: e.camada, cor: e.cor, geometria: g, props: Default::default() })
                } else {
                    Mudanca::Trocar(e.id, g)
                }
            })
            .collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova(if copiar { "edicao.copiados" } else { "edicao.alterados" }).com("n", &n));
        if encerrar {
            self.limpar_selecao();
            self.ferramenta = Ferramenta::Nenhuma;
        }
    }

    fn esticar_selecao(&mut self, doc: &mut Documento, dx: f64, dy: f64) {
        let Some(janela) = self.ultima_janela else { return };
        self.podar_selecao(doc);
        let ms: Vec<Mudanca> = self.selecao.iter().filter_map(|id| doc.entidade(*id).and_then(|e| esticar(&e.geometria, janela, dx, dy).map(|g| Mudanca::Trocar(e.id, g)))).collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova("edicao.alterados").com("n", &n));
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    fn aplicar_matriz(&mut self, doc: &mut Documento, texto: &str) {
        let partes: Vec<&str> = texto.split(|c: char| c.is_whitespace() || c == ';').filter(|s| !s.is_empty()).collect();
        let numeros: Vec<f64> = partes.iter().filter_map(|s| self.regional.ler_numero(s)).collect();
        let (linhas, colunas) = (numeros.first().copied().unwrap_or(0.0), numeros.get(1).copied().unwrap_or(0.0));
        if numeros.len() != 4 || partes.len() != 4 || linhas < 1.0 || colunas < 1.0 || linhas.fract() != 0.0 || colunas.fract() != 0.0 {
            self.dizer(Mensagem::nova("edicao.matrizInvalida"));
            return;
        }
        self.podar_selecao(doc);
        let originais: Vec<_> = self.selecao.iter().filter_map(|id| doc.entidade(*id)).cloned().collect();
        let refs: Vec<&_> = originais.iter().collect();
        let ms: Vec<Mudanca> = matriz(&refs, linhas as usize, colunas as usize, numeros[2], numeros[3])
            .into_iter()
            .map(|(k, g)| Mudanca::Novo(NovaEntidade { camada: originais[k].camada, cor: originais[k].cor, geometria: g, props: Default::default() }))
            .collect();
        let n = ms.len().to_string();
        doc.aplicar_mudancas(ms);
        self.dizer(Mensagem::nova("edicao.copiados").com("n", &n));
        self.limpar_selecao();
        self.ferramenta = Ferramenta::Nenhuma;
    }

    /// Clique quando a ferramenta é de edição (ou nenhuma: escolher objetos). `false` se a ferramenta é outra.
    pub(crate) fn clique_de_edicao(&mut self, doc: &mut Documento, ponto: Ponto, tolerancia: f64, aditivo: bool) -> bool {
        match self.ferramenta.clone() {
            Ferramenta::Nenhuma => self.selecionar_por_clique(doc, ponto, tolerancia, aditivo, false),
            Ferramenta::Editar { cmd, etapa } => self.clique_em_etapa(doc, cmd, etapa, ponto, tolerancia, aditivo),
            Ferramenta::Aparar => self.clique_aparar(doc, ponto, tolerancia),
            Ferramenta::Estender => self.clique_estender(doc, ponto, tolerancia),
            Ferramenta::Concordancia { primeira } => self.clique_concordancia(doc, primeira, ponto, tolerancia),
            Ferramenta::Deslocar { alvo } => self.clique_deslocar(doc, alvo, ponto, tolerancia),
            Ferramenta::Retangulo { canto: None } => self.ferramenta = Ferramenta::Retangulo { canto: Some(ponto) },
            Ferramenta::Retangulo { canto: Some(a) } => {
                if a.x == ponto.x || a.y == ponto.y {
                    self.dizer(Mensagem::nova("retangulo.invalido"));
                } else {
                    doc.adicionar_geometria(retangulo(a, ponto));
                    self.ferramenta = Ferramenta::Nenhuma;
                }
            }
            Ferramenta::Elipse { mut pontos } => {
                pontos.push(ponto);
                if let [c, ponta, fim] = pontos.as_slice() {
                    match elipse(*c, *ponta, distancia_ao_eixo(*c, *ponta, *fim)) {
                        Some(g) => {
                            doc.adicionar_geometria(g);
                            self.ferramenta = Ferramenta::Nenhuma;
                        }
                        None => {
                            self.dizer(Mensagem::nova("elipse.invalida"));
                            pontos.pop();
                            self.ferramenta = Ferramenta::Elipse { pontos };
                        }
                    }
                } else {
                    self.ferramenta = Ferramenta::Elipse { pontos };
                }
            }
            Ferramenta::Hachura => match hachura_com_padrao(doc, ponto, if self.hachura_solida { crate::padroes::SOLIDA } else { &self.hachura_padrao }, self.hachura_escala) {
                Ok(g) => {
                    doc.adicionar_geometria(g);
                }
                Err(_) => self.dizer(Mensagem::nova("hachura.semContorno")),
            },
            Ferramenta::Contar => match escolher(doc, ponto, tolerancia).and_then(|id| crate::contagem::iguais_a(doc, id)) {
                Some(c) => {
                    let n = c.ids.len().to_string();
                    let mut camadas: Vec<String> = c.por_camada.iter().take(4).map(|(nome, q)| format!("{nome}: {q}")).collect();
                    if c.por_camada.len() > 4 {
                        camadas.push("…".into());
                    }
                    self.selecao = c.ids.into_iter().collect();
                    self.rev_selecao += 1;
                    self.dizer(Mensagem::nova("contar.resultado").com("n", &n).com("camadas", &camadas.join("; ")));
                }
                None => self.dizer(Mensagem::nova("contar.nenhum")),
            },
            outra => return self.clique_de_anotacao(doc, outra, ponto, tolerancia),
        }
        true
    }

    fn clique_em_etapa(&mut self, doc: &mut Documento, cmd: CmdEdicao, etapa: Etapa, ponto: Ponto, tolerancia: f64, aditivo: bool) {
        match etapa {
            Etapa::Objetos => self.selecionar_por_clique(doc, ponto, tolerancia, aditivo, cmd == CmdEdicao::Esticar),
            Etapa::Base if cmd == CmdEdicao::CriarBloco => self.ferramenta = Ferramenta::Editar { cmd, etapa: Etapa::Nome(ponto) },
            Etapa::Base => self.ferramenta = Ferramenta::Editar { cmd, etapa: Etapa::Destino(ponto) },
            Etapa::Destino(base) => self.destino(doc, cmd, base, ponto),
            Etapa::Espelho1 => self.ferramenta = Ferramenta::Editar { cmd, etapa: Etapa::Espelho2(ponto) },
            Etapa::Espelho2(a) => match Transf::espelhar(a, ponto) {
                Some(t) => self.aplicar_transformacao(doc, &t, true, true),
                None => self.dizer(Mensagem::nova("edicao.espelhoInvalido")),
            },
            Etapa::Matriz => self.dizer(Mensagem::nova("edicao.matriz")),
            Etapa::Alvo => self.clique_alvo_da_camada(doc, ponto, tolerancia),
            Etapa::Nome(_) => self.dizer(Mensagem::nova("bloco.nome")),
        }
    }

    fn destino(&mut self, doc: &mut Documento, cmd: CmdEdicao, base: Ponto, ponto: Ponto) {
        match cmd {
            CmdEdicao::Esticar => self.esticar_selecao(doc, ponto.x - base.x, ponto.y - base.y),
            CmdEdicao::Copiar => self.aplicar_transformacao(doc, &Transf::mover(ponto.x - base.x, ponto.y - base.y), true, false),
            _ => match transformacao_do_destino(cmd, base, ponto) {
                Some(t) => self.aplicar_transformacao(doc, &t, false, true),
                None => self.dizer(Mensagem::nova("edicao.destinoInvalido")),
            },
        }
    }

    fn clique_aparar(&mut self, doc: &mut Documento, ponto: Ponto, tolerancia: f64) {
        let Some(id) = escolher(doc, ponto, tolerancia) else { return };
        match aparar(doc, id, ponto) {
            Ok(ms) => {
                doc.aplicar_mudancas(ms);
            }
            Err(r) => self.dizer(Mensagem::nova(match r {
                Recusa::SemCorte => "aparar.semCorte",
                _ => "aparar.naoSuporta",
            })),
        }
    }

    fn clique_estender(&mut self, doc: &mut Documento, ponto: Ponto, tolerancia: f64) {
        let Some(id) = escolher(doc, ponto, tolerancia) else { return };
        match estender(doc, id, ponto) {
            Ok(ms) => {
                doc.aplicar_mudancas(ms);
            }
            Err(r) => self.dizer(Mensagem::nova(match r {
                Recusa::SemCorte => "estender.semLimite",
                _ => "estender.naoSuporta",
            })),
        }
    }

    fn clique_concordancia(&mut self, doc: &mut Documento, primeira: Option<(Id, Ponto)>, ponto: Ponto, tolerancia: f64) {
        let Some(id) = escolher(doc, ponto, tolerancia) else { return };
        let Some(primeira) = primeira else {
            self.ferramenta = Ferramenta::Concordancia { primeira: Some((id, ponto)) };
            return;
        };
        match concordar(doc, primeira, (id, ponto), self.raio_concordancia) {
            Ok(ms) => {
                doc.aplicar_mudancas(ms);
                self.ferramenta = Ferramenta::Nenhuma;
            }
            Err(r) => {
                self.dizer(Mensagem::nova(match r {
                    Recusa::Paralelas => "concordancia.paralelas",
                    Recusa::RaioGrande => "concordancia.raioGrande",
                    _ => "concordancia.soLinhas",
                }));
                self.ferramenta = Ferramenta::Concordancia { primeira: None };
            }
        }
    }

    /// A cópia deslocada de `geo` para o lado do `cursor`. Distância 0 = passar pelo cursor.
    fn deslocada(&self, doc: &Documento, geo: &Geometria, cursor: Ponto) -> Option<Geometria> {
        self.deslocada_ou_recusa(doc, geo, cursor).ok()
    }

    fn deslocada_ou_recusa(&self, doc: &Documento, geo: &Geometria, cursor: Ponto) -> Result<Geometria, Recusa> {
        let d = if self.distancia_deslocar > 0.0 { self.distancia_deslocar } else { distancia(doc, geo, cursor) };
        deslocar(geo, d, cursor)
    }

    /// Primeiro clique: escolhe o objeto (fica realçado). Segundo: cria a cópia do lado clicado e volta a pedir objeto.
    fn clique_deslocar(&mut self, doc: &mut Documento, alvo: Option<Id>, ponto: Ponto, tolerancia: f64) {
        let Some(id) = alvo else {
            if let Some(id) = escolher(doc, ponto, tolerancia) {
                self.limpar_selecao();
                self.selecao.insert(id);
                self.rev_selecao += 1;
                self.ferramenta = Ferramenta::Deslocar { alvo: Some(id) };
            }
            return;
        };
        let Some(original) = doc.entidade(id).cloned() else {
            self.limpar_selecao();
            self.ferramenta = Ferramenta::Deslocar { alvo: None };
            return;
        };
        match self.deslocada_ou_recusa(doc, &original.geometria, ponto) {
            Ok(geometria) => {
                doc.aplicar_mudancas(vec![Mudanca::Novo(NovaEntidade { camada: original.camada, cor: original.cor, geometria, props: crate::documento::Propriedades { grupo: None, ..original.props.clone() } })]);
                self.limpar_selecao();
                self.ferramenta = Ferramenta::Deslocar { alvo: None };
            }
            Err(r) => self.dizer(Mensagem::nova(match r {
                Recusa::RaioGrande => "deslocar.naoCabe",
                _ => "deslocar.naoSuporta",
            })),
        }
    }

    /// Texto digitado quando a ferramenta é de edição. `false` se a ferramenta é outra.
    pub(crate) fn digitar_de_edicao(&mut self, doc: &mut Documento, texto: &str) -> bool {
        let ferramenta = self.ferramenta.clone();
        match ferramenta {
            Ferramenta::Editar { cmd, etapa } => self.digitar_em_etapa(doc, cmd, etapa, texto),
            Ferramenta::Aparar | Ferramenta::Estender | Ferramenta::Retangulo { .. } | Ferramenta::Elipse { .. } | Ferramenta::Concordancia { .. } | Ferramenta::Deslocar { .. } | Ferramenta::Hachura => self.digitar_em_ferramenta(doc, &ferramenta, texto),
            outra => return self.digitar_de_anotacao(doc, &outra, texto),
        }
        true
    }

    fn digitar_em_etapa(&mut self, doc: &mut Documento, cmd: CmdEdicao, etapa: Etapa, texto: &str) {
        if texto.is_empty() {
            match etapa {
                Etapa::Objetos => self.depois_da_selecao(doc, cmd),
                _ => {
                    // Enter no meio do comando (ou ao fim do copiar): encerra.
                    self.limpar_selecao();
                    self.ferramenta = Ferramenta::Nenhuma;
                }
            }
            return;
        }
        if etapa == Etapa::Matriz {
            return self.aplicar_matriz(doc, texto);
        }
        if let Etapa::Nome(base) = etapa {
            return self.criar_bloco(doc, base, texto);
        }
        if let Etapa::Destino(base) = etapa {
            // Ângulo e fator também se digitam, sem clicar.
            if let (Some(n), CmdEdicao::Girar | CmdEdicao::Escala) = (self.regional.ler_numero(texto), cmd) {
                let t = if cmd == CmdEdicao::Girar { Some(Transf::girar(base, n)) } else { (n > 0.0).then(|| Transf::escalar(base, n)) };
                match t {
                    Some(t) => self.aplicar_transformacao(doc, &t, false, true),
                    None => self.dizer(Mensagem::nova("edicao.destinoInvalido")),
                }
                return;
            }
        }
        self.digitar_ponto_ou_comando(doc, texto);
    }

    /// Coordenada digitada vira clique; nome de comando troca de comando; o resto é erro.
    fn digitar_ponto_ou_comando(&mut self, doc: &mut Documento, texto: &str) {
        if let Some(p) = self.ler_ponto(texto) {
            self.clique_de_edicao(doc, p, 0.0, false);
        } else if let Some(c) = self.comandos.resolver(texto) {
            self.executar(doc, c);
        } else {
            self.ponto_invalido(texto);
        }
    }

    fn digitar_em_ferramenta(&mut self, doc: &mut Documento, f: &Ferramenta, texto: &str) {
        if texto.is_empty() {
            self.ferramenta = Ferramenta::Nenhuma;
            return;
        }
        match f {
            Ferramenta::Concordancia { primeira: None } if self.regional.ler_numero(texto).is_some() => {
                let raio = self.regional.ler_numero(texto).unwrap_or(0.0);
                if raio >= 0.0 {
                    self.raio_concordancia = raio;
                } else {
                    self.dizer(Mensagem::nova("concordancia.raioNegativo"));
                }
            }
            Ferramenta::Deslocar { .. } if self.regional.ler_numero(texto).is_some() => {
                let d = self.regional.ler_numero(texto).unwrap_or(-1.0);
                if d >= 0.0 {
                    self.distancia_deslocar = d;
                } else {
                    self.dizer(Mensagem::nova("deslocar.negativa"));
                }
            }
            Ferramenta::Elipse { pontos } if pontos.len() == 2 && self.regional.ler_numero(texto).is_some() => {
                let menor = self.regional.ler_numero(texto).unwrap_or(0.0);
                match elipse(pontos[0], pontos[1], menor) {
                    Some(g) => {
                        doc.adicionar_geometria(g);
                        self.ferramenta = Ferramenta::Nenhuma;
                    }
                    None => self.dizer(Mensagem::nova("elipse.invalida")),
                }
            }
            Ferramenta::Hachura => match self.comandos.resolver_opcao(texto) {
                Some("solida") => self.hachura_solida = true,
                Some("padrao") => self.hachura_solida = false,
                // o nome de um padrão da biblioteca (CONCRETO, TERRA…) escolhe o padrão
                _ if crate::padroes::reconhecer(texto).is_some() => {
                    let nome = crate::padroes::reconhecer(texto).unwrap_or(crate::padroes::PADRAO_DE_FABRICA);
                    self.hachura_solida = nome == crate::padroes::SOLIDA;
                    if !self.hachura_solida {
                        self.hachura_padrao = nome.to_string();
                    }
                }
                _ => self.digitar_ponto_ou_comando(doc, texto),
            },
            _ => self.digitar_ponto_ou_comando(doc, texto),
        }
    }
}

/// A transformação de um destino (girar: ângulo do vetor base→cursor; escala: a distância; mover: o deslocamento).
fn transformacao_do_destino(cmd: CmdEdicao, base: Ponto, cursor: Ponto) -> Option<Transf> {
    match cmd {
        CmdEdicao::Mover | CmdEdicao::Copiar => Some(Transf::mover(cursor.x - base.x, cursor.y - base.y)),
        CmdEdicao::Girar => (base.distancia(cursor) > 0.0).then(|| Transf::girar(base, base.angulo_ate(cursor))),
        CmdEdicao::Escala => (base.distancia(cursor) > 0.0).then(|| Transf::escalar(base, base.distancia(cursor))),
        _ => None,
    }
}

/// A distância de `p` ao eixo maior da elipse (o outro semi-eixo).
fn distancia_ao_eixo(centro: Ponto, ponta: Ponto, p: Ponto) -> f64 {
    let (mx, my) = (ponta.x - centro.x, ponta.y - centro.y);
    let maior = mx.hypot(my);
    if maior < 1e-12 {
        return 0.0;
    }
    ((p.x - centro.x) * my - (p.y - centro.y) * mx).abs() / maior
}

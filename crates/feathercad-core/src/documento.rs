//! O desenho: camadas, entidades e o histórico de desfazer/refazer.

use serde::{Deserialize, Serialize};

use crate::geom::{Ponto, Transf, Vertice};

/// Identificador estável de uma entidade.
pub type Id = u64;

/// Uma camada. A cor é `0xRRGGBB`.
#[derive(Clone, Debug, PartialEq)]
pub struct Camada {
    pub nome: String,
    pub cor: u32,
    /// Ativada (a lâmpada): desligada não aparece, mas o objeto continua contando nos cálculos do desenho.
    pub ligada: bool,
    /// Congelada: some do desenho e das escolhas.
    pub congelada: bool,
    /// Bloqueada: aparece, mas os objetos não podem ser escolhidos nem editados.
    pub bloqueada: bool,
    /// Plotar: sai na impressão (ainda não há impressão; o valor só é guardado).
    pub plotar: bool,
    pub tipo_linha: TipoLinha,
    /// Espessura de linha em mm; `None` = padrão.
    pub espessura: Option<f32>,
    /// Transparência, de 0 (opaca) a 90.
    pub transparencia: u8,
}

impl Camada {
    /// Uma camada comum: ligada, livre, plotável, contínua, sem espessura própria.
    pub fn nova(nome: impl Into<String>, cor: u32) -> Self {
        Self { nome: nome.into(), cor, ligada: true, congelada: false, bloqueada: false, plotar: true, tipo_linha: TipoLinha::Continua, espessura: None, transparencia: 0 }
    }

    /// Aparece no desenho? (ligada e não congelada)
    pub fn visivel(&self) -> bool {
        self.ligada && !self.congelada
    }
}

/// Alinhamento horizontal do texto em relação ao ponto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlinhaH {
    Esquerda,
    Centro,
    Direita,
}

/// Alinhamento vertical do texto em relação ao ponto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlinhaV {
    Base,
    Baixo,
    Meio,
    Topo,
}

/// Uma família de linhas paralelas de um padrão de hachura, **já no tamanho e no giro finais**
/// (é assim que o DXF guarda). A linha `k` passa por `base + k·(dx, dy)` — o deslocamento está em
/// coordenadas do **mundo** — e segue a direção `angulo`. Tracejado não é guardado (linha contínua).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinhaDePadrao {
    pub angulo: f64,
    pub base: Ponto,
    pub dx: f64,
    pub dy: f64,
}

/// A forma de uma entidade.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometria {
    Linha { a: Ponto, b: Ponto },
    /// Segmentos retos e arcos (por *bulge*); `fechada` liga o último ao primeiro.
    Polilinha { vertices: Vec<Vertice>, fechada: bool },
    Circulo { centro: Ponto, raio: f64 },
    /// Arco de circunferência: do ângulo `inicio` ao `fim`, em graus, no sentido anti-horário.
    Arco { centro: Ponto, raio: f64, inicio: f64, fim: f64 },
    /// Uma cópia de um bloco (`Documento::blocos()[bloco]`) em `ponto`, com escala e giro (graus).
    /// Ponto (`POINT`): uma marca de tamanho fixo na tela.
    Ponto { p: Ponto },
    /// Hachura: região delimitada por `contornos` (par-ímpar: ilha dentro de laço fica vazada).
    /// `padrao` vazio = preenchimento sólido; senão, as famílias de linhas que a cobrem.
    Hachura { contornos: Vec<Vec<Vertice>>, padrao: Vec<LinhaDePadrao>, nome: String },
    Insercao { bloco: usize, ponto: Ponto, escala_x: f64, escala_y: f64, rotacao: f64 },
    /// Texto de uma linha. `rotacao` em graus; `altura` em unidades do desenho.
    Texto { ponto: Ponto, altura: f64, rotacao: f64, conteudo: String, h: AlinhaH, v: AlinhaV },
}

/// Largura aproximada de um caractere, em alturas (só para enquadrar e recortar).
pub const LARGURA_DO_CARACTERE: f64 = 0.6;

impl Geometria {
    /// Chama `f` em cada ponto onde o cursor pode grudar (extremidades, vértices, pontos de inserção).
    pub fn para_cada_ponto_notavel(&self, f: &mut impl FnMut(Ponto)) {
        match self {
            Geometria::Linha { a, b } => {
                f(*a);
                f(*b);
            }
            Geometria::Polilinha { vertices, .. } => vertices.iter().for_each(|v| f(v.p)),
            Geometria::Texto { ponto, .. } | Geometria::Insercao { ponto, .. } | Geometria::Ponto { p: ponto } => f(*ponto),
            // A hachura não é ponto de snap: o contorno já existe como entidade própria.
            Geometria::Hachura { .. } => {}
            Geometria::Circulo { centro, raio } => {
                f(*centro);
                for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
                    f(Ponto::novo(centro.x + raio * dx, centro.y + raio * dy));
                }
            }
            Geometria::Arco { centro, raio, inicio, fim } => {
                f(*centro);
                for ang in [*inicio, *fim] {
                    f(Ponto::novo(centro.x + raio * ang.to_radians().cos(), centro.y + raio * ang.to_radians().sin()));
                }
            }
        }
    }

    /// Chama `f` nos pontos que limitam a forma (para enquadrar). O arco conta só pelos vértices.
    pub fn para_cada_ponto_de_caixa(&self, f: &mut impl FnMut(Ponto)) {
        if let Geometria::Arco { centro, raio, inicio, fim } = self {
            // As pontas e os quadrantes que o arco atravessa.
            let varredura = crate::geom::varredura_graus(*inicio, *fim);
            let mut ponto = |ang: f64| f(Ponto::novo(centro.x + raio * ang.to_radians().cos(), centro.y + raio * ang.to_radians().sin()));
            ponto(*inicio);
            ponto(*fim);
            for quadrante in [0.0, 90.0, 180.0, 270.0] {
                if crate::geom::normalizar_graus(quadrante - inicio) <= varredura {
                    ponto(quadrante);
                }
            }
            return;
        }
        if let Geometria::Hachura { contornos, .. } = self {
            contornos.iter().flatten().for_each(|v| f(v.p));
            return;
        }
        self.para_cada_ponto_notavel(f);
        if let Geometria::Texto { ponto, altura, rotacao, conteudo, .. } = self {
            let largura = altura * LARGURA_DO_CARACTERE * conteudo.chars().count() as f64;
            let r = rotacao.to_radians();
            f(Ponto::novo(ponto.x + largura * r.cos(), ponto.y + largura * r.sin()));
        }
    }
}

/// Algo desenhado: geometria + camada + cor própria (`None` = `PorCamada`).
#[derive(Clone, Debug, PartialEq)]
pub struct Entidade {
    pub id: Id,
    pub camada: usize,
    pub cor: Option<u32>,
    pub geometria: Geometria,
    pub props: Propriedades,
}

/// Um bloco: um pedaço de desenho com nome, para ser inserido várias vezes (`INSERT`).
#[derive(Clone, Debug, PartialEq)]
pub struct Bloco {
    pub nome: String,
    /// O ponto do bloco que cai sobre o ponto de inserção.
    pub base: Ponto,
    pub entidades: Vec<NovaEntidade>,
}

/// Blocos dentro de blocos: até aqui se desce (protege de bloco que se insere em si mesmo).
pub const PROFUNDIDADE_MAXIMA: u8 = 8;

/// O tipo de linha (o traçado: contínuo, tracejado…). Os comprimentos do traço são de tela, não do desenho.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoLinha {
    Continua,
    Tracejada,
    Oculta,
    Eixo,
    Pontilhada,
    Fantasma,
}

impl TipoLinha {
    /// Todos, na ordem em que aparecem para escolher.
    pub const TODOS: [TipoLinha; 6] = [TipoLinha::Continua, TipoLinha::Tracejada, TipoLinha::Oculta, TipoLinha::Eixo, TipoLinha::Pontilhada, TipoLinha::Fantasma];

    /// O nome no DXF (`CONTINUOUS`, `DASHED`…).
    pub fn nome_dxf(self) -> &'static str {
        match self {
            TipoLinha::Continua => "CONTINUOUS",
            TipoLinha::Tracejada => "DASHED",
            TipoLinha::Oculta => "HIDDEN",
            TipoLinha::Eixo => "CENTER",
            TipoLinha::Pontilhada => "DOT",
            TipoLinha::Fantasma => "PHANTOM",
        }
    }

    /// O tipo de um nome de DXF (`DASHED2`, `HIDDENX2`, `ACAD_ISO02W100`… casam pelo começo); `None` = por camada ou desconhecido.
    pub fn de_nome_dxf(nome: &str) -> Option<Self> {
        let n = nome.trim().to_uppercase();
        [("CONTINUOUS", Self::Continua), ("DASHED", Self::Tracejada), ("HIDDEN", Self::Oculta), ("CENTER", Self::Eixo), ("DOT", Self::Pontilhada), ("PHANTOM", Self::Fantasma)]
            .into_iter()
            .find(|(prefixo, _)| n.starts_with(prefixo))
            .map(|(_, t)| t)
    }

    /// O padrão de traço e vão, em pixels (`traço, vão, traço, vão…`); vazio = contínuo.
    pub fn padrao_px(self) -> &'static [f64] {
        match self {
            TipoLinha::Continua => &[],
            TipoLinha::Tracejada => &[9.0, 5.0],
            TipoLinha::Oculta => &[5.0, 3.0],
            TipoLinha::Eixo => &[18.0, 3.0, 3.0, 3.0],
            TipoLinha::Pontilhada => &[1.0, 5.0],
            TipoLinha::Fantasma => &[18.0, 3.0, 3.0, 3.0, 3.0, 3.0],
        }
    }
}

/// O que uma entidade tem além da forma, da camada e da cor. `None` = o padrão (por camada).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Propriedades {
    /// Espessura de linha em milímetros.
    #[serde(default)]
    pub espessura: Option<f32>,
    #[serde(default)]
    pub tipo_linha: Option<TipoLinha>,
    /// O grupo (agrupar): quem tem o mesmo número seleciona junto.
    #[serde(default)]
    pub grupo: Option<u32>,
}

/// Uma entidade que ainda não tem id (vinda de um arquivo).
#[derive(Clone, Debug, PartialEq)]
pub struct NovaEntidade {
    pub camada: usize,
    pub cor: Option<u32>,
    pub geometria: Geometria,
    pub props: Propriedades,
}

/// Um passo do histórico: o que **desfazer** faz para voltar.
#[derive(Clone, Debug)]
pub(crate) enum Passo {
    /// A entidade foi adicionada: desfazer a remove.
    Adicionou(Id),
    /// A entidade foi removida: desfazer a devolve.
    Removeu(Entidade),
    /// A entidade foi trocada por outra de mesmo id: desfazer devolve a que estava (a que vai aqui).
    Trocou(Entidade),
    /// Vários passos de uma edição só (mover 30 objetos = um Ctrl+Z), na ordem em que se aplicam ao desfazer.
    Lote(Vec<Passo>),
    /// A tabela de camadas (e qual é a atual) como estava: propriedades de camada mudam por captura.
    Camadas { camadas: Vec<Camada>, atual: usize },
    /// Camadas, entidades e blocos como estavam (excluir camada remapeia índices por todo o desenho).
    Tudo(Box<Captura>),
}

/// Uma captura das partes do desenho que a exclusão de camada mexe.
#[derive(Clone, Debug)]
pub(crate) struct Captura {
    pub camadas: Vec<Camada>,
    pub atual: usize,
    pub entidades: Vec<Entidade>,
    pub blocos: Vec<Bloco>,
}

/// Uma mudança pedida ao documento; um lote delas é **um** passo do histórico.
#[derive(Clone, Debug, PartialEq)]
pub enum Mudanca {
    /// Uma entidade nova (com a camada e a cor dela; ganha id).
    Novo(NovaEntidade),
    Remover(Id),
    /// Troca a forma de uma entidade (mesmo id, mesma camada e cor).
    Trocar(Id, Geometria),
    /// Muda camada, cor e/ou propriedades (o que vier `Some`); a forma fica.
    Alterar { id: Id, camada: Option<usize>, cor: Option<Option<u32>>, props: Option<Propriedades> },
}

/// A unidade em que o desenho é medido. As coordenadas seguem sendo números puros; a unidade
/// é o que eles **significam** (e o que vai no `$INSUNITS` do DXF).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unidade {
    Milimetro,
    Centimetro,
    Metro,
}

impl Unidade {
    pub const TODAS: [Unidade; 3] = [Self::Milimetro, Self::Centimetro, Self::Metro];

    /// O nome curto (`mm`, `cm`, `m`), o mesmo da ponte e do `.fcdz`.
    pub fn nome(self) -> &'static str {
        match self {
            Self::Milimetro => "mm",
            Self::Centimetro => "cm",
            Self::Metro => "m",
        }
    }

    pub fn de_nome(nome: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|u| u.nome() == nome)
    }

    /// Quantos milímetros vale uma unidade (para converter `10cm` digitado em `m`, por exemplo).
    pub fn em_milimetros(self) -> f64 {
        match self {
            Self::Milimetro => 1.0,
            Self::Centimetro => 10.0,
            Self::Metro => 1000.0,
        }
    }

    /// O código do `$INSUNITS` do DXF.
    pub fn codigo_dxf(self) -> i32 {
        match self {
            Self::Milimetro => 4,
            Self::Centimetro => 5,
            Self::Metro => 6,
        }
    }

    /// Códigos que o app não trata (polegada, pé…) ou `0` (sem unidade) viram `None`.
    pub fn de_codigo_dxf(codigo: i32) -> Option<Self> {
        Self::TODAS.into_iter().find(|u| u.codigo_dxf() == codigo)
    }
}

#[cfg(test)]
mod testes_unidade {
    use super::*;

    #[test]
    fn nome_e_codigo_do_dxf_ida_e_volta() {
        for u in Unidade::TODAS {
            assert_eq!(Unidade::de_nome(u.nome()), Some(u));
            assert_eq!(Unidade::de_codigo_dxf(u.codigo_dxf()), Some(u));
        }
        assert_eq!(Unidade::de_codigo_dxf(0), None);
        assert_eq!(Unidade::de_nome("pol"), None);
    }

    #[test]
    fn trocar_a_unidade_suja_mas_criar_com_ela_nao() {
        let d = Documento::default().com_unidade(Some(Unidade::Metro));
        assert!(!d.sujo());
        let mut d = d;
        d.definir_unidade(Some(Unidade::Metro));
        assert!(!d.sujo());
        d.definir_unidade(Some(Unidade::Centimetro));
        assert!(d.sujo());
    }
}

/// O documento aberto.
#[derive(Debug)]
pub struct Documento {
    /// Em que unidade o desenho é trabalhado (perguntada ao começar); `None` = arquivo sem essa informação.
    pub(crate) unidade: Option<Unidade>,
    pub(crate) camadas: Vec<Camada>,
    pub(crate) camada_atual: usize,
    pub(crate) blocos: Vec<Bloco>,
    /// A caixa de cada bloco no seu próprio espaço (para recortar e enquadrar sem descer neles).
    pub(crate) caixas_dos_blocos: Vec<Option<(Ponto, Ponto)>>,
    pub(crate) entidades: Vec<Entidade>,
    pub(crate) proximo_id: Id,
    versao: u64,
    limpo_na_versao: u64,
    pub(crate) desfazer: Vec<Passo>,
    pub(crate) refazer: Vec<Passo>,
}

impl Default for Documento {
    fn default() -> Self {
        Self {
            unidade: None,
            camadas: vec![Camada::nova("0", 0xFF_FF_FF)],
            camada_atual: 0,
            blocos: Vec::new(),
            caixas_dos_blocos: Vec::new(),
            entidades: Vec::new(),
            proximo_id: 1,
            versao: 0,
            limpo_na_versao: 0,
            desfazer: Vec::new(),
            refazer: Vec::new(),
        }
    }
}

impl Documento {
    /// Monta um documento a partir de um arquivo lido: camadas e entidades. Nasce
    /// limpo (sem alteração não salva) e sem histórico. Sempre há ao menos uma
    /// camada; entidade com camada inexistente vai para a `0`.
    pub fn de_partes(camadas: Vec<Camada>, camada_atual: usize, entidades: Vec<NovaEntidade>) -> Self {
        Self::de_partes_com_blocos(camadas, camada_atual, Vec::new(), entidades)
    }

    /// Como `de_partes`, com as definições de bloco que os `Insercao` referenciam (por índice).
    pub fn de_partes_com_blocos(mut camadas: Vec<Camada>, camada_atual: usize, blocos: Vec<Bloco>, entidades: Vec<NovaEntidade>) -> Self {
        if camadas.is_empty() {
            camadas.push(Camada::nova("0", 0xFF_FF_FF));
        }
        let total = camadas.len();
        let mut doc = Self { camada_atual: if camada_atual < total { camada_atual } else { 0 }, camadas, ..Self::default() };
        for n in entidades {
            let id = doc.proximo_id;
            doc.proximo_id += 1;
            let camada = if n.camada < total { n.camada } else { 0 };
            doc.entidades.push(Entidade { id, camada, cor: n.cor, geometria: n.geometria, props: n.props });
        }
        doc.blocos = blocos;
        doc.caixas_dos_blocos = (0..doc.blocos.len()).map(|i| doc.caixa_do_bloco(i, 0)).collect();
        doc.versao = 1;
        doc.limpo_na_versao = 1;
        doc
    }

    pub fn blocos(&self) -> &[Bloco] {
        &self.blocos
    }

    /// O índice do bloco de nome `nome` (sem diferenciar maiúscula de minúscula).
    pub fn bloco_por_nome(&self, nome: &str) -> Option<usize> {
        self.blocos.iter().position(|b| b.nome.eq_ignore_ascii_case(nome.trim()))
    }

    /// Os nomes dos blocos que fazem sentido inserir à mão (sem os anônimos `*D1`, `*U2`…).
    pub fn nomes_de_blocos_inseriveis(&self) -> Vec<String> {
        self.blocos.iter().filter(|b| !b.nome.starts_with('*')).map(|b| b.nome.clone()).collect()
    }

    /// A caixa do bloco no espaço dele, já calculada.
    pub fn caixa_guardada_do_bloco(&self, indice: usize) -> Option<(Ponto, Ponto)> {
        self.caixas_dos_blocos.get(indice).copied().flatten()
    }

    /// Calcula a caixa de um bloco descendo nos blocos de dentro (com limite de profundidade).
    pub(crate) fn caixa_do_bloco(&self, indice: usize, profundidade: u8) -> Option<(Ponto, Ponto)> {
        let mut caixa: Option<(Ponto, Ponto)> = None;
        let mut juntar = |p: Ponto| {
            caixa = Some(match caixa {
                None => (p, p),
                Some((min, max)) => (Ponto::novo(min.x.min(p.x), min.y.min(p.y)), Ponto::novo(max.x.max(p.x), max.y.max(p.y))),
            });
        };
        for n in &self.blocos.get(indice)?.entidades {
            match &n.geometria {
                Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } if profundidade < PROFUNDIDADE_MAXIMA => {
                    let dentro = self.blocos.get(*bloco).map(|b| b.base).unwrap_or(*ponto);
                    if let Some((min, max)) = self.caixa_do_bloco(*bloco, profundidade + 1) {
                        let t = Transf::de_insercao(dentro, *ponto, *escala_x, *escala_y, *rotacao);
                        for c in [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)] {
                            juntar(t.aplicar(c));
                        }
                    }
                }
                Geometria::Insercao { .. } => {}
                g => g.para_cada_ponto_de_caixa(&mut juntar),
            }
        }
        caixa
    }

    pub fn unidade(&self) -> Option<Unidade> {
        self.unidade
    }

    /// Troca a unidade **depois** de o desenho existir: conta como alteração (fica "sujo").
    pub fn definir_unidade(&mut self, u: Option<Unidade>) {
        if self.unidade != u {
            self.unidade = u;
            self.mudou();
        }
    }

    /// A unidade de quem acabou de abrir/criar o documento: não conta como alteração.
    #[must_use]
    pub fn com_unidade(mut self, u: Option<Unidade>) -> Self {
        self.unidade = u;
        self
    }

    /// Sobe a versão: tudo que muda o desenho passa por aqui.
    pub(crate) fn mudou(&mut self) {
        self.versao += 1;
    }

    /// Número que sobe a cada mudança (a interface compara para saber se o quadro vale).
    pub fn versao(&self) -> u64 {
        self.versao
    }

    /// Há alteração não salva?
    pub fn sujo(&self) -> bool {
        self.versao != self.limpo_na_versao
    }

    /// Marca o estado atual como salvo.
    pub fn marcar_salvo(&mut self) {
        self.limpo_na_versao = self.versao;
    }

    pub fn camadas(&self) -> &[Camada] {
        &self.camadas
    }

    pub fn camada_atual(&self) -> usize {
        self.camada_atual
    }

    /// Torna atual a camada de índice `indice`.
    pub fn definir_camada_atual(&mut self, indice: usize) -> Result<(), String> {
        if indice >= self.camadas.len() {
            return Err(format!("não existe a camada de número {indice}"));
        }
        self.camada_atual = indice;
        Ok(())
    }

    pub fn entidades(&self) -> &[Entidade] {
        &self.entidades
    }

    /// A entidade de id `id`, se ainda existe.
    pub fn entidade(&self, id: Id) -> Option<&Entidade> {
        self.entidades.iter().find(|e| e.id == id)
    }

    /// Aplica as mudanças como **um** passo de desfazer. Devolve os ids das entidades novas, na ordem.
    /// Pedido sobre id que não existe é ignorado; se nada mudou, o histórico não ganha passo.
    pub fn aplicar_mudancas(&mut self, mudancas: Vec<Mudanca>) -> Vec<Id> {
        let (mut inversos, mut novos) = (Vec::new(), Vec::new());
        for m in mudancas {
            match m {
                Mudanca::Novo(n) => {
                    let id = self.proximo_id;
                    self.proximo_id += 1;
                    let camada = if n.camada < self.camadas.len() { n.camada } else { 0 };
                    self.entidades.push(Entidade { id, camada, cor: n.cor, geometria: n.geometria, props: n.props });
                    inversos.push(Passo::Adicionou(id));
                    novos.push(id);
                }
                Mudanca::Remover(id) => {
                    if let Some(pos) = self.entidades.iter().position(|e| e.id == id) {
                        inversos.push(Passo::Removeu(self.entidades.remove(pos)));
                    }
                }
                Mudanca::Alterar { id, camada, cor, props } => {
                    let total = self.camadas.len();
                    if let Some(e) = self.entidades.iter_mut().find(|e| e.id == id) {
                        let antiga = e.clone();
                        if let Some(c) = camada.filter(|c| *c < total) {
                            e.camada = c;
                        }
                        if let Some(c) = cor {
                            e.cor = c;
                        }
                        if let Some(p) = props {
                            e.props = p;
                        }
                        inversos.push(Passo::Trocou(antiga));
                    }
                }
                Mudanca::Trocar(id, geometria) => {
                    if let Some(e) = self.entidades.iter_mut().find(|e| e.id == id) {
                        let antiga = Entidade { geometria: std::mem::replace(&mut e.geometria, geometria), ..e.clone() };
                        inversos.push(Passo::Trocou(antiga));
                    }
                }
            }
        }
        if !inversos.is_empty() {
            inversos.reverse();
            self.desfazer.push(Passo::Lote(inversos));
            self.refazer.clear();
            self.mudou();
        }
        novos
    }

    /// A cor final de uma entidade: a própria, ou a da camada (`PorCamada`).
    pub fn cor_resolvida(&self, e: &Entidade) -> u32 {
        e.cor.unwrap_or_else(|| self.camadas.get(e.camada).map_or(0xFF_FF_FF, |c| c.cor))
    }

    /// A camada da entidade está bloqueada? (aparece, mas não se escolhe nem se edita)
    pub fn bloqueada(&self, e: &Entidade) -> bool {
        self.camadas.get(e.camada).is_some_and(|c| c.bloqueada)
    }

    /// A camada da entidade aparece? (ligada e não congelada)
    pub fn visivel(&self, e: &Entidade) -> bool {
        self.camadas.get(e.camada).is_some_and(Camada::visivel)
    }

    /// Acrescenta uma entidade pronta (na camada atual) e devolve o id.
    fn acrescentar(&mut self, geometria: Geometria) -> Id {
        let id = self.proximo_id;
        self.proximo_id += 1;
        self.entidades.push(Entidade { id, camada: self.camada_atual, cor: None, geometria, props: Propriedades::default() });
        self.desfazer.push(Passo::Adicionou(id));
        self.refazer.clear();
        self.mudou();
        id
    }

    /// Acrescenta uma forma pronta na camada atual (retângulo, elipse, hachura…).
    pub fn adicionar_geometria(&mut self, geometria: Geometria) -> Id {
        self.acrescentar(geometria)
    }

    /// Desenha uma polilinha na camada atual. Precisa de ao menos dois vértices.
    pub fn adicionar_polilinha(&mut self, vertices: Vec<Vertice>, fechada: bool) -> Option<Id> {
        (vertices.len() >= 2).then(|| self.acrescentar(Geometria::Polilinha { vertices, fechada }))
    }

    /// Insere uma cópia do bloco `bloco` em `ponto` (escala 1, sem giro), na camada atual.
    pub fn adicionar_insercao(&mut self, bloco: usize, ponto: Ponto) -> Option<Id> {
        (bloco < self.blocos.len()).then(|| self.acrescentar(Geometria::Insercao { bloco, ponto, escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }))
    }

    /// Desenha um círculo na camada atual. O raio precisa ser positivo.
    pub fn adicionar_circulo(&mut self, centro: Ponto, raio: f64) -> Option<Id> {
        (raio > 0.0 && raio.is_finite()).then(|| self.acrescentar(Geometria::Circulo { centro, raio }))
    }

    /// Desenha um arco (graus, anti-horário do início ao fim) na camada atual.
    pub fn adicionar_arco(&mut self, centro: Ponto, raio: f64, inicio: f64, fim: f64) -> Option<Id> {
        (raio > 0.0 && raio.is_finite()).then(|| self.acrescentar(Geometria::Arco { centro, raio, inicio, fim }))
    }

    /// A menor caixa que contém tudo o que se vê (camada ligada); `None` se não há nada.
    pub fn caixa(&self) -> Option<(Ponto, Ponto)> {
        let mut caixa: Option<(Ponto, Ponto)> = None;
        for e in self.entidades.iter().filter(|e| self.visivel(e)) {
            let mut juntar = |p: Ponto| {
                caixa = Some(match caixa {
                    None => (p, p),
                    Some((min, max)) => (Ponto::novo(min.x.min(p.x), min.y.min(p.y)), Ponto::novo(max.x.max(p.x), max.y.max(p.y))),
                });
            };
            if let Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } = &e.geometria {
                if let (Some((min, max)), Some(b)) = (self.caixa_guardada_do_bloco(*bloco), self.blocos.get(*bloco)) {
                    let t = Transf::de_insercao(b.base, *ponto, *escala_x, *escala_y, *rotacao);
                    for c in [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)] {
                        juntar(t.aplicar(c));
                    }
                }
            } else {
                e.geometria.para_cada_ponto_de_caixa(&mut juntar);
            }
        }
        caixa
    }

    /// Desenha uma linha na camada atual e devolve o id dela.
    pub fn adicionar_linha(&mut self, a: Ponto, b: Ponto) -> Id {
        self.acrescentar(Geometria::Linha { a, b })
    }

    pub fn pode_desfazer(&self) -> bool {
        !self.desfazer.is_empty()
    }

    pub fn pode_refazer(&self) -> bool {
        !self.refazer.is_empty()
    }

    /// Quantos passos o histórico tem agora: a "marca" para [`Documento::juntar_historico`] e [`Documento::desfazer_ate`].
    pub fn marca_do_historico(&self) -> usize {
        self.desfazer.len()
    }

    /// Junta em **um** passo de desfazer tudo o que entrou no histórico depois de `marca` (um lote de ferramentas = um Ctrl+Z).
    pub fn juntar_historico(&mut self, marca: usize) {
        if self.desfazer.len() <= marca + 1 {
            return;
        }
        let mut passos = self.desfazer.split_off(marca);
        passos.reverse(); // o último feito é o primeiro a desfazer
        self.desfazer.push(Passo::Lote(passos));
        self.refazer.clear();
    }

    /// Desfaz tudo o que entrou no histórico depois de `marca` e esquece o refazer (cancelar um lote pela metade).
    pub fn desfazer_ate(&mut self, marca: usize) {
        while self.desfazer.len() > marca && self.desfazer() {}
        self.refazer.clear();
    }

    /// Desfaz o último passo; `false` se não há nada a desfazer.
    pub fn desfazer(&mut self) -> bool {
        let Some(passo) = self.desfazer.pop() else { return false };
        let inverso = self.aplicar(passo);
        self.refazer.push(inverso);
        true
    }

    /// Refaz o último passo desfeito; `false` se não há nada a refazer.
    pub fn refazer(&mut self) -> bool {
        let Some(passo) = self.refazer.pop() else { return false };
        let inverso = self.aplicar(passo);
        self.desfazer.push(inverso);
        true
    }

    /// Aplica um passo e devolve o passo que o desfaz.
    fn aplicar(&mut self, passo: Passo) -> Passo {
        self.mudou();
        match passo {
            Passo::Adicionou(id) => {
                let pos = self.entidades.iter().position(|e| e.id == id);
                // O histórico só guarda ids que existem; se sumiu, o passo inverso é vazio.
                let entidade = pos.map(|p| self.entidades.remove(p));
                match entidade {
                    Some(e) => Passo::Removeu(e),
                    None => Passo::Adicionou(id),
                }
            }
            Passo::Removeu(e) => {
                let id = e.id;
                self.entidades.push(e);
                Passo::Adicionou(id)
            }
            Passo::Trocou(nova) => match self.entidades.iter_mut().find(|e| e.id == nova.id) {
                Some(e) => Passo::Trocou(std::mem::replace(e, nova)),
                None => Passo::Trocou(nova),
            },
            Passo::Camadas { camadas, atual } => {
                let antes = Passo::Camadas { camadas: std::mem::replace(&mut self.camadas, camadas), atual: self.camada_atual };
                self.camada_atual = atual;
                antes
            }
            Passo::Tudo(captura) => {
                let Captura { camadas, atual, entidades, blocos } = *captura;
                let antes = Captura {
                    camadas: std::mem::replace(&mut self.camadas, camadas),
                    atual: std::mem::replace(&mut self.camada_atual, atual),
                    entidades: std::mem::replace(&mut self.entidades, entidades),
                    blocos: std::mem::replace(&mut self.blocos, blocos),
                };
                self.caixas_dos_blocos = (0..self.blocos.len()).map(|i| self.caixa_do_bloco(i, 0)).collect();
                Passo::Tudo(Box::new(antes))
            }
            Passo::Lote(passos) => {
                let mut inversos: Vec<Passo> = passos.into_iter().map(|p| self.aplicar(p)).collect();
                inversos.reverse();
                Passo::Lote(inversos)
            }
        }
    }
}

#[cfg(test)]
mod testes_do_lote {
    use super::*;

    #[test]
    fn juntar_historico_vira_um_passo_e_desfazer_ate_volta_ao_inicio() {
        let mut d = Documento::default();
        d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(1.0, 0.0));
        let marca = d.marca_do_historico();
        for i in 0..3 {
            d.adicionar_linha(Ponto::novo(0.0, f64::from(i)), Ponto::novo(5.0, f64::from(i)));
        }
        assert_eq!(d.marca_do_historico(), marca + 3);
        d.juntar_historico(marca);
        assert_eq!(d.marca_do_historico(), marca + 1, "três passos viram um");
        assert!(d.desfazer());
        assert_eq!(d.entidades().len(), 1, "um desfazer tira as três");
        assert!(d.refazer());
        assert_eq!(d.entidades().len(), 4);
        d.desfazer_ate(marca);
        assert_eq!((d.entidades().len(), d.pode_refazer()), (1, false), "cancelar o lote volta ao começo");
    }
}

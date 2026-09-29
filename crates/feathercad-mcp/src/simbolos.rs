//! A biblioteca de símbolos de **projeto elétrico** (NBR 5444): desenhada uma vez aqui, chamada pelo nome.
//! `cad_simbolos` lista, `cad_simbolo` põe um símbolo no desenho. Só faz sentido em projeto elétrico.
//!
//! Cada símbolo é desenhado em **milímetros de papel**, com a origem no seu ponto de ligação; ao colocá-lo
//! ele é multiplicado pela escala do desenho (`escala` = denominador, 50 para 1:50) e convertido para a
//! unidade do desenho. Assim `tomada_media` tem o mesmo tamanho no papel em qualquer escala e unidade.
//! Os textos (circuito, comando, potência) ficam sempre na horizontal, qualquer que seja o giro.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria, Id, Mudanca, NovaEntidade, Propriedades, TipoLinha};
use feathercad_core::edicao::transformar;
use feathercad_core::geom::{Ponto, Transf, Vertice};
use serde_json::{json, Value};

use crate::json::{numero_opcional, ponto_de, texto_de, texto_opcional, Res};
use crate::mutacoes::{camada_por_nome, cor_hex};

/// Uma peça em milímetros de papel: formas e, para cada uma, o tipo de linha (`None` = contínua).
#[derive(Default)]
pub struct Peca {
    pub formas: Vec<(Geometria, Option<TipoLinha>)>,
}

type P = (f64, f64);

fn vert(pts: &[P]) -> Vec<Vertice> {
    pts.iter().map(|&(x, y)| Vertice { p: Ponto::novo(x, y), bulge: 0.0 }).collect()
}

impl Peca {
    pub fn linha(&mut self, a: P, b: P) {
        self.formas.push((Geometria::Linha { a: Ponto::novo(a.0, a.1), b: Ponto::novo(b.0, b.1) }, None));
    }
    pub fn poli(&mut self, pts: &[P], fechada: bool) {
        self.formas.push((Geometria::Polilinha { vertices: vert(pts), fechada }, None));
    }
    pub fn retangulo(&mut self, a: P, b: P) {
        self.poli(&[a, (b.0, a.1), b, (a.0, b.1)], true);
    }
    pub fn circulo(&mut self, c: P, raio: f64) {
        self.formas.push((Geometria::Circulo { centro: Ponto::novo(c.0, c.1), raio }, None));
    }
    /// Área preenchida (hachura sólida).
    pub fn cheio(&mut self, pts: &[P]) {
        self.formas.push((Geometria::Hachura { contornos: vec![vert(pts)], padrao: Vec::new(), nome: "SOLID".into() }, None));
    }
    pub fn texto(&mut self, p: P, altura: f64, conteudo: &str) {
        self.formas.push((Geometria::Texto { ponto: Ponto::novo(p.0, p.1), altura, rotacao: 0.0, conteudo: conteudo.to_string(), h: AlinhaH::Esquerda, v: AlinhaV::Base }, None));
    }
}

/// Os rótulos e medidas que a IA pode dar a um símbolo.
struct Par {
    circuito: Option<String>,
    comando: Option<String>,
    potencia: Option<String>,
    quantidade: u32,
    altura_texto: f64,
}

/// Número ou texto, como texto (`4` e `"4"` valem o mesmo).
fn texto_flex(args: &Value, nome: &str) -> Option<String> {
    match args.get(nome)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl Par {
    fn de(args: &Value) -> Self {
        Self {
            circuito: texto_flex(args, "circuito"),
            comando: texto_flex(args, "comando"),
            potencia: texto_flex(args, "potencia"),
            quantidade: numero_opcional(args, "quantidade").map_or(1, |n| n.clamp(1.0, 6.0) as u32),
            altura_texto: numero_opcional(args, "altura_texto").filter(|h| *h > 0.0).unwrap_or(1.8),
        }
    }

    /// "100" vira "100W"; "600VA" fica como está; com mais de uma lâmpada, "2x100W".
    fn potencia_formatada(&self) -> Option<String> {
        let p = self.potencia.as_ref()?;
        let p = if p.parse::<f64>().is_ok() { format!("{p}W") } else { p.clone() };
        Some(if self.quantidade > 1 { format!("{}x{p}", self.quantidade) } else { p })
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Nivel {
    Baixa,
    Media,
    Alta,
}

/// Tomada de parede: barra na parede, haste e um triângulo por tomada (vazio = baixa, meio cheio = média, cheio = alta).
fn tomada(nivel: Nivel, p: &Par) -> Peca {
    let mut c = Peca::default();
    c.linha((-2.5, 0.0), (2.5, 0.0));
    c.linha((0.0, 0.0), (0.0, -2.0));
    for k in 0..p.quantidade {
        let (base, ponta) = (-2.0 - 3.4 * f64::from(k), -2.0 - 3.4 * f64::from(k + 1));
        c.poli(&[(-1.8, base), (1.8, base), (0.0, ponta)], true);
        match nivel {
            Nivel::Baixa => {}
            Nivel::Media => c.cheio(&[(-1.8, base), (0.0, base), (0.0, ponta)]),
            Nivel::Alta => c.cheio(&[(-1.8, base), (1.8, base), (0.0, ponta)]),
        }
    }
    let h = p.altura_texto;
    if let Some(t) = &p.circuito {
        c.texto((3.0, -3.0), h, t);
    }
    if let Some(t) = p.potencia_formatada() {
        c.texto((3.0, -3.0 - h * 1.4), h, &t);
    }
    c
}

fn tomada_piso(p: &Par) -> Peca {
    let mut c = Peca::default();
    c.retangulo((-3.0, -3.0), (3.0, 3.0));
    c.poli(&[(-1.5, -2.0), (-1.5, 2.0), (1.8, 0.0)], true);
    if let Some(t) = &p.circuito {
        c.texto((3.6, -0.6), p.altura_texto, t);
    }
    c
}

/// O texto do ponto de luz no padrão da NBR: letra do comando no alto à direita, circuito entre dois traços
/// embaixo à esquerda e a potência (com a quantidade de lâmpadas) embaixo à direita.
fn rotulos_de_luz(c: &mut Peca, p: &Par, raio: f64, cx: f64) {
    let h = p.altura_texto;
    if let Some(l) = &p.comando {
        c.texto((cx + raio * 0.8, raio * 0.9), h, l);
    }
    if let Some(t) = &p.circuito {
        c.texto((cx - raio - h * 2.4, -raio - h * 0.6), h, &format!("-{t}-"));
    }
    if let Some(t) = p.potencia_formatada() {
        c.texto((cx + raio * 0.3, -raio - h * 1.2), h, &t);
    }
}

fn ponto_de_luz(embutido: bool, p: &Par) -> Peca {
    let mut c = Peca::default();
    c.circulo((0.0, 0.0), 2.5);
    if embutido {
        c.circulo((0.0, 0.0), 1.7);
    }
    rotulos_de_luz(&mut c, p, 2.5, 0.0);
    c
}

fn arandela(p: &Par) -> Peca {
    let mut c = Peca::default();
    c.linha((0.0, -2.8), (0.0, 2.8));
    c.circulo((2.5, 0.0), 2.5);
    rotulos_de_luz(&mut c, p, 2.5, 2.5);
    c
}

/// Ponto de luz a vapor de mercúrio: o mesmo círculo, com a potência marcada "VM".
fn vapor_de_mercurio(p: &Par) -> Peca {
    let mut c = ponto_de_luz(false, &Par { potencia: p.potencia.as_ref().map(|w| if w.to_uppercase().ends_with("VM") { w.clone() } else { format!("{}-VM", if w.parse::<f64>().is_ok() { format!("{w}W") } else { w.clone() }) }), circuito: p.circuito.clone(), comando: p.comando.clone(), quantidade: 1, altura_texto: p.altura_texto });
    c.circulo((0.0, 0.0), 3.1);
    c
}

/// Interruptor: bolinha, a letra S (com o número de seções/vias) e a letra do ponto que comanda.
fn interruptor(rotulo: &str, p: &Par) -> Peca {
    let mut c = Peca::default();
    let h = p.altura_texto * 1.2;
    c.circulo((0.0, 0.0), 1.1);
    c.texto((1.7, -h * 0.35), h, rotulo);
    if let Some(l) = &p.comando {
        c.texto((1.7 + h * 0.62 * rotulo.chars().count() as f64, -h * 1.1), p.altura_texto * 0.9, l);
    }
    if let Some(t) = &p.circuito {
        c.texto((1.7, h * 0.9), p.altura_texto * 0.9, &format!("-{t}-"));
    }
    c
}

/// Faixa de traços a 45° dentro do retângulo de `x0` a `x1`, entre `y0` e `y1`.
fn hachurado(c: &mut Peca, x0: f64, x1: f64, y0: f64, y1: f64, passo: f64) {
    let mut x = x0;
    while x + (y1 - y0) <= x1 + 1e-9 {
        c.linha((x, y0), (x + (y1 - y0), y1));
        x += passo;
    }
}

/// Quadro de distribuição (NBR 5444, 6.1 a 6.4): parcial = retângulo cheio; geral = retângulo com um triângulo cheio;
/// embutido = hachura nas pontas; aparente = hachura na base. Ancorado no centro da base.
fn quadro(geral: bool, embutido: bool, p: &Par) -> Peca {
    let mut c = Peca::default();
    c.retangulo((-4.0, 0.0), (4.0, 2.5));
    if geral {
        c.cheio(&[(-4.0, 0.0), (4.0, 0.0), (-4.0, 2.5)]);
    } else {
        c.cheio(&[(-4.0, 0.0), (4.0, 0.0), (4.0, 2.5), (-4.0, 2.5)]);
    }
    if embutido {
        hachurado(&mut c, -6.4, -4.0, 0.0, 2.4, 1.2);
        hachurado(&mut c, 4.0, 6.4, 0.0, 2.4, 1.2);
    } else {
        c.linha((-4.0, -0.6), (4.0, -0.6));
        hachurado(&mut c, -4.0, 4.0, -0.6, -1.6, 1.0);
    }
    if let Some(t) = p.potencia_formatada() {
        c.texto((-4.0, 3.4), p.altura_texto, &t);
    }
    c
}

fn caixa_telefone() -> Peca {
    let mut c = Peca::default();
    c.linha((-6.0, 0.0), (-3.0, 0.0));
    c.linha((3.0, 0.0), (6.0, 0.0));
    c.retangulo((-3.0, -1.0), (3.0, 1.0));
    hachurado(&mut c, -3.0, 1.0, -1.0, 1.0, 1.2);
    c
}

fn caixa_medidor(p: &Par) -> Peca {
    let mut c = Peca::default();
    c.linha((-6.5, 0.0), (-3.5, 0.0));
    c.linha((3.5, 0.0), (6.5, 0.0));
    c.retangulo((-3.5, -1.4), (3.5, 1.4));
    c.texto((-2.7, -0.55), p.altura_texto * 0.9, "MED");
    c
}

fn caixa_de_passagem_no_piso() -> Peca {
    let mut c = Peca::default();
    c.retangulo((-2.0, -2.0), (2.0, 2.0));
    c.texto((-0.6, -0.7), 1.8, "P");
    c.linha((-5.0, 0.0), (-2.0, 0.0));
    c.linha((2.0, 0.0), (5.0, 0.0));
    c
}

/// Ponto de luz do diagrama **multifilar**: círculo com X e fios dos dois lados.
fn luz_multifilar() -> Peca {
    let mut c = Peca::default();
    c.circulo((0.0, 0.0), 2.5);
    c.linha((-1.8, -1.8), (1.8, 1.8));
    c.linha((-1.8, 1.8), (1.8, -1.8));
    c.linha((-6.0, 0.0), (-2.5, 0.0));
    c.linha((2.5, 0.0), (6.0, 0.0));
    c
}

/// Disjuntor/contato do multifilar: fio, pivô, alavanca inclinada, contato e fio.
fn disjuntor() -> Peca {
    let mut c = Peca::default();
    c.linha((-6.0, 0.0), (-2.0, 0.0));
    c.circulo((-1.5, 0.0), 0.5);
    c.linha((-1.2, 0.3), (2.2, 2.0));
    c.circulo((2.5, 0.0), 0.5);
    c.linha((3.0, 0.0), (6.0, 0.0));
    c
}

/// Nó (ponto de ligação) do multifilar: bolinha cheia.
fn no_de_ligacao() -> Peca {
    let mut c = Peca::default();
    let pts: Vec<P> = (0..8).map(|k| (0.7 * (std::f64::consts::TAU * f64::from(k) / 8.0).cos(), 0.7 * (std::f64::consts::TAU * f64::from(k) / 8.0).sin())).collect();
    c.cheio(&pts);
    c
}

/// Um símbolo do catálogo: nome, o que é, e o que ele aceita além de `ponto`.
pub struct Info {
    pub nome: &'static str,
    pub descricao: &'static str,
    pub parametros: &'static str,
}

const PAREDE: &str = "parede (norte|sul|leste|oeste = de que lado do símbolo está a parede)";

pub const CATALOGO: [Info; 22] = [
    Info { nome: "tomada_baixa", descricao: "Tomada baixa (triângulo vazio, ~30 cm do piso). `quantidade` 1/2/3 = simples/dupla/tripla.", parametros: "parede, quantidade, circuito, potencia" },
    Info { nome: "tomada_media", descricao: "Tomada média (triângulo meio cheio, ~110 cm; bancada de cozinha).", parametros: "parede, quantidade, circuito, potencia" },
    Info { nome: "tomada_alta", descricao: "Tomada alta (triângulo cheio, ~200 cm; chuveiro, ar-condicionado).", parametros: "parede, quantidade, circuito, potencia" },
    Info { nome: "tomada_piso", descricao: "Tomada no piso (triângulo dentro de um quadrado).", parametros: "circuito" },
    Info { nome: "ponto_luz_teto", descricao: "Ponto de luz incandescente no teto (unifilar): círculo, comando (a, b…) no alto, -circuito- e potência.", parametros: "comando, circuito, potencia, quantidade" },
    Info { nome: "ponto_luz_embutido", descricao: "Ponto de luz no teto, embutido (círculo duplo).", parametros: "comando, circuito, potencia, quantidade" },
    Info { nome: "arandela", descricao: "Ponto de luz na parede (arandela); indicar a altura na legenda.", parametros: "comando, circuito, potencia" },
    Info { nome: "ponto_luz_vapor_mercurio", descricao: "Ponto de luz a vapor de mercúrio no teto (potência marcada -VM).", parametros: "comando, circuito, potencia" },
    Info { nome: "interruptor_simples", descricao: "Interruptor de uma seção (S) com a letra do ponto que comanda.", parametros: "comando, circuito" },
    Info { nome: "interruptor_paralelo", descricao: "Interruptor paralelo / three-way (S3): comanda o mesmo ponto de dois lugares.", parametros: "comando, circuito" },
    Info { nome: "interruptor_intermediario", descricao: "Interruptor intermediário / four-way (S4): fica entre dois paralelos (3 ou mais lugares).", parametros: "comando, circuito" },
    Info { nome: "interruptor_duas_secoes", descricao: "Interruptor de duas seções (SS).", parametros: "comando, circuito" },
    Info { nome: "quadro_parcial_aparente", descricao: "Quadro parcial de luz e força, aparente (6.1).", parametros: "potencia (carga, ex. 4500W)" },
    Info { nome: "quadro_parcial_embutido", descricao: "Quadro parcial de luz e força, embutido (6.2).", parametros: "potencia" },
    Info { nome: "quadro_geral_aparente", descricao: "Quadro geral de luz e força, aparente (6.3).", parametros: "potencia" },
    Info { nome: "quadro_geral_embutido", descricao: "Quadro geral de luz e força, embutido (6.4). Ancorado no centro da base.", parametros: "potencia" },
    Info { nome: "caixa_telefone", descricao: "Caixa de telefones (6.5).", parametros: "" },
    Info { nome: "caixa_medidor", descricao: "Caixa para medidor (6.6).", parametros: "" },
    Info { nome: "caixa_passagem_piso", descricao: "Caixa de passagem no piso (quadrado com P).", parametros: "" },
    Info { nome: "luz_multifilar", descricao: "Ponto de luz do diagrama multifilar (círculo com X e fios).", parametros: "" },
    Info { nome: "no_de_ligacao", descricao: "Bolinha cheia de derivação/ligação do multifilar (onde o circuito pega na barra).", parametros: "" },
    Info { nome: "disjuntor", descricao: "Disjuntor/contato do multifilar (fio, pivô, alavanca, contato). Uma vez por polo; use `no_de_ligacao` para marcar a derivação.", parametros: "" },
];

fn montar(nome: &str, p: &Par) -> Option<Peca> {
    Some(match nome {
        "tomada_baixa" => tomada(Nivel::Baixa, p),
        "tomada_media" => tomada(Nivel::Media, p),
        "tomada_alta" => tomada(Nivel::Alta, p),
        "tomada_piso" => tomada_piso(p),
        "ponto_luz_teto" => ponto_de_luz(false, p),
        "ponto_luz_embutido" => ponto_de_luz(true, p),
        "arandela" => arandela(p),
        "ponto_luz_vapor_mercurio" => vapor_de_mercurio(p),
        "interruptor_simples" => interruptor("S", p),
        "interruptor_paralelo" => interruptor("S3", p),
        "interruptor_intermediario" => interruptor("S4", p),
        "interruptor_duas_secoes" => interruptor("SS", p),
        "quadro_parcial_aparente" => quadro(false, false, p),
        "quadro_parcial_embutido" => quadro(false, true, p),
        "quadro_geral_aparente" => quadro(true, false, p),
        "quadro_geral_embutido" => quadro(true, true, p),
        "caixa_telefone" => caixa_telefone(),
        "caixa_medidor" => caixa_medidor(p),
        "caixa_passagem_piso" => caixa_de_passagem_no_piso(),
        "luz_multifilar" => luz_multifilar(),
        "disjuntor" => disjuntor(),
        "no_de_ligacao" => no_de_ligacao(),
        _ => return None,
    })
}

/// O que a IA lê em `cad_simbolos`.
pub fn listar() -> Value {
    let todos: Vec<Value> = CATALOGO.iter().map(|i| json!({ "nome": i.nome, "descricao": i.descricao, "aceita": i.parametros })).collect();
    json!({
        "simbolos": todos,
        "comuns": "ponto [x,y] (obrigatório), escala (denominador, padrão 50 = 1:50), rotacao (graus anti-horário), camada, cor, altura_texto (mm no papel, padrão 1,8)",
        "parede": "tomadas: `parede` diz onde está a parede em relação ao símbolo; ele aponta para o lado oposto (parede norte → aponta para baixo/sul). `rotacao` faz o mesmo em graus.",
        "so_para": "projeto elétrico (NBR 5444/5410)",
    })
}

/// Do lado da parede para o giro (a base do símbolo aponta para o sul, com a parede ao norte).
fn giro_da_parede(nome: &str) -> Option<f64> {
    Some(match nome.to_lowercase().as_str() {
        "norte" | "n" => 0.0,
        "oeste" | "o" | "w" => 90.0,
        "sul" | "s" => 180.0,
        "leste" | "l" | "e" => 270.0,
        _ => return None,
    })
}

/// Quantas unidades do desenho vale 1 mm de papel na escala `denominador`.
pub fn mm_de_papel(doc: &Documento, denominador: f64) -> Res<f64> {
    let un = doc.unidade().ok_or("o desenho ainda não tem unidade: use cad_novo primeiro")?;
    Ok(denominador / un.em_milimetros())
}

/// Leva a peça (mm de papel) para o desenho: escala, giro e posição. Texto sempre na horizontal.
pub fn colocar(peca: &Peca, s: f64, giro: f64, alvo: Ponto) -> Vec<(Geometria, Option<TipoLinha>)> {
    let o = Ponto::novo(0.0, 0.0);
    peca.formas
        .iter()
        .map(|(g, t)| {
            let g = transformar(&transformar(&transformar(g, &Transf::escalar(o, s)), &Transf::girar(o, giro)), &Transf::mover(alvo.x, alvo.y));
            let g = match g {
                Geometria::Texto { ponto, altura, conteudo, h, v, .. } => Geometria::Texto { ponto, altura, rotacao: 0.0, conteudo, h, v },
                outra => outra,
            };
            (g, *t)
        })
        .collect()
}

/// Grava as formas no desenho, na camada e cor pedidas (um passo de desfazer).
pub fn gravar(doc: &mut Documento, formas: Vec<(Geometria, Option<TipoLinha>)>, args: &Value, espessura_da_primeira: Option<f32>) -> Res<Vec<Id>> {
    let cor = texto_opcional(args, "cor").map(cor_hex).transpose()?;
    let camada = match texto_opcional(args, "camada") {
        Some(n) => camada_por_nome(doc, n)?,
        None => doc.camada_atual(),
    };
    let ms = formas.into_iter().enumerate().map(|(i, (geometria, tipo))| Mudanca::Novo(NovaEntidade { camada, cor, geometria, props: Propriedades { tipo_linha: tipo, espessura: if i == 0 { espessura_da_primeira } else { None }, ..Propriedades::default() } })).collect();
    Ok(doc.aplicar_mudancas(ms))
}

/// `cad_simbolo`: põe o símbolo `nome` em `ponto`.
pub fn simbolo(doc: &mut Documento, args: &Value) -> Res<Value> {
    let nome = texto_de(args, "nome")?;
    let par = Par::de(args);
    let peca = montar(nome, &par).ok_or_else(|| format!("símbolo `{nome}` não existe; veja cad_simbolos"))?;
    let alvo = ponto_de(args, "ponto")?;
    let s = mm_de_papel(doc, numero_opcional(args, "escala").filter(|e| *e > 0.0).unwrap_or(50.0))?;
    let giro = match texto_opcional(args, "parede") {
        Some(p) => giro_da_parede(p).ok_or_else(|| format!("{PAREDE}: `{p}` não vale"))?,
        None => numero_opcional(args, "rotacao").unwrap_or(0.0),
    };
    let ids = gravar(doc, colocar(&peca, s, giro, alvo), args, None)?;
    Ok(json!({ "simbolo": nome, "ids": ids, "ponto": [alvo.x, alvo.y] }))
}

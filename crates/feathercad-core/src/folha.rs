//! Folhas (pranchetas): a moldura do desenho com margem de furação, marcas de centro, faixa de notas/legenda/revisões e carimbo.
//!
//! Uma folha é **um bloco** (`FOLHA-1`, `FOLHA-2`…) desenhado em milímetros de papel e inserido com a escala do desenho:
//! trocar a escala da folha é trocar a escala da inserção — nenhuma geometria é refeita. O `pdf.rs` do `feathercad-io` exporta
//! uma página por folha. Tudo aqui é conta do motor; a interface só pergunta papel, escala e os campos do carimbo.

use serde::{Deserialize, Serialize};

use crate::documento::{AlinhaH, AlinhaV, Bloco, Camada, Captura, Documento, Espaco, Geometria, Id, NovaEntidade, Passo, Propriedades, Unidade};
use crate::edicao::caixa_da_entidade;
use crate::geom::{Ponto, Vertice};

/// Todo bloco de folha começa por isto.
pub const PREFIXO: &str = "FOLHA-";
const CAMADA_FOLHA: &str = "FOLHA";
const CAMADA_CARIMBO: &str = "CARIMBO";

/// Os denominadores de escala oferecidos (1:1 … 1:10000). Escala de ampliação (2:1) é o inverso: 0,5.
pub const ESCALAS: [f64; 21] = [1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 75.0, 100.0, 125.0, 150.0, 200.0, 250.0, 500.0, 750.0, 1000.0, 1500.0, 2000.0, 2500.0, 5000.0, 10000.0];

/// Os papéis (largura × altura em mm, deitados). O modelo de folha é deitado; a base do desenho é o A1.
pub const PAPEIS: [(&str, f64, f64); 9] = [
    ("A4", 297.0, 210.0),
    ("A3", 420.0, 297.0),
    ("A2", 594.0, 420.0),
    ("A1", 841.0, 594.0),
    ("A0", 1189.0, 841.0),
    ("2A0", 1682.0, 1189.0),
    ("4A0", 2378.0, 1682.0),
    ("8A0", 3364.0, 2378.0),
    ("16A0", 4756.0, 3364.0),
];

/// O tamanho do papel pelo nome (`A1`, `2A0`…), sem diferenciar maiúscula.
pub fn papel_mm(nome: &str) -> Option<(f64, f64)> {
    PAPEIS.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(nome.trim())).map(|(_, w, h)| (*w, *h))
}

/// O nome do papel de um tamanho (aceita deitado ou em pé); `"personalizado"` se não é nenhum.
pub fn nome_do_papel(a: f64, b: f64) -> &'static str {
    let (maior, menor) = (a.max(b), a.min(b));
    PAPEIS.iter().find(|(_, w, h)| (w - maior).abs() < 1.0 && (h - menor).abs() < 1.0).map_or("personalizado", |(n, _, _)| n)
}

/// A escala como se escreve: `1:100`, ou `2:1` para ampliação.
pub fn texto_da_escala(denominador: f64) -> String {
    let n = |v: f64| if (v - v.round()).abs() < 1e-9 { format!("{}", v.round() as i64) } else { format!("{v:.2}") };
    if denominador >= 1.0 {
        format!("1:{}", n(denominador))
    } else {
        format!("{}:1", n(1.0 / denominador))
    }
}

/// O que o carimbo diz. Campo vazio fica com o texto-guia do modelo.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Campos {
    pub cliente: String,
    pub fase: String,
    pub disciplina: String,
    pub titulo: String,
    pub subtitulo: String,
    pub responsavel: String,
    pub numero: String,
    pub revisao: String,
    pub revisao_descricao: String,
    /// O número da folha no carimbo; vazio = a ordem de criação (01, 02…).
    pub folha: String,
    /// A linha horizontal no meio da folha (divide a área em duas vistas, como na prancheta do escritório).
    pub divisoria: bool,
}

/// Uma folha do desenho, como a interface a lista.
#[derive(Clone, Debug, Serialize)]
pub struct InfoFolha {
    pub indice: usize,
    pub id: Id,
    pub nome: String,
    pub papel: String,
    pub largura_mm: f64,
    pub altura_mm: f64,
    /// O denominador (100 = 1:100).
    pub escala: f64,
    pub escala_texto: String,
    /// A folha em unidades do desenho (canto inferior esquerdo e superior direito).
    pub min: Ponto,
    pub max: Ponto,
}

/// Onde cada campo do carimbo mora (mm de papel, base A1) e o texto-guia de quando está vazio.
const X_CARIMBO: f64 = 656.4;
const CAMPOS_DO_CARIMBO: [(&str, f64, f64, f64, &str); 10] = [
    ("cliente", X_CARIMBO + 1.5, 67.2, 2.8, "NOME DO CLIENTE"),
    ("fase", X_CARIMBO + 1.5, 59.5, 2.8, "FASE DE PROJETO"),
    ("disciplina", X_CARIMBO + 1.5, 51.8, 2.8, "DISCIPLINA"),
    ("titulo", X_CARIMBO + 1.5, 46.0, 2.6, "TÍTULO DO DESENHO"),
    ("subtitulo", X_CARIMBO + 1.5, 42.6, 1.9, ""),
    ("responsavel", 507.7, 32.0, 2.0, "RESPONSÁVEL TÉCNICO / EMPRESA"),
    ("revisao_descricao", 603.7, 58.0, 1.7, ""),
    ("numero", X_CARIMBO + 30.0, 20.0, 3.4, "—"),
    ("folha", X_CARIMBO + 96.5, 20.0, 3.4, ""),
    ("revisao", X_CARIMBO + 165.0, 20.0, 3.4, "00"),
];

fn valor_do_campo<'a>(campos: &'a Campos, nome: &str) -> &'a str {
    match nome {
        "cliente" => &campos.cliente,
        "fase" => &campos.fase,
        "disciplina" => &campos.disciplina,
        "titulo" => &campos.titulo,
        "subtitulo" => &campos.subtitulo,
        "responsavel" => &campos.responsavel,
        "numero" => &campos.numero,
        "revisao" => &campos.revisao,
        "revisao_descricao" => &campos.revisao_descricao,
        "folha" => &campos.folha,
        _ => "",
    }
}

fn guardar_o_campo(campos: &mut Campos, nome: &str, valor: String) {
    match nome {
        "cliente" => campos.cliente = valor,
        "fase" => campos.fase = valor,
        "disciplina" => campos.disciplina = valor,
        "titulo" => campos.titulo = valor,
        "subtitulo" => campos.subtitulo = valor,
        "responsavel" => campos.responsavel = valor,
        "numero" => campos.numero = valor,
        "revisao" => campos.revisao = valor,
        "revisao_descricao" => campos.revisao_descricao = valor,
        "folha" => campos.folha = valor,
        _ => {}
    }
}

// ---------------------------------------------------------------- o modelo de folha (em mm de papel, base A1 841 x 594)

pub const BASE_W: f64 = 841.0;
/// A faixa inferior (mm de papel, base A1): onde começam as caixas de notas, legenda, revisões/responsável e carimbo, e o topo da faixa.
pub const X_NOTAS: f64 = 165.3;
pub const X_LEG: f64 = 505.7;
pub const X_REV: f64 = 656.4;
pub const Y_FAIXA: f64 = 99.0;
/// A área útil para o desenho: dentro da moldura, acima da faixa, com 10 mm de respiro (base A1).
pub const AREA: (f64, f64, f64, f64) = (35.0, 109.0, 821.0, 574.0);

struct Pena {
    f: f64,
    folha: usize,
    carimbo: usize,
    fora: Vec<NovaEntidade>,
}

impl Pena {
    fn nova(&mut self, camada: usize, geometria: Geometria) {
        self.fora.push(NovaEntidade { camada, cor: None, geometria, props: Propriedades::default(), espaco: Espaco::Modelo });
    }
    fn linha(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let f = self.f;
        self.nova(self.carimbo, Geometria::Linha { a: Ponto::novo(x1 * f, y1 * f), b: Ponto::novo(x2 * f, y2 * f) });
    }
    fn retangulo(&mut self, camada: usize, x: f64, y: f64, w: f64, h: f64) {
        let f = self.f;
        let v = |px: f64, py: f64| Vertice { p: Ponto::novo(px * f, py * f), bulge: 0.0 };
        self.nova(camada, Geometria::Polilinha { vertices: vec![v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)], fechada: true });
    }
    fn texto(&mut self, x: f64, y: f64, altura: f64, h: AlinhaH, conteudo: &str) {
        if conteudo.is_empty() {
            return;
        }
        let f = self.f;
        self.nova(self.carimbo, Geometria::Texto { ponto: Ponto::novo(x * f, y * f), altura: altura * f, rotacao: 0.0, conteudo: conteudo.to_string(), h, v: AlinhaV::Base });
    }
}

fn ou<'a>(valor: &'a str, guia: &'a str) -> &'a str {
    if valor.trim().is_empty() {
        guia
    } else {
        valor.trim()
    }
}

/// As entidades da folha em mm de papel já na proporção do `papel` (a moldura é a primeira, na camada `FOLHA`).
fn desenhar(largura_mm: f64, escala: f64, campos: &Campos, numero_da_folha: usize, folha: usize, carimbo: usize) -> Vec<NovaEntidade> {
    let mut p = Pena { f: largura_mm / BASE_W, folha, carimbo, fora: Vec::new() };
    let (folha_c, esq, cen) = (p.folha, AlinhaH::Esquerda, AlinhaH::Centro);
    p.retangulo(folha_c, 0.0, 0.0, 841.0, 594.0);
    p.retangulo(p.carimbo, 25.0, 10.0, 806.0, 574.0);
    for x in [210.6, 341.0, 471.4, 656.4] {
        p.linha(x, 584.0, x, 594.0);
        p.linha(x, 0.0, x, 10.0);
    }
    p.linha(0.0, 300.0, 25.0, 300.0);
    p.linha(831.0, 300.0, 841.0, 300.0);
    if campos.divisoria {
        p.linha(25.0, 341.8, 831.0, 341.8);
    }
    // faixa inferior: notas, legenda, revisões e carimbo
    let (x_notas, x_leg, x_rev, y_faixa) = (X_NOTAS, X_LEG, X_REV, Y_FAIXA);
    p.linha(25.0, y_faixa, 831.0, y_faixa);
    for x in [x_notas, x_leg, x_rev] {
        p.linha(x, 10.0, x, y_faixa);
    }
    p.linha(25.0, y_faixa - 6.0, x_rev, y_faixa - 6.0);
    p.texto((25.0 + x_notas) / 2.0, y_faixa - 4.5, 2.2, cen, "NOTAS");
    p.texto((x_notas + x_leg) / 2.0, y_faixa - 4.5, 2.2, cen, "LEGENDA");
    // revisões
    let cols = [x_leg, x_leg + 9.6, x_leg + 98.0, x_leg + 110.0, x_leg + 122.0, x_leg + 135.0, x_rev];
    for i in 1..8 {
        p.linha(x_leg, y_faixa - 6.1 * f64::from(i), x_rev, y_faixa - 6.1 * f64::from(i));
    }
    for y in [56.3, 50.2, 44.1] {
        p.linha(x_leg, y, x_rev, y);
    }
    for x in &cols[1..6] {
        p.linha(*x, 50.2, *x, y_faixa);
    }
    for (x, t) in [(cols[0] + 2.0, "REV"), (cols[1] + 30.0, "DESCRIÇÃO"), (cols[2] + 1.5, "FINAL"), (cols[3] + 2.5, "INI."), (cols[4] + 2.5, "VIS."), (cols[5] + 4.0, "DATA")] {
        p.texto(x, 52.0, 1.7, esq, t);
    }
    p.texto((x_leg + x_rev) / 2.0, 45.6, 2.2, cen, "REVISÕES");
    p.texto(x_leg + 2.0, 39.0, 1.9, esq, "RESPONSÁVEL TÉCNICO / EMPRESA:");
    // carimbo
    let (x0, x1) = (x_rev, 831.0);
    for y in [80.0, 72.5, 64.8, 57.1, 49.4, 41.7, 33.0] {
        p.linha(x0, y, x1, y);
    }
    p.linha(x0 + 88.5, 80.0, x0 + 88.5, y_faixa);
    p.retangulo(p.carimbo, x0 + 32.7, 82.0, 31.9, 15.0);
    p.retangulo(p.carimbo, x0 + 120.4, 82.0, 22.3, 15.0);
    for x in [x0 + 28.7, x0 + 94.9, x0 + 160.2] {
        p.linha(x, 10.0, x, 41.7);
    }
    p.texto(x0 + 1.5, 75.0, 1.6, esq, "PROJETO");
    let numero_padrao = format!("{numero_da_folha:02}");
    for (nome, x, y, h, guia) in CAMPOS_DO_CARIMBO {
        let texto = ou(valor_do_campo(campos, nome), guia);
        p.texto(x, y, h, esq, if nome == "folha" && campos.folha.trim().is_empty() { &numero_padrao } else { texto });
    }
    for (x, t) in [(x0 + 1.5, "ESCALA"), (x0 + 30.0, "Nº DO DESENHO"), (x0 + 96.5, "FOLHA"), (x0 + 161.5, "REV.")] {
        p.texto(x, 35.0, 1.5, esq, t);
    }
    p.texto(x0 + 1.5, 20.0, 3.4, esq, &texto_da_escala(escala));
    p.fora
}

// ---------------------------------------------------------------- no documento

impl Documento {
    fn milimetros_da_unidade(&self) -> f64 {
        self.unidade().unwrap_or(Unidade::Milimetro).em_milimetros()
    }

    fn indice_da_camada_ou_nova(&mut self, nome: &str, cor: u32, espessura: Option<f32>) -> usize {
        if let Some(i) = self.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(nome)) {
            return i;
        }
        let mut c = Camada::nova(nome, cor);
        c.espessura = espessura;
        self.camadas.push(c);
        self.camadas.len() - 1
    }

    fn captura(&self) -> Captura {
        Captura { camadas: self.camadas.clone(), atual: self.camada_atual, entidades: self.entidades.clone(), blocos: self.blocos.clone() }
    }

    /// As folhas do desenho, na ordem em que foram criadas.
    pub fn folhas_inseridas(&self) -> Vec<InfoFolha> {
        let em_mm = self.milimetros_da_unidade();
        let mut lista = Vec::new();
        for e in &self.entidades {
            let Geometria::Insercao { bloco, ponto, escala_x, rotacao, .. } = &e.geometria else { continue };
            let Some(b) = self.blocos.get(*bloco).filter(|b| b.nome.starts_with(PREFIXO)) else { continue };
            let Some((min, max)) = self.caixa_guardada_do_bloco(*bloco) else { continue };
            if rotacao.abs() > 1e-9 || *escala_x <= 0.0 {
                continue;
            }
            let (w, h) = (max.x - min.x, max.y - min.y);
            let escala = escala_x * em_mm;
            lista.push(InfoFolha {
                indice: lista.len(),
                id: e.id,
                nome: b.nome.clone(),
                papel: nome_do_papel(w, h).to_string(),
                largura_mm: w,
                altura_mm: h,
                escala,
                escala_texto: texto_da_escala(escala),
                min: Ponto::novo(ponto.x + (min.x - b.base.x) * escala_x, ponto.y + (min.y - b.base.y) * escala_x),
                max: Ponto::novo(ponto.x + (max.x - b.base.x) * escala_x, ponto.y + (max.y - b.base.y) * escala_x),
            });
        }
        lista
    }

    /// A caixa de tudo o que **não** é folha (o "produto" que vai ser enquadrado).
    fn caixa_do_modelo(&self) -> Option<(Ponto, Ponto)> {
        let mut caixa: Option<(Ponto, Ponto)> = None;
        for e in self.entidades.iter().filter(|e| self.visivel(e)) {
            if let Geometria::Insercao { bloco, .. } = &e.geometria {
                if self.blocos.get(*bloco).is_some_and(|b| b.nome.starts_with(PREFIXO)) {
                    continue;
                }
            }
            if let Some((a, b)) = caixa_da_entidade(self, &e.geometria) {
                caixa = Some(match caixa {
                    None => (a, b),
                    Some((min, max)) => (Ponto::novo(min.x.min(a.x), min.y.min(a.y)), Ponto::novo(max.x.max(b.x), max.y.max(b.y))),
                });
            }
        }
        caixa
    }

    /// A caixa delimitadora de uma lista de IDs de entidades.
    pub fn caixa_dos_ids(&self, ids: &[Id]) -> Option<(Ponto, Ponto)> {
        let mut caixa: Option<(Ponto, Ponto)> = None;
        for id in ids {
            if let Some(e) = self.entidade(*id) {
                if let Some((a, b)) = caixa_da_entidade(self, &e.geometria) {
                    caixa = Some(match caixa {
                        None => (a, b),
                        Some((min, max)) => (Ponto::novo(min.x.min(a.x), min.y.min(a.y)), Ponto::novo(max.x.max(b.x), max.y.max(b.y))),
                    });
                }
            }
        }
        caixa
    }

    /// A menor escala oferecida em que uma determinada caixa cabe na área útil da folha.
    pub fn escala_que_cabe_na_caixa(&self, (min, max): (Ponto, Ponto), largura_mm: f64) -> f64 {
        let f = largura_mm / BASE_W;
        let (aw, ah) = ((AREA.2 - AREA.0) * f, (AREA.3 - AREA.1) * f);
        let em_mm = self.milimetros_da_unidade();
        let precisa = ((max.x - min.x) * em_mm / aw).max((max.y - min.y) * em_mm / ah);
        ESCALAS.iter().copied().find(|e| *e >= precisa).unwrap_or_else(|| precisa.ceil())
    }

    /// A menor escala oferecida em que o modelo cabe na área útil da folha.
    fn escala_que_cabe(&self, largura_mm: f64) -> f64 {
        let Some(c) = self.caixa_do_modelo() else { return 100.0 };
        self.escala_que_cabe_na_caixa(c, largura_mm)
    }

    /// Onde a inserção deve ficar para o centro da área útil coincidir com `centro`.
    fn ponto_para_centrar(&self, centro: Ponto, largura_mm: f64, escala: f64) -> Ponto {
        let (k, f) = (escala / self.milimetros_da_unidade(), largura_mm / BASE_W);
        Ponto::novo(centro.x - k * (AREA.0 + AREA.2) / 2.0 * f, centro.y - k * (AREA.1 + AREA.3) / 2.0 * f)
    }

    /// Insere uma folha nova permitindo especificar opcionalmente IDs de objetos alvo para cobrir apenas a seleção/objeto.
    pub fn folha_inserir_ids(&mut self, papel: &str, escala: Option<f64>, campos: &Campos, ids_alvo: Option<&[Id]>) -> Result<InfoFolha, String> {
        let (largura, _) = papel_mm(papel).ok_or_else(|| format!("papel `{papel}` não existe ({})", PAPEIS.map(|(n, _, _)| n).join(", ")))?;
        let caixa_alvo = ids_alvo.and_then(|ids| self.caixa_dos_ids(ids)).or_else(|| self.caixa_do_modelo());
        let escala = match escala {
            Some(e) if e.is_finite() && e > 0.0 && e <= 1.0e6 => e,
            Some(e) => return Err(format!("escala inválida: {e}")),
            None => match caixa_alvo {
                Some(c) => self.escala_que_cabe_na_caixa(c, largura),
                None => 100.0,
            },
        };
        let existentes = self.folhas_inseridas();
        let captura = self.captura();
        let (camada_folha, camada_carimbo) = (self.indice_da_camada_ou_nova(CAMADA_FOLHA, 0xFF_FF_FF, Some(0.5)), self.indice_da_camada_ou_nova(CAMADA_CARIMBO, 0x8C_8C_8C, None));
        let mut n = existentes.len() + 1;
        while self.bloco_por_nome(&format!("{PREFIXO}{n}")).is_some() {
            n += 1;
        }
        let entidades = desenhar(largura, escala, campos, existentes.len() + 1, camada_folha, camada_carimbo);
        let k = escala / self.milimetros_da_unidade();

        let ponto = match (ids_alvo.is_some(), caixa_alvo, existentes.iter().max_by(|a, b| a.max.x.total_cmp(&b.max.x))) {
            (true, Some((a, b)), _) => self.ponto_para_centrar(Ponto::novo((a.x + b.x) / 2.0, (a.y + b.y) / 2.0), largura, escala),
            (false, _, Some(ultima)) => Ponto::novo(ultima.max.x + 0.05 * (ultima.max.x - ultima.min.x), ultima.min.y),
            (false, Some((a, b)), None) => self.ponto_para_centrar(Ponto::novo((a.x + b.x) / 2.0, (a.y + b.y) / 2.0), largura, escala),
            (_, None, _) => Ponto::novo(0.0, 0.0),
        };

        self.desfazer.push(Passo::Tudo(Box::new(captura)));
        self.refazer.clear();
        self.blocos.push(Bloco { nome: format!("{PREFIXO}{n}"), base: Ponto::novo(0.0, 0.0), entidades });
        let indice = self.blocos.len() - 1;
        self.caixas_dos_blocos.push(self.caixa_do_bloco(indice, 0));
        let id = self.proximo_id;
        self.proximo_id += 1;
        self.entidades.push(crate::documento::Entidade { id, camada: camada_folha, cor: None, geometria: Geometria::Insercao { bloco: indice, ponto, escala_x: k, escala_y: k, rotacao: 0.0 }, props: Propriedades::default(), espaco: Espaco::Modelo });
        self.mudou();
        self.folhas_inseridas().into_iter().find(|f| f.id == id).ok_or_else(|| "a folha não apareceu".to_string())
    }

    /// Insere uma folha nova. `escala` `None` = a menor que faz o desenho caber. A primeira folha se centra no desenho;
    /// as seguintes vão à direita da última. Um passo de desfazer. Devolve a folha criada.
    pub fn folha_inserir(&mut self, papel: &str, escala: Option<f64>, campos: &Campos) -> Result<InfoFolha, String> {
        self.folha_inserir_ids(papel, escala, campos, None)
    }

    /// Os campos do carimbo como estão (o texto-guia volta como vazio).
    pub fn folha_campos(&self, indice: usize) -> Result<Campos, String> {
        let info = self.folhas_inseridas().get(indice).cloned().ok_or_else(|| format!("a folha {} não existe", indice + 1))?;
        let bloco = self.entidades.iter().find(|e| e.id == info.id).and_then(|e| match &e.geometria {
            Geometria::Insercao { bloco, .. } => self.blocos.get(*bloco),
            _ => None,
        });
        let bloco = bloco.ok_or("a folha sumiu")?;
        let f = info.largura_mm / BASE_W;
        let mut campos = Campos::default();
        for (nome, x, y, _, guia) in CAMPOS_DO_CARIMBO {
            let achado = bloco.entidades.iter().find_map(|n| match &n.geometria {
                Geometria::Texto { ponto, conteudo, .. } if (ponto.x - x * f).abs() < 1e-6 && (ponto.y - y * f).abs() < 1e-6 => Some(conteudo.clone()),
                _ => None,
            });
            let valor = achado.filter(|v| v != guia && !(nome == "folha" && v.chars().all(|c| c.is_ascii_digit()) && v.len() == 2)).unwrap_or_default();
            guardar_o_campo(&mut campos, nome, valor);
        }
        Ok(campos)
    }

    /// Reescreve os campos do carimbo. Campo vazio volta ao texto-guia (o subtítulo some). Um passo de desfazer.
    pub fn folha_campos_definir(&mut self, indice: usize, campos: &Campos) -> Result<InfoFolha, String> {
        let info = self.folhas_inseridas().get(indice).cloned().ok_or_else(|| format!("a folha {} não existe", indice + 1))?;
        let Some(Geometria::Insercao { bloco, .. }) = self.entidades.iter().find(|e| e.id == info.id).map(|e| e.geometria.clone()) else { return Err("a folha sumiu".into()) };
        let camada_carimbo = self.indice_da_camada_ou_nova(CAMADA_CARIMBO, 0x8C_8C_8C, None);
        self.desfazer.push(Passo::Tudo(Box::new(self.captura())));
        self.refazer.clear();
        let f = info.largura_mm / BASE_W;
        let numero_padrao = format!("{:02}", indice + 1);
        let b = self.blocos.get_mut(bloco).ok_or("a folha sumiu")?;
        for (nome, x, y, altura, guia) in CAMPOS_DO_CARIMBO {
            let pedido = valor_do_campo(campos, nome).trim();
            let novo = if pedido.is_empty() { if nome == "folha" { numero_padrao.as_str() } else { guia } } else { pedido };
            let pos = b.entidades.iter().position(|n| matches!(&n.geometria, Geometria::Texto { ponto, .. } if (ponto.x - x * f).abs() < 1e-6 && (ponto.y - y * f).abs() < 1e-6));
            match (pos, novo.is_empty()) {
                (Some(i), true) => {
                    b.entidades.remove(i);
                }
                (Some(i), false) => {
                    if let Geometria::Texto { conteudo, .. } = &mut b.entidades[i].geometria {
                        *conteudo = novo.to_string();
                    }
                }
                (None, false) => b.entidades.push(NovaEntidade {
                    camada: camada_carimbo,
                    cor: None,
                    geometria: Geometria::Texto { ponto: Ponto::novo(x * f, y * f), altura: altura * f, rotacao: 0.0, conteudo: novo.to_string(), h: AlinhaH::Esquerda, v: AlinhaV::Base },
                    props: Propriedades::default(),
                    espaco: Espaco::Modelo,
                }),
                (None, true) => {}
            }
        }
        self.mudou();
        self.folhas_inseridas().into_iter().find(|i| i.id == info.id).ok_or_else(|| "a folha não apareceu".to_string())
    }

    /// Troca a escala de uma folha mantendo no mesmo lugar o centro da área útil (a folha continua enquadrando o que enquadrava).
    /// Também atualiza o texto da escala no carimbo. Um passo de desfazer.
    pub fn folha_escala(&mut self, indice: usize, escala: f64) -> Result<InfoFolha, String> {
        if !(escala.is_finite() && escala > 0.0 && escala <= 1.0e6) {
            return Err(format!("escala inválida: {escala}"));
        }
        let info = self.folhas_inseridas().get(indice).cloned().ok_or_else(|| format!("a folha {} não existe", indice + 1))?;
        let f = info.largura_mm / BASE_W;
        let (k_antigo, k) = (info.escala / self.milimetros_da_unidade(), escala / self.milimetros_da_unidade());
        let (cx, cy) = ((AREA.0 + AREA.2) / 2.0 * f, (AREA.1 + AREA.3) / 2.0 * f);
        let Some(Geometria::Insercao { bloco, ponto, .. }) = self.entidades.iter().find(|e| e.id == info.id).map(|e| e.geometria.clone()) else { return Err("a folha sumiu".into()) };
        let centro = Ponto::novo(ponto.x + k_antigo * cx, ponto.y + k_antigo * cy);
        let novo = Ponto::novo(centro.x - k * cx, centro.y - k * cy);
        self.desfazer.push(Passo::Tudo(Box::new(self.captura())));
        self.refazer.clear();
        if let Some(e) = self.entidades.iter_mut().find(|e| e.id == info.id) {
            e.geometria = Geometria::Insercao { bloco, ponto: novo, escala_x: k, escala_y: k, rotacao: 0.0 };
        }
        if let Some(b) = self.blocos.get_mut(bloco) {
            let dono = b.entidades.iter_mut().find_map(|n| match &mut n.geometria {
                Geometria::Texto { conteudo, .. } if conteudo == &texto_da_escala(info.escala) => Some(conteudo),
                _ => None,
            });
            if let Some(c) = dono {
                *c = texto_da_escala(escala);
            }
        }
        self.mudou();
        self.folhas_inseridas().into_iter().find(|f| f.id == info.id).ok_or_else(|| "a folha não apareceu".to_string())
    }

    /// Escolhe a menor escala em que o desenho (tudo o que não é folha) cabe na folha e centra a folha nele.
    pub fn folha_ajustar(&mut self, indice: usize) -> Result<InfoFolha, String> {
        let info = self.folhas_inseridas().get(indice).cloned().ok_or_else(|| format!("a folha {} não existe", indice + 1))?;
        let (min, max) = self.caixa_do_modelo().ok_or("não há desenho para enquadrar")?;
        let escala = self.escala_que_cabe(info.largura_mm);
        let Some(Geometria::Insercao { bloco, .. }) = self.entidades.iter().find(|e| e.id == info.id).map(|e| e.geometria.clone()) else { return Err("a folha sumiu".into()) };
        let novo = self.ponto_para_centrar(Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0), info.largura_mm, escala);
        let k = escala / self.milimetros_da_unidade();
        self.desfazer.push(Passo::Tudo(Box::new(self.captura())));
        self.refazer.clear();
        if let Some(e) = self.entidades.iter_mut().find(|e| e.id == info.id) {
            e.geometria = Geometria::Insercao { bloco, ponto: novo, escala_x: k, escala_y: k, rotacao: 0.0 };
        }
        if let Some(b) = self.blocos.get_mut(bloco) {
            for n in &mut b.entidades {
                if let Geometria::Texto { conteudo, .. } = &mut n.geometria {
                    if *conteudo == texto_da_escala(info.escala) {
                        *conteudo = texto_da_escala(escala);
                    }
                }
            }
        }
        self.mudou();
        self.folhas_inseridas().into_iter().find(|f| f.id == info.id).ok_or_else(|| "a folha não apareceu".to_string())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn com_linha(unidade: Unidade, comprimento: f64) -> Documento {
        let mut d = Documento::default().com_unidade(Some(unidade));
        d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(comprimento, comprimento / 2.0));
        d
    }

    #[test]
    fn escala_em_metros_bate_com_a_conta_de_papel() {
        // 500 m x 250 m em A1: área útil 786 x 465 mm -> 500 m pedem 636 mm de papel: a menor escala oferecida é 1:750
        let mut d = com_linha(Unidade::Metro, 500.0);
        let f = d.folha_inserir("A1", None, &Campos::default()).unwrap();
        assert_eq!(f.escala, 750.0);
        assert_eq!(f.papel, "A1");
        // a folha de 841 mm a 1:750 tem 630,75 m e cobre o desenho
        assert_eq!((f.max.x - f.min.x).round(), 631.0);
        assert!(f.min.x < 0.0 && f.max.x > 500.0);
    }

    #[test]
    fn trocar_a_escala_mantem_o_centro_e_atualiza_o_carimbo() {
        let mut d = com_linha(Unidade::Metro, 200.0);
        let a = d.folha_inserir("A1", Some(500.0), &Campos::default()).unwrap();
        // o centro da área útil (não o da folha inteira: a faixa inferior pesa) é o ponto fixo
        let centro = |f: &InfoFolha| (f.min.x + (f.max.x - f.min.x) * 428.0 / 841.0, f.min.y + (f.max.y - f.min.y) * 341.5 / 594.0);
        let b = d.folha_escala(0, 1000.0).unwrap();
        assert_eq!(b.escala, 1000.0);
        assert!((centro(&a).0 - centro(&b).0).abs() < 1e-6 && (centro(&a).1 - centro(&b).1).abs() < 1e-6);
        let bloco = &d.blocos()[d.bloco_por_nome("FOLHA-1").unwrap()];
        assert!(bloco.entidades.iter().any(|e| matches!(&e.geometria, Geometria::Texto { conteudo, .. } if conteudo == "1:1000")));
        assert!(!bloco.entidades.iter().any(|e| matches!(&e.geometria, Geometria::Texto { conteudo, .. } if conteudo == "1:500")));
    }

    #[test]
    fn ajustar_escolhe_a_menor_escala_que_cabe_e_desfaz_de_uma_vez() {
        let mut d = com_linha(Unidade::Milimetro, 100.0);
        d.folha_inserir("A3", Some(1000.0), &Campos::default()).unwrap();
        let f = d.folha_ajustar(0).unwrap();
        assert_eq!(f.escala, 1.0, "100 mm de desenho cabem em A3 a 1:1");
        assert!(d.desfazer());
        assert_eq!(d.folhas_inseridas()[0].escala, 1000.0);
        assert!(d.desfazer());
        assert!(d.folhas_inseridas().is_empty());
    }

    #[test]
    fn as_folhas_seguintes_vao_a_direita_e_papel_ruim_da_erro() {
        let mut d = com_linha(Unidade::Metro, 100.0);
        let a = d.folha_inserir("A3", Some(500.0), &Campos::default()).unwrap();
        let b = d.folha_inserir("A3", Some(500.0), &Campos::default()).unwrap();
        assert!(b.min.x > a.max.x);
        assert_eq!(b.nome, "FOLHA-2");
        assert!(d.folha_inserir("B7", None, &Campos::default()).is_err());
        assert!(d.folha_escala(5, 100.0).is_err());
    }

    #[test]
    fn campos_do_carimbo_lidos_e_reescritos_e_desfeitos() {
        let mut d = com_linha(Unidade::Metro, 100.0);
        d.folha_inserir("A1", Some(500.0), &Campos { titulo: "TÚNEL".into(), ..Campos::default() }).unwrap();
        let lidos = d.folha_campos(0).unwrap();
        assert_eq!(lidos.titulo, "TÚNEL");
        assert_eq!(lidos.cliente, "", "o texto-guia volta como vazio");
        let novos = Campos { cliente: "Prefeitura".into(), titulo: "PLANTA".into(), subtitulo: "trecho 1".into(), revisao: "B".into(), ..Campos::default() };
        d.folha_campos_definir(0, &novos).unwrap();
        let depois = d.folha_campos(0).unwrap();
        assert_eq!((depois.cliente.as_str(), depois.titulo.as_str(), depois.subtitulo.as_str(), depois.revisao.as_str()), ("Prefeitura", "PLANTA", "trecho 1", "B"));
        d.folha_campos_definir(0, &Campos::default()).unwrap();
        assert_eq!(d.folha_campos(0).unwrap().subtitulo, "", "subtítulo vazio some");
        assert!(d.desfazer());
        assert_eq!(d.folha_campos(0).unwrap().titulo, "PLANTA");
    }

    #[test]
    fn texto_da_escala_e_ampliacao() {
        assert_eq!(texto_da_escala(100.0), "1:100");
        assert_eq!(texto_da_escala(0.5), "2:1");
    }

    #[test]
    fn folha_inserir_enquadra_apenas_objetos_selecionados() {
        let mut d = com_linha(Unidade::Metro, 10.0);
        let id2 = d.adicionar_linha(Ponto::novo(200.0, 0.0), Ponto::novo(210.0, 0.0));

        let f = d.folha_inserir_ids("A4", None, &Campos::default(), Some(&[id2])).unwrap();
        let centro_folha_x = (f.min.x + f.max.x) / 2.0;
        assert!((centro_folha_x - 205.0).abs() < 20.0, "a folha centra perto de X=205 (objeto selecionado)");
    }
}

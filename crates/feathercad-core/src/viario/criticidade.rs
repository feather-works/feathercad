//! Análise dos elementos do traçado: para cada **curva** (TS a ST) e cada **tangente** diz o que é crítico, o que pede atenção e
//! onde há **aterro** (greide acima do existente) ou **escavação** (greide abaixo), com os motivos escritos. Os limiares são
//! `Criterios` (ajustáveis); as contas são do motor, a ferramenta MCP e as folhas só leem o resultado.
//!
//! Convenções: progressivas absolutas (m); levante = cota do greide − cota existente (positivo = aterro/lastro, negativo =
//! escavação/rebaixo); ripagem = afastamento do eixo existente em relação ao de projeto (m).

use super::alinhamento::Alinhamento;
use super::greide::Greide;
use super::superelevacao::{Regras, ResultadoCurva};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Nivel {
    Normal,
    Atencao,
    Critico,
}

impl Nivel {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Atencao => "ATENÇÃO",
            Self::Critico => "CRÍTICO",
        }
    }
}

/// Os limiares da análise. Os padrões são pontos de partida de projeto, não texto de norma: ajuste-os ao seu critério.
#[derive(Clone, Debug, PartialEq)]
pub struct Criterios {
    /// Atenção quando h adotado chega a esta fração de h_max.
    pub fracao_h: f64,
    /// Atenção quando a insuficiência chega a esta fração de I_max.
    pub fracao_i: f64,
    /// Atenção quando R < esta vez o R_min.
    pub folga_r: f64,
    /// Atenção quando V_max da curva < esta vez a velocidade diretriz.
    pub folga_v: f64,
    /// Atenção quando Ls < esta vez o L_min.
    pub folga_ls: f64,
    /// Ripagem máxima (m) a partir da qual há atenção / é crítico.
    pub ripagem_atencao: f64,
    pub ripagem_critica: f64,
    /// |levante| (m) abaixo do qual não se fala em aterro/escavação; a partir de `movimento_atencao` há atenção e de `movimento_critico` é crítico.
    pub movimento_minimo: f64,
    pub movimento_atencao: f64,
    pub movimento_critico: f64,
    /// Rampa (%) de atenção e crítica.
    pub rampa_atencao_pct: f64,
    pub rampa_critica_pct: f64,
    /// Tangente mais curta que isto (m) entre curvas pede atenção.
    pub tangente_minima: f64,
    /// Menos observações que isto no elemento: pouca base para confiar na análise.
    pub observacoes_minimas: usize,
    /// Largura da plataforma (m) para o volume de terraplenagem (estimativa a partir do eixo).
    pub largura_plataforma: f64,
}

impl Default for Criterios {
    fn default() -> Self {
        Self {
            fracao_h: 0.95,
            fracao_i: 0.80,
            folga_r: 1.25,
            folga_v: 1.15,
            folga_ls: 1.03,
            ripagem_atencao: 0.50,
            ripagem_critica: 0.75,
            movimento_minimo: 0.05,
            movimento_atencao: 0.10,
            movimento_critico: 0.20,
            rampa_atencao_pct: 1.0,
            rampa_critica_pct: 1.5,
            tangente_minima: 30.0,
            observacoes_minimas: 3,
            largura_plataforma: 4.0,
        }
    }
}

/// Um ponto do levantamento projetado no traçado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observacao {
    pub s: f64,
    /// Cota do existente (m).
    pub z: f64,
    /// Ripagem (m, com sinal).
    pub ripagem: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Natureza {
    Aterro,
    Escavacao,
}

impl Natureza {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Aterro => "aterro",
            Self::Escavacao => "escavação",
        }
    }
}

/// Um trecho contínuo de aterro ou de escavação.
#[derive(Clone, Debug, PartialEq)]
pub struct Zona {
    pub natureza: Natureza,
    pub de: f64,
    pub ate: f64,
    /// O maior |levante| da zona, m (sempre positivo).
    pub maximo: f64,
    /// A progressiva onde ele acontece.
    pub onde: f64,
    pub nivel: Nivel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    /// Índice da curva (0 = a primeira).
    Curva(usize),
    /// Índice da tangente (0 = a anterior à primeira curva).
    Tangente(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Motivo {
    pub nivel: Nivel,
    pub texto: String,
}

#[derive(Clone, Debug)]
pub struct Elemento {
    pub tipo: Tipo,
    pub de: f64,
    pub ate: f64,
    pub nivel: Nivel,
    pub motivos: Vec<Motivo>,
    pub observacoes: usize,
    pub levante_min: f64,
    pub levante_max: f64,
    pub levante_medio: f64,
    pub ripagem_max: f64,
    pub ripagem_rms: f64,
    pub rampa_min_pct: f64,
    pub rampa_max_pct: f64,
    pub cota_min: f64,
    pub cota_max: f64,
    pub zonas: Vec<Zona>,
}

impl Elemento {
    pub fn comprimento(&self) -> f64 {
        self.ate - self.de
    }
}

/// Volumes de terraplenagem estimados e a curva de massa.
#[derive(Clone, Debug, Default)]
pub struct Volumes {
    pub aterro_m3: f64,
    pub escavacao_m3: f64,
    pub largura: f64,
    /// (progressiva, volume acumulado em m³; aterro positivo).
    pub curva_de_massa: Vec<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct Analise {
    pub elementos: Vec<Elemento>,
    /// Todas as zonas do traçado, em ordem.
    pub zonas: Vec<Zona>,
    pub volumes: Volumes,
    pub criterios: Criterios,
}

impl Analise {
    pub fn contagem(&self, n: Nivel) -> usize {
        self.elementos.iter().filter(|e| e.nivel == n).count()
    }
}

/// Passo da varredura do levante, m.
const PASSO: f64 = 5.0;
/// Lacuna máxima (m) entre observações para ainda se interpolar a cota existente.
const LACUNA_MAXIMA: f64 = 30.0;

/// Cota existente em `s` por interpolação linear entre as observações vizinhas (`None` dentro de uma lacuna grande).
pub fn cota_existente(obs: &[Observacao], s: f64) -> Option<f64> {
    let i = obs.partition_point(|o| o.s < s);
    match (i.checked_sub(1).map(|k| obs[k]), obs.get(i)) {
        (Some(a), Some(b)) if b.s - a.s <= LACUNA_MAXIMA => Some(a.z + (b.z - a.z) * (s - a.s) / (b.s - a.s).max(1e-9)),
        (Some(a), Some(b)) => ((s - a.s).abs() <= 5.0).then_some(a.z).or_else(|| ((b.s - s).abs() <= 5.0).then_some(b.z)),
        (Some(a), None) => ((s - a.s).abs() <= 5.0).then_some(a.z),
        (None, Some(b)) => ((b.s - s).abs() <= 5.0).then_some(b.z),
        (None, None) => None,
    }
}

/// Levante em cada estação: (progressiva, levante, cota do greide).
fn levantes(a: &Alinhamento, g: &Greide, obs: &[Observacao]) -> Vec<(f64, f64, f64)> {
    let (p0, pf) = (a.progressiva_inicial, a.progressiva_final());
    let n = ((pf - p0) / PASSO).ceil() as usize;
    (0..=n)
        .filter_map(|i| {
            let s = (p0 + i as f64 * PASSO).min(pf);
            let zg = g.z(s - p0);
            cota_existente(obs, s).map(|ze| (s, zg - ze, zg))
        })
        .collect()
}

fn nivel_do_movimento(m: f64, c: &Criterios) -> Nivel {
    if m >= c.movimento_critico {
        Nivel::Critico
    } else if m >= c.movimento_atencao {
        Nivel::Atencao
    } else {
        Nivel::Normal
    }
}

/// As zonas contínuas de aterro e de escavação (de pelo menos duas estações seguidas).
fn zonas_de(lev: &[(f64, f64, f64)], c: &Criterios) -> Vec<Zona> {
    let mut zonas: Vec<Zona> = Vec::new();
    let mut atual: Option<(Natureza, usize, usize)> = None;
    let natureza = |l: f64| (l.abs() >= c.movimento_minimo).then_some(if l > 0.0 { Natureza::Aterro } else { Natureza::Escavacao });
    let fechar = |ini: usize, fim: usize, nat: Natureza, zonas: &mut Vec<Zona>| {
        if fim <= ini {
            return;
        }
        let (k, m) = (ini..=fim).map(|j| (j, lev[j].1.abs())).fold((ini, 0.0), |b, x| if x.1 > b.1 { x } else { b });
        zonas.push(Zona { natureza: nat, de: lev[ini].0, ate: lev[fim].0, maximo: m, onde: lev[k].0, nivel: nivel_do_movimento(m, c) });
    };
    for (i, l) in lev.iter().enumerate() {
        let nat = natureza(l.1);
        let contigua = i > 0 && lev[i].0 - lev[i - 1].0 <= PASSO * 1.5;
        match (atual, nat) {
            (Some((n0, ini, _)), Some(n1)) if n0 == n1 && contigua => atual = Some((n0, ini, i)),
            (Some((n0, ini, fim)), _) => {
                fechar(ini, fim, n0, &mut zonas);
                atual = nat.map(|n| (n, i, i));
            }
            (None, Some(n1)) => atual = Some((n1, i, i)),
            (None, None) => {}
        }
    }
    if let Some((n0, ini, fim)) = atual {
        fechar(ini, fim, n0, &mut zonas);
    }
    zonas
}

fn volumes(lev: &[(f64, f64, f64)], c: &Criterios) -> Volumes {
    let mut v = Volumes { largura: c.largura_plataforma, ..Volumes::default() };
    let mut acumulado = 0.0;
    for j in lev.windows(2) {
        let (ds, l) = (j[1].0 - j[0].0, (j[0].1 + j[1].1) / 2.0);
        if ds > PASSO * 1.5 {
            continue;
        }
        let dv = l * c.largura_plataforma * ds;
        if dv > 0.0 { v.aterro_m3 += dv } else { v.escavacao_m3 -= dv }
        acumulado += dv;
        if v.curva_de_massa.last().map_or(true, |u| j[1].0 - u.0 >= 20.0) {
            v.curva_de_massa.push((j[1].0, acumulado));
        }
    }
    v
}

fn pct(x: f64) -> String {
    format!("{x:.1}").replace('.', ",")
}

/// Os motivos de uma curva: o que fica perto de um limite ou além dele.
fn motivos_da_curva(i: usize, a: &Alinhamento, r: &ResultadoCurva, regras: &Regras, c: &Criterios) -> Vec<Motivo> {
    let cv = &a.curvas[i];
    let mut m: Vec<Motivo> = Vec::new();
    let mut dizer = |nivel: Nivel, texto: String| m.push(Motivo { nivel, texto });
    if !r.ok_raio {
        dizer(Nivel::Critico, format!("raio R = {:.0} m menor que o mínimo da velocidade ({:.0} m)", r.raio, r.r_min));
    } else if r.raio < c.folga_r * r.r_min {
        dizer(Nivel::Atencao, format!("raio R = {:.0} m com folga de só {}% sobre o mínimo ({:.0} m)", r.raio, pct((r.raio / r.r_min - 1.0) * 100.0), r.r_min));
    }
    if !r.ok_insuficiencia {
        dizer(Nivel::Critico, format!("insuficiência de superelevação I = {} mm acima do limite ({:.0} mm)", pct(r.i_mm), regras.i_max_mm));
    } else if r.i_mm >= c.fracao_i * regras.i_max_mm {
        dizer(Nivel::Atencao, format!("insuficiência I = {} mm, {:.0}% do limite ({:.0} mm)", pct(r.i_mm), r.i_mm / regras.i_max_mm * 100.0, regras.i_max_mm));
    }
    if r.h_mm >= c.fracao_h * regras.h_max_mm {
        dizer(Nivel::Atencao, format!("superelevação h = {:.0} mm no limite ({:.0} mm): sem reserva para aumentar", r.h_mm, regras.h_max_mm));
    }
    if !r.ok_transicao {
        dizer(Nivel::Critico, format!("transição Ls = {:.0} m menor que a mínima ({} m)", r.ls, pct(r.l_min)));
    } else if r.ls < c.folga_ls * r.l_min {
        dizer(Nivel::Atencao, format!("transição Ls = {:.0} m quase no mínimo ({} m)", r.ls, pct(r.l_min)));
    }
    if r.v_max < regras.v * c.folga_v && r.ok_raio {
        dizer(Nivel::Atencao, format!("velocidade máxima da curva {} km/h, folga pequena sobre a diretriz ({:.0} km/h)", pct(r.v_max), regras.v));
    }
    if cv.lc < regras.lc_min {
        dizer(Nivel::Atencao, format!("trecho circular curto: Lc = {} m (mínimo {:.0} m)", pct(cv.lc), regras.lc_min));
    }
    m
}

fn motivos_da_tangente(de: f64, ate: f64, entre_curvas: bool, c: &Criterios) -> Vec<Motivo> {
    if entre_curvas && ate - de < c.tangente_minima {
        vec![Motivo { nivel: Nivel::Atencao, texto: format!("tangente curta entre curvas: {} m (referência {:.0} m)", pct(ate - de), c.tangente_minima) }]
    } else {
        Vec::new()
    }
}

/// Estatísticas e motivos que valem para os dois tipos de elemento.
fn medidas(de: f64, ate: f64, lev: &[(f64, f64, f64)], obs: &[Observacao], g: &Greide, p0: f64, c: &Criterios) -> (Elemento, Vec<Motivo>) {
    let no = |s: f64| s >= de - 1e-9 && s <= ate + 1e-9;
    let dentro: Vec<&(f64, f64, f64)> = lev.iter().filter(|l| no(l.0)).collect();
    let (lmin, lmax) = dentro.iter().fold((f64::MAX, f64::MIN), |m, l| (m.0.min(l.1), m.1.max(l.1)));
    let medio = dentro.iter().map(|l| l.1).sum::<f64>() / dentro.len().max(1) as f64;
    let rips: Vec<f64> = obs.iter().filter(|o| no(o.s)).map(|o| o.ripagem.abs()).collect();
    let rip_max = rips.iter().copied().fold(0.0, f64::max);
    let rip_rms = (rips.iter().map(|r| r * r).sum::<f64>() / rips.len().max(1) as f64).sqrt();
    let passos = (((ate - de) / PASSO).ceil() as usize).max(1);
    let rampas: Vec<f64> = (0..=passos).map(|k| g.rampa((de + (ate - de) * k as f64 / passos as f64) - p0) * 100.0).collect();
    // elemento sem estação do levantamento (tangente de poucos metros): as cotas são as do greide nas pontas
    let cotas: Vec<f64> = if dentro.is_empty() { vec![g.z(de - p0), g.z(ate - p0)] } else { dentro.iter().map(|l| l.2).collect() };
    let mut motivos = Vec::new();
    if rips.len() < c.observacoes_minimas {
        motivos.push(Motivo { nivel: Nivel::Atencao, texto: format!("só {} ponto(s) do levantamento no elemento: pouca base para confiar na análise", rips.len()) });
    }
    if rip_max >= c.ripagem_critica {
        motivos.push(Motivo { nivel: Nivel::Critico, texto: format!("ripagem máxima de {} m (limite crítico {} m): grande deslocamento da via", pct(rip_max), pct(c.ripagem_critica)) });
    } else if rip_max >= c.ripagem_atencao {
        motivos.push(Motivo { nivel: Nivel::Atencao, texto: format!("ripagem máxima de {} m (atenção a partir de {} m)", pct(rip_max), pct(c.ripagem_atencao)) });
    }
    let rampa_abs = rampas.iter().map(|r| r.abs()).fold(0.0, f64::max);
    if rampa_abs >= c.rampa_critica_pct {
        motivos.push(Motivo { nivel: Nivel::Critico, texto: format!("rampa de {}% (crítica a partir de {}%)", pct(rampa_abs), pct(c.rampa_critica_pct)) });
    } else if rampa_abs >= c.rampa_atencao_pct {
        motivos.push(Motivo { nivel: Nivel::Atencao, texto: format!("rampa de {}% (atenção a partir de {}%)", pct(rampa_abs), pct(c.rampa_atencao_pct)) });
    }
    let e = Elemento {
        tipo: Tipo::Tangente(0),
        de,
        ate,
        nivel: Nivel::Normal,
        motivos: Vec::new(),
        observacoes: rips.len(),
        levante_min: if lmin == f64::MAX { 0.0 } else { lmin },
        levante_max: if lmax == f64::MIN { 0.0 } else { lmax },
        levante_medio: medio,
        ripagem_max: rip_max,
        ripagem_rms: rip_rms,
        rampa_min_pct: rampas.iter().copied().fold(f64::MAX, f64::min),
        rampa_max_pct: rampas.iter().copied().fold(f64::MIN, f64::max),
        cota_min: cotas.iter().copied().fold(f64::MAX, f64::min),
        cota_max: cotas.iter().copied().fold(f64::MIN, f64::max),
        zonas: Vec::new(),
    };
    (e, motivos)
}

/// Analisa o traçado todo. `obs` pode vir em qualquer ordem.
pub fn analisar(a: &Alinhamento, tabela: &[ResultadoCurva], regras: &Regras, g: &Greide, obs: &[Observacao], c: &Criterios) -> Analise {
    let mut obs = obs.to_vec();
    obs.sort_by(|x, y| x.s.total_cmp(&y.s));
    let lev = levantes(a, g, &obs);
    let zonas = zonas_de(&lev, c);
    let (p0, pf) = (a.progressiva_inicial, a.progressiva_final());
    // limites dos elementos na ordem do traçado: T0, C0, T1, C1, …, Tn
    let mut quebras: Vec<(Tipo, f64, f64, bool)> = Vec::new();
    let mut cursor = p0;
    for (i, cv) in a.curvas.iter().enumerate() {
        quebras.push((Tipo::Tangente(i), cursor, cv.ts_marco.progressiva, i > 0));
        quebras.push((Tipo::Curva(i), cv.ts_marco.progressiva, cv.st.progressiva, false));
        cursor = cv.st.progressiva;
    }
    quebras.push((Tipo::Tangente(a.curvas.len()), cursor, pf, !a.curvas.is_empty()));
    let mut elementos = Vec::new();
    for (tipo, de, ate, entre) in quebras {
        let (mut e, mut motivos) = medidas(de, ate, &lev, &obs, g, p0, c);
        e.tipo = tipo;
        match tipo {
            Tipo::Curva(i) => motivos.extend(motivos_da_curva(i, a, &tabela[i], regras, c)),
            Tipo::Tangente(_) => motivos.extend(motivos_da_tangente(de, ate, entre && ate > de, c)),
        }
        e.zonas = zonas.iter().filter(|z| z.ate > de && z.de < ate).map(|z| Zona { de: z.de.max(de), ate: z.ate.min(ate), ..z.clone() }).collect();
        for z in e.zonas.iter().filter(|z| z.nivel > Nivel::Normal) {
            motivos.push(Motivo { nivel: z.nivel, texto: format!("{} de até {} m entre as progressivas {:.0} e {:.0} (máximo em {:.0})", z.natureza.nome(), pct(z.maximo), z.de, z.ate, z.onde) });
        }
        motivos.sort_by_key(|m| std::cmp::Reverse(m.nivel));
        e.nivel = motivos.first().map_or(Nivel::Normal, |m| m.nivel);
        e.motivos = motivos;
        elementos.push(e);
    }
    Analise { elementos, zonas, volumes: volumes(&lev, c), criterios: c.clone() }
}

#[cfg(test)]
mod testes {
    use super::super::alinhamento::EntradaCurva;
    use super::super::greide::{ajustar, Opcoes};
    use super::super::superelevacao::calcular;
    use super::*;
    use crate::geom::Ponto;

    fn via() -> Alinhamento {
        let d = 70f64.to_radians();
        Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(700.0, 0.0), Ponto::novo(700.0 + 700.0 * d.cos(), 700.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap()
    }

    /// Existente plano com uma "barriga" de 0,3 m entre 200 e 300 m (o greide de mínimos quadrados corta e aterra).
    fn observacoes(a: &Alinhamento) -> Vec<Observacao> {
        (0..=(a.comprimento / 5.0) as usize)
            .map(|i| {
                let s = i as f64 * 5.0;
                Observacao { s, z: 50.0 - if (200.0..300.0).contains(&s) { 0.3 } else { 0.0 } + 0.0 * s, ripagem: if (900.0..920.0).contains(&s) { 1.2 } else { 0.05 } }
            })
            .collect()
    }

    #[test]
    fn classifica_curvas_e_tangentes_e_acha_aterro() {
        let a = via();
        let regras = Regras::default();
        let tabela = calcular(&a, &regras);
        let obs = observacoes(&a);
        // greide liso (plano a 50 m): onde o existente afunda 0,3 m há aterro de 0,3 m
        let plano: Vec<(f64, f64)> = obs.iter().map(|o| (o.s, 50.0)).collect();
        let g = ajustar(&plano, &Opcoes { regra: super::super::greide::Regra::SoLevante, ..Opcoes::default() }).unwrap();
        let an = analisar(&a, &tabela, &regras, &g, &obs, &Criterios::default());
        assert_eq!(an.elementos.len(), 3, "T0, C0, T1");
        assert!(matches!(an.elementos[1].tipo, Tipo::Curva(0)));
        // a ripagem de 1,2 m em 900–920 cai na curva ou na tangente seguinte e é crítica
        assert!(an.elementos.iter().any(|e| e.nivel == Nivel::Critico && e.motivos.iter().any(|m| m.texto.contains("ripagem"))), "{:?}", an.elementos.iter().map(|e| (e.nivel, e.motivos.len())).collect::<Vec<_>>());
        assert!(an.zonas.iter().any(|z| z.natureza == Natureza::Aterro && z.maximo >= 0.1), "{:?}", an.zonas);
        assert!(an.volumes.aterro_m3 > 0.0);
        assert!((an.contagem(Nivel::Normal) + an.contagem(Nivel::Atencao) + an.contagem(Nivel::Critico)) == 3);
    }

    #[test]
    fn raio_abaixo_do_minimo_e_critico() {
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(500.0, 0.0), Ponto::novo(500.0 + 500.0 * d.cos(), 500.0 * d.sin())], &[Some(EntradaCurva { raio: 150.0, ls: 40.0 })], 0.0).unwrap();
        let regras = Regras::default();
        let tabela = calcular(&a, &regras);
        let obs: Vec<Observacao> = (0..=(a.comprimento / 5.0) as usize).map(|i| Observacao { s: i as f64 * 5.0, z: 10.0, ripagem: 0.0 }).collect();
        let perfil: Vec<(f64, f64)> = obs.iter().map(|o| (o.s, o.z)).collect();
        let g = ajustar(&perfil, &Opcoes::default()).unwrap();
        let an = analisar(&a, &tabela, &regras, &g, &obs, &Criterios::default());
        let c = an.elementos.iter().find(|e| matches!(e.tipo, Tipo::Curva(_))).unwrap();
        assert_eq!(c.nivel, Nivel::Critico);
        assert!(c.motivos.iter().any(|m| m.texto.contains("raio")), "{:?}", c.motivos);
    }

    #[test]
    fn cota_existente_interpola_e_respeita_lacuna() {
        let obs = [Observacao { s: 0.0, z: 10.0, ripagem: 0.0 }, Observacao { s: 20.0, z: 12.0, ripagem: 0.0 }, Observacao { s: 100.0, z: 20.0, ripagem: 0.0 }];
        assert!((cota_existente(&obs, 10.0).unwrap() - 11.0).abs() < 1e-9);
        assert!(cota_existente(&obs, 60.0).is_none(), "lacuna de 80 m não se interpola");
        assert!((cota_existente(&obs, 22.0).unwrap() - 12.0).abs() < 1e-9, "perto da borda da lacuna vale a cota vizinha");
    }
}


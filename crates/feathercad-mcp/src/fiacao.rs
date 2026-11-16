//! `cad_fiacao`: cabos de quadro como na montagem real — quinas com curva (a "voltinha" do cabo dobrado),
//! ponteira na ponta que entra no borne, uma camada por condutor — e a régua de bornes de saída.

use feathercad_core::camadas::Propriedade;
use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria};
use feathercad_core::geom::{Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{numero_opcional, Res};
use crate::mutacoes::camada_por_nome;
use crate::pecas::{aplicar, caixa_das, fator_papel, relatorio, Peca, Pincel};

/// Condutor → (camada, cor). Neutro azul-claro e terra verde (NBR 5410); fases preto, vermelho e marrom.
fn condutor(nome: &str) -> Res<(&'static str, u32)> {
    Ok(match nome.to_ascii_uppercase().as_str() {
        "R" | "L1" | "F" | "FASE" => ("DI-Fio-Fase-R", 0x000000),
        "S" | "L2" => ("DI-Fio-Fase-S", 0xD32F2F),
        "T" | "L3" => ("DI-Fio-Fase-T", 0x8D6E63),
        "N" | "NEUTRO" => ("DI-Fio-Neutro", 0x1E88E5),
        "PE" | "TERRA" => ("DI-Fio-Terra", 0x2E7D32),
        "RETORNO" => ("DI-Fio-Retorno", 0xF9A825),
        o => return Err(format!("condutor `{o}`: use R, S, T, N, PE ou retorno")),
    })
}

/// Camada do condutor: criada com a cor e a espessura do cabo; se já existe, fica como o usuário deixou.
fn garantir_camada(doc: &mut Documento, nome: &str, cor: u32) -> Res<()> {
    if doc.camadas().iter().any(|c| c.nome.eq_ignore_ascii_case(nome)) {
        return Ok(());
    }
    let i = camada_por_nome(doc, nome)?;
    for p in [Propriedade::Cor(cor), Propriedade::Espessura(Some(0.4))] {
        doc.definir_propriedade_de_camadas(&[i], p).map_err(|e| format!("camada `{nome}`: {}", e.chave()))?;
    }
    Ok(())
}

fn arred(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn sub(a: Ponto, b: Ponto) -> (f64, f64) {
    (a.x - b.x, a.y - b.y)
}

/// Polilinha com cada quina trocada por um arco de raio `raio` (menor se o trecho for curto:
/// a curva usa no máximo metade de cada trecho vizinho, então duas curvas seguidas não se cruzam).
pub fn arredondar(pts: &[Ponto], raio: f64) -> Vec<Vertice> {
    let mut v = vec![Vertice { p: pts[0], bulge: 0.0 }];
    for i in 1..pts.len().saturating_sub(1) {
        let (a, b, c) = (pts[i - 1], pts[i], pts[i + 1]);
        let (u, w) = (sub(b, a), sub(c, b));
        let (lu, lw) = (u.0.hypot(u.1), w.0.hypot(w.1));
        let cruz = u.0 * w.1 - u.1 * w.0;
        let giro = cruz.atan2(u.0 * w.0 + u.1 * w.1); // ângulo da curva, com sinal (positivo = anti-horário)
        if lu < 1e-9 || lw < 1e-9 || giro.abs() < 1e-3 || giro.abs() > std::f64::consts::PI - 1e-3 {
            v.push(Vertice { p: b, bulge: 0.0 });
            continue;
        }
        let meio = (giro.abs() / 2.0).tan();
        let t = (raio * meio).min(lu / 2.0).min(lw / 2.0);
        let (ua, wa) = ((u.0 / lu, u.1 / lu), (w.0 / lw, w.1 / lw));
        v.push(Vertice { p: Ponto::novo(b.x - ua.0 * t, b.y - ua.1 * t), bulge: (giro / 4.0).tan() });
        v.push(Vertice { p: Ponto::novo(b.x + wa.0 * t, b.y + wa.1 * t), bulge: 0.0 });
    }
    v.push(Vertice { p: pts[pts.len() - 1], bulge: 0.0 });
    v
}

fn pontos(f: &Value) -> Res<Vec<Ponto>> {
    let l = f.get("pontos").and_then(Value::as_array).ok_or("`pontos` precisa ser uma lista de [x, y]")?;
    let p: Vec<Ponto> = l
        .iter()
        .map(|q| match q.as_array().map(|a| (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64))) {
            Some((Some(x), Some(y))) => Ok(Ponto::novo(x, y)),
            _ => Err("cada ponto é [x, y]".to_string()),
        })
        .collect::<Res<_>>()?;
    let mut sem_repetir: Vec<Ponto> = Vec::with_capacity(p.len());
    for q in p {
        if sem_repetir.last().map_or(true, |u| (u.x - q.x).hypot(u.y - q.y) > 1e-9) {
            sem_repetir.push(q);
        }
    }
    if sem_repetir.len() < 2 {
        return Err("um fio precisa de ao menos 2 pontos diferentes".into());
    }
    Ok(sem_repetir)
}

/// Ponteira tubular: retângulo de `comp` × `larg` terminando em `ponta`, na direção que o cabo chega.
/// Devolve o ponto onde a isolação do cabo termina.
fn ponteira(p: &mut Pincel, ponta: Ponto, antes: Ponto, comp: f64, larg: f64) -> Ponto {
    let (dx, dy) = sub(ponta, antes);
    let l = dx.hypot(dy);
    let comp = comp.min(l * 0.8);
    let (ux, uy) = (dx / l, dy / l);
    let (nx, ny) = (-uy * larg / 2.0, ux * larg / 2.0);
    let base = Ponto::novo(ponta.x - ux * comp, ponta.y - uy * comp);
    let v = [Ponto::novo(base.x + nx, base.y + ny), Ponto::novo(ponta.x + nx, ponta.y + ny), Ponto::novo(ponta.x - nx, ponta.y - ny), Ponto::novo(base.x - nx, base.y - ny)];
    p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Polilinha { vertices: v.map(|p| Vertice { p, bulge: 0.0 }).to_vec(), fechada: true } });
    base
}

fn bornes(p: &mut Pincel, b: &Value, k: f64, saida: &mut Vec<Value>) -> Res<()> {
    let o = b.get("origem").and_then(Value::as_array).and_then(|a| Some(Ponto::novo(a.first()?.as_f64()?, a.get(1)?.as_f64()?))).ok_or("borne: `origem` [x, y] (canto de baixo à esquerda)")?;
    let tipos: Vec<&str> = b.get("tipos").and_then(Value::as_array).ok_or("borne: `tipos` [\"R\",\"N\",\"PE\"…]")?.iter().map(|t| t.as_str().unwrap_or("R")).collect();
    let rotulos: Vec<&str> = b.get("rotulos").and_then(Value::as_array).map(|l| l.iter().map(|t| t.as_str().unwrap_or("")).collect()).unwrap_or_default();
    let passo = numero_opcional(b, "passo").unwrap_or(6.2) * k;
    let h = numero_opcional(b, "altura").unwrap_or(42.0) * k;
    let r = 2.0 * k;
    let mut lista = Vec::new();
    for (i, t) in tipos.iter().enumerate() {
        let (_, cor) = condutor(t)?;
        let x = o.x + i as f64 * passo;
        let cx = x + passo / 2.0;
        p.em("DI-Bornes", None);
        p.retangulo(Ponto::novo(x, o.y), Ponto::novo(x + passo, o.y + h));
        // faixa da cor do condutor no corpo (cinza = fase, azul = neutro, verde = terra)
        p.em("DI-Bornes", Some(if t.eq_ignore_ascii_case("N") || t.eq_ignore_ascii_case("PE") { cor } else { 0x78909C }));
        p.retangulo(Ponto::novo(x + 0.8 * k, o.y + h * 0.35), Ponto::novo(x + passo - 0.8 * k, o.y + h * 0.65));
        p.em("DI-Bornes", None);
        let (topo, base) = (Ponto::novo(cx, o.y + h - 5.0 * k), Ponto::novo(cx, o.y + 5.0 * k));
        for c in [topo, base] {
            p.pecas.push(Peca { camada: p.camada.clone(), cor: None, g: Geometria::Circulo { centro: c, raio: r } });
            p.linha(Ponto::novo(c.x - r * 0.6, c.y), Ponto::novo(c.x + r * 0.6, c.y));
        }
        let rot = rotulos.get(i).copied().unwrap_or("");
        if !rot.is_empty() {
            p.texto(Ponto::novo(cx, o.y + h / 2.0), 2.2 * k, 90.0, rot, AlinhaH::Centro, AlinhaV::Meio);
        }
        lista.push(json!({ "tipo": t, "rotulo": rot, "parafuso_topo": [arred(topo.x), arred(topo.y)], "parafuso_base": [arred(base.x), arred(base.y)] }));
    }
    saida.push(json!({ "origem": [o.x, o.y], "largura": passo * tipos.len() as f64, "altura": h, "bornes": lista }));
    Ok(())
}

pub fn fiacao(doc: &mut Documento, args: &Value) -> Res<Value> {
    let k = fator_papel(doc, args)?;
    let raio_padrao = numero_opcional(args, "raio").unwrap_or(4.0) * k;
    let mut p = Pincel::novo("DI-Bornes");
    let mut reguas = Vec::new();
    for b in args.get("bornes").and_then(Value::as_array).into_iter().flatten() {
        bornes(&mut p, b, k, &mut reguas)?;
    }
    let fios = args.get("fios").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    for (i, f) in fios.iter().enumerate() {
        let erro = |e: String| format!("fio {i}: {e}");
        let (camada, cor) = condutor(f.get("condutor").and_then(Value::as_str).unwrap_or("R")).map_err(erro)?;
        garantir_camada(doc, camada, cor)?;
        let mut pts = pontos(f).map_err(erro)?;
        p.em(camada, None); // cor da camada: esconder/imprimir por condutor
        let n = pts.len();
        if f.get("ponteira_fim").and_then(Value::as_bool).unwrap_or(true) {
            pts[n - 1] = ponteira(&mut p, pts[n - 1], pts[n - 2], 4.0 * k, 1.6 * k);
        }
        if f.get("ponteira_inicio").and_then(Value::as_bool).unwrap_or(true) {
            pts[0] = ponteira(&mut p, pts[0], pts[1], 4.0 * k, 1.6 * k);
        }
        let raio = numero_opcional(f, "raio").map_or(raio_padrao, |r| r * k);
        p.pecas.push(Peca { camada: camada.into(), cor: None, g: Geometria::Polilinha { vertices: arredondar(&pts, raio), fechada: false } });
    }
    if p.pecas.is_empty() {
        return Err("nada a desenhar: passe `fios` e/ou `bornes`".into());
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["reguas"] = json!(reguas);
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn quina_vira_arco_tangente_e_trecho_curto_encolhe_o_raio() {
        let p = |x, y| Ponto::novo(x, y);
        let v = arredondar(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)], 3.0);
        assert_eq!(v.len(), 4);
        let perto = |a: Ponto, b: Ponto| assert!((a.x - b.x).hypot(a.y - b.y) < 1e-9, "{a:?} ≠ {b:?}");
        perto(v[1].p, p(7.0, 0.0));
        assert!((v[1].bulge - (std::f64::consts::FRAC_PI_2 / 4.0).tan()).abs() < 1e-12, "90° anti-horário");
        perto(v[2].p, p(10.0, 3.0));
        // trecho de 2 entre duas quinas: cada curva usa no máximo 1
        let v = arredondar(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, -2.0), p(20.0, -2.0)], 3.0);
        perto(v[2].p, p(10.0, -1.0));
        perto(v[3].p, p(10.0, -1.0));
        assert!(v[1].bulge < 0.0, "virar para baixo é horário");
    }

    #[test]
    fn fio_cria_a_camada_do_condutor_com_a_cor_e_os_bornes_dizem_onde_ligar() {
        let mut d = Documento::default();
        let r = fiacao(&mut d, &json!({ "fios": [{ "condutor": "N", "pontos": [[0, 0], [0, 20], [30, 20]] }], "bornes": [{ "origem": [50, 0], "tipos": ["R", "N", "PE"] }] })).unwrap();
        let c = d.camadas().iter().find(|c| c.nome == "DI-Fio-Neutro").unwrap();
        assert_eq!(c.cor, 0x1E88E5);
        let b = &r["reguas"][0]["bornes"];
        assert_eq!(b.as_array().unwrap().len(), 3);
        assert_eq!(b[1]["parafuso_topo"], json!([59.3, 37.0]));
        assert!(fiacao(&mut d, &json!({ "fios": [{ "condutor": "X", "pontos": [[0, 0], [1, 1]] }] })).is_err());
    }
}

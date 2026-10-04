//! `cad_via`: a superestrutura ferroviária sobre o alinhamento guardado. Planta: trilhos (bordas do boleto) e
//! dormentes ao longo do eixo. Seção: trilho, dormente inclinado pela superelevação, lastro, sublastro e cotas.
//! Perfis simplificados (ilustração de projeto, não desenho de fabricante).

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::geom::Ponto;
use feathercad_core::trajeto::Trajeto;
use feathercad_core::viario::alinhamento::Alinhamento;
use feathercad_core::viario::superelevacao::{calcular, h_em};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, cor_de, formatar, relatorio, caixa_das, Peca, Pincel};

struct Via {
    /// Distância eixo a eixo dos trilhos, m.
    b: f64,
    /// Meia largura do boleto, m.
    hb: f64,
    dorm_c: f64,
    dorm_l: f64,
    /// Contorno do trilho em m (origem no centro do topo; y para baixo é negativo), do componente `trilho_*`.
    perfil: Vec<(f64, f64)>,
    dorm_h: f64,
    espaco: f64,
}

/// A via: trilho (componente `trilho_*`, padrão TR-57) e dormente (componente `dormente_*`, padrão madeira); o que vier em
/// `dormente: {comprimento, largura, altura, espaco}` vale sobre o padrão do arquivo. `dormente: "concreto"` escolhe o arquivo.
fn via_de(args: &Value) -> Res<Via> {
    let bitola = numero_opcional(args, "bitola").unwrap_or(1.0);
    let perfil = crate::componentes::perfil_em_metros(texto_opcional(args, "trilho").unwrap_or("TR-57"), "trilho")?;
    let topo = perfil.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    let hb_arquivo = perfil.iter().filter(|p| (p.1 - topo).abs() < 1e-9).map(|p| p.0.abs()).fold(0.0, f64::max);
    let boleto = numero_opcional(args, "boleto_mm").map_or(2.0 * hb_arquivo, |b| b / 1000.0);
    let d = args.get("dormente").cloned().unwrap_or(json!({}));
    let nome_dorm = d.as_str().map(str::to_string).or_else(|| d.get("tipo").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| "madeira".into());
    let padrao = crate::componentes::parametros_numericos(&nome_dorm, "dormente")?;
    let dado = |k: &str, fallback: f64| numero_opcional(&d, k).or_else(|| padrao.iter().find(|(n, _)| n == k).map(|(_, v)| *v)).unwrap_or(fallback);
    Ok(Via {
        b: numero_opcional(args, "B").unwrap_or(bitola + 0.075), hb: boleto / 2.0, perfil,
        dorm_c: dado("comprimento", 2.0), dorm_l: dado("largura", 0.22), dorm_h: dado("altura", 0.20), espaco: dado("espaco", 0.60),
    })
}

/// Metros → unidades do desenho.
fn metros(doc: &Documento) -> f64 {
    1000.0 / doc.unidade().map_or(1000.0, feathercad_core::documento::Unidade::em_milimetros)
}

fn planta(ag: &mut Agente, a: &Alinhamento, args: &Value) -> Res<Value> {
    let v = via_de(args)?;
    let km = metros(&ag.doc);
    let (s0, s1) = (numero_opcional(args, "de").unwrap_or(0.0).max(0.0), numero_opcional(args, "ate").unwrap_or(a.comprimento).min(a.comprimento));
    if s1 <= s0 {
        return Err(format!("`ate` precisa ser maior que `de`; o alinhamento mede {:.1}", a.comprimento));
    }
    let amostra = Trajeto::novo(&a.amostrar(numero_opcional(args, "passo_amostra").unwrap_or(1.0))).ok_or("alinhamento vazio")?;
    let trecho = Trajeto::novo(&amostra.trecho(s0, s1)).ok_or("trecho vazio")?;
    let mut p = Pincel::novo(texto_opcional(args, "camada_dormentes").unwrap_or("DORMENTES"));
    p.cor = cor_de(args, "cor_dormentes")?.or(Some(0xA0_A0_A0));
    let mut n_dorm = 0usize;
    if args.get("dormentes").and_then(Value::as_bool).unwrap_or(true) {
        let mut s = (s0 / v.espaco / km).ceil() * v.espaco * km + v.espaco * km / 2.0;
        let (cc, ll) = (v.dorm_c * km / 2.0, v.dorm_l * km / 2.0);
        while s < s1 {
            let am = a.em(s + a.progressiva_inicial);
            let (sn, cs) = am.direcao.to_radians().sin_cos();
            let canto = |along: f64, across: f64| Ponto::novo(am.ponto.x + cs * along - sn * across, am.ponto.y + sn * along + cs * across);
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: feathercad_core::documento::Geometria::Polilinha { vertices: [canto(-ll, -cc), canto(ll, -cc), canto(ll, cc), canto(-ll, cc)].map(|q| feathercad_core::geom::Vertice { p: q, bulge: 0.0 }).to_vec(), fechada: true } });
            n_dorm += 1;
            s += v.espaco * km;
        }
    }
    p.em(texto_opcional(args, "camada_trilhos").unwrap_or("TRILHOS"), cor_de(args, "cor_trilhos")?.or(Some(0xFF_FF_FF)));
    for desloc in [v.b / 2.0 - v.hb, v.b / 2.0 + v.hb, -v.b / 2.0 + v.hb, -v.b / 2.0 - v.hb] {
        p.polilinha(&trecho.paralela(desloc * km));
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(&mut ag.doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["dormentes"] = json!(n_dorm);
    r["trilhos"] = json!(4);
    r["B"] = json!(v.b);
    Ok(r)
}

/// Seção tipo em `ponto` (centro da via, topo do trilho). Rotaciona trilhos e dormente pela superelevação.
fn secao(ag: &mut Agente, a: &Alinhamento, args: &Value) -> Res<Value> {
    let v = via_de(args)?;
    let km = metros(&ag.doc) * numero_opcional(args, "ampliacao").unwrap_or(1.0);
    let origem = ponto_de(args, "ponto")?;
    let regras = crate::regras::carregar(args)?;
    let h_mm = match (numero_opcional(args, "h_mm"), numero_opcional(args, "progressiva")) {
        (Some(h), _) => h,
        (None, Some(s)) => h_em(a, &calcular(a, &regras), s),
        _ => 0.0,
    };
    let lado: f64 = if texto_opcional(args, "externo") == Some("esquerdo") { -1.0 } else { 1.0 };
    let theta = (h_mm / 1000.0 / v.b).asin() * lado;
    let altura_trilho = -v.perfil.iter().map(|p| p.1).fold(f64::MAX, f64::min);
    let (lastro, ombro, talude, sublastro) = (numero_opcional(args, "lastro").unwrap_or(0.30), numero_opcional(args, "ombro").unwrap_or(0.40), numero_opcional(args, "talude").unwrap_or(1.5), numero_opcional(args, "sublastro").unwrap_or(0.20));
    // pivô: o trilho de dentro (o de menor cota), ao nível do topo do trilho
    let pivo = (-lado * v.b / 2.0, 0.0);
    let gira = |x: f64, y: f64| {
        let (dx, dy) = (x - pivo.0, y - pivo.1);
        let (s, c) = theta.sin_cos();
        Ponto::novo(origem.x + (pivo.0 + dx * c - dy * s) * km, origem.y + (pivo.1 + dx * s + dy * c) * km)
    };
    let fixo = |x: f64, y: f64| Ponto::novo(origem.x + x * km, origem.y + y * km);
    let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("SECAO_VIA"));
    let ht = 0.05 * km;
    for lado_trilho in [-1.0, 1.0] {
        let cx = lado_trilho * v.b / 2.0;
        let perfil = &v.perfil;
        let pts: Vec<Ponto> = perfil.iter().map(|(x, y)| gira(cx + x, *y)).collect();
        p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: feathercad_core::documento::Geometria::Polilinha { vertices: pts.iter().map(|q| feathercad_core::geom::Vertice { p: *q, bulge: 0.0 }).collect(), fechada: true } });
    }
    let (dx0, dy0) = (v.dorm_c / 2.0, -altura_trilho);
    let dorm: Vec<Ponto> = [(-dx0, dy0), (dx0, dy0), (dx0, dy0 - v.dorm_h), (-dx0, dy0 - v.dorm_h)].iter().map(|(x, y)| gira(*x, *y)).collect();
    p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: feathercad_core::documento::Geometria::Polilinha { vertices: dorm.iter().map(|q| feathercad_core::geom::Vertice { p: *q, bulge: 0.0 }).collect(), fechada: true } });
    // lastro: topo acompanha o dormente (ombro de cada lado), base horizontal `lastro` abaixo do dormente do lado baixo
    let (xe, xd) = (-dx0 - ombro, dx0 + ombro);
    let topo_y = |x: f64| { let q = gira(x, dy0); (q.y - origem.y) / km };
    let base_y = topo_y(pivo.0.signum() * dx0) - v.dorm_h - lastro;
    let (ye, yd) = (topo_y(xe).max(base_y), topo_y(xd).max(base_y));
    let (xbe, xbd) = (xe - (ye - base_y) * talude, xd + (yd - base_y) * talude);
    let lastro_pts = [fixo(xe, ye), fixo(xd, yd), fixo(xbd, base_y), fixo(xbe, base_y)];
    p.pecas.push(Peca { camada: p.camada.clone(), cor: Some(0x80_80_80), g: feathercad_core::documento::Geometria::Polilinha { vertices: lastro_pts.iter().map(|q| feathercad_core::geom::Vertice { p: *q, bulge: 0.0 }).collect(), fechada: true } });
    let sub = [fixo(xbe, base_y), fixo(xbd, base_y), fixo(xbd + sublastro * talude, base_y - sublastro), fixo(xbe - sublastro * talude, base_y - sublastro)];
    p.pecas.push(Peca { camada: p.camada.clone(), cor: Some(0x60_60_60), g: feathercad_core::documento::Geometria::Polilinha { vertices: sub.iter().map(|q| feathercad_core::geom::Vertice { p: *q, bulge: 0.0 }).collect(), fechada: true } });
    p.cor = Some(0xC0_C0_C0);
    let dec = |x: f64| formatar(x, 3, true);
    for (txt, pos) in [(format!("bitola {} m (eixo a eixo {})", dec(v.b - 2.0 * v.hb), dec(v.b)), fixo(0.0, 0.25)), (format!("h = {} mm", formatar(h_mm, 0, false)), fixo(0.0, 0.40)), (format!("lastro e = {} m", dec(lastro)), fixo(0.0, base_y - 0.5 * lastro)), (format!("sublastro e = {} m", dec(sublastro)), fixo(0.0, base_y - 1.4 * sublastro))] {
        p.texto(pos, ht, 0.0, &txt, AlinhaH::Centro, AlinhaV::Meio);
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(&mut ag.doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["superelevacao_mm"] = json!(h_mm);
    r["inclinacao_graus"] = json!(theta.to_degrees());
    r["nota"] = json!("perfil de trilho simplificado (ilustração); dimensões reais em metros, `ampliacao` aumenta o desenho");
    Ok(r)
}

pub fn via(ag: &mut Agente, args: &Value) -> Res<Value> {
    let (_, (a, _)) = crate::viario::achar(ag, args)?;
    let a = a.clone();
    match texto_opcional(args, "desenho").unwrap_or("planta") {
        "planta" => planta(ag, &a, args),
        "secao" => secao(ag, &a, args),
        o => Err(format!("`desenho`: planta ou secao, não `{o}`")),
    }
}

#[cfg(test)]
mod testes {
    use feathercad_core::documento::Unidade;
    use feathercad_core::viario::alinhamento::EntradaCurva;

    use super::*;

    fn agente() -> Agente {
        let mut ag = Agente { doc: feathercad_core::documento::Documento::default().com_unidade(Some(Unidade::Metro)), ..Agente::default() };
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(700.0, 0.0), Ponto::novo(700.0 + 700.0 * d.cos(), 700.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        ag.alinhamentos.insert("A1".into(), (a, 20.0));
        ag.alinhamento_atual = Some("A1".into());
        ag
    }

    #[test]
    fn planta_tem_quatro_trilhos_e_dormentes_a_cada_60_cm() {
        let mut ag = agente();
        let r = via(&mut ag, &json!({ "de": 0, "ate": 60 })).unwrap();
        assert_eq!(r["trilhos"], 4);
        assert_eq!(r["dormentes"], 100);
        assert_eq!(r["criadas"], 104);
    }

    #[test]
    fn secao_inclina_com_a_superelevacao_da_curva() {
        let mut ag = agente();
        let r = via(&mut ag, &json!({ "desenho": "secao", "ponto": [0, 0], "h_mm": 100, "regras": { "V": 80 } })).unwrap();
        assert!((r["inclinacao_graus"].as_f64().unwrap() - (0.1f64 / 1.075).asin().to_degrees()).abs() < 1e-9);
        assert!(r["criadas"].as_u64().unwrap() >= 8);
        let plana = via(&mut ag, &json!({ "desenho": "secao", "ponto": [50, 0] })).unwrap();
        assert_eq!(plana["superelevacao_mm"], 0.0);
        assert!(via(&mut ag, &json!({ "desenho": "xx" })).is_err());
    }

    #[test]
    fn trilho_e_dormente_vem_dos_arquivos_da_biblioteca() {
        let mut ag = agente();
        // concreto tem largura 0,26 e espaçamento 0,60 no arquivo; `espaco` do pedido vale sobre o do arquivo
        let r = via(&mut ag, &json!({ "de": 0, "ate": 60, "dormente": { "tipo": "concreto", "espaco": 1.0 } })).unwrap();
        assert_eq!(r["dormentes"], 60);
        assert!(via(&mut ag, &json!({ "trilho": "TR-99", "de": 0, "ate": 10 })).is_err());
        assert!(via(&mut ag, &json!({ "dormente": "pedra", "de": 0, "ate": 10 })).is_err());
    }
}

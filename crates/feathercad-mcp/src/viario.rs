//! Ferramentas viárias: `cad_alinhamento` (traçado com clotoides), `cad_alinhamento_ponto` (consulta por estaca) e
//! `cad_superelevacao`. A conta é do `feathercad_core::viario`; aqui só se lê JSON, guarda o alinhamento (na sessão
//! do servidor, por nome) e desenha.

use feathercad_core::documento::{AlinhaH, AlinhaV, Geometria};
use feathercad_core::geom::Ponto;
use feathercad_core::trajeto::{estaca_texto, km_texto};
use feathercad_core::viario::alinhamento::{Alinhamento, Curva, EntradaCurva, Marco};
use feathercad_core::viario::superelevacao::{calcular, memorial};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{arred, lista_de_pontos, numero_opcional, texto_opcional, Res};
use crate::pecas::{aplicar, cor_de, fator_papel, Peca, Pincel};

fn marco_json(m: &Marco, passo: f64) -> Value {
    json!({ "estaca": estaca_texto(m.progressiva, passo), "km": km_texto(m.progressiva), "progressiva": arred(m.progressiva), "E": arred(m.ponto.x), "N": arred(m.ponto.y) })
}

pub fn curva_json(c: &Curva, passo: f64) -> Value {
    json!({
        "curva": c.pi, "pi": [arred(c.pi_ponto_x()), arred(c.pi_ponto_y())], "delta": arred(c.delta_graus), "lado": if c.lado > 0 { "esquerda" } else { "direita" },
        "R": c.raio, "Ls": c.ls, "Lc": arred(c.lc), "Ts": arred(c.ts), "E": arred(c.e), "theta_s": arred(c.theta_s_graus), "A": arred(c.a), "p": arred(c.p), "k": arred(c.k), "xs": arred(c.xs), "ys": arred(c.ys),
        "TS": marco_json(&c.ts_marco, passo), "SC": marco_json(&c.sc, passo), "CS": marco_json(&c.cs, passo), "ST": marco_json(&c.st, passo), "PI_estaca": estaca_texto(c.pi_progressiva, passo),
    })
}

trait PontoDoPi {
    fn pi_ponto_x(&self) -> f64;
    fn pi_ponto_y(&self) -> f64;
}
impl PontoDoPi for Curva {
    fn pi_ponto_x(&self) -> f64 {
        self.pi_ponto.x
    }
    fn pi_ponto_y(&self) -> f64 {
        self.pi_ponto.y
    }
}

fn entradas(args: &Value, n_pis: usize) -> Res<Vec<Option<EntradaCurva>>> {
    let lista = args.get("curvas").and_then(Value::as_array).ok_or("falta `curvas`: uma por PI interno, [{R, Ls}, null, …]")?;
    if lista.len() != n_pis.saturating_sub(2) {
        return Err(format!("{n_pis} pontos têm {} PIs internos: mande {} entradas em `curvas` (`null` onde o PI não tem curva)", n_pis.saturating_sub(2), n_pis.saturating_sub(2)));
    }
    lista
        .iter()
        .enumerate()
        .map(|(i, c)| match c {
            Value::Null => Ok(None),
            o => {
                let raio = o.get("R").or_else(|| o.get("raio")).and_then(Value::as_f64).ok_or_else(|| format!("curva {i}: falta `R`"))?;
                Ok(Some(EntradaCurva { raio, ls: o.get("Ls").or_else(|| o.get("ls")).and_then(Value::as_f64).unwrap_or(0.0) }))
            }
        })
        .collect()
}

fn flag(d: &Value, nome: &str) -> bool {
    match d {
        Value::Bool(b) => *b && nome == "eixo",
        o => o.get(nome).and_then(Value::as_bool).unwrap_or(false),
    }
}

/// Texto que acompanha a direção `g` sem ficar de cabeça para baixo: (giro, alinhamento horizontal).
pub(crate) fn legivel(g: f64) -> (f64, AlinhaH) {
    let g = g.rem_euclid(360.0);
    if g > 90.0 + 1e-6 && g <= 270.0 + 1e-6 { (g - 180.0, AlinhaH::Direita) } else { (g, AlinhaH::Esquerda) }
}

pub fn desenhar(ag: &mut Agente, a: &Alinhamento, args: &Value, passo_estaca: f64) -> Res<Value> {
    let d = args.get("desenhar").cloned().unwrap_or(Value::Bool(false));
    let base = texto_opcional(args, "camada_base").unwrap_or("ALINHAMENTO");
    let k = fator_papel(&ag.doc, args)?;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(2.5) * k;
    let cor = cor_de(args, "cor")?;
    let mut ids = 0usize;
    let pontos = a.amostrar(numero_opcional(args, "passo_amostra").unwrap_or(1.0));
    if flag(&d, "eixo") {
        let mut p = Pincel::novo(&format!("{base}_EIXO"));
        p.cor = cor;
        p.polilinha(&pontos);
        ids += aplicar(&mut ag.doc, p.pecas)?.len();
    }
    for (chave, passo, rotulo, sufixo) in [("estacas", passo_estaca, "estaca", "ESTACAS"), ("km", 1000.0, "km", "KM")] {
        if flag(&d, chave) {
            let mut o = json!({ "pontos": pontos.iter().map(|p| json!([p.x, p.y])).collect::<Vec<_>>(), "modo": "marcas", "passo": passo, "rotulo": rotulo, "progressiva_inicial": a.progressiva_inicial,
                "passo_estaca": passo_estaca, "camada": format!("{base}_{sufixo}"), "altura_texto": numero_opcional(args, "altura_texto").unwrap_or(2.5) });
            if chave == "estacas" {
                o["rotular_a_cada"] = json!(args.get("rotular_estacas_a_cada").and_then(Value::as_u64).unwrap_or(5));
            }
            if chave == "km" {
                o["tamanho"] = json!(numero_opcional(args, "tamanho_marca").unwrap_or(3.0) * 2.0);
                // o rótulo do km vai do lado oposto ao das estacas (senão os dois se sobrepõem)
                o["lado"] = json!("direita");
            }
            if let Some(e) = args.get("escala") {
                o["escala"] = e.clone();
            }
            let r = crate::ao_longo::ao_longo(&mut ag.doc, &o)?;
            ids += r["criadas"].as_u64().unwrap_or(0) as usize;
        }
    }
    if flag(&d, "notaveis") {
        let mut p = Pincel::novo(&format!("{base}_NOTAVEIS"));
        p.cor = cor;
        for (n, c) in a.curvas.iter().enumerate() {
            for (nome, m) in [("TS", &c.ts_marco), ("SC", &c.sc), ("CS", &c.cs), ("ST", &c.st)] {
                let am = a.em(m.progressiva);
                p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro: m.ponto, raio: 0.5 * ht } });
                // sempre à direita do eixo: as estacas ficam à esquerda e os rótulos não se sobrepõem
                let (giro, h) = legivel(am.direcao - 90.0);
                let normal = (-(am.direcao.to_radians().sin()), am.direcao.to_radians().cos());
                let afast = -1.2 * ht;
                let rot = if flag(&d, "rotulos") { format!("{nome}{} {}", n + 1, estaca_texto(m.progressiva, passo_estaca)) } else { nome.to_string() };
                p.texto(Ponto::novo(m.ponto.x + normal.0 * afast, m.ponto.y + normal.1 * afast), ht, giro, &rot, h, AlinhaV::Meio);
            }
        }
        ids += aplicar(&mut ag.doc, p.pecas)?.len();
    }
    Ok(json!(ids))
}

/// `cad_alinhamento`: calcula, guarda sob `nome` (padrão `A1`) e, se pedido, desenha.
pub fn alinhamento(ag: &mut Agente, args: &Value) -> Res<Value> {
    let pis = lista_de_pontos(args.get("pis").ok_or("falta `pis`: [[E,N], …] do ponto inicial ao final")?, "pis")?;
    let curvas = entradas(args, pis.len())?;
    let passo_estaca = numero_opcional(args, "passo_estaca").unwrap_or(20.0);
    let inicial = numero_opcional(args, "progressiva_inicial").unwrap_or(0.0);
    let a = Alinhamento::novo(&pis, &curvas, inicial)?;
    let nome = texto_opcional(args, "nome").unwrap_or("A1").to_string();
    let desenhadas = desenhar(ag, &a, args, passo_estaca)?;
    let r = json!({
        "alinhamento": nome, "comprimento": arred(a.comprimento), "progressiva_final": arred(a.progressiva_final()), "estaca_final": estaca_texto(a.progressiva_final(), passo_estaca),
        "km_final": km_texto(a.progressiva_final()), "curvas": a.curvas.iter().map(|c| curva_json(c, passo_estaca)).collect::<Vec<_>>(), "formas_desenhadas": desenhadas,
        "nota": "guardado na sessão do servidor com este nome: cad_alinhamento_ponto e cad_superelevacao o consultam; refaça com cad_alinhamento se reiniciar",
    });
    ag.alinhamentos.insert(nome.clone(), (a, passo_estaca));
    ag.alinhamento_atual = Some(nome);
    Ok(r)
}

pub fn achar<'a>(ag: &'a Agente, args: &Value) -> Res<(String, &'a (Alinhamento, f64))> {
    let nome = texto_opcional(args, "alinhamento").map(str::to_string).or_else(|| ag.alinhamento_atual.clone()).ok_or("nenhum alinhamento: rode cad_alinhamento primeiro")?;
    ag.alinhamentos.get(&nome).map(|x| (nome.clone(), x)).ok_or_else(|| format!("o alinhamento `{nome}` não existe nesta sessão (existem: {})", ag.alinhamentos.keys().cloned().collect::<Vec<_>>().join(", ")))
}

/// Lê `117+12,66` (ou `117+12.66`) como progressiva.
fn ler_estaca(t: &str, passo: f64) -> Option<f64> {
    let (n, r) = t.split_once('+')?;
    Some(n.trim().parse::<f64>().ok()? * passo + r.trim().replace(',', ".").parse::<f64>().ok()?)
}

pub fn alinhamento_ponto(ag: &Agente, args: &Value) -> Res<Value> {
    let (nome, (a, passo)) = achar(ag, args)?;
    let s = match (numero_opcional(args, "progressiva"), texto_opcional(args, "estaca")) {
        (Some(s), _) => s,
        (None, Some(t)) => ler_estaca(t, *passo).ok_or_else(|| format!("estaca `{t}` inválida (use `117+12,66`)"))?,
        _ => return Err("diga a `progressiva` (m) ou a `estaca` (`117+12,66`)".into()),
    };
    let am = a.em(s);
    Ok(json!({
        "alinhamento": nome, "progressiva": arred(s), "estaca": estaca_texto(s, *passo), "km": km_texto(s), "E": arred(am.ponto.x), "N": arred(am.ponto.y),
        "azimute": arred(am.azimute), "direcao": arred(am.direcao), "curvatura": arred(am.curvatura * 1e6) / 1e6,
        "raio": if am.curvatura.abs() > 1e-12 { json!(arred(1.0 / am.curvatura.abs())) } else { Value::Null }, "elemento": am.tipo.letra(), "curva": am.curva.map(|i| i + 1),
        "fora_do_alinhamento": s < a.progressiva_inicial - 1e-9 || s > a.progressiva_final() + 1e-9,
    }))
}

pub fn superelevacao(ag: &mut Agente, args: &Value) -> Res<Value> {
    let regras = crate::regras::carregar(args)?;
    let (nome, (a, passo)) = achar(ag, args)?;
    let (mut a, passo) = (a.clone(), *passo);
    let mut ajustadas: Vec<Value> = Vec::new();
    if args.get("Ls_auto").or_else(|| args.get("ls_auto")).and_then(Value::as_bool).unwrap_or(false) {
        let tabela = calcular(&a, &regras);
        let mut novas = a.entradas.clone();
        for (c, r) in a.curvas.iter().zip(&tabela) {
            if let Some(Some(e)) = novas.get_mut(c.pi - 1) {
                if r.ls_sugerido > e.ls + 1e-9 {
                    ajustadas.push(json!({ "curva": c.pi, "Ls_antes": e.ls, "Ls_depois": r.ls_sugerido }));
                    e.ls = r.ls_sugerido;
                }
            }
        }
        if !ajustadas.is_empty() {
            a = Alinhamento::novo(&a.pis, &novas, a.progressiva_inicial)?;
            ag.alinhamentos.insert(nome.clone(), (a.clone(), passo));
        }
    }
    let tabela = calcular(&a, &regras);
    let curvas: Vec<Value> = a.curvas.iter().zip(&tabela).map(|(c, r)| {
        let mut v = serde_json::to_value(r).unwrap_or_default();
        v["lado"] = json!(if c.lado > 0 { "esquerda" } else { "direita" });
        v["TS"] = json!(estaca_texto(c.ts_marco.progressiva, passo));
        v["ST"] = json!(estaca_texto(c.st.progressiva, passo));
        v["situacao"] = json!(if r.ok { "✔" } else { "✘" });
        v
    }).collect();
    let memorial_texto: Vec<String> = a.curvas.iter().zip(&tabela).map(|(c, r)| memorial(c, r, &regras)).collect();
    Ok(json!({
        "alinhamento": nome, "regras": regras.nome, "V": regras.v, "todas_atendem": tabela.iter().all(|r| r.ok),
        "curvas": curvas, "memorial": memorial_texto.join("\n\n"), "Ls_ajustados": ajustadas,
        "nota": "os limites são os usuais de bitola métrica de fábrica, não conferidos no texto da NBR 16961:2021 — ajuste em `regras`",
    }))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn c6() -> Value {
        let d = 93.92f64.to_radians();
        json!({ "pis": [[0, 0], [2000, 0], [2000.0 + 2000.0 * d.cos(), 2000.0 * d.sin()]], "curvas": [{ "R": 350, "Ls": 70 }], "desenhar": { "eixo": true, "estacas": true, "notaveis": true, "rotulos": true }, "escala": 1000 })
    }

    #[test]
    fn alinhamento_superelevacao_e_consulta_por_estaca() {
        let mut ag = Agente { doc: feathercad_core::documento::Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro)), ..Agente::default() };
        let r = alinhamento(&mut ag, &c6()).unwrap();
        let c = &r["curvas"][0];
        assert!((c["Lc"].as_f64().unwrap() - 503.75).abs() < 0.05);
        assert!((c["Ts"].as_f64().unwrap() - 410.44).abs() < 0.1);
        assert!(r["formas_desenhadas"].as_u64().unwrap() > 10);
        let s = superelevacao(&mut ag, &json!({})).unwrap();
        assert_eq!(s["todas_atendem"], true);
        assert!((s["curvas"][0]["he_mm"].as_f64().unwrap() - 102.2).abs() < 0.05);
        assert!(s["memorial"].as_str().unwrap().contains("Curva 1"));
        let ts = c["TS"]["progressiva"].as_f64().unwrap();
        let p = alinhamento_ponto(&ag, &json!({ "progressiva": ts + 1.0 })).unwrap();
        assert_eq!(p["elemento"], "E");
        assert!((p["E"].as_f64().unwrap() - c["TS"]["E"].as_f64().unwrap() - 1.0).abs() < 0.01);
        let de_texto = alinhamento_ponto(&ag, &json!({ "estaca": c["TS"]["estaca"] })).unwrap();
        assert!((de_texto["progressiva"].as_f64().unwrap() - ts).abs() < 0.01, "a estaca em texto volta à progressiva");
        let p0 = alinhamento_ponto(&ag, &json!({ "progressiva": 100 })).unwrap();
        assert_eq!(p0["elemento"], "T");
    }

    #[test]
    fn ls_auto_aumenta_a_transicao_curta() {
        let mut ag = Agente::default();
        let d = 93.92f64.to_radians();
        alinhamento(&mut ag, &json!({ "pis": [[0, 0], [2000, 0], [2000.0 + 2000.0 * d.cos(), 2000.0 * d.sin()]], "curvas": [{ "R": 350, "Ls": 40 }] })).unwrap();
        let s = superelevacao(&mut ag, &json!({ "Ls_auto": true })).unwrap();
        assert_eq!(s["Ls_ajustados"][0]["Ls_depois"], 70.0);
        assert_eq!(s["todas_atendem"], true);
    }

    #[test]
    fn numero_de_curvas_errado_diz_quantas() {
        let mut ag = Agente::default();
        let e = alinhamento(&mut ag, &json!({ "pis": [[0, 0], [10, 0], [10, 10]], "curvas": [] })).unwrap_err();
        assert!(e.contains("1 entradas"), "{e}");
    }
}

/// Confere o motor viário contra os números do projeto ferroviário de 01/10/2026 (`dados_calculo.json`, só onde existe).
#[cfg(test)]
mod referencia {
    use feathercad_core::viario::greide::{ajustar, estatistica_do_levante, Opcoes};
    use feathercad_core::viario::superelevacao::{calcular, Regras};
    use serde_json::Value;

    use super::*;

    fn dados() -> Option<Value> {
        let t = std::fs::read_to_string(std::path::Path::new(&std::env::var_os("HOME")?).join("Documentos/FeatherCAD/ferrovia_otimizacao/dados_calculo.json")).ok()?;
        serde_json::from_str(&t).ok()
    }

    fn lista(v: &Value) -> Vec<f64> {
        v.as_array().unwrap().iter().filter_map(Value::as_f64).collect()
    }

    #[test]
    fn greide_levante_como_o_python() {
        let Some(d) = dados() else { return };
        let perfil: Vec<(f64, f64)> = lista(&d["s_exist"]).into_iter().zip(lista(&d["z_exist"])).collect();
        let g = ajustar(&perfil, &Opcoes { comprimento: d["L"].as_f64().unwrap(), ..Opcoes::default() }).unwrap();
        let (min, med, max) = estatistica_do_levante(&g, &perfil);
        eprintln!("PIVs {} (ref {}) levante min {min:.3} med {med:.3} max {max:.3} (ref med {:.3} max {:.3})", g.pivs.len(), d["pivs"].as_array().unwrap().len(), d["stats"]["lev_med"], d["stats"]["lev_max"]);
        assert!((med - d["stats"]["lev_med"].as_f64().unwrap()).abs() < 0.01, "médio {med}");
        assert!((max - d["stats"]["lev_max"].as_f64().unwrap()).abs() < 0.02, "máximo {max}");
        assert!(min > -0.03, "rebaixo {min}");
    }

    #[test]
    fn alinhamento_de_referencia_tem_6246_m_e_as_curvas_da_tabela() {
        let Some(d) = dados() else { return };
        let pis: Vec<Ponto> = d["PI"].as_array().unwrap().iter().map(|p| { let v = lista(p); Ponto::novo(v[0], v[1]) }).collect();
        let curvas: Vec<Option<EntradaCurva>> = d["curvas"].as_array().unwrap().iter().map(|c| Some(EntradaCurva { raio: c["R"].as_f64().unwrap(), ls: c["Ls"].as_f64().unwrap() })).collect();
        let a = Alinhamento::novo(&pis, &curvas, 0.0).unwrap();
        assert!((a.comprimento - d["L"].as_f64().unwrap()).abs() < 0.5, "comprimento {} x {}", a.comprimento, d["L"]);
        for (c, ref_c) in a.curvas.iter().zip(d["curvas"].as_array().unwrap()) {
            assert!((c.ts - ref_c["Ts"].as_f64().unwrap()).abs() < 0.05, "Ts {} x {}", c.ts, ref_c["Ts"]);
            assert!((c.lc - ref_c["Lc"].as_f64().unwrap()).abs() < 0.05, "Lc {} x {}", c.lc, ref_c["Lc"]);
            assert!((c.ts_marco.progressiva - ref_c["TS"]["s"].as_f64().unwrap()).abs() < 0.05, "TS {} x {}", c.ts_marco.progressiva, ref_c["TS"]["s"]);
        }
        let sup = calcular(&a, &Regras::default());
        for (r, ref_c) in sup.iter().zip(d["curvas"].as_array().unwrap()) {
            assert!((r.he_mm - ref_c["he"].as_f64().unwrap() * 1000.0).abs() < 0.05, "he");
            assert!((r.h_mm - ref_c["hp"].as_f64().unwrap() * 1000.0).abs() < 0.05, "h");
        }
    }
}

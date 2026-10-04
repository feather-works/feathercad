//! Ferramentas viárias que ligam a nuvem ao projeto: `cad_alinhamento_ajustar`, `cad_greide` e `cad_tabela_estacas`.
//! O que vem da nuvem (`feicoes`) é consultado pelo nome guardado na sessão: os pontos não passam pela conversa.

use feathercad_core::geom::Ponto;
use feathercad_core::trajeto::{estaca_texto, km_texto, Trajeto};
use feathercad_core::viario::ajuste::{ajustar, ajustar_auto};
use feathercad_core::viario::alinhamento::Alinhamento;
use feathercad_core::viario::greide::{ajustar as ajustar_greide, estatistica_do_levante, Opcoes, Regra};
use feathercad_core::viario::superelevacao::{calcular, h_em};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::dados::{com_linhas, ler_lista};
use crate::json::{arred, lista_de_pontos, numero_opcional, texto_opcional, Res};
use crate::viario::{achar, curva_json, desenhar};

/// Os pares de uma feição projetados no alinhamento: (distância ao longo do projeto, afastamento, índice do par).
fn projetados(a: &Alinhamento, ag: &Agente, nome: &str, sem_suspeitos: bool) -> Res<Vec<(f64, f64, usize)>> {
    let f = ag.feicoes.get(nome).ok_or_else(|| format!("as feições `{nome}` não existem nesta sessão: rode cad_nuvem_feicoes"))?;
    let t = Trajeto::novo(&a.amostrar(0.5)).ok_or("alinhamento vazio")?;
    let mut v: Vec<(f64, f64, usize)> = f.pares.iter().enumerate().filter(|(i, _)| !(sem_suspeitos && f.suspeitos.contains(i))).map(|(i, p)| { let (d, s) = t.projetar(p.meio); (s + a.progressiva_inicial, d, i) }).collect();
    v.sort_by(|x, y| x.0.total_cmp(&y.0));
    Ok(v)
}

fn lista_de_raios(v: &Value) -> Res<Vec<f64>> {
    v.as_array().ok_or("`raios` precisa ser uma lista de números")?.iter().map(|r| r.as_f64().ok_or_else(|| "raio inválido".to_string())).collect()
}

/// `cad_alinhamento_ajustar`: PIs e raios que minimizam a ripagem em relação ao eixo existente.
pub fn alinhamento_ajustar(ag: &mut Agente, args: &Value) -> Res<Value> {
    let regras = crate::regras::carregar(args)?;
    let existente: Vec<Ponto> = if let Some(p) = args.get("pontos") {
        let l = ler_lista(p)?;
        lista_de_pontos(&Value::Array(l), "pontos")?
    } else {
        let nome = texto_opcional(args, "feicoes").unwrap_or("F1");
        let f = ag.feicoes.get(nome).ok_or_else(|| format!("as feições `{nome}` não existem: rode cad_nuvem_feicoes (e cad_nuvem_qualidade para tirar os suspeitos), ou passe `pontos`"))?;
        let excluir = args.get("excluir_suspeitos").and_then(Value::as_bool).unwrap_or(true);
        f.pares.iter().enumerate().filter(|(i, _)| !(excluir && f.suspeitos.contains(i))).map(|(_, p)| p.meio).collect()
    };
    if existente.len() < 5 {
        return Err("o eixo existente tem pontos de menos para ajustar".into());
    }
    let voltas = args.get("voltas").and_then(Value::as_u64).unwrap_or(3) as usize;
    let arred_r = numero_opcional(args, "arredondar_R").unwrap_or(5.0);
    let meta = numero_opcional(args, "meta_rms").unwrap_or(0.25);
    let tol_dp = numero_opcional(args, "tolerancia_dp").unwrap_or(1.5);
    let (aj, juntar) = if let Some(p) = args.get("pis") {
        let pis = lista_de_pontos(p, "pis")?;
        let raios = args.get("raios").map(lista_de_raios).transpose()?;
        (ajustar(&existente, &pis, raios.as_deref(), &regras, voltas, arred_r)?, None)
    } else {
        let (a, j) = ajustar_auto(&existente, &regras, voltas, arred_r, meta, tol_dp)?;
        (a, Some(j))
    };
    let nome = texto_opcional(args, "nome").unwrap_or("A1").to_string();
    let passo_estaca = numero_opcional(args, "passo_estaca").unwrap_or(20.0);
    let desenhadas = desenhar(ag, &aj.alinhamento, args, passo_estaca)?;
    let r = json!({
        "alinhamento": nome, "curvas_n": aj.raios.len(), "comprimento": arred(aj.alinhamento.comprimento), "estaca_final": estaca_texto(aj.alinhamento.progressiva_final(), passo_estaca),
        "ripagem": { "rms": arred(aj.rms), "maxima": arred(aj.maxima), "pontos": aj.ripagem.len() }, "limite_de_juncao_escolhido": juntar,
        "pis": aj.pis.iter().map(|p| json!([arred(p.x), arred(p.y)])).collect::<Vec<_>>(), "raios": aj.raios,
        "curvas": aj.alinhamento.curvas.iter().map(|c| curva_json(c, passo_estaca)).collect::<Vec<_>>(), "formas_desenhadas": desenhadas,
        "regras": regras.nome, "nota": "Ls de cada curva = transição mínima da superelevação (cad_superelevacao confere); raios arredondados; guardado com este nome",
    });
    ag.alinhamentos.insert(nome.clone(), (aj.alinhamento, passo_estaca));
    ag.alinhamento_atual = Some(nome);
    Ok(r)
}

fn regra_de(args: &Value) -> Res<Regra> {
    match texto_opcional(args, "regra").unwrap_or("so_levante") {
        "so_levante" => Ok(Regra::SoLevante),
        "minimos_quadrados" => Ok(Regra::MinimosQuadrados),
        o => Err(format!("`regra`: so_levante ou minimos_quadrados, não `{o}`")),
    }
}

/// `cad_greide`: greide de projeto por PIVs e curvas verticais, sobre o perfil existente.
pub fn greide(ag: &mut Agente, args: &Value) -> Res<Value> {
    let (nome_a, (a, passo)) = achar(ag, args)?;
    let (a, passo) = (a.clone(), *passo);
    let perfil: Vec<(f64, f64)> = if let Some(p) = args.get("perfil_existente") {
        ler_lista(p)?.iter().map(|q| match (q.get(0).and_then(Value::as_f64), q.get(1).and_then(Value::as_f64)) { (Some(s), Some(z)) => Ok((s - a.progressiva_inicial, z)), _ => Err("cada ponto do perfil é [estaca_m, cota]".to_string()) }).collect::<Res<_>>()?
    } else {
        let nome = texto_opcional(args, "feicoes").unwrap_or("F1");
        let proj = projetados(&a, ag, nome, true)?;
        let f = &ag.feicoes[nome];
        proj.iter().map(|(s, _, i)| (s - a.progressiva_inicial, f.pares[*i].z)).collect()
    };
    let o = Opcoes { regra: regra_de(args)?, rebaixo_max: numero_opcional(args, "rebaixo_max").unwrap_or(0.02), tolerancia: numero_opcional(args, "tolerancia").unwrap_or(0.08), rv: numero_opcional(args, "Rv").unwrap_or(5000.0), comprimento: a.comprimento };
    let g = ajustar_greide(&perfil, &o)?;
    let (min, med, max) = estatistica_do_levante(&g, &perfil);
    let nome = texto_opcional(args, "nome").unwrap_or("G1").to_string();
    let rampa_max = numero_opcional(args, "rampa_max");
    let rampas: Vec<f64> = g.nos_s.windows(2).zip(g.nos_z.windows(2)).map(|(s, z)| (z[1] - z[0]) / (s[1] - s[0]).max(1e-12)).collect();
    let acima = rampa_max.map_or(0, |m| rampas.iter().filter(|r| r.abs() > m).count());
    let pivs: Vec<Value> = g.pivs.iter().map(|p| json!({ "estaca": estaca_texto(p.s + a.progressiva_inicial, passo), "s": arred(p.s + a.progressiva_inicial), "cota": arred(p.z), "rampa_chegada_pct": arred(p.g1 * 100.0), "rampa_saida_pct": arred(p.g2 * 100.0), "Lv": p.lv, "tipo": if p.convexa { "convexa" } else { "côncava" } })).collect();
    let passo_linhas = numero_opcional(args, "passo").unwrap_or(20.0);
    let n = (a.comprimento / passo_linhas).floor() as usize + 1;
    let linhas: Vec<Value> = (0..n).map(|k| { let s = k as f64 * passo_linhas; json!([estaca_texto(s + a.progressiva_inicial, passo), arred(s + a.progressiva_inicial), arred(g.z(s)), arred(g.rampa(s) * 100.0)]) }).collect();
    let resumo = json!({
        "greide": nome, "alinhamento": nome_a, "regra": texto_opcional(args, "regra").unwrap_or("so_levante"), "pivs": pivs, "rampas_pct": rampas.iter().map(|r| arred(r * 100.0)).collect::<Vec<_>>(),
        "rampa_maxima_pct": arred(rampas.iter().fold(0.0f64, |m, r| m.max(r.abs())) * 100.0), "trechos_acima_da_rampa_max": acima,
        "levante": { "minimo": arred(min), "medio": arred(med), "maximo": arred(max) }, "rebaixo_maximo": arred((-min).max(0.0)), "pontos_do_existente": perfil.len(),
        "colunas": ["estaca", "progressiva_m", "cota_projeto", "rampa_pct"],
    });
    let r = com_linhas(ag, args, &format!("{nome}_cotas"), resumo, linhas)?;
    ag.greides.insert(nome, g);
    Ok(r)
}

const PADRAO: [&str; 12] = ["estaca", "km", "elemento", "curva", "h_atual", "h_projeto", "flecha_medida", "flecha_projetada", "ripagem", "cota_existente", "cota_projeto", "levante"];

/// Interpolação linear em série ordenada por `x` (preso às pontas).
fn interpolar(xs: &[f64], ys: &[f64], x: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let j = xs.partition_point(|v| *v < x);
    if j == 0 {
        return Some(ys[0]);
    }
    if j >= xs.len() {
        return Some(ys[xs.len() - 1]);
    }
    let t = (x - xs[j - 1]) / (xs[j] - xs[j - 1]).max(1e-12);
    Some(ys[j - 1] + (ys[j] - ys[j - 1]) * t)
}

/// Curvatura do círculo por 3 pontos consecutivos (positiva = esquerda).
fn curvatura3(a: Ponto, b: Ponto, c: Ponto) -> f64 {
    let (ab, bc, ca) = (a.distancia(b), b.distancia(c), c.distancia(a));
    let cruz = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if ab * bc * ca < 1e-12 { 0.0 } else { 2.0 * cruz / (ab * bc * ca) }
}

/// `cad_tabela_estacas`: as linhas (uma por estaca) prontas para o `cad_tabela`, sem a IA copiar números.
pub fn tabela_estacas(ag: &mut Agente, args: &Value) -> Res<Value> {
    let regras = crate::regras::carregar(args)?;
    let (nome_a, (a, passo_est)) = achar(ag, args)?;
    let (a, passo_est) = (a.clone(), *passo_est);
    let tabela_h = calcular(&a, &regras);
    let g = texto_opcional(args, "greide").and_then(|n| ag.greides.get(n)).or_else(|| ag.greides.values().next()).cloned();
    let nome_f = texto_opcional(args, "existente").or_else(|| texto_opcional(args, "feicoes")).unwrap_or("F1");
    let proj = projetados(&a, ag, nome_f, false).ok();
    let (st, rip, zex, hx, flecha) = match (&proj, ag.feicoes.get(nome_f)) {
        (Some(p), Some(f)) => {
            let pares: Vec<_> = p.iter().map(|(_, _, i)| &f.pares[*i]).collect();
            let meios: Vec<Ponto> = f.pares.iter().map(|q| q.meio).collect();
            let kex: Vec<f64> = p.iter().map(|(_, _, i)| if *i == 0 || *i + 1 >= meios.len() { 0.0 } else { curvatura3(meios[i - 1], meios[*i], meios[i + 1]) }).collect();
            (p.iter().map(|x| x.0).collect::<Vec<_>>(), p.iter().map(|x| x.1).collect::<Vec<_>>(), pares.iter().map(|q| q.z).collect::<Vec<_>>(), pares.iter().map(|q| q.nivelamento).collect::<Vec<_>>(), kex)
        }
        _ => (vec![], vec![], vec![], vec![], vec![]),
    };
    let colunas: Vec<String> = match args.get("colunas") {
        Some(c) => c.as_array().ok_or("`colunas` é uma lista de nomes")?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
        None => PADRAO.iter().map(ToString::to_string).collect(),
    };
    if let Some(c) = colunas.iter().find(|c| !PADRAO.contains(&c.as_str()) && !["E", "N", "curvatura", "progressiva"].contains(&c.as_str())) {
        return Err(format!("coluna `{c}` não existe (disponíveis: {}, E, N, curvatura, progressiva)", PADRAO.join(", ")));
    }
    let passo = numero_opcional(args, "passo").unwrap_or(20.0);
    let so_curvas = args.get("so_curvas").and_then(Value::as_bool).unwrap_or(false);
    let n = (a.comprimento / passo + 1e-9).floor() as usize + 1;
    let mut linhas: Vec<Value> = Vec::new();
    for k in 0..n {
        let s = a.progressiva_inicial + k as f64 * passo;
        let am = a.em(s);
        if so_curvas && am.curva.is_none() {
            continue;
        }
        let perto = st.iter().any(|x| (x - s).abs() <= 12.0);
        let h_proj = h_em(&a, &tabela_h, s);
        let sinal = if am.curvatura.abs() > 1e-9 { -am.curvatura.signum() } else { 1.0 };
        let (z_pr, z_ex) = (g.as_ref().map(|g| g.z(s - a.progressiva_inicial)), interpolar(&st, &zex, s));
        let linha: Vec<Value> = colunas
            .iter()
            .map(|c| match c.as_str() {
                "estaca" => json!(estaca_texto(s, passo_est)),
                "km" => json!(km_texto(s)),
                "elemento" => json!(am.tipo.letra()),
                "curva" => json!(am.curva.map(|i| i + 1)),
                "progressiva" => json!(arred(s)),
                "E" => json!(arred(am.ponto.x)),
                "N" => json!(arred(am.ponto.y)),
                "curvatura" => json!(arred(am.curvatura * 1e6) / 1e6),
                "h_atual" => if perto { interpolar(&st, &hx, s).map_or(Value::Null, |v| json!(arred((v * 1000.0 * sinal * 10.0).round() / 10.0))) } else { Value::Null },
                "h_projeto" => json!(arred((h_proj * 10.0).round() / 10.0)),
                "flecha_medida" => interpolar(&st, &flecha, s).map_or(Value::Null, |v| json!(arred(v * regras.corda * regras.corda / 8.0 * 1000.0 * 10.0).round() / 10.0)),
                "flecha_projetada" => json!(arred(am.curvatura * regras.corda * regras.corda / 8.0 * 1000.0 * 10.0).round() / 10.0),
                "ripagem" => interpolar(&st, &rip, s).map_or(Value::Null, |v| json!(arred(v))),
                "cota_existente" => z_ex.map_or(Value::Null, |v| json!(arred(v))),
                "cota_projeto" => z_pr.map_or(Value::Null, |v| json!(arred(v))),
                "levante" => match (z_pr, z_ex) { (Some(p), Some(e)) => json!(arred(p - e)), _ => Value::Null },
                _ => Value::Null,
            })
            .collect();
        linhas.push(Value::Array(linha));
    }
    let resumo = json!({ "alinhamento": nome_a, "colunas": colunas, "linhas_n": linhas.len(), "passo": passo, "com_existente": proj.is_some(), "com_greide": g.is_some(),
        "unidades": { "h_atual": "mm (+ = trilho externo mais alto)", "h_projeto": "mm", "flecha": "mm (corda das regras; + = esquerda)", "ripagem": "m (+ = existente à esquerda do projeto)", "cota": "m", "levante": "m" },
        "dica": "para desenhar, passe `desenhar_tabela` (argumentos de cad_tabela: origem, escala, titulo, blocos…) ou mande estas linhas a cad_tabela" });
    let mut r = com_linhas(ag, args, "tabela_estacas", resumo, linhas.clone())?;
    if let Some(t) = args.get("desenhar_tabela") {
        let mut o = t.clone();
        let titulos = |c: &str| match c { "estaca" => "Estaca", "km" => "Km", "elemento" => "Elem.", "curva" => "Curva", "h_atual" => "h atual", "h_projeto" => "h proj.", "flecha_medida" => "f med.", "flecha_projetada" => "f proj.", "ripagem" => "Ripag.", "cota_existente" => "Cota ex.", "cota_projeto" => "Cota pr.", "levante" => "Lev.", outro => outro }.to_string();
        let cols: Vec<Value> = colunas.iter().map(|c| json!({ "titulo": titulos(c), "largura": if c == "estaca" || c == "km" { 18.0 } else { 14.0 }, "alinhamento": if c == "estaca" || c == "km" || c == "elemento" || c == "curva" { "centro" } else { "direita" } })).collect();
        o["colunas"] = o.get("colunas").cloned().unwrap_or(Value::Array(cols));
        o["linhas"] = Value::Array(linhas.iter().map(|l| Value::Array(l.as_array().cloned().unwrap_or_default().iter().map(|c| if c.is_null() { json!("-") } else { c.clone() }).collect())).collect());
        r["tabela_desenhada"] = crate::tabela::tabela(&mut ag.doc, &o)?;
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use feathercad_core::viario::alinhamento::EntradaCurva;

    use super::*;

    /// Escreve a nuvem de uma via sintética (curva R 400 com transição), trilhos a ±0,53 m, cota subindo 1 ‰.
    fn nuvem() -> (String, f64) {
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(500.0, 0.0), Ponto::novo(500.0 + 500.0 * d.cos(), 500.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        let mut t = String::from("E,N,Z\n");
        let n = (a.comprimento / 5.0) as usize;
        for i in 0..=n {
            let s = i as f64 * 5.0;
            let am = a.em(s);
            let r = am.direcao.to_radians();
            for lado in [0.53, -0.53] {
                t += &format!("{:.3},{:.3},{:.4}\n", am.ponto.x - r.sin() * lado, am.ponto.y + r.cos() * lado, 10.0 + s * 0.001);
            }
        }
        let p = std::env::temp_dir().join(format!("fcad_via_{}.csv", std::process::id()));
        std::fs::write(&p, t).unwrap();
        (p.display().to_string(), a.comprimento)
    }

    #[test]
    fn da_nuvem_ao_projeto_sem_copiar_numeros() {
        let (c, comprimento) = nuvem();
        let mut ag = Agente { doc: feathercad_core::documento::Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro)), ..Agente::default() };
        crate::nuvem_ferramentas::secoes(&mut ag, &json!({ "caminho": c, "inicio": [0, 0], "direcao": 0 })).unwrap();
        crate::nuvem_ferramentas::feicoes(&mut ag, &json!({})).unwrap();
        crate::nuvem_ferramentas::qualidade_ferramenta(&mut ag, &json!({})).unwrap();
        let r = alinhamento_ajustar(&mut ag, &json!({ "voltas": 1, "regras": { "V": 80 } })).unwrap();
        assert!(r["ripagem"]["rms"].as_f64().unwrap() < 1.0, "{}", r["ripagem"]);
        assert!((r["comprimento"].as_f64().unwrap() - comprimento).abs() < 15.0, "{} x {comprimento}", r["comprimento"]);
        let g = greide(&mut ag, &json!({})).unwrap();
        assert!(g["levante"]["medio"].as_f64().unwrap().abs() < 0.1, "{}", g["levante"]);
        crate::viario::superelevacao(&mut ag, &json!({ "regras": { "V": 80 } })).unwrap();
        let t = tabela_estacas(&mut ag, &json!({ "regras": { "V": 80 }, "colunas": ["estaca", "elemento", "h_projeto", "cota_projeto", "levante"], "desenhar_tabela": { "origem": [0, -100], "escala": 1000 } })).unwrap();
        assert!(t["linhas_n"].as_u64().unwrap() > 20);
        assert!(t["tabela_desenhada"]["criadas"].as_u64().unwrap() > 50, "{t}");
        let _ = std::fs::remove_file(&c);
    }
}

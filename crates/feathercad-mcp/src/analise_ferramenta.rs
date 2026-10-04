//! `cad_analise` e `cad_geografico`: a análise de criticidade do traçado (curvas e tangentes, aterro e escavação, volumes) e a
//! conversão UTM → graus, do motor, em JSON — para as folhas e para quem quiser montar as suas (um script, por exemplo).

use feathercad_core::geodesia::{graus_minutos_segundos, utm_para_geografico};
use feathercad_core::viario::criticidade::{analisar, Elemento, Tipo, Zona};
use feathercad_core::viario::superelevacao::calcular;
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{arred, numero_de, par, texto_opcional, Res};

/// Os pontos do levantamento projetados no traçado: progressiva, cota e ripagem (suspeitos de fora).
pub(crate) fn observacoes(a: &feathercad_core::viario::alinhamento::Alinhamento, f: &crate::nuvem_ferramentas::Feicoes) -> Vec<feathercad_core::viario::criticidade::Observacao> {
    let t = feathercad_core::trajeto::Trajeto::novo(&a.amostrar(0.5));
    let mut v: Vec<_> = f
        .pares
        .iter()
        .enumerate()
        .filter(|(i, _)| !f.suspeitos.contains(i))
        .filter_map(|(_, p)| t.as_ref().map(|t| { let (d, s) = t.projetar(p.meio); feathercad_core::viario::criticidade::Observacao { s: s + a.progressiva_inicial, z: p.z, ripagem: d } }))
        .collect();
    v.sort_by(|x, y| x.s.total_cmp(&y.s));
    v
}

/// Os limiares da análise: os padrões de `Criterios` com o que vier em `dados.criterios` por cima.
pub(crate) fn criterios_de(d: &Value) -> feathercad_core::viario::criticidade::Criterios {
    let mut c = feathercad_core::viario::criticidade::Criterios::default();
    let Some(o) = d.get("criterios").and_then(Value::as_object) else { return c };
    let n = |k: &str| o.get(k).and_then(Value::as_f64);
    let campos: [(&str, &mut f64); 14] = [("fracao_h", &mut c.fracao_h), ("fracao_i", &mut c.fracao_i), ("folga_r", &mut c.folga_r), ("folga_v", &mut c.folga_v), ("folga_ls", &mut c.folga_ls), ("ripagem_atencao", &mut c.ripagem_atencao), ("ripagem_critica", &mut c.ripagem_critica), ("movimento_minimo", &mut c.movimento_minimo), ("movimento_atencao", &mut c.movimento_atencao), ("movimento_critico", &mut c.movimento_critico), ("rampa_atencao_pct", &mut c.rampa_atencao_pct), ("rampa_critica_pct", &mut c.rampa_critica_pct), ("tangente_minima", &mut c.tangente_minima), ("largura_plataforma", &mut c.largura_plataforma)];
    for (nome, campo) in campos {
        if let Some(v) = n(nome) {
            *campo = v;
        }
    }
    c
}


fn zona_json(z: &Zona) -> Value {
    json!({ "natureza": z.natureza.nome(), "de": arred(z.de), "ate": arred(z.ate), "maximo_m": arred(z.maximo), "onde": arred(z.onde), "nivel": z.nivel.nome() })
}

/// Os números da curva (geometria e superelevação) para quem monta a folha do elemento.
fn curva_json(a: &feathercad_core::viario::alinhamento::Alinhamento, r: &feathercad_core::viario::superelevacao::ResultadoCurva, i: usize) -> Value {
    let c = &a.curvas[i];
    let marco = |m: &feathercad_core::viario::alinhamento::Marco| json!({ "progressiva": arred(m.progressiva), "E": arred(m.ponto.x), "N": arred(m.ponto.y) });
    json!({ "raio": c.raio, "ls": c.ls, "lc": arred(c.lc), "delta_graus": arred(c.delta_graus), "lado": if c.lado > 0 { "esquerda" } else { "direita" }, "ts_m": arred(c.ts), "e_m": arred(c.e),
        "theta_s_graus": arred(c.theta_s_graus), "pi": { "progressiva": arred(c.pi_progressiva), "E": arred(c.pi_ponto.x), "N": arred(c.pi_ponto.y) },
        "TS": marco(&c.ts_marco), "SC": marco(&c.sc), "CS": marco(&c.cs), "ST": marco(&c.st),
        "he_mm": arred(r.he_mm), "h_mm": r.h_mm, "i_mm": arred(r.i_mm), "anc": arred(r.anc * 1e4) / 1e4, "r_min": arred(r.r_min), "v_max": arred(r.v_max), "l_min": arred(r.l_min), "ls_sugerido": r.ls_sugerido,
        "rampa_efetiva": r.rampa_efetiva, "flecha_mm": arred(r.flecha_mm), "excesso_v_lenta_mm": arred(r.excesso_v_lenta_mm), "atende": r.ok })
}

fn elemento_json(e: &Elemento) -> Value {
    let (tipo, n) = match e.tipo {
        Tipo::Curva(i) => ("curva", i + 1),
        Tipo::Tangente(i) => ("tangente", i + 1),
    };
    json!({ "tipo": tipo, "n": n, "de": arred(e.de), "ate": arred(e.ate), "comprimento": arred(e.comprimento()), "nivel": e.nivel.nome(),
        "motivos": e.motivos.iter().map(|m| json!({ "nivel": m.nivel.nome(), "texto": m.texto })).collect::<Vec<_>>(), "observacoes": e.observacoes,
        "levante_m": { "min": arred(e.levante_min), "max": arred(e.levante_max), "medio": arred(e.levante_medio) }, "ripagem_m": { "max": arred(e.ripagem_max), "rms": arred(e.ripagem_rms) },
        "rampa_pct": { "min": arred(e.rampa_min_pct), "max": arred(e.rampa_max_pct) }, "cota_m": { "min": arred(e.cota_min), "max": arred(e.cota_max) },
        "zonas": e.zonas.iter().map(zona_json).collect::<Vec<_>>() })
}

/// `cad_analise`: o que é crítico, o que pede atenção e onde há aterro ou escavação, de cada curva e tangente.
pub fn analise(ag: &Agente, args: &Value) -> Res<Value> {
    use feathercad_core::viario::criticidade::Nivel;
    let (nome_a, (a, _)) = crate::viario::achar(ag, args)?;
    let regras = crate::regras::carregar(args)?;
    let tabela = calcular(a, &regras);
    let g = texto_opcional(args, "greide").and_then(|n| ag.greides.get(n)).or_else(|| ag.greides.values().next()).ok_or("não há greide: rode cad_greide")?;
    let nome_f = texto_opcional(args, "existente").unwrap_or("F1");
    let f = ag.feicoes.get(nome_f).ok_or_else(|| format!("as feições `{nome_f}` não existem: rode cad_nuvem_feicoes"))?;
    let obs = observacoes(a, f);
    let an = analisar(a, &tabela, &regras, g, &obs, &criterios_de(args));
    let mut elementos: Vec<Value> = an.elementos.iter().map(elemento_json).collect();
    for (v, e) in elementos.iter_mut().zip(&an.elementos) {
        if let Tipo::Curva(i) = e.tipo {
            v["curva"] = curva_json(a, &tabela[i], i);
        }
    }
    Ok(json!({
        "alinhamento": nome_a, "criterios": format!("{:?}", an.criterios), "observacoes": obs.len(),
        "contagem": { "criticos": an.contagem(Nivel::Critico), "atencao": an.contagem(Nivel::Atencao), "normais": an.contagem(Nivel::Normal) },
        "elementos": elementos,
        "regras": { "nome": regras.nome, "B": regras.b, "V": regras.v, "v_lenta": regras.v_lenta, "h_max_mm": regras.h_max_mm, "i_max_mm": regras.i_max_mm, "dh_dt_mm_s": regras.dh_dt_mm_s, "rampa": regras.rampa, "psi": regras.psi, "ls_min": regras.ls_min, "lc_min": regras.lc_min, "corda": regras.corda },
        "alinhamento_geral": { "comprimento": arred(a.comprimento), "progressiva_inicial": a.progressiva_inicial, "progressiva_final": arred(a.progressiva_final()), "curvas": a.curvas.len() },
        "pivs": g.pivs.iter().map(|p| json!({ "progressiva": arred(a.progressiva_inicial + p.s), "cota": arred(p.z), "rampa_chegada_pct": arred(p.g1 * 100.0), "rampa_saida_pct": arred(p.g2 * 100.0), "lv": p.lv, "convexa": p.convexa })).collect::<Vec<_>>(),
        "zonas": an.zonas.iter().map(zona_json).collect::<Vec<_>>(),
        "volumes": { "aterro_m3": arred(an.volumes.aterro_m3), "escavacao_m3": arred(an.volumes.escavacao_m3), "saldo_m3": arred(an.volumes.aterro_m3 - an.volumes.escavacao_m3), "largura_plataforma_m": an.volumes.largura,
            "curva_de_massa": an.volumes.curva_de_massa.iter().map(|(s, v)| json!([arred(*s), arred(*v)])).collect::<Vec<_>>() },
        "nota": "levante = greide − existente (+ aterro, − escavação); volumes estimados a partir do eixo dos trilhos × largura da plataforma, sem seção do terreno",
    }))
}

/// `cad_geografico`: UTM → latitude/longitude (+ convergência e escala) de `pontos`, na `zona` e hemisfério dados.
pub fn geografico(args: &Value) -> Res<Value> {
    let zona = numero_de(args, "zona")?;
    if !(1.0..=60.0).contains(&zona) || zona.fract() != 0.0 {
        return Err("`zona` é o número da zona UTM (1 a 60)".into());
    }
    let sul = texto_opcional(args, "hemisferio").map_or(true, |h| h.to_lowercase().starts_with('s'));
    let pontos = args.get("pontos").and_then(Value::as_array).ok_or("falta `pontos`: [[E, N], …]")?;
    let mut saida = Vec::new();
    for (i, p) in pontos.iter().enumerate() {
        let (e, n) = match (p.get(0).and_then(Value::as_f64), p.get(1).and_then(Value::as_f64)) {
            (Some(e), Some(n)) => (e, n),
            _ => return Err(format!("ponto {i}: use [E, N]")),
        };
        let g = utm_para_geografico(e, n, zona as u32, sul);
        saida.push(json!({ "utm": par(feathercad_core::geom::Ponto::novo(e, n)), "latitude": g.latitude, "longitude": g.longitude, "latitude_dms": graus_minutos_segundos(g.latitude, 'N', 'S'), "longitude_dms": graus_minutos_segundos(g.longitude, 'L', 'O'),
            "convergencia_graus": arred(g.convergencia * 1e4) / 1e4, "escala": g.escala }));
    }
    Ok(json!({ "zona": zona, "hemisferio": if sul { "sul" } else { "norte" }, "datum": "SIRGAS 2000 / WGS 84 (GRS 80)", "pontos": saida, "meridiano_central": feathercad_core::geodesia::meridiano_central(zona as u32), "nota": "a zona é uma premissa: confirme no levantamento" }))
}

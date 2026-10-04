//! Ferramentas de nuvem de pontos de levantamento linear: `cad_nuvem_secoes`, `cad_nuvem_feicoes`, `cad_nuvem_qualidade`.
//! A conta é do `feathercad_core::nuvem`; aqui se lê o arquivo, guarda o resultado na sessão (por nome) e responde.

use std::collections::BTreeSet;

use feathercad_core::documento::Geometria;
use feathercad_core::geom::Ponto;
use feathercad_core::nuvem::{bordos_de_plataforma, pares_de_trilho, qualidade, rastrear_eixo, Par, ParamBordos};
use feathercad_core::trajeto::Trajeto;
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::dados::com_linhas;
use crate::json::{arred, numero_opcional, par, ponto, texto_opcional, Res};
use crate::pecas::{aplicar, Peca};

/// Uma nuvem lida e o eixo de referência (rastreado ou escolhido).
#[derive(Clone)]
pub struct Nuvem {
    pub pontos: Vec<Ponto>,
    pub eixo: Vec<Ponto>,
}

/// Os pares de trilho de uma nuvem e o que a qualidade apontou.
#[derive(Clone)]
pub struct Feicoes {
    pub pares: Vec<Par>,
    pub suspeitos: BTreeSet<usize>,
}

fn percentil(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut o = v.to_vec();
    o.sort_by(f64::total_cmp);
    o[((o.len() - 1) as f64 * p).round() as usize]
}

/// A nuvem `nuvem` da sessão, ou a lida de `caminho` (e guardada sob `nome`).
fn nuvem_de(ag: &mut Agente, args: &Value) -> Res<(String, Nuvem)> {
    if let Some(c) = texto_opcional(args, "caminho") {
        let texto = std::fs::read_to_string(c).map_err(|e| format!("não consegui ler `{c}`: {e}"))?;
        let pontos = feathercad_core::topografia::ler_nuvem_bruta(&texto)?;
        let eixo = match args.get("eixo") {
            Some(e) if e.as_u64().is_some() => {
                let id = e.as_u64().unwrap_or(0);
                let ent = ag.doc.entidade(id).ok_or_else(|| format!("a entidade #{id} não existe"))?;
                let t = Trajeto::de_geometria(&ent.geometria).ok_or_else(|| format!("#{id} não é linha, polilinha nem arco"))?;
                t.pontos
            }
            Some(Value::String(s)) if s != "auto" => return Err("`eixo`: o id de uma polilinha ou \"auto\"".into()),
            _ => {
                let inicio = args.get("inicio").map(|p| ponto(p, "inicio")).transpose()?;
                let eixo = rastrear_eixo(&pontos, inicio, numero_opcional(args, "direcao"));
                if eixo.len() < 2 {
                    return Err("não consegui rastrear o eixo pela nuvem: passe `inicio` e `direcao`, ou o `eixo` (id de uma polilinha)".into());
                }
                eixo
            }
        };
        let nome = texto_opcional(args, "nome").unwrap_or("N1").to_string();
        ag.nuvens.insert(nome.clone(), Nuvem { pontos, eixo });
        ag.nuvem_atual = Some(nome);
    }
    let nome = texto_opcional(args, "nuvem").map(str::to_string).or_else(|| ag.nuvem_atual.clone()).ok_or("diga o `caminho` da nuvem (ou `nuvem`: o nome de uma já lida nesta sessão)")?;
    ag.nuvens.get(&nome).cloned().map(|n| (nome.clone(), n)).ok_or_else(|| format!("a nuvem `{nome}` não existe nesta sessão"))
}

/// `cad_nuvem_secoes`: lê a nuvem, acha o eixo e devolve cada ponto como (estaca, afastamento, cota).
pub fn secoes(ag: &mut Agente, args: &Value) -> Res<Value> {
    let (nome, n) = nuvem_de(ag, args)?;
    let eixo = Trajeto::novo(&n.eixo).ok_or("o eixo da nuvem é curto demais")?;
    let mut linhas: Vec<(f64, f64, f64, Ponto)> = n.pontos.iter().map(|p| { let (d, s) = eixo.projetar(*p); (s, d, p.z, *p) }).collect();
    linhas.sort_by(|a, b| a.0.total_cmp(&b.0));
    let agrupar = numero_opcional(args, "agrupar_m").unwrap_or(3.0);
    let secoes = 1 + linhas.windows(2).filter(|j| j[1].0 - j[0].0 >= agrupar).count();
    let resumo = json!({ "nuvem": nome, "pontos": n.pontos.len(), "eixo": { "vertices": n.eixo.len(), "comprimento": arred(eixo.comprimento()) }, "secoes": secoes,
        "colunas": ["estaca_m", "afastamento_m", "cota", "E", "N"], "nota": "afastamento positivo = à esquerda do eixo; o eixo foi rastreado pela nuvem (ou é o `eixo` dado)" });
    let rows = linhas.iter().map(|(s, d, z, p)| json!([arred(*s), arred(*d), arred(*z), arred(p.x), arred(p.y)])).collect();
    com_linhas(ag, args, &format!("{nome}_secoes"), resumo, rows)
}

/// `cad_nuvem_feicoes`: acha os pares de trilho em cada seção (tipo `trilhos`).
pub fn feicoes(ag: &mut Agente, args: &Value) -> Res<Value> {
    let tipo = texto_opcional(args, "tipo").unwrap_or("trilhos");
    if tipo != "trilhos" && tipo != "bordos" {
        return Err(format!("tipo `{tipo}` ainda não existe (`trilhos` ou `bordos`; meio-fio virá)"));
    }
    let (nome_nuvem, n) = nuvem_de(ag, args)?;
    let eixo = Trajeto::novo(&n.eixo).ok_or("o eixo da nuvem é curto demais")?;
    if tipo == "bordos" {
        return bordos(ag, args, &nome_nuvem, &n, &eixo);
    }
    let centro = numero_opcional(args, "eixo_a_eixo").unwrap_or_else(|| numero_opcional(args, "bitola").unwrap_or(1.0) + 0.06);
    let tol = numero_opcional(args, "tolerancia").unwrap_or(0.10);
    let f = pares_de_trilho(&n.pontos, &eixo, centro, tol, numero_opcional(args, "afastamento_max").unwrap_or(3.5), numero_opcional(args, "agrupar_m").unwrap_or(3.0));
    let nome = texto_opcional(args, "nome").unwrap_or("F1").to_string();
    let bit: Vec<f64> = f.pares.iter().map(|p| p.bitola).collect();
    let niv: Vec<f64> = f.pares.iter().map(|p| p.nivelamento * 1000.0).collect();
    let gaps: Vec<(f64, f64)> = f.pares.windows(2).map(|j| (j[0].s, j[1].s)).collect();
    let maior = gaps.iter().copied().fold((0.0, 0.0), |m, g| if g.1 - g.0 > m.1 - m.0 { g } else { m });
    let esp: Vec<f64> = gaps.iter().map(|g| g.1 - g.0).collect();
    let resumo = json!({
        "feicoes": nome, "nuvem": nome_nuvem, "tipo": tipo, "secoes": f.secoes, "pares": f.pares.len(), "secoes_sem_par": f.sem_par.len(),
        "bitola_medida": { "mediana": arred(percentil(&bit, 0.5)), "p5": arred(percentil(&bit, 0.05)), "p95": arred(percentil(&bit, 0.95)) },
        "nivelamento_transversal_mm": { "mediana": arred(percentil(&niv, 0.5)), "p5": arred(percentil(&niv, 0.05)), "p95": arred(percentil(&niv, 0.95)), "max_abs": arred(niv.iter().fold(0.0f64, |m, x| m.max(x.abs()))) },
        "espacamento_mediano_m": arred(percentil(&esp, 0.5)), "maior_lacuna": { "de": arred(maior.0), "ate": arred(maior.1), "m": arred(maior.1 - maior.0) },
        "colunas": ["indice", "estaca_m", "E", "N", "cota", "nivelamento_mm", "bitola"],
        "nota": "nivelamento = trilho esquerdo − direito (esquerda = lado + do eixo rastreado); guardado na sessão como `feicoes`",
    });
    let rows = f.pares.iter().enumerate().map(|(i, p)| json!([i, arred(p.s), arred(p.meio.x), arred(p.meio.y), arred(p.z), arred(p.nivelamento * 1000.0), arred(p.bitola)])).collect();
    let r = com_linhas(ag, args, &format!("{nome}_pares"), resumo, rows)?;
    ag.feicoes.insert(nome, Feicoes { pares: f.pares, suspeitos: BTreeSet::new() });
    Ok(r)
}

/// `cad_nuvem_feicoes {tipo: "bordos"}`: crista da plataforma e pé do talude, de cada lado, em cada seção.
fn bordos(ag: &mut Agente, args: &Value, nome_nuvem: &str, n: &Nuvem, eixo: &Trajeto) -> Res<Value> {
    let padrao = ParamBordos::default();
    let par_b = ParamBordos {
        tolerancia_z: numero_opcional(args, "tolerancia_z").unwrap_or(padrao.tolerancia_z),
        buraco: numero_opcional(args, "buraco_m").unwrap_or(padrao.buraco),
        afast_max: numero_opcional(args, "afastamento_max").unwrap_or(padrao.afast_max),
        inclinacao_pe: numero_opcional(args, "inclinacao_pe").unwrap_or(padrao.inclinacao_pe),
        agrupar: numero_opcional(args, "agrupar_m").unwrap_or(padrao.agrupar),
    };
    let b = bordos_de_plataforma(&n.pontos, eixo, &par_b);
    let nome = texto_opcional(args, "nome").unwrap_or("B1").to_string();
    let largura = |lado: i8| -> Vec<f64> { b.iter().filter(|x| x.lado == lado).map(|x| x.afastamento).collect() };
    let com_pe = b.iter().filter(|x| x.pe.is_some()).count();
    let resumo = json!({
        "feicoes": nome, "nuvem": nome_nuvem, "tipo": "bordos", "bordas": b.len(), "com_pe_de_talude": com_pe,
        "afastamento_da_crista_m": { "esquerda_mediana": arred(percentil(&largura(1), 0.5)), "direita_mediana": arred(percentil(&largura(-1), 0.5)) },
        "colunas": ["estaca_m", "lado", "afastamento_crista", "cota_crista", "E", "N", "afastamento_pe", "cota_pe"],
        "nota": "lado: 1 esquerda, -1 direita do eixo. Crista = fim da parte plana (cota a `tolerancia_z` da do eixo); pé = onde a descida do talude acaba (inclinação < `inclinacao_pe`). Seção sem plataforma de um lado não tem borda desse lado.",
    });
    let rows = b.iter().map(|x| json!([arred(x.s), x.lado, arred(x.afastamento), arred(x.crista.z), arred(x.crista.x), arred(x.crista.y), x.pe_afastamento.map(arred), x.pe.map(|p| arred(p.z))])).collect();
    if args.get("desenhar").and_then(Value::as_bool).unwrap_or(false) {
        let mut p = crate::pecas::Pincel::novo(texto_opcional(args, "camada").unwrap_or("BORDOS"));
        for lado in [1i8, -1] {
            let cristas: Vec<Ponto> = b.iter().filter(|x| x.lado == lado).map(|x| x.crista).collect();
            if cristas.len() > 1 {
                p.polilinha(&cristas);
            }
            let pes: Vec<Ponto> = b.iter().filter(|x| x.lado == lado).filter_map(|x| x.pe).collect();
            if pes.len() > 1 {
                p.polilinha(&pes);
            }
        }
        aplicar(&mut ag.doc, p.pecas)?;
    }
    com_linhas(ag, args, &format!("{nome}_bordos"), resumo, rows)
}

/// `cad_nuvem_qualidade`: cota fora da mediana local, lacunas, cotas idênticas em série e densidade. Marca no desenho.
pub fn qualidade_ferramenta(ag: &mut Agente, args: &Value) -> Res<Value> {
    let nome = texto_opcional(args, "feicoes").unwrap_or("F1").to_string();
    let f = ag.feicoes.get(&nome).ok_or_else(|| format!("as feições `{nome}` não existem: rode cad_nuvem_feicoes antes"))?.clone();
    let (desvio, janela, lacuna, serie) = (numero_opcional(args, "desvio").unwrap_or(0.3), numero_opcional(args, "janela").unwrap_or(3.0) as usize, numero_opcional(args, "lacuna").unwrap_or(20.0), numero_opcional(args, "serie").unwrap_or(4.0) as usize);
    let q = qualidade(&f.pares, desvio, janela, lacuna, serie);
    let suspeitos: BTreeSet<usize> = q.fora_da_mediana.iter().map(|x| x.0).collect();
    if args.get("marcar").and_then(Value::as_bool).unwrap_or(false) {
        let camada = texto_opcional(args, "camada").unwrap_or("INTERVENCAO").to_string();
        let raio = numero_opcional(args, "raio_marca").unwrap_or(3.0);
        let mut pecas: Vec<Peca> = Vec::new();
        for i in &suspeitos {
            pecas.push(Peca { camada: camada.clone(), cor: Some(0xFF_00_00), g: Geometria::Circulo { centro: f.pares[*i].meio, raio } });
        }
        for (a, b) in &q.lacunas {
            let (pa, pb) = (f.pares.iter().find(|p| p.s == *a), f.pares.iter().find(|p| p.s == *b));
            if let (Some(pa), Some(pb)) = (pa, pb) {
                pecas.push(Peca { camada: camada.clone(), cor: Some(0xFF_A5_00), g: Geometria::Linha { a: pa.meio, b: pb.meio } });
            }
        }
        aplicar(&mut ag.doc, pecas)?;
    }
    if let Some(m) = ag.feicoes.get_mut(&nome) {
        m.suspeitos = suspeitos;
    }
    let lim = |v: Vec<Value>| -> Vec<Value> { v.into_iter().take(40).collect() };
    let densidade: Vec<f64> = q.densidade.iter().map(|d| d.1 as f64).collect();
    Ok(json!({
        "feicoes": nome, "pares": f.pares.len(),
        "cota_fora_da_mediana": { "total": q.fora_da_mediana.len(), "lista": lim(q.fora_da_mediana.iter().map(|(i, z, m)| json!({ "indice": i, "estaca_m": arred(f.pares[*i].s), "E": arred(f.pares[*i].meio.x), "N": arred(f.pares[*i].meio.y), "cota": arred(*z), "mediana_local": arred(*m) })).collect()) },
        "lacunas": { "total": q.lacunas.len(), "lista": lim(q.lacunas.iter().map(|(a, b)| json!({ "de": arred(*a), "ate": arred(*b), "m": arred(b - a) })).collect()) },
        "cotas_identicas_em_serie": { "trechos": q.cotas_identicas.len(), "pares_com_nivelamento_zero": q.nivelamento_zero, "de_total": f.pares.len(),
            "lista": lim(q.cotas_identicas.iter().map(|(a, b)| json!({ "de_estaca_m": arred(f.pares[*a].s), "ate_estaca_m": arred(f.pares[*b].s), "pares": b - a + 1 })).collect()),
            "alerta": if q.nivelamento_zero * 2 > f.pares.len() { "mais da metade dos pares tem os dois trilhos na MESMA cota: a superelevação medida não é confiável (cota copiada?)" } else { "" } },
        "densidade_por_100m": { "mediana": percentil(&densidade, 0.5), "minima": densidade.iter().copied().fold(f64::MAX, f64::min) },
        "suspeitos_marcados": suspeitos_len(&ag.feicoes, &nome), "nota": "os pares de cota fora da mediana ficam como suspeitos e são deixados de fora do ajuste (cad_alinhamento_ajustar)",
        "posicao_exemplo": f.pares.first().map(|p| par(p.meio)),
    }))
}

fn suspeitos_len(m: &std::collections::BTreeMap<String, Feicoes>, nome: &str) -> usize {
    m.get(nome).map_or(0, |f| f.suspeitos.len())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn escrever_nuvem() -> String {
        let mut t = String::from("E,N,Z\n");
        for i in 0..80 {
            let x = f64::from(i) * 5.0;
            t += &format!("{x},0.53,10.0\n{x},-0.53,{}\n", if i == 40 { 12.0 } else { 10.0 });
        }
        let p = std::env::temp_dir().join(format!("fcad_nuvem_{}.csv", std::process::id()));
        std::fs::write(&p, t).unwrap();
        p.display().to_string()
    }

    #[test]
    fn secoes_feicoes_e_qualidade_encadeadas_pela_sessao() {
        let c = escrever_nuvem();
        let mut ag = Agente::default();
        let s = secoes(&mut ag, &json!({ "caminho": c, "inicio": [0, 0], "direcao": 0 })).unwrap();
        assert_eq!(s["pontos"], 160);
        assert!(s["secoes"].as_u64().unwrap() >= 78);
        let f = feicoes(&mut ag, &json!({ "bitola": 1.0 })).unwrap();
        assert!(f["pares"].as_u64().unwrap() >= 78, "{f}");
        assert!((f["bitola_medida"]["mediana"].as_f64().unwrap() - 1.06).abs() < 1e-6);
        let q = qualidade_ferramenta(&mut ag, &json!({ "marcar": true })).unwrap();
        assert_eq!(q["cota_fora_da_mediana"]["total"], 1, "{q}");
        assert!(ag.doc.entidades().iter().any(|e| matches!(e.geometria, Geometria::Circulo { .. })), "o suspeito foi marcado no desenho");
        assert_eq!(ag.feicoes["F1"].suspeitos.len(), 1);
        assert!(feicoes(&mut ag, &json!({ "tipo": "meio-fio" })).is_err());
        let _ = std::fs::remove_file(&c);
    }

    #[test]
    fn bordos_devolvem_crista_e_pe_e_desenham() {
        let mut t = String::from("E,N,Z\n");
        for i in 0..20 {
            let x = f64::from(i) * 5.0;
            for lado in [1.0, -1.0] {
                for (d, z) in [(0.5, 10.0), (1.5, 10.0), (2.5, 10.0), (3.0, 10.0), (4.0, 9.5), (5.0, 9.0), (6.0, 8.5), (7.0, 8.0), (9.0, 8.0)] {
                    t += &format!("{x},{},{z}\n", lado * d);
                }
            }
        }
        let c = std::env::temp_dir().join(format!("fcad_bordos_{}.csv", std::process::id()));
        std::fs::write(&c, t).unwrap();
        let mut ag = Agente::default();
        let r = feicoes(&mut ag, &json!({ "caminho": c.display().to_string(), "inicio": [0, 0], "direcao": 0, "tipo": "bordos", "desenhar": true, "agrupar_m": 1.0 })).unwrap();
        assert!(r["bordas"].as_u64().unwrap() >= 36, "{r}");
        assert!((r["afastamento_da_crista_m"]["esquerda_mediana"].as_f64().unwrap() - 3.0).abs() < 1e-6, "{r}");
        assert!(r["com_pe_de_talude"].as_u64().unwrap() >= 36);
        assert!(ag.doc.entidades().iter().any(|e| matches!(e.geometria, Geometria::Polilinha { .. })));
        let _ = std::fs::remove_file(&c);
    }
}

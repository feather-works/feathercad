//! `cad_detalhe`: copia a região `janela` do modelo, ampliada, para dentro de um bloco `DETALHE-n` (como a folha
//! é o bloco `FOLHA-n`), com moldura, título e malha de coordenadas **do modelo original** nas bordas.
//! É o embrião da "janela de vista" (viewport) que o app ainda não tem. Refazer = apagar o bloco e chamar de novo.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Espaco, Geometria, NovaEntidade, Propriedades};
use feathercad_core::edicao::{caixa_da_entidade, transformar};
use feathercad_core::geom::{Ponto, Transf, Vertice};
use serde_json::{json, Value};

use crate::compacto::formas_abertas;
use crate::grafico::recortar;
use crate::json::{janela, numero_de, numero_opcional, ponto_de, texto_opcional, Res};
use crate::mutacoes::camada_por_nome;
use crate::pecas::{casas_do_passo, formatar};

pub fn detalhe(doc: &mut Documento, args: &Value) -> Res<Value> {
    let (min, max) = janela(args.get("janela").ok_or("falta `janela` (região do modelo a ampliar)")?)?;
    let destino = ponto_de(args, "ponto")?;
    let s_detalhe = numero_de(args, "escala_detalhe")?;
    if s_detalhe <= 0.0 {
        return Err("`escala_detalhe` precisa ser maior que zero (20 = 1:20)".into());
    }
    let folhas = doc.folhas_inseridas();
    let s_folha = match (numero_opcional(args, "escala_folha"), args.get("folha").and_then(Value::as_u64)) {
        (Some(s), _) if s > 0.0 => s,
        (_, Some(n)) => folhas.get((n as usize).saturating_sub(1)).map(|f| f.escala).ok_or_else(|| format!("a folha {n} não existe"))?,
        _ => folhas.first().map(|f| f.escala).ok_or("diga `escala_folha` (a escala em que o desenho está na folha) ou `folha` (nº da folha inserida)")?,
    };
    let mm = doc.unidade().map_or(1.0, feathercad_core::documento::Unidade::em_milimetros);
    let k = s_folha / mm; // unidades do desenho por mm de papel
    let ampliacao = s_folha / s_detalhe;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(2.5) * k;
    let (w, h) = ((max.x - min.x) * ampliacao, (max.y - min.y) * ampliacao);
    let t = Transf::de_insercao(min, destino, ampliacao, ampliacao, 0.0);

    let (mut recortadas, mut omitidas) = (0usize, 0usize);
    let mut saida: Vec<NovaEntidade> = Vec::new();
    let dentro = |a: Ponto, b: Ponto| a.x >= min.x && b.x <= max.x && a.y >= min.y && b.y <= max.y;
    let toca = |a: Ponto, b: Ponto| a.x <= max.x && b.x >= min.x && a.y <= max.y && b.y >= min.y;
    let nova = |camada: usize, cor: Option<u32>, g: Geometria| NovaEntidade { camada, cor, geometria: g, props: Propriedades::default(), espaco: Espaco::default() };
    let recorte = |a: Ponto, b: Ponto| recortar((a.x - min.x, a.y - min.y), (b.x - min.x, b.y - min.y), max.x - min.x, max.y - min.y).map(|(p, q)| (Ponto::novo(p.0 + min.x, p.1 + min.y), Ponto::novo(q.0 + min.x, q.1 + min.y)));
    for (camada, cor, _, g) in formas_abertas(doc, None) {
        let retas: Option<Vec<(Ponto, Ponto)>> = match &g {
            Geometria::Linha { a, b } => Some(vec![(*a, *b)]),
            Geometria::Polilinha { vertices, fechada } if vertices.iter().all(|v: &Vertice| v.bulge == 0.0) => {
                let n = vertices.len();
                Some((0..if *fechada { n } else { n.saturating_sub(1) }).map(|i| (vertices[i].p, vertices[(i + 1) % n].p)).collect())
            }
            _ => None,
        };
        if let Some(segs) = retas {
            for (a, b) in segs {
                if let Some((p, q)) = recorte(a, b) {
                    if p.distancia(a) > 1e-9 || q.distancia(b) > 1e-9 {
                        recortadas += 1;
                    }
                    saida.push(nova(camada, cor, transformar(&Geometria::Linha { a: p, b: q }, &t)));
                }
            }
            continue;
        }
        let Some((a, b)) = caixa_da_entidade(doc, &g) else { continue };
        if dentro(a, b) {
            saida.push(nova(camada, cor, transformar(&g, &t)));
        } else if toca(a, b) {
            omitidas += 1;
        }
    }

    let camada_detalhe = camada_por_nome(doc, texto_opcional(args, "camada").unwrap_or("DETALHE"))?;
    let (o, f) = (Ponto::novo(destino.x, destino.y), Ponto::novo(destino.x + w, destino.y + h));
    let moldura = [o, Ponto::novo(f.x, o.y), f, Ponto::novo(o.x, f.y)];
    for i in 0..4 {
        saida.push(nova(camada_detalhe, None, Geometria::Linha { a: moldura[i], b: moldura[(i + 1) % 4] }));
    }
    let numero_do_bloco = (1..).find(|n| doc.bloco_por_nome(&format!("DETALHE-{n}")).is_none()).unwrap_or(1);
    let titulo = texto_opcional(args, "titulo").map_or_else(|| format!("DETALHE {numero_do_bloco}"), str::to_string);
    saida.push(nova(camada_detalhe, None, Geometria::Texto { ponto: Ponto::novo(o.x + w / 2.0, f.y + 0.8 * ht), altura: 1.3 * ht, rotacao: 0.0, conteudo: format!("{titulo} - ESCALA 1:{}", formatar(s_detalhe, casas_do_passo(s_detalhe), false)), h: AlinhaH::Centro, v: AlinhaV::Base }));
    let passo = numero_opcional(args, "malha").filter(|p| *p > 0.0);
    if let Some(p) = passo {
        let casas = casas_do_passo(p);
        let virgula = args.get("virgula").and_then(Value::as_bool).unwrap_or(false);
        let marca = |de: f64, ate: f64| -> Vec<f64> { (((de / p) - 1e-9).ceil() as i64..=((ate / p) + 1e-9).floor() as i64).take(500).map(|n| n as f64 * p).collect() };
        for x in marca(min.x, max.x) {
            let px = destino.x + (x - min.x) * ampliacao;
            saida.push(nova(camada_detalhe, None, Geometria::Linha { a: Ponto::novo(px, o.y), b: Ponto::novo(px, o.y - 0.6 * ht) }));
            saida.push(nova(camada_detalhe, None, Geometria::Texto { ponto: Ponto::novo(px, o.y - 0.8 * ht), altura: ht, rotacao: 0.0, conteudo: formatar(x, casas, virgula), h: AlinhaH::Centro, v: AlinhaV::Topo }));
        }
        for y in marca(min.y, max.y) {
            let py = destino.y + (y - min.y) * ampliacao;
            saida.push(nova(camada_detalhe, None, Geometria::Linha { a: Ponto::novo(o.x, py), b: Ponto::novo(o.x - 0.6 * ht, py) }));
            saida.push(nova(camada_detalhe, None, Geometria::Texto { ponto: Ponto::novo(o.x - 0.8 * ht, py), altura: ht, rotacao: 0.0, conteudo: formatar(y, casas, virgula), h: AlinhaH::Direita, v: AlinhaV::Meio }));
        }
    }
    let total = saida.len();
    let id = doc.adicionar_bloco_inserido(&format!("DETALHE-{numero_do_bloco}"), saida, Ponto::novo(0.0, 0.0), 1.0, camada_detalhe).ok_or("não consegui criar o bloco do detalhe")?;
    Ok(json!({
        "bloco": format!("DETALHE-{numero_do_bloco}"), "id": id, "ampliacao": ampliacao, "formas": total,
        "caixa": { "min": [o.x, o.y], "max": [f.x, f.y], "largura": w, "altura": h },
        "segmentos_recortados_na_janela": recortadas, "omitidos_por_cortar_a_janela": omitidas,
        "nota": "círculo, arco, texto, hachura e polilinha com arco só entram inteiros; reta é recortada na janela. Para refazer: cad_desfazer e chame de novo",
    }))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn amplia_e_recorta_a_regiao() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        d.adicionar_linha(Ponto::novo(0.0, 5.0), Ponto::novo(100.0, 5.0));
        d.adicionar_circulo(Ponto::novo(5.0, 5.0), 2.0);
        d.adicionar_circulo(Ponto::novo(10.0, 5.0), 2.0);
        let r = detalhe(&mut d, &json!({ "janela": [[0, 0], [10, 10]], "ponto": [500, 500], "escala_detalhe": 100, "escala_folha": 1000, "malha": 5 })).unwrap();
        assert_eq!(r["ampliacao"], 10.0);
        assert_eq!(r["caixa"]["largura"], 100.0);
        assert_eq!(r["segmentos_recortados_na_janela"], 1);
        assert_eq!(r["omitidos_por_cortar_a_janela"], 1, "o círculo em x=10 corta a borda");
        assert!(d.bloco_por_nome("DETALHE-1").is_some());
        d.desfazer();
        assert!(d.bloco_por_nome("DETALHE-1").is_none(), "um passo de desfazer");
    }
}

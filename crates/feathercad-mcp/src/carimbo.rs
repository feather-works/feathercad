//! `cad_carimbo`: o que o carimbo da folha não guarda em campos — as **linhas do responsável técnico** (projetista, CREA, ART,
//! proprietário, área, data…) — e a **geografia da folha** em coordenadas do desenho: onde ficam as caixas NOTAS, LEGENDA,
//! REVISÕES/RESPONSÁVEL e a área útil, para pôr o conteúdo (cad_notas_padrao, cad_tabela) *dentro* delas e não solto ao lado.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::folha::{AREA, BASE_W, X_LEG, X_NOTAS, X_REV, Y_FAIXA};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{numero_opcional, par, Res};
use crate::pecas::{aplicar, caixa_das, relatorio, Pincel};

/// Converte mm de papel (base A1) da folha em coordenadas do desenho.
struct Mapa {
    min: Ponto,
    por_mm: f64,
}

impl Mapa {
    fn ponto(&self, x: f64, y: f64) -> Ponto {
        Ponto::novo(self.min.x + x * self.por_mm, self.min.y + y * self.por_mm)
    }

    fn caixa(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
        let (a, b) = (self.ponto(x0, y0), self.ponto(x1, y1));
        json!({ "min": par(a), "max": par(b), "largura": b.x - a.x, "altura": b.y - a.y, "largura_mm_papel": x1 - x0, "altura_mm_papel": y1 - y0 })
    }
}

/// `cad_carimbo` {indice (0), linhas: [texto…] (até 5, a partir da 2ª linha da caixa; a 1ª é o campo `responsavel` do carimbo), altura_texto (2,0 mm de papel)}.
pub fn carimbo(doc: &mut Documento, args: &Value) -> Res<Value> {
    let indice = numero_opcional(args, "indice").map_or(0, |n| n as usize);
    let info = doc.folhas_inseridas().get(indice).cloned().ok_or_else(|| format!("a folha {} não existe (veja cad_folha listar)", indice + 1))?;
    let em_mm = doc.unidade().map_or(1.0, feathercad_core::documento::Unidade::em_milimetros);
    let f = info.largura_mm / BASE_W;
    let m = Mapa { min: info.min, por_mm: f * info.escala / em_mm };
    let mut r = json!({
        "folha": indice, "escala": info.escala, "papel": info.papel, "fator_papel": f,
        "faixa": {
            "notas": m.caixa(25.0, 10.0, X_NOTAS, Y_FAIXA - 6.0),
            "legenda": m.caixa(X_NOTAS, 10.0, X_LEG, Y_FAIXA - 6.0),
            "responsavel": m.caixa(X_LEG, 10.0, X_REV, 39.0),
            "area_util": m.caixa(AREA.0, AREA.1, AREA.2, AREA.3),
        },
        "como_usar": "cad_texto_bloco/cad_tabela com `origem` = canto superior esquerdo da caixa (x = min.x, y = max.y) e `escala` = a da folha: as medidas valem em mm de papel; `largura_mm_papel` e `altura_mm_papel` dão o tamanho de cada caixa",
    });
    if let Some(linhas) = args.get("linhas").and_then(Value::as_array) {
        let altura = numero_opcional(args, "altura_texto").filter(|h| *h > 0.0).unwrap_or(2.0);
        let mut p = Pincel::novo("CARIMBO");
        for (i, l) in linhas.iter().filter_map(Value::as_str).take(5).enumerate() {
            p.texto(m.ponto(X_LEG + 2.0, 28.2 - 3.6 * i as f64), altura * f * m.por_mm / f, 0.0, l, AlinhaH::Esquerda, AlinhaV::Base);
        }
        let caixa = caixa_das(&p.pecas);
        let ids = aplicar(doc, p.pecas)?;
        r["escrito"] = relatorio(&ids, caixa);
        r["aviso"] = json!("as linhas são texto solto da camada CARIMBO: se mover a folha, mova-as junto");
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::documento::Unidade;
    use feathercad_core::folha::Campos;

    #[test]
    fn caixas_da_faixa_ficam_dentro_da_folha_e_as_linhas_entram_na_de_responsavel() {
        let mut d = Documento::default().com_unidade(Some(Unidade::Metro));
        d.folha_inserir_ids("A1", Some(100.0), &Campos::default(), None).unwrap();
        let r = carimbo(&mut d, &json!({ "linhas": ["Projetista: X", "CREA 123", "ART 456"] })).unwrap();
        let resp = &r["faixa"]["responsavel"];
        let (rx0, rx1) = (resp["min"][0].as_f64().unwrap(), resp["max"][0].as_f64().unwrap());
        assert!((resp["largura_mm_papel"].as_f64().unwrap() - 150.7).abs() < 0.01);
        assert_eq!(r["escrito"]["criadas"], json!(3));
        let c = &r["escrito"]["caixa"];
        assert!(c["min"][0].as_f64().unwrap() >= rx0 && c["max"][0].as_f64().unwrap() <= rx1, "{r}");
        // a caixa de notas começa na moldura (25 mm de papel = 2,5 m a 1:100) e termina em 165,3 mm
        assert!((r["faixa"]["notas"]["largura_mm_papel"].as_f64().unwrap() - 140.3).abs() < 0.01);
        assert!(carimbo(&mut d, &json!({ "indice": 3 })).is_err());
    }
}

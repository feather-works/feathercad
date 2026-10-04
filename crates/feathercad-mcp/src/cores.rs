//! Contraste das cores com o fundo do aplicativo (tema claro ou escuro). O branco e o preto exatos (#FFFFFF, #000000) são
//! a cor "automática" do CAD e trocam sozinhos conforme o fundo; qualquer outra cor fica como foi escolhida, e uma cor quase
//! branca no tema claro (amarelo-limão, cinza claro) ou quase preta no escuro (azul-marinho) some. Aqui se mede e se avisa.

/// Luminância relativa (WCAG) de uma cor `0xRRGGBB`.
pub fn luminancia(cor: u32) -> f64 {
    let canal = |v: u32| {
        let c = f64::from(v & 0xFF) / 255.0;
        if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * canal(cor >> 16) + 0.7152 * canal(cor >> 8) + 0.0722 * canal(cor)
}

/// Fundos de referência (os do tema do aplicativo).
const LUMINANCIA_FUNDO_CLARO: f64 = 0.91;
const LUMINANCIA_FUNDO_ESCURO: f64 = 0.02;
/// Abaixo disto (razão de contraste) a cor quase some.
const CONTRASTE_MINIMO: f64 = 2.0;

/// Razão de contraste entre a cor e o fundo do tema.
pub fn contraste(cor: u32, tema_claro: bool) -> f64 {
    let (a, b) = (luminancia(cor), if tema_claro { LUMINANCIA_FUNDO_CLARO } else { LUMINANCIA_FUNDO_ESCURO });
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// O aviso para uma cor que não se vê no tema (`None` = serve, é #FFFFFF/#000000 automático, ou o tema é desconhecido).
pub fn aviso(cor: u32, tema_claro: Option<bool>) -> Option<String> {
    let claro = tema_claro?;
    if cor == 0xFF_FF_FF || cor == 0x00_00_00 || contraste(cor, claro) >= CONTRASTE_MINIMO {
        return None;
    }
    Some(format!(
        "a cor #{cor:06X} quase não aparece no tema {}: {}. Use #FFFFFF/#000000 (trocam sozinhos com o tema) ou uma cor de contraste médio (laranja, ciano, magenta, verde, vermelho).",
        if claro { "claro" } else { "escuro" },
        if claro { "é clara demais para o fundo claro" } else { "é escura demais para o fundo escuro" }
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn claro_demais_no_claro_e_escuro_demais_no_escuro_avisam() {
        assert!(aviso(0xFF_FF_00, Some(true)).is_some(), "amarelo some no fundo claro");
        assert!(aviso(0xD0_D0_D0, Some(true)).is_some());
        assert!(aviso(0x00_00_8B, Some(false)).is_some(), "azul-marinho some no fundo escuro");
        assert!(aviso(0xFF_7F_00, Some(true)).is_none() && aviso(0x00_BF_FF, Some(false)).is_none());
        assert!(aviso(0xFF_FF_FF, Some(true)).is_none() && aviso(0x00_00_00, Some(false)).is_none(), "branco e preto são automáticos");
        assert!(aviso(0xFF_FF_00, None).is_none(), "sem tema conhecido não se avisa");
    }
}

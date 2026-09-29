//! Exportar para PDF: uma página por folha, texto com acento, estrutura válida.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria, Mudanca, NovaEntidade, Propriedades, Unidade};
use feathercad_core::geom::{Ponto, Vertice};
use feathercad_io::pdf::{exportar_folhas, exportar_folhas_escolhidas, folhas, Opcoes, Orientacao, Papel};

fn retangulo(x: f64, y: f64, w: f64, h: f64) -> Geometria {
    let v = |x, y| Vertice { p: Ponto::novo(x, y), bulge: 0.0 };
    Geometria::Polilinha { vertices: vec![v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)], fechada: true }
}

/// Um documento com `n` folhas de 100 x 50, lado a lado, cada uma com moldura interna, uma linha e um texto.
fn com_folhas(n: usize) -> Documento {
    let mut doc = Documento::default().com_unidade(Some(Unidade::Milimetro));
    doc.nova_camada("FOLHA", None).unwrap();
    let folha = doc.camadas().len() - 1;
    doc.definir_camada_atual(folha).unwrap();
    for i in 0..n {
        let x = i as f64 * 150.0;
        doc.adicionar_geometria(retangulo(x, 0.0, 100.0, 50.0));
        doc.adicionar_geometria(retangulo(x + 5.0, 5.0, 90.0, 40.0));
        doc.adicionar_geometria(Geometria::Linha { a: Ponto::novo(x + 10.0, 10.0), b: Ponto::novo(x + 90.0, 40.0) });
        doc.adicionar_geometria(Geometria::Texto { ponto: Ponto::novo(x + 10.0, 25.0), altura: 5.0, rotacao: 0.0, conteudo: "Instalação Ø25 (a\\b)".into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
    }
    doc
}

fn contem(bytes: &[u8], trecho: &[u8]) -> bool {
    bytes.windows(trecho.len()).any(|w| w == trecho)
}

#[test]
fn uma_pagina_por_folha_da_esquerda_para_a_direita() {
    let doc = com_folhas(3);
    let f = folhas(&doc);
    assert_eq!(f.len(), 3, "a moldura de dentro não conta como folha");
    assert!(f.windows(2).all(|p| p[0].0.x < p[1].0.x));
    let pdf = exportar_folhas(&doc, &Opcoes::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    assert!(contem(&pdf, b"/Count 3"));
    // A3 deitado (folha mais larga que alta): 1190,551 x 841,89 pt.
    assert!(contem(&pdf, b"/MediaBox [0 0 1190.551 841.89]") || contem(&pdf, b"/MediaBox [0 0 1190.551 841.89"), "{}", String::from_utf8_lossy(&pdf[..600.min(pdf.len())]));
}

#[test]
fn texto_com_acento_vai_em_winansi_e_parenteses_sao_escapados() {
    let pdf = exportar_folhas(&com_folhas(1), &Opcoes { papel: Papel::A4, ..Opcoes::default() }).unwrap();
    assert!(contem(&pdf, b"Instala\xE7\xE3o \xD825 \\(a\\\\b\\)"), "ç, ã e Ø saem como um byte só; ( ) e \\ com barra");
    assert!(contem(&pdf, b"/MediaBox [0 0 841.89 595.276]"), "A4 deitado");
}

#[test]
fn reticencias_e_aspas_tem_byte_proprio() {
    let mut doc = com_folhas(1);
    doc.adicionar_geometria(Geometria::Texto { ponto: Ponto::novo(10.0, 40.0), altura: 3.0, rotacao: 0.0, conteudo: "R. das Ferra… “x”".into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
    let pdf = exportar_folhas(&doc, &Opcoes::default()).unwrap();
    assert!(contem(&pdf, b"Ferra\x85 \x93x\x94"));
}

#[test]
fn a_tabela_de_referencias_aponta_para_os_objetos() {
    let pdf = exportar_folhas(&com_folhas(2), &Opcoes::default()).unwrap();
    let texto = String::from_utf8_lossy(&pdf).into_owned();
    let inicio: usize = texto.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
    assert!(pdf[inicio..].starts_with(b"xref"));
    let tabela = &texto[texto.find("xref\n").unwrap()..];
    let linhas: Vec<&str> = tabela.lines().skip(3).take_while(|l| l.ends_with(" n ")).collect();
    assert_eq!(linhas.len(), 4 + 2 * 2, "catálogo, páginas, duas fontes (Helvetica e Symbol) e (página + conteúdo) por folha");
    for (i, l) in linhas.iter().enumerate() {
        let onde: usize = l[..10].parse().unwrap();
        assert!(pdf[onde..].starts_with(format!("{} 0 obj", i + 1).as_bytes()), "objeto {} fora do lugar", i + 1);
    }
}

#[test]
fn sem_folhas_sai_o_desenho_inteiro_e_vazio_e_erro() {
    let mut doc = Documento::default().com_unidade(Some(Unidade::Centimetro));
    assert!(exportar_folhas(&doc, &Opcoes::default()).is_err());
    doc.adicionar_geometria(Geometria::Linha { a: Ponto::novo(0.0, 0.0), b: Ponto::novo(400.0, 100.0) });
    assert_eq!(folhas(&doc).len(), 1);
    let pdf = exportar_folhas(&doc, &Opcoes::default()).unwrap();
    assert!(contem(&pdf, b"/Count 1"));
}

#[test]
fn a_orientacao_forca_em_pe_ou_deitado() {
    let doc = com_folhas(1); // folha mais larga que alta: automática seria deitado
    let vertical = exportar_folhas(&doc, &Opcoes { papel: Papel::A4, orientacao: Orientacao::Vertical, ..Opcoes::default() }).unwrap();
    assert!(contem(&vertical, b"/MediaBox [0 0 595.276 841.89]"), "A4 em pé");
    let horizontal = exportar_folhas(&doc, &Opcoes { papel: Papel::A4, orientacao: Orientacao::Horizontal, ..Opcoes::default() }).unwrap();
    assert!(contem(&horizontal, b"/MediaBox [0 0 841.89 595.276]"), "A4 deitado");
}

#[test]
fn so_as_folhas_escolhidas_saem_na_ordem_pedida() {
    let doc = com_folhas(4);
    let pdf = exportar_folhas_escolhidas(&doc, Some(&[2, 0]), &Opcoes::default()).unwrap();
    assert!(contem(&pdf, b"/Count 2"));
    assert!(exportar_folhas_escolhidas(&doc, Some(&[9]), &Opcoes::default()).is_err(), "folha que não existe");
    assert!(exportar_folhas_escolhidas(&doc, Some(&[]), &Opcoes::default()).is_err(), "nenhuma folha");
    assert!(contem(&exportar_folhas_escolhidas(&doc, None, &Opcoes::default()).unwrap(), b"/Count 4"), "sem escolha, todas");
}

#[test]
fn letras_gregas_saem_na_fonte_symbol_e_nao_como_interrogacao() {
    let mut doc = com_folhas(1);
    doc.adicionar_geometria(Geometria::Texto { ponto: Ponto::novo(10.0, 30.0), altura: 3.0, rotacao: 0.0, conteudo: "\u{0394} = 60\u{00b0}, \u{03b8}s \u{2192} R".into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
    let pdf = exportar_folhas(&doc, &Opcoes::default()).unwrap();
    assert!(contem(&pdf, b"/BaseFont /Symbol"));
    assert!(contem(&pdf, b"/F2 ") && contem(&pdf, b"(D) Tj"), "o delta vai na fonte Symbol");
    assert!(contem(&pdf, b"(q) Tj") && contem(&pdf, b"(\xAE) Tj"), "theta e a seta também");
    assert!(contem(&pdf, b" = 60\xB0, ") && !contem(&pdf, b"(? = 60"), "o resto segue em Helvetica, com o grau");
}

#[test]
fn no_papel_cinza_claro_vira_preto_e_amarelo_escurece_sem_perder_o_tom() {
    let mut doc = Documento::default().com_unidade(Some(Unidade::Centimetro));
    let linha = |y: f64, cor: u32| Mudanca::Novo(NovaEntidade { camada: 0, cor: Some(cor), geometria: Geometria::Linha { a: Ponto::novo(0.0, y), b: Ponto::novo(100.0, y) }, props: Propriedades::default() });
    doc.aplicar_mudancas(vec![linha(0.0, 0xC8C8C8), linha(10.0, 0xFFD84A)]);
    let papel = String::from_utf8_lossy(&exportar_folhas(&doc, &Opcoes::default()).unwrap()).into_owned();
    assert!(papel.contains("0 0 0 RG"), "cinza claro impresso em preto");
    assert!(!papel.contains("1 0.847 0.29 RG"), "o amarelo da tela não vai igual para o papel");
    let escuro = papel.lines().find(|l| l.ends_with(" RG 0 w") && !l.starts_with("0 0 0")).expect("traço colorido");
    let v: Vec<f64> = escuro.split_whitespace().take(3).map(|x| x.parse().unwrap()).collect();
    assert!(v[0] > v[2] * 2.0 && 0.3 * v[0] + 0.6 * v[1] + 0.1 * v[2] < 0.5, "continua amarelo (vermelho e verde altos, azul baixo) e mais escuro: {v:?}");
    let tela = String::from_utf8_lossy(&exportar_folhas(&doc, &Opcoes { modo_papel: false, ..Opcoes::default() }).unwrap()).into_owned();
    assert!(tela.contains("0.784 0.784 0.784 RG"), "com as cores da tela nada muda");
}

#[test]
fn a_linha_fina_e_fio_de_cabelo_para_o_zoom_nao_engrossar() {
    let pdf = exportar_folhas(&com_folhas(1), &Opcoes::default()).unwrap();
    assert!(contem(&pdf, b" RG 0 w"), "1 px de espessura = `0 w`: 1 pixel da tela em qualquer zoom");
}

#[test]
fn circulo_pequeno_sai_redondo_e_nao_como_octogono() {
    let mut doc = Documento::default().com_unidade(Some(Unidade::Centimetro));
    doc.adicionar_geometria(Geometria::Circulo { centro: Ponto::novo(50.0, 50.0), raio: 2.0 });
    doc.adicionar_geometria(Geometria::Linha { a: Ponto::novo(0.0, 0.0), b: Ponto::novo(400.0, 400.0) });
    let pdf = String::from_utf8_lossy(&exportar_folhas(&doc, &Opcoes::default()).unwrap()).into_owned();
    let cordas = pdf.lines().filter(|l| l.ends_with(" l") && l.contains(" m ")).count();
    assert!(cordas >= 20, "o círculo (6 pt de raio) tem que ter pelo menos 20 cordas (antes eram 8, um octógono), tem {cordas}");
}

#[test]
fn miniatura_e_svg_com_tracos_e_desenho_vazio_nao_tem() {
    let svg = feathercad_io::pdf::miniatura_svg(&com_folhas(1), 320.0, 200.0).expect("miniatura");
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>") && svg.contains("<path"), "{svg}");
    assert!(feathercad_io::pdf::miniatura_svg(&Documento::default(), 320.0, 200.0).is_none());
}

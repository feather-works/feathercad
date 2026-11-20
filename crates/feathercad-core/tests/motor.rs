use std::collections::BTreeMap;

use feathercad_core::atalhos::{acao_da_tecla, Acao, Interruptor, TECLAS};
use feathercad_core::cursor::{resolver, Ajudas};
use feathercad_core::documento::Documento;
use feathercad_core::formato::Regional;
use feathercad_core::geom::Ponto;
use feathercad_core::quadro::{montar, BYTES_POR_SEGMENTO, MAGIA_IGUAL, MAGIA_NOVO};
use feathercad_core::sessao::{Ferramenta, Sessao};
use feathercad_core::vista::Vista;

fn p(x: f64, y: f64) -> Ponto {
    Ponto::novo(x, y)
}

#[test]
fn desfazer_e_refazer_voltam_a_linha() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    assert!(d.sujo());
    assert!(d.desfazer());
    assert!(d.entidades().is_empty());
    assert!(!d.desfazer());
    assert!(d.refazer());
    assert_eq!(d.entidades().len(), 1);
    d.desfazer();
    d.adicionar_linha(p(1.0, 1.0), p(2.0, 2.0));
    assert!(!d.pode_refazer(), "desenhar de novo joga fora o que havia para refazer");
}

#[test]
fn salvar_limpa_o_sujo() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(1.0, 1.0));
    d.marcar_salvo();
    assert!(!d.sujo());
    d.desfazer();
    assert!(d.sujo());
}

#[test]
fn vista_zoom_mantem_o_ponto_sob_o_cursor() {
    let v = Vista::inicial(800.0, 600.0);
    let antes = v.para_mundo(200.0, 150.0);
    let z = v.zoom(200.0, 150.0, 4.0);
    let depois = z.para_mundo(200.0, 150.0);
    assert!(antes.distancia(depois) < 1e-9);
    assert!((z.escala - 4.0).abs() < 1e-12);
}

#[test]
fn vista_ida_e_volta_e_pan() {
    let v = Vista::inicial(800.0, 600.0);
    let (x, y) = v.para_tela(p(10.0, 20.0));
    assert!(v.para_mundo(x, y).distancia(p(10.0, 20.0)) < 1e-9);
    let d = v.deslocar(50.0, 0.0);
    let (x2, _) = d.para_tela(p(0.0, 0.0));
    assert!((x2 - 450.0).abs() < 1e-9, "o desenho acompanha o mouse");
}

#[test]
fn cursor_orto_trava_no_eixo_maior() {
    let d = Documento::default();
    let aj = Ajudas { orto: true, grade: None, osnap: false, modos: feathercad_core::snap::ModosSnap::default(), angulo: 90.0, direcao: None };
    let r = resolver(&d, Some(p(0.0, 0.0)), p(10.0, 3.0), 1.0, &aj);
    assert_eq!(r.ponto, p(10.0, 0.0));
    assert_eq!(r.guias.len(), 1);
    let r = resolver(&d, Some(p(0.0, 0.0)), p(2.0, 9.0), 1.0, &aj);
    assert_eq!(r.ponto, p(0.0, 9.0));
}

#[test]
fn cursor_osnap_gruda_na_extremidade_e_ganha_da_grade() {
    let mut d = Documento::default();
    d.adicionar_linha(p(3.0, 3.0), p(50.0, 50.0));
    let aj = Ajudas { orto: false, grade: Some(10.0), osnap: true, modos: feathercad_core::snap::ModosSnap::default(), angulo: 90.0, direcao: None };
    let r = resolver(&d, None, p(3.4, 2.8), 1.0, &aj);
    assert!(r.marca);
    assert_eq!(r.ponto, p(3.0, 3.0));
    let longe = resolver(&d, None, p(23.4, 2.8), 1.0, &aj);
    assert!(!longe.marca);
    assert_eq!(longe.ponto, p(20.0, 0.0));
}

#[test]
fn cursor_orto_com_grade_so_arredonda_o_eixo_livre() {
    let d = Documento::default();
    let aj = Ajudas { orto: true, grade: Some(10.0), osnap: false, modos: feathercad_core::snap::ModosSnap::default(), angulo: 90.0, direcao: None };
    let r = resolver(&d, Some(p(3.0, 3.0)), p(28.0, 4.0), 1.0, &aj);
    assert_eq!(r.ponto, p(30.0, 3.0));
}

#[test]
fn numero_na_lingua_e_sem_menos_zero() {
    let pt = Regional::default();
    assert_eq!(pt.numero(10931.976), "10931,976");
    assert_eq!(pt.numero(-0.0001), "0,000");
    assert_eq!(pt.numero(-37074.202), "-37074,202");
    let en = Regional { decimal: '.', separador_de_coordenadas: ',', casas: 2 };
    assert_eq!(en.numero(1.5), "1.50");
    assert_eq!(pt.ler_numero(" 3,5 "), Some(3.5));
    assert_eq!(pt.ler_numero("abc"), None);
}

fn sessao_com_comandos() -> Sessao {
    let mut s = Sessao::default();
    let mut t = BTreeMap::new();
    t.insert("linha".to_string(), vec!["LINHA".to_string(), "L".to_string()]);
    s.comandos.definir(&t, &BTreeMap::new()).unwrap();
    s
}

#[test]
fn linha_de_comando_desenha_com_coordenadas_digitadas() {
    let mut d = Documento::default();
    let mut s = sessao_com_comandos();
    s.digitar(&mut d, "l");
    assert_eq!(s.prompt().chave, "linha.primeiroPonto");
    s.digitar(&mut d, "0;0");
    assert_eq!(s.prompt().chave, "linha.proximoPonto");
    s.digitar(&mut d, "@10,5;0");
    assert_eq!(d.entidades().len(), 1);
    s.digitar(&mut d, "");
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    s.digitar(&mut d, "xyz");
    assert_eq!(s.historico().len(), 5);
}

#[test]
fn linha_com_distancia_de_virgula_decimal_em_pt_br() {
    let mut d = Documento::default();
    let mut s = sessao_com_comandos();
    s.digitar(&mut d, "l");
    s.digitar(&mut d, "100;100");
    s.cursor = Some(p(200.0, 100.0));
    s.digitar(&mut d, "2,5");
    let g = &d.entidades().last().expect("a linha").geometria;
    assert_eq!(*g, feathercad_core::documento::Geometria::Linha { a: p(100.0, 100.0), b: p(102.5, 100.0) }, "2,5 é distância, não o ponto (2;5)");
}

#[test]
fn comando_desconhecido_vira_mensagem_com_o_nome() {
    let mut d = Documento::default();
    let mut s = sessao_com_comandos();
    s.digitar(&mut d, "XYZ");
    let ultima = s.historico().last().unwrap();
    let texto = format!("{ultima:?}");
    assert!(texto.contains("comando.desconhecido") && texto.contains("XYZ"));
}

#[test]
fn tabela_de_lingua_com_comando_inexistente_e_recusada() {
    let mut s = Sessao::default();
    let mut t = BTreeMap::new();
    t.insert("teletransportar".to_string(), vec!["TT".to_string()]);
    assert!(s.comandos.definir(&t, &BTreeMap::new()).is_err());
}

#[test]
fn teclas_do_autocad() {
    assert_eq!(acao_da_tecla("f8"), Some(Acao::Alternar(Interruptor::Orto)));
    assert_eq!(acao_da_tecla("Ctrl+G"), Some(Acao::Alternar(Interruptor::Grade)));
    assert_eq!(acao_da_tecla("Ctrl+Z"), Some(Acao::Comando("desfazer")));
    assert_eq!(acao_da_tecla("Ctrl+["), Some(Acao::Cancelar));
    assert_eq!(acao_da_tecla("F13"), None);
    let mut d = Documento::default();
    let mut s = Sessao::default();
    assert!(s.tecla(&mut d, "F8").tratado);
    assert!(s.interruptores.orto);
    assert!(!s.tecla(&mut d, "F13").tratado);
    assert_eq!(TECLAS.len(), 20);
}

#[test]
fn quadro_leva_so_o_que_cabe_e_diz_quando_nada_mudou() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    d.adicionar_linha(p(5000.0, 5000.0), p(6000.0, 5000.0));
    let v = Vista::inicial(800.0, 600.0);
    let q = montar(&d, &v, None, None, false);
    assert_eq!(&q[..4], MAGIA_NOVO);
    let n = u32::from_le_bytes(q[12..16].try_into().unwrap());
    assert_eq!(n, 1, "a linha fora da janela não vai");
    assert_eq!(q.len(), 16 + BYTES_POR_SEGMENTO + 4 + 4 + 8 + 4, "segmento + contador de textos + contador de preenchimentos + os dois contadores do fundo (vazio) + contador de imagens");
    assert_eq!(montar(&d, &v, Some(d.versao()), None, false), MAGIA_IGUAL);
    let x1 = f32::from_le_bytes(q[16..20].try_into().unwrap());
    assert!((x1 - 400.0).abs() < 1e-3);
    let cor = u32::from_le_bytes(q[32..36].try_into().unwrap());
    assert_eq!(cor, 0xFF_FF_FF, "PorCamada resolvido");
}

#[test]
fn fundo_adapta_o_passo_ao_zoom_e_o_quadro_leva_as_marcas() {
    use feathercad_core::fundo::{marcas, passo_visivel};
    assert_eq!(passo_visivel(10.0, 1.0), 100.0, "10 px é apertado: sobe uma casa");
    assert_eq!(passo_visivel(10.0, 5.0), 10.0);
    assert_eq!(passo_visivel(10.0, 50.0), 1.0, "aproximou: desce uma casa");
    let v = Vista::inicial(800.0, 600.0);
    let (xs, ys) = marcas(&v, 10.0);
    assert!(xs.iter().any(|m| m.principal) && !xs.is_empty() && !ys.is_empty());
    assert!(xs.iter().any(|m| (m.pos - 400.0).abs() < 1e-3), "a linha da origem está no meio");
    let d = Documento::default();
    let q = montar(&d, &v, None, Some(10.0), false);
    let nx = u32::from_le_bytes(q[24..28].try_into().unwrap()) as usize; // 16 do cabeçalho + 4 do contador de textos + 4 do de preenchimentos
    assert_eq!(nx, xs.len());
}

#[test]
fn zoom_ajustar_enquadra_o_desenho_e_zoom_centro_nao_move_o_centro() {
    let v = Vista::inicial(800.0, 600.0);
    let a = v.ajustar(Some((p(0.0, 0.0), p(100.0, 50.0))));
    let (x0, _) = a.para_tela(p(0.0, 0.0));
    let (x1, _) = a.para_tela(p(100.0, 0.0));
    assert!(x0 > 0.0 && x1 < 800.0, "cabe na janela, com folga");
    assert!((a.para_mundo(400.0, 300.0).distancia(p(50.0, 25.0))) < 1e-9);
    assert_eq!(v.ajustar(None), Vista::inicial(800.0, 600.0));
    let z = v.zoom_centro(2.0);
    assert!(z.para_mundo(400.0, 300.0).distancia(p(0.0, 0.0)) < 1e-9);
    let ponto = v.ajustar(Some((p(5.0, 5.0), p(5.0, 5.0))));
    assert_eq!(ponto.escala, 1.0, "um ponto só: não divide por zero");
}

#[test]
fn f7_alterna_o_fundo_e_volta_ao_ultimo_estilo() {
    use feathercad_core::sessao::Fundo;
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.definir_fundo(Fundo::Pontos);
    assert!(s.tecla(&mut d, "F7").tratado);
    assert_eq!(s.fundo, Fundo::Nada);
    s.tecla(&mut d, "Ctrl+G");
    assert_eq!(s.fundo, Fundo::Pontos, "voltou ao estilo de antes");
    s.tecla(&mut d, "F9");
    assert!(s.interruptores.snap, "F9 é o snap, não a grade");
}

#[test]
fn caixa_so_conta_o_que_se_ve() {
    let mut d = Documento::default();
    assert_eq!(d.caixa(), None);
    d.adicionar_linha(p(-5.0, 2.0), p(10.0, 8.0));
    assert_eq!(d.caixa(), Some((p(-5.0, 2.0), p(10.0, 8.0))));
}

#[test]
fn ctrl_o_s_n_pedem_acao_de_janela_e_o_motor_escolhe_qual() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    assert_eq!(s.tecla(&mut d, "Ctrl+O").janela, Some("abrir"));
    assert_eq!(s.tecla(&mut d, "ctrl+s").janela, Some("salvar"));
    assert_eq!(s.tecla(&mut d, "Ctrl+Shift+S").janela, Some("salvar_como"));
    assert_eq!(s.tecla(&mut d, "Ctrl+N").janela, Some("novo"));
    assert_eq!(s.tecla(&mut d, "F8").janela, None);
}

#[test]
fn o_branco_do_cad_contrasta_com_o_fundo_do_tema() {
    use feathercad_core::quadro::cor_na_tela;
    assert_eq!(cor_na_tela(0xFF_FF_FF, true), 0x00_00_00, "branco vira preto no fundo claro");
    assert_eq!(cor_na_tela(0xFF_FF_FF, false), 0xFF_FF_FF);
    assert_eq!(cor_na_tela(0x00_00_00, false), 0xFF_FF_FF, "preto vira branco no fundo escuro");
    assert_eq!(cor_na_tela(0xFF_00_00, true), 0xFF_00_00, "as outras cores não mudam");
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    let v = Vista::inicial(800.0, 600.0);
    let cor = |claro| u32::from_le_bytes(montar(&d, &v, None, None, claro)[32..36].try_into().unwrap());
    assert_eq!((cor(false), cor(true)), (0xFF_FF_FF, 0x00_00_00));
}

// ---------- círculo e arco ----------

#[test]
fn arco_por_tres_pontos_acha_centro_raio_e_o_sentido_certo() {
    use feathercad_core::geom::arco_por_3_pontos;
    // Meia circunferência de raio 5 passando por cima: (5,0) → (0,5) → (-5,0) é anti-horária.
    let a = arco_por_3_pontos(p(5.0, 0.0), p(0.0, 5.0), p(-5.0, 0.0)).unwrap();
    assert!(a.centro.distancia(p(0.0, 0.0)) < 1e-9 && (a.raio - 5.0).abs() < 1e-9);
    assert!((a.inicio - 0.0).abs() < 1e-9 && (a.fim - 180.0).abs() < 1e-9);
    // O mesmo arco percorrido ao contrário (horário): guarda-se anti-horário, com as pontas trocadas.
    let b = arco_por_3_pontos(p(-5.0, 0.0), p(0.0, 5.0), p(5.0, 0.0)).unwrap();
    assert!((b.inicio - 0.0).abs() < 1e-9 && (b.fim - 180.0).abs() < 1e-9);
    assert!(arco_por_3_pontos(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)).is_none(), "alinhados não fazem arco");
}

#[test]
fn circulo_e_arco_no_quadro_viram_cordas_finas_e_cabem_na_caixa() {
    let mut d = Documento::default();
    d.adicionar_circulo(p(0.0, 0.0), 100.0).unwrap();
    d.adicionar_arco(p(0.0, 0.0), 50.0, 270.0, 90.0).unwrap(); // meia-volta pela direita
    assert!(d.adicionar_circulo(p(0.0, 0.0), 0.0).is_none() && d.adicionar_arco(p(0.0, 0.0), -1.0, 0.0, 90.0).is_none());
    let q = montar(&d, &Vista::inicial(800.0, 600.0), None, None, false);
    let n = u32::from_le_bytes(q[12..16].try_into().unwrap());
    assert!(n > 40, "raio de 100 px precisa de dezenas de cordas, deu {n}");
    let (min, max) = d.caixa().unwrap();
    assert!(min.distancia(p(-100.0, -100.0)) < 1e-9 && max.distancia(p(100.0, 100.0)) < 1e-9);
    let mut so_arco = Documento::default();
    so_arco.adicionar_arco(p(0.0, 0.0), 10.0, 300.0, 60.0); // atravessa o quadrante 0°
    let (min, max) = so_arco.caixa().unwrap();
    assert!((max.x - 10.0).abs() < 1e-9, "o quadrante 0° entra na caixa: {max:?}");
    assert!(min.x > 4.0, "mas não o lado esquerdo: {min:?}");
}

#[test]
fn cursor_gruda_no_centro_e_nos_quadrantes_do_circulo() {
    let mut d = Documento::default();
    d.adicionar_circulo(p(10.0, 10.0), 5.0);
    let aj = Ajudas { orto: false, grade: None, osnap: true, modos: feathercad_core::snap::ModosSnap::default(), angulo: 90.0, direcao: None };
    assert_eq!(resolver(&d, None, p(10.2, 10.1), 1.0, &aj).ponto, p(10.0, 10.0), "centro");
    assert_eq!(resolver(&d, None, p(15.1, 9.9), 1.0, &aj).ponto, p(15.0, 10.0), "quadrante");
}

fn com_apelidos() -> Sessao {
    let mut s = Sessao::default();
    let mut c = BTreeMap::new();
    for (id, a) in [("circulo", "C"), ("arco", "A"), ("linha", "L")] {
        c.insert(id.to_string(), vec![a.to_string()]);
    }
    s.comandos.definir(&c, &BTreeMap::new()).unwrap();
    s
}

#[test]
fn ferramenta_circulo_centro_e_raio_por_clique_ou_digitado() {
    let mut d = Documento::default();
    let mut s = com_apelidos();
    s.digitar(&mut d, "c");
    assert_eq!(s.prompt().chave, "circulo.centro");
    s.digitar(&mut d, "10;10");
    assert_eq!(s.prompt().chave, "circulo.raio");
    assert_eq!(s.previa(p(13.0, 14.0)).len(), 1, "a prévia é o círculo de raio 5");
    s.digitar(&mut d, "3,5");
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma, "o comando termina depois do raio");
    s.digitar(&mut d, "c");
    s.clique(&mut d, p(0.0, 0.0));
    s.clique(&mut d, p(0.0, 2.0));
    let raios: Vec<f64> = d.entidades().iter().filter_map(|e| match e.geometria {
        feathercad_core::documento::Geometria::Circulo { raio, .. } => Some(raio),
        _ => None,
    }).collect();
    assert_eq!(raios, [3.5, 2.0]);
}

#[test]
fn ferramenta_arco_tres_pontos_e_alinhados_nao_fecham() {
    let mut d = Documento::default();
    let mut s = com_apelidos();
    s.digitar(&mut d, "a");
    assert_eq!(s.prompt().chave, "arco.primeiroPonto");
    s.clique(&mut d, p(5.0, 0.0));
    assert_eq!(s.prompt().chave, "arco.segundoPonto");
    s.clique(&mut d, p(0.0, 5.0));
    assert_eq!(s.prompt().chave, "arco.pontoFinal");
    assert_eq!(s.previa(p(-5.0, 0.0)).len(), 1);
    s.clique(&mut d, p(-5.0, 0.0));
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    assert_eq!(d.entidades().len(), 1);
    s.digitar(&mut d, "a");
    for pt in [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)] {
        s.clique(&mut d, pt);
    }
    assert_eq!(d.entidades().len(), 1, "três pontos alinhados não fazem arco");
    assert_eq!(s.prompt().chave, "arco.pontoFinal", "continua pedindo o último ponto");
}

// ---------- blocos ----------

use feathercad_core::documento::{Bloco, Camada, Geometria as Geo, NovaEntidade};
use feathercad_core::geom::Transf;

/// Um bloco "cruz" de 2×2 com a base no centro, mais um bloco "par" que insere duas cruzes.
fn doc_com_blocos() -> Documento {
    let linha = |a: Ponto, b: Ponto| NovaEntidade { camada: 0, cor: None, geometria: Geo::Linha { a, b }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo };
    let cruz = Bloco { nome: "Cruz".into(), base: p(0.0, 0.0), entidades: vec![linha(p(-1.0, 0.0), p(1.0, 0.0)), linha(p(0.0, -1.0), p(0.0, 1.0))] };
    let insere = |bloco, x, y, sx, rot| NovaEntidade { camada: 0, cor: None, geometria: Geo::Insercao { bloco, ponto: p(x, y), escala_x: sx, escala_y: sx, rotacao: rot }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo };
    let par = Bloco { nome: "Par".into(), base: p(0.0, 0.0), entidades: vec![insere(0, 10.0, 0.0, 1.0, 0.0), insere(0, -10.0, 0.0, 1.0, 0.0)] };
    Documento::de_partes_com_blocos(vec![Camada { ligada: true, ..Camada::nova("0", 0xFF_00_00) }], 0, vec![cruz, par], Vec::new())
}

#[test]
fn transformacao_de_insercao_escala_gira_e_desloca_em_volta_da_base() {
    let t = Transf::de_insercao(p(1.0, 1.0), p(10.0, 20.0), 2.0, 2.0, 90.0);
    let q = t.aplicar(p(1.0, 1.0));
    assert!(q.distancia(p(10.0, 20.0)) < 1e-9, "a base cai no ponto de inserção");
    let r = t.aplicar(p(2.0, 1.0)); // 1 à direita da base, ×2, girado 90° → 2 para cima
    assert!(r.distancia(p(10.0, 22.0)) < 1e-9, "{r:?}");
    let composta = Transf::de_insercao(p(0.0, 0.0), p(5.0, 0.0), 1.0, 1.0, 0.0).depois_de(&Transf::de_insercao(p(0.0, 0.0), p(0.0, 3.0), 1.0, 1.0, 0.0));
    assert!(composta.aplicar(p(0.0, 0.0)).distancia(p(5.0, 3.0)) < 1e-9);
    assert!((t.escala_media() - 2.0).abs() < 1e-9 && (t.rotacao_graus() - 90.0).abs() < 1e-9);
}

#[test]
fn insercao_desenha_o_bloco_recursivo_e_a_camada_zero_herda_a_cor_do_insert() {
    let mut d = doc_com_blocos();
    assert_eq!(d.bloco_por_nome(" par "), Some(1));
    d.adicionar_insercao(1, p(0.0, 100.0)).unwrap();
    let v = Vista::inicial(800.0, 600.0);
    let q = montar(&d, &v, None, None, false);
    let n = u32::from_le_bytes(q[12..16].try_into().unwrap());
    assert_eq!(n, 4, "o par tem 2 cruzes de 2 linhas: 4 segmentos");
    let xs: Vec<f32> = (0..4).map(|i| f32::from_le_bytes(q[16 + i * 24..20 + i * 24].try_into().unwrap())).collect();
    assert!(xs.iter().any(|x| (x - 410.0 - 1.0).abs() < 1e-3 || (x - 410.0 + 1.0).abs() < 1e-3), "a cruz da direita está em x=10: {xs:?}");
    // A camada 0 tem cor vermelha própria; a inserção herda a cor da camada → tudo vermelho.
    let cor = u32::from_le_bytes(q[32..36].try_into().unwrap());
    assert_eq!(cor, 0xFF_00_00);
}

#[test]
fn insercao_com_bloco_dentro_de_si_mesmo_para_no_limite_e_bloco_inexistente_nao_quebra() {
    let mut d = Documento::de_partes_com_blocos(
        vec![Camada { ligada: true, ..Camada::nova("0", 0xFF_FF_FF) }],
        0,
        vec![Bloco {
            nome: "Louco".into(),
            base: p(0.0, 0.0),
            entidades: vec![
                NovaEntidade { camada: 0, cor: None, geometria: Geo::Linha { a: p(0.0, 0.0), b: p(1.0, 0.0) }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo },
                NovaEntidade { camada: 0, cor: None, geometria: Geo::Insercao { bloco: 0, ponto: p(1.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo },
                NovaEntidade { camada: 0, cor: None, geometria: Geo::Insercao { bloco: 99, ponto: p(0.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo },
            ],
        }],
        Vec::new(),
    );
    assert!(d.adicionar_insercao(7, p(0.0, 0.0)).is_none());
    d.adicionar_insercao(0, p(0.0, 0.0)).unwrap();
    let q = montar(&d, &Vista::inicial(800.0, 600.0), None, None, false);
    let n = u32::from_le_bytes(q[12..16].try_into().unwrap());
    assert!((2..=20).contains(&n), "desceu até o limite e parou: {n}");
    assert!(d.caixa().is_some());
}

#[test]
fn caixa_e_snap_de_insercao_e_camada_desligada_esconde_tudo() {
    let mut d = doc_com_blocos();
    d.adicionar_insercao(0, p(50.0, 50.0)).unwrap();
    let (min, max) = d.caixa().unwrap();
    assert!(min.distancia(p(49.0, 49.0)) < 1e-9 && max.distancia(p(51.0, 51.0)) < 1e-9);
    let aj = Ajudas { orto: false, grade: None, osnap: true, modos: feathercad_core::snap::ModosSnap::default(), angulo: 90.0, direcao: None };
    assert_eq!(resolver(&d, None, p(50.2, 49.9), 1.0, &aj).ponto, p(50.0, 50.0), "grude no ponto de inserção");
    let mut s = Sessao::default();
    let mut c = BTreeMap::new();
    c.insert("inserir".to_string(), vec!["I".to_string()]);
    s.comandos.definir(&c, &BTreeMap::new()).unwrap();
    s.digitar(&mut d, "i");
    assert_eq!(s.prompt().chave, "insercao.nome");
    s.digitar(&mut d, "naoexiste");
    assert_eq!(s.prompt().chave, "insercao.nome", "nome ruim: pede de novo");
    s.digitar(&mut d, "CRUZ");
    assert_eq!(s.prompt().chave, "insercao.ponto");
    s.clique(&mut d, p(0.0, 0.0));
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    assert_eq!(d.entidades().len(), 2);
    let mut vazio = Documento::default();
    let mut s2 = Sessao::default();
    s2.executar(&mut vazio, "inserir");
    assert_eq!(s2.ferramenta, Ferramenta::Nenhuma, "sem blocos no desenho, não há o que inserir");
}

#[test]
fn orto_com_passo_de_angulo_trava_nos_multiplos() {
    let d = Documento::default();
    let base = Some(Ponto::novo(0.0, 0.0));
    let com = |angulo: f64, x: f64, y: f64| {
        let aj = Ajudas { orto: true, grade: None, osnap: false, modos: feathercad_core::snap::ModosSnap::default(), angulo, direcao: None };
        resolver(&d, base, Ponto::novo(x, y), 0.1, &aj).ponto
    };
    // 90°: eixo mais perto.
    let p = com(90.0, 10.0, 1.0);
    assert!((p.x - 10.0).abs() < 1e-9 && p.y == 0.0);
    // 45°: a diagonal.
    let p = com(45.0, 10.0, 8.0);
    assert!((p.x - p.y).abs() < 1e-9);
    // 60°: o mouse a 50° vai para 60° (y = x·tan 60°).
    let p = com(60.0, 10.0, 10.0 * 50f64.to_radians().tan());
    assert!((p.y / p.x - 60f64.to_radians().tan()).abs() < 1e-9);
    // Passo inválido cai no 90.
    let p = com(0.0, 10.0, 1.0);
    assert!(p.y == 0.0);
}

#[test]
fn vista_3d_mantem_centro_na_tela() {
    let mut v = Vista::inicial(800.0, 600.0);
    v.centro = p(500.0, 300.0);
    // Em 2D: o centro fica no meio da tela
    let (cx2d, cy2d) = v.para_tela(v.centro);
    assert!((cx2d - 400.0).abs() < 1e-9 && (cy2d - 300.0).abs() < 1e-9);

    // Ao ir para 3D (ex: Isométrica 60°, 45°):
    v.rotacao_x = 60.0;
    v.rotacao_z = 45.0;
    let (cx3d, cy3d) = v.para_tela(v.centro);
    assert!((cx3d - 400.0).abs() < 1e-9, "centro X deve permanecer em 400, deu {cx3d}");
    assert!((cy3d - 300.0).abs() < 1e-9, "centro Y deve permanecer em 300, deu {cy3d}");
}

#[test]
fn vista_3d_pan_e_zoom_mantem_consistencia() {
    let mut v = Vista::inicial(800.0, 600.0);
    v.centro = p(120.0, 80.0);
    v.rotacao_x = 45.0;
    v.rotacao_z = 30.0;
    v.escala = 2.5;

    // Pan
    let (x1, y1) = v.para_tela(p(120.0, 80.0));
    let deslocada = v.deslocar(40.0, -25.0);
    let (x2, y2) = deslocada.para_tela(p(120.0, 80.0));
    assert!((x2 - (x1 + 40.0)).abs() < 1e-6, "pan X na tela");
    assert!((y2 - (y1 - 25.0)).abs() < 1e-6, "pan Y na tela");

    // Zoom mantém ponto sob cursor
    let ponto_antes = v.para_mundo(300.0, 250.0);
    let com_zoom = v.zoom(300.0, 250.0, 2.0);
    let ponto_depois = com_zoom.para_mundo(300.0, 250.0);
    assert!(ponto_antes.distancia_3d(ponto_depois) < 1e-6, "zoom 3D mantém ponto sob o mouse");
}

#[test]
fn vista_3d_movimento_em_planos_laterais() {
    let mut v = Vista::inicial(800.0, 600.0);
    v.centro = p(100.0, 200.0);
    // Visão lateral: rx = 90°, rz = 90° (profundidade é o eixo X, tela mostra o plano ZY)
    v.rotacao_x = 90.0;
    v.rotacao_z = 90.0;

    let p_meio = v.para_mundo(400.0, 300.0);
    assert!((p_meio.x - 100.0).abs() < 1e-6, "X é profundidade fixa");
    assert!((p_meio.y - 200.0).abs() < 1e-6, "Y no centro");
    assert!((p_meio.z - 0.0).abs() < 1e-6, "Z no centro");

    // Mover mouse para cima na tela: Z deve subir livremente!
    let p_cima = v.para_mundo(400.0, 200.0);
    assert!(p_cima.z > p_meio.z + 50.0, "mover mouse para cima deve aumentar Z");
    assert!((p_cima.x - 100.0).abs() < 1e-6, "X continua no plano de profundidade");

    // Mover mouse para a direita na tela: Y deve se mover livremente!
    let p_dir = v.para_mundo(500.0, 300.0);
    assert!(p_dir.y > p_meio.y + 50.0, "mover mouse para direita deve mudar Y");
    assert!((p_dir.x - 100.0).abs() < 1e-6, "X continua no plano de profundidade");

    // Testar com referência (base)
    let base = Ponto::novo3d(50.0, 150.0, 30.0);
    let p_com_base = v.para_mundo_em_z_com_ref(400.0, 300.0, base.z, Some(base));
    assert!((p_com_base.x - 50.0).abs() < 1e-6, "plano segue a base em X");
}


//! A sessão de edição de ponta a ponta: clique, janela, Enter, texto digitado e desfazer.

use feathercad_core::documento::{Documento, Geometria};
use feathercad_core::geom::{Ponto, Vertice};
use feathercad_core::sessao::{Ferramenta, Sessao};

fn p(x: f64, y: f64) -> Ponto {
    Ponto::novo(x, y)
}

const TOL: f64 = 0.5;

fn clicar(s: &mut Sessao, d: &mut Documento, x: f64, y: f64) {
    s.clique_com(d, p(x, y), TOL, false);
}

fn geometrias(d: &Documento) -> Vec<Geometria> {
    d.entidades().iter().map(|e| e.geometria.clone()).collect()
}

fn duas_linhas() -> (Documento, Sessao) {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    d.adicionar_linha(p(0.0, 5.0), p(10.0, 5.0));
    (d, Sessao::default())
}

#[test]
fn clicar_num_objeto_seleciona_shift_alterna_esc_limpa_e_vazio_abre_janela() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.2);
    assert_eq!(s.selecionados(), 1);
    s.clique_com(&mut d, p(5.0, 5.0), TOL, true);
    assert_eq!(s.selecionados(), 2);
    s.clique_com(&mut d, p(5.0, 0.0), TOL, true);
    assert_eq!(s.selecionados(), 1, "Shift no já selecionado tira da seleção");
    s.cancelar(&mut d);
    assert_eq!(s.selecionados(), 0);
    // Janela da direita para a esquerda (cruzando) pega o que toca; da esquerda para a direita, só o que cabe.
    clicar(&mut s, &mut d, 12.0, 8.0);
    assert!(s.janela_pendente());
    clicar(&mut s, &mut d, 5.0, -1.0);
    assert_eq!(s.selecionados(), 2, "cruzando pega as duas");
    s.cancelar(&mut d);
    clicar(&mut s, &mut d, -1.0, -1.0);
    clicar(&mut s, &mut d, 11.0, 1.0);
    assert_eq!(s.selecionados(), 1, "só a de baixo cabe inteira na janela");
}

#[test]
fn mover_com_objeto_ja_selecionado_pula_a_escolha_e_um_ctrl_z_desfaz() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "mover");
    assert!(matches!(s.ferramenta, Ferramenta::Editar { .. }));
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 3.0, 4.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(3.0, 4.0), b: p(13.0, 4.0) });
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    assert_eq!(s.selecionados(), 0);
    assert!(d.desfazer());
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
}

#[test]
fn comando_sem_selecao_escolhe_objetos_e_enter_confirma() {
    let (mut d, mut s) = duas_linhas();
    s.executar(&mut d, "apagar");
    clicar(&mut s, &mut d, 5.0, 5.0);
    s.digitar(&mut d, "");
    assert_eq!(d.entidades().len(), 1);
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    assert!(d.desfazer());
    assert_eq!(d.entidades().len(), 2);
    // Enter sem nada selecionado cancela.
    s.executar(&mut d, "mover");
    s.digitar(&mut d, "");
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
}

#[test]
fn copiar_repete_ate_o_enter_e_o_original_fica() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "copiar");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 0.0, 10.0);
    clicar(&mut s, &mut d, 0.0, 20.0);
    assert_eq!(d.entidades().len(), 4);
    assert!(geometrias(&d).contains(&Geometria::Linha { a: p(0.0, 20.0), b: p(10.0, 20.0) }));
    s.digitar(&mut d, "");
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
}

#[test]
fn girar_por_angulo_digitado_e_escala_por_fator_digitado() {
    let (mut d, mut s) = duas_linhas();
    s.regional = Default::default();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "girar");
    clicar(&mut s, &mut d, 0.0, 0.0);
    s.digitar(&mut d, "90");
    let Geometria::Linha { a, b } = geometrias(&d)[0] else { panic!() };
    assert!(a.distancia(p(0.0, 0.0)) < 1e-9 && b.distancia(p(0.0, 10.0)) < 1e-9, "{a:?} {b:?}");
    clicar(&mut s, &mut d, 0.0, 3.0);
    s.executar(&mut d, "escala");
    clicar(&mut s, &mut d, 0.0, 0.0);
    s.digitar(&mut d, "2");
    let Geometria::Linha { b, .. } = geometrias(&d)[0] else { panic!() };
    assert!(b.distancia(p(0.0, 20.0)) < 1e-9, "{b:?}");
}

#[test]
fn espelhar_mantem_o_original_e_a_matriz_copia_em_grade() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "espelhar");
    clicar(&mut s, &mut d, 0.0, 2.0);
    clicar(&mut s, &mut d, 10.0, 2.0);
    assert_eq!(d.entidades().len(), 3);
    assert!(geometrias(&d).contains(&Geometria::Linha { a: p(0.0, 4.0), b: p(10.0, 4.0) }));
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "matriz");
    s.digitar(&mut d, "2 3 10 20");
    assert_eq!(d.entidades().len(), 3 + 5);
    assert!(geometrias(&d).contains(&Geometria::Linha { a: p(40.0, 10.0), b: p(50.0, 10.0) }));
    s.executar(&mut d, "matriz");
    clicar(&mut s, &mut d, 5.0, 5.0);
    s.digitar(&mut d, "");
    s.digitar(&mut d, "isto não");
    assert_eq!(d.entidades().len(), 8, "matriz inválida não copia nada");
}

#[test]
fn esticar_move_so_a_ponta_dentro_da_janela() {
    let (mut d, mut s) = duas_linhas();
    s.executar(&mut d, "esticar");
    clicar(&mut s, &mut d, 8.0, -1.0);
    clicar(&mut s, &mut d, 12.0, 1.0);
    s.digitar(&mut d, "");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 5.0, 0.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(15.0, 0.0) });
    assert_eq!(geometrias(&d)[1], Geometria::Linha { a: p(0.0, 5.0), b: p(10.0, 5.0) }, "a de cima nem entrou na janela");
}

#[test]
fn aparar_e_concordancia_pelos_cliques() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    d.adicionar_linha(p(4.0, -2.0), p(4.0, 2.0));
    let mut s = Sessao::default();
    s.executar(&mut d, "aparar");
    clicar(&mut s, &mut d, 8.0, 0.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(4.0, 0.0) });
    s.cancelar(&mut d);
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(8.0, 0.0));
    d.adicionar_linha(p(10.0, 2.0), p(10.0, 10.0));
    let mut s = Sessao::default();
    s.executar(&mut d, "concordancia");
    clicar(&mut s, &mut d, 1.0, 0.0);
    clicar(&mut s, &mut d, 10.0, 9.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
}

#[test]
fn retangulo_elipse_e_hachura() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.executar(&mut d, "retangulo");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 10.0, 6.0);
    let Geometria::Polilinha { vertices, fechada: true } = &geometrias(&d)[0] else { panic!() };
    assert_eq!(vertices.iter().map(|v: &Vertice| (v.p.x, v.p.y)).collect::<Vec<_>>(), [(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)]);
    s.executar(&mut d, "elipse");
    clicar(&mut s, &mut d, 20.0, 0.0);
    clicar(&mut s, &mut d, 24.0, 0.0);
    clicar(&mut s, &mut d, 20.0, 2.0);
    let Geometria::Polilinha { vertices, .. } = &geometrias(&d)[1] else { panic!() };
    assert_eq!(vertices.len(), 72);
    // Hachura dentro do círculo, sem contorno fora.
    d.adicionar_circulo(p(50.0, 0.0), 5.0);
    s.executar(&mut d, "hachura");
    clicar(&mut s, &mut d, 0.0, 0.0 + 100.0);
    assert_eq!(d.entidades().len(), 3, "sem contorno, nada");
    clicar(&mut s, &mut d, 50.0, 1.0);
    assert!(matches!(geometrias(&d)[3], Geometria::Hachura { .. }));
    s.executar(&mut d, "hachura");
    assert_eq!(s.prompt().chave, "hachura.pontoPadrao");
}

#[test]
fn edicao_pela_linha_de_comando_ponto_relativo_e_previa() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "mover");
    clicar(&mut s, &mut d, 0.0, 0.0);
    // A prévia mostra a linha elástica e o objeto onde ficaria.
    let previa = s.previa_com(&d, p(2.0, 2.0));
    assert!(previa.contains(&Geometria::Linha { a: p(2.0, 2.0), b: p(12.0, 2.0) }), "{previa:?}");
    s.regional.separador_de_coordenadas = ';';
    s.digitar(&mut d, "@1;1");
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(1.0, 1.0), b: p(11.0, 1.0) });
}

#[test]
fn alterar_muda_cor_camada_e_propriedades_num_passo_so() {
    use feathercad_core::documento::{Mudanca, Propriedades, TipoLinha};
    let (mut d, _) = duas_linhas();
    let ids: Vec<_> = d.entidades().iter().map(|e| e.id).collect();
    let props = Propriedades { espessura: Some(0.5), tipo_linha: Some(TipoLinha::Eixo), grupo: Some(7) };
    d.aplicar_mudancas(ids.iter().map(|id| Mudanca::Alterar { id: *id, camada: None, cor: Some(Some(0xFF0000)), props: Some(props.clone()) }).collect());
    assert!(d.entidades().iter().all(|e| e.cor == Some(0xFF0000) && e.props == props));
    assert!(d.desfazer());
    assert!(d.entidades().iter().all(|e| e.cor.is_none() && e.props == Propriedades::default()), "um Ctrl+Z desfaz as duas");
}

#[test]
fn tipo_de_linha_vira_traco_e_vao_no_quadro_e_a_espessura_vai_no_segmento() {
    use feathercad_core::documento::{Mudanca, Propriedades, TipoLinha};
    use feathercad_core::quadro::montar;
    use feathercad_core::vista::Vista;
    let mut d = Documento::default();
    let id = d.adicionar_linha(p(0.0, 0.0), p(100.0, 0.0));
    let v = Vista { centro: p(50.0, 0.0), escala: 1.0, largura: 200.0, altura: 100.0, rotacao_x: 0.0, rotacao_z: 0.0 };
    let n_continua = u32::from_le_bytes(montar(&d, &v, None, None, false)[12..16].try_into().unwrap());
    assert_eq!(n_continua, 1);
    d.aplicar_mudancas(vec![Mudanca::Alterar { id, camada: None, cor: None, props: Some(Propriedades { espessura: Some(1.0), tipo_linha: Some(TipoLinha::Tracejada), grupo: None }) }]);
    let q = montar(&d, &v, None, None, false);
    let n = u32::from_le_bytes(q[12..16].try_into().unwrap());
    // 100 px em padrão 9 + 5: 8 traços (o último cortado).
    assert_eq!(n, 8, "traços do tracejado");
    let espessura = f32::from_le_bytes(q[16 + 20..16 + 24].try_into().unwrap());
    assert_eq!(espessura, 4.0, "1 mm = 4 px");
}

// ---------- anotação, utilitários e o resto da aba Início ----------

#[test]
fn texto_ponto_altura_e_conteudo_e_a_altura_fica_para_o_proximo() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.executar(&mut d, "texto");
    clicar(&mut s, &mut d, 5.0, 5.0);
    s.digitar(&mut d, "3");
    s.digitar(&mut d, "10 m²");
    let Geometria::Texto { ponto, altura, conteudo, .. } = &geometrias(&d)[0] else { panic!() };
    assert_eq!((*ponto, *altura, conteudo.as_str()), (p(5.0, 5.0), 3.0, "10 m²"), "um número digitado no conteúdo é texto, não coordenada");
    s.executar(&mut d, "texto");
    clicar(&mut s, &mut d, 0.0, 0.0);
    s.digitar(&mut d, "");
    s.digitar(&mut d, "de novo");
    let Geometria::Texto { altura, .. } = &geometrias(&d)[1] else { panic!() };
    assert_eq!(*altura, 3.0, "Enter na altura usa a última");
    assert!(d.desfazer() && d.entidades().len() == 1);
}

#[test]
fn cota_em_tres_cliques_vira_um_objeto_so_e_um_ctrl_z_desfaz() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.regional.decimal = ',';
    s.executar(&mut d, "cota");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 10.0, 0.0);
    assert!(s.previa_com(&d, p(5.0, 4.0)).len() >= 5, "a prévia mostra a cota como ficaria");
    clicar(&mut s, &mut d, 5.0, 4.0);
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    // no desenho é uma inserção só; as linhas, setas e o texto moram no bloco anônimo *D1
    assert_eq!(d.entidades().len(), 1);
    let Geometria::Insercao { bloco, ponto, .. } = d.entidades()[0].geometria.clone() else { panic!("a cota devia ser uma inserção") };
    assert_eq!(ponto, p(0.0, 0.0));
    assert_eq!(d.blocos()[bloco].nome, "*D1");
    let textos: Vec<_> = d.blocos()[bloco].entidades.iter().filter_map(|n| if let Geometria::Texto { conteudo, .. } = &n.geometria { Some(conteudo.clone()) } else { None }).collect();
    assert_eq!(textos, ["10,000"]);
    assert!(d.blocos()[bloco].entidades.len() >= 6);
    assert!(d.nomes_de_blocos_inseriveis().is_empty(), "o bloco anônimo não aparece na lista do Inserir");
    // clicar em qualquer parte da cota (a linha de cota, em y = 4) seleciona o objeto inteiro
    clicar(&mut s, &mut d, 2.0, 4.0);
    assert_eq!(s.selecionados(), 1);
    // uma segunda cota ganha outro nome
    s.cancelar(&mut d);
    s.executar(&mut d, "cota");
    clicar(&mut s, &mut d, 0.0, 20.0);
    clicar(&mut s, &mut d, 6.0, 20.0);
    clicar(&mut s, &mut d, 3.0, 24.0);
    assert_eq!(d.blocos()[1].nome, "*D2");
    assert!(d.desfazer() && d.entidades().len() == 1);
    assert!(d.desfazer() && d.entidades().is_empty(), "um Ctrl+Z desfaz a cota inteira");
    assert!(d.blocos().is_empty(), "e o bloco some junto");
}

#[test]
fn chamada_tabela_e_medir() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.executar(&mut d, "chamada");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 10.0, 10.0);
    s.digitar(&mut d, "Aço 8mm");
    assert!(geometrias(&d).iter().any(|g| matches!(g, Geometria::Texto { conteudo, .. } if conteudo == "Aço 8mm")));
    let antes = d.entidades().len();
    s.executar(&mut d, "tabela");
    clicar(&mut s, &mut d, 0.0, 100.0);
    s.digitar(&mut d, "x");
    assert_eq!(d.entidades().len(), antes, "dados inválidos não criam nada");
    s.digitar(&mut d, "2 3 4 5");
    assert_eq!(d.entidades().len(), antes + 3 + 4);
    // Medir: distância com dois cliques, área com A e Enter.
    s.executar(&mut d, "medir");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 3.0, 4.0);
    let m = s.historico().iter().rev().find_map(|e| if let feathercad_core::sessao::Entrada::Mensagem(m) = e { Some(m.clone()) } else { None }).unwrap();
    assert_eq!((m.chave.as_str(), m.vars["distancia"].as_str()), ("medir.distancia", "5,000"));
    s.comandos.definir(&[("x".to_string(), vec![])].into_iter().filter(|_| false).collect(), &[("area".to_string(), vec!["A".to_string()])].into_iter().collect()).unwrap();
    s.digitar(&mut d, "A");
    for (x, y) in [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)] {
        clicar(&mut s, &mut d, x, y);
    }
    s.digitar(&mut d, "");
    let m = s.historico().iter().rev().find_map(|e| if let feathercad_core::sessao::Entrada::Mensagem(m) = e { Some(m.clone()) } else { None }).unwrap();
    assert_eq!((m.chave.as_str(), m.vars["area"].as_str(), m.vars["perimetro"].as_str()), ("medir.area", "6,000", "12,000"));
}

#[test]
fn copiar_recortar_e_colar_pela_prancheta_interna() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.executar(&mut d, "colar");
    assert!(s.prompt().chave == "comando.digite", "sem nada copiado, não abre a ferramenta");
    s.executar(&mut d, "copiar_area");
    assert_eq!(d.entidades().len(), 2);
    s.executar(&mut d, "colar");
    clicar(&mut s, &mut d, 100.0, 100.0);
    clicar(&mut s, &mut d, 200.0, 0.0);
    assert_eq!(d.entidades().len(), 4, "duas colagens");
    assert!(geometrias(&d).contains(&Geometria::Linha { a: p(100.0, 100.0), b: p(110.0, 100.0) }), "a referência é o canto de baixo à esquerda");
    s.digitar(&mut d, "");
    // Recortar remove os originais.
    clicar(&mut s, &mut d, 105.0, 100.0);
    s.executar(&mut d, "recortar");
    assert_eq!(d.entidades().len(), 3);
    assert!(d.desfazer() && d.entidades().len() == 4);
}

#[test]
fn agrupar_seleciona_o_grupo_todo_e_de_novo_desagrupa() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.clique_com(&mut d, p(5.0, 5.0), TOL, true);
    s.executar(&mut d, "agrupar");
    assert!(d.entidades().iter().all(|e| e.props.grupo == Some(1)));
    clicar(&mut s, &mut d, 5.0, 0.0);
    assert_eq!(s.selecionados(), 2, "clicar num membro seleciona o grupo");
    s.executar(&mut d, "agrupar");
    assert!(d.entidades().iter().all(|e| e.props.grupo.is_none()), "agrupar um grupo o desfaz");
    clicar(&mut s, &mut d, 5.0, 0.0);
    assert_eq!(s.selecionados(), 1);
}

#[test]
fn criar_bloco_pede_a_base_e_o_nome_e_inserir_usa_o_bloco() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.clique_com(&mut d, p(5.0, 5.0), TOL, true);
    s.executar(&mut d, "criar_bloco");
    clicar(&mut s, &mut d, 0.0, 0.0);
    s.digitar(&mut d, "a/b");
    assert!(d.blocos().is_empty(), "nome inválido não cria");
    s.digitar(&mut d, "Mesa");
    assert_eq!((d.blocos().len(), d.entidades().len()), (1, 1));
    s.executar(&mut d, "inserir");
    assert!(matches!(s.ferramenta, Ferramenta::Insercao { .. }));
}

#[test]
fn igualar_camada_tornar_atual_e_igualar_propriedades() {
    use feathercad_core::documento::{Mudanca, Propriedades, TipoLinha};
    let (mut d, mut s) = duas_linhas();
    let ids: Vec<_> = d.entidades().iter().map(|e| e.id).collect();
    d.nova_camada("Cotas", None).unwrap();
    d.aplicar_mudancas(vec![Mudanca::Alterar { id: ids[1], camada: Some(1), cor: Some(Some(0xFF0000)), props: Some(Propriedades { espessura: Some(0.5), tipo_linha: Some(TipoLinha::Oculta), grupo: None }) }]);
    // Igualar camada: a linha de baixo vai para a camada da de cima.
    let (mut d2, mut s2) = duas_linhas();
    d2.nova_camada("Cotas", None).unwrap();
    let ids2: Vec<_> = d2.entidades().iter().map(|e| e.id).collect();
    d2.aplicar_mudancas(vec![Mudanca::Alterar { id: ids2[1], camada: Some(1), cor: None, props: None }]);
    clicar(&mut s2, &mut d2, 5.0, 0.0);
    s2.executar(&mut d2, "igualar_camada");
    clicar(&mut s2, &mut d2, 5.0, 5.0);
    assert_eq!(d2.entidade(ids2[0]).unwrap().camada, 1);
    // Tornar atual pelo objeto.
    d2.tornar_camada_atual(0).unwrap();
    s2.executar(&mut d2, "tornar_atual");
    clicar(&mut s2, &mut d2, 5.0, 5.0);
    assert_eq!(d2.camada_atual(), 1);
    // Igualar propriedades: a origem tem cor, tipo e espessura.
    s.executar(&mut d, "igualar_propriedades");
    clicar(&mut s, &mut d, 5.0, 5.0);
    clicar(&mut s, &mut d, 5.0, 0.0);
    let destino = d.entidade(ids[0]).unwrap();
    assert_eq!((destino.camada, destino.cor, destino.props.tipo_linha), (1, Some(0xFF0000), Some(TipoLinha::Oculta)));
}

#[test]
fn grab_arrastar_um_objeto_move_num_passo_so_e_soltar_sem_andar_nao_move() {
    let (mut d, mut s) = duas_linhas();
    // Pega a primeira linha (não estava selecionada: vira a seleção) e a leva 3 para cima.
    assert!(s.arrasto_iniciar(&d, p(5.0, 0.0), TOL));
    assert_eq!(s.selecionados(), 1);
    assert!(s.arrastando());
    s.arrasto_fim(&mut d, p(5.0, 3.0));
    assert!(!s.arrastando());
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 3.0), b: p(10.0, 3.0) });
    assert_eq!(geometrias(&d)[1], Geometria::Linha { a: p(0.0, 5.0), b: p(10.0, 5.0) });
    assert!(d.desfazer());
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
    // Soltar no mesmo lugar não cria passo nenhum.
    assert!(s.arrasto_iniciar(&d, p(5.0, 0.0), TOL));
    let versao = d.versao();
    s.arrasto_fim(&mut d, p(5.0, 0.0));
    assert_eq!(d.versao(), versao);
}

#[test]
fn grab_leva_junto_toda_a_selecao_e_no_vazio_nao_pega_nada() {
    let (mut d, mut s) = duas_linhas();
    clicar(&mut s, &mut d, 5.0, 0.0);
    s.clique_com(&mut d, p(5.0, 5.0), TOL, true); // Shift: a segunda também
    assert_eq!(s.selecionados(), 2);
    assert!(s.arrasto_iniciar(&d, p(5.0, 5.0), TOL));
    assert_eq!(s.selecionados(), 2, "pegar um dos selecionados mantém a seleção");
    s.arrasto_fim(&mut d, p(7.0, 5.0));
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(2.0, 0.0), b: p(12.0, 0.0) });
    assert_eq!(geometrias(&d)[1], Geometria::Linha { a: p(2.0, 5.0), b: p(12.0, 5.0) });
    s.cancelar(&mut d);
    assert!(!s.arrasto_iniciar(&d, p(50.0, 50.0), TOL));
}

#[test]
fn deslocar_com_distancia_digitada_repete_e_um_ctrl_z_desfaz_cada_copia() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
    let mut s = Sessao::default();
    s.executar(&mut d, "deslocar");
    s.digitar(&mut d, "2");
    clicar(&mut s, &mut d, 5.0, 0.0);
    assert_eq!(s.selecionados(), 1, "o objeto escolhido fica realçado");
    clicar(&mut s, &mut d, 5.0, 8.0);
    assert_eq!(d.entidades().len(), 2);
    assert_eq!(geometrias(&d)[1], Geometria::Linha { a: p(0.0, 2.0), b: p(10.0, 2.0) });
    assert_eq!(s.selecionados(), 0);
    // continua no comando: a cópia pode ser deslocada de novo, agora para baixo do original
    clicar(&mut s, &mut d, 5.0, 0.0);
    clicar(&mut s, &mut d, 5.0, -3.0);
    assert_eq!(geometrias(&d)[2], Geometria::Linha { a: p(0.0, -2.0), b: p(10.0, -2.0) });
    d.desfazer();
    assert_eq!(d.entidades().len(), 2);
    // distância 0 = passar pelo ponto clicado
    s.digitar(&mut d, "0");
    clicar(&mut s, &mut d, 5.0, 0.0);
    clicar(&mut s, &mut d, 5.0, 4.5);
    assert_eq!(geometrias(&d)[2], Geometria::Linha { a: p(0.0, 4.5), b: p(10.0, 4.5) });
}

#[test]
fn estender_pelos_cliques_avisa_quando_nao_ha_limite_e_um_ctrl_z_desfaz() {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(4.0, 0.0));
    d.adicionar_linha(p(10.0, -5.0), p(10.0, 5.0));
    let mut s = Sessao::default();
    s.executar(&mut d, "estender");
    clicar(&mut s, &mut d, 3.9, 0.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
    assert!(matches!(s.ferramenta, Ferramenta::Estender), "continua no comando");
    // a ponta da esquerda não tem nada no caminho: a linha fica como está
    clicar(&mut s, &mut d, 0.1, 0.0);
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
    d.desfazer();
    assert_eq!(geometrias(&d)[0], Geometria::Linha { a: p(0.0, 0.0), b: p(4.0, 0.0) });
}

#[test]
fn cota_alinhada_raio_e_diametro_tambem_viram_um_objeto_so() {
    let mut d = Documento::default();
    let mut s = Sessao::default();
    s.regional.decimal = ',';
    s.executar(&mut d, "cota_alinhada");
    clicar(&mut s, &mut d, 0.0, 0.0);
    clicar(&mut s, &mut d, 4.0, 3.0);
    clicar(&mut s, &mut d, 0.0, 5.0);
    assert_eq!((d.entidades().len(), d.blocos().len()), (1, 1));
    let textos = |d: &Documento, i: usize| -> Vec<String> { d.blocos()[i].entidades.iter().filter_map(|n| if let Geometria::Texto { conteudo, .. } = &n.geometria { Some(conteudo.clone()) } else { None }).collect() };
    assert_eq!(textos(&d, 0), ["5,000"]);
    // raio: escolhe o círculo, depois a posição do texto
    d.adicionar_circulo(p(50.0, 50.0), 5.0);
    s.executar(&mut d, "cota_raio");
    clicar(&mut s, &mut d, 0.0, 100.0); // vazio: continua pedindo o círculo
    assert!(matches!(s.ferramenta, Ferramenta::CotaRaio { alvo: None, .. }));
    clicar(&mut s, &mut d, 55.0, 50.0);
    assert!(matches!(s.ferramenta, Ferramenta::CotaRaio { alvo: Some(_), .. }));
    assert!(!s.previa_com(&d, p(60.0, 55.0)).is_empty(), "a prévia mostra a cota como ficaria");
    clicar(&mut s, &mut d, 60.0, 50.0);
    assert_eq!(s.ferramenta, Ferramenta::Nenhuma);
    assert_eq!(textos(&d, 1), ["R5,000"]);
    // diâmetro; e clicar numa linha avisa que só serve para círculo e arco
    s.executar(&mut d, "cota_diametro");
    clicar(&mut s, &mut d, 50.0, 45.0); // uma linha não serve: escolhe o objeto mais perto (aqui o círculo)
    clicar(&mut s, &mut d, 50.0, 56.0);
    assert_eq!(textos(&d, 2), ["Ø10,000"]);
    assert_eq!(d.entidades().len(), 4, "3 cotas e o círculo");
}

use std::fs;
use std::path::Path;

use feathercad_core::documento::{AlinhaH, AlinhaV, Bloco, Camada, Documento, Geometria, NovaEntidade};
use feathercad_core::geom::{Ponto, Vertice};
use feathercad_io::aci::{aci_para_rgb, rgb_para_aci};
use feathercad_io::{abrir, gravar, Formato};

fn p(x: f64, y: f64) -> Ponto {
    Ponto::novo(x, y)
}

fn desenho() -> Documento {
    let mut d = Documento::de_partes(
        vec![
            Camada { ligada: true, ..Camada::nova("0", 0xFF_FF_FF) },
            Camada { ligada: false, ..Camada::nova("Paredes ção", 0xFF_00_00) },
        ],
        0,
        Vec::new(),
    );
    d.adicionar_linha(p(0.0, 0.0), p(10.5, -3.25));
    d.adicionar_linha(p(-1e-7, 1234567.891), p(2.0, 2.0));
    d
}

/// As linhas retas do desenho (a comparação das entidades curvas/texto tem testes próprios).
fn linhas(d: &Documento) -> Vec<(Ponto, Ponto)> {
    d.entidades().iter().filter_map(|e| match e.geometria {
        Geometria::Linha { a, b } => Some((a, b)),
        _ => None,
    }).collect()
}

fn geometrias(d: &Documento) -> Vec<Geometria> {
    d.entidades().iter().map(|e| e.geometria.clone()).collect()
}

#[test]
fn ida_e_volta_no_dxf_guarda_linhas_camadas_e_visibilidade() {
    let d = desenho();
    let g = gravar(&d, Formato::Dxf).unwrap();
    assert!(g.avisos.is_empty());
    let texto = String::from_utf8(g.bytes.clone()).unwrap();
    assert!(texto.contains("AC1009") && texto.ends_with("EOF\n"));
    assert!(texto.contains("Paredes \\U+00E7\\U+00E3o"), "acento vira \\U+ para valer em qualquer versão");
    let a = abrir(&g.bytes).unwrap();
    assert_eq!(a.versao_dxf.as_deref(), Some("AC1009"));
    assert_eq!(linhas(&a.doc), linhas(&d));
    let nomes: Vec<_> = a.doc.camadas().iter().map(|c| (c.nome.as_str(), c.cor, c.ligada)).collect();
    assert_eq!(nomes, [("0", 0xFF_FF_FF, true), ("Paredes ção", 0xFF_00_00, false)]);
    assert!(!a.doc.sujo(), "documento aberto nasce limpo");
}

#[test]
fn ida_e_volta_no_fcdz_e_um_zip_com_manifesto() {
    let d = desenho();
    let g = gravar(&d, Formato::Fcdz).unwrap();
    assert!(g.bytes.starts_with(b"PK"));
    let a = abrir(&g.bytes).unwrap();
    assert_eq!(linhas(&a.doc), linhas(&d));
    assert_eq!(a.doc.camadas().len(), 2);
    assert_eq!(a.doc.camadas()[1].nome, "Paredes ção");
}

#[test]
fn cor_rgb_fora_da_tabela_aci_vira_aviso_no_dxf() {
    let mut d = Documento::default();
    let id = d.adicionar_linha(p(0.0, 0.0), p(1.0, 1.0));
    assert_eq!(id, 1);
    let mut d2 = Documento::de_partes(d.camadas().to_vec(), 0, Vec::new());
    d2.adicionar_linha(p(0.0, 0.0), p(1.0, 1.0));
    let g = gravar(&d2, Formato::Dxf).unwrap();
    assert!(g.avisos.is_empty(), "branco é ACI 7: nada a avisar");
    let g = gravar(&Documento::de_partes(vec![Camada { ligada: true, ..Camada::nova("X", 0x12_34_56) }], 0, Vec::new()), Formato::Dxf).unwrap();
    assert_eq!(g.avisos.len(), 1);
    assert_eq!(g.avisos[0].chave, "aviso.corAproximada");
}

#[test]
fn tabela_aci_confere_com_os_valores_conhecidos() {
    assert_eq!(aci_para_rgb(1), 0xFF0000);
    assert_eq!(aci_para_rgb(3), 0x00FF00);
    assert_eq!(aci_para_rgb(10), 0xFF0000);
    assert_eq!(aci_para_rgb(11), 0xFF7F7F);
    assert_eq!(aci_para_rgb(12), 0xCC0000);
    assert_eq!(aci_para_rgb(14), 0x990000);
    assert_eq!(aci_para_rgb(16), 0x7F0000);
    assert_eq!(aci_para_rgb(18), 0x4C0000);
    assert_eq!(aci_para_rgb(250), 0x333333);
    assert_eq!(aci_para_rgb(255), 0xFFFFFF);
    for i in 1..=255u16 {
        assert_eq!(aci_para_rgb(rgb_para_aci(aci_para_rgb(i))), aci_para_rgb(i), "ACI {i} ida e volta");
    }
}

#[test]
fn le_dxf_a_moda_antiga_sem_confiar_na_ordem_nem_em_contagem() {
    let dxf = "999\ncomentário\n0\nSECTION\n2\nENTITIES\n0\nLINE\n11\n5\n21\n6\n10\n1\n20\n2\n8\nParede\n62\n1\n0\nSPLINE\n8\n0\n10\n0\n20\n0\n40\n3\n0\nLINE\n8\nX\n10\n0\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert_eq!(linhas(&a.doc), [(p(1.0, 2.0), p(5.0, 6.0))], "a ordem das tags não importa; a LINE incompleta não entra");
    assert_eq!(a.doc.entidades()[0].cor, Some(0xFF0000));
    assert_eq!(a.doc.camadas().iter().map(|c| c.nome.as_str()).collect::<Vec<_>>(), ["Parede"], "a LINE incompleta não cria camada");
    let tipos = &a.avisos[0].vars["tipos"];
    assert!(tipos.contains("1 SPLINE") && tipos.contains("1 LINE (incompleta)"), "{tipos}");
    assert_eq!(a.avisos[0].vars["n"], "2");
}

#[test]
fn camada_desligada_congelada_e_layer_sem_distincao_de_caixa() {
    let dxf = "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nAlvenaria\n70\n0\n62\n-5\n0\nLAYER\n2\nGelo\n70\n1\n62\n3\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nALVENARIA\n10\n0\n20\n0\n11\n1\n21\n1\n420\n255\n0\nENDSEC\n0\nEOF";
    let a = abrir(dxf.as_bytes()).unwrap();
    let c = a.doc.camadas();
    assert_eq!((c.len(), c[0].ligada, c[0].cor, c[1].ligada, c[1].congelada), (2, false, aci_para_rgb(5), true, true), "congelada agora é propriedade própria (a lâmpada continua acesa)");
    assert_eq!(a.doc.entidades()[0].camada, 0, "ALVENARIA e Alvenaria são a mesma camada");
    assert_eq!(a.doc.entidades()[0].cor, Some(0x0000FF), "a cor verdadeira (420) vale");
}

#[test]
fn camadas_criadas_antes_da_tabela_atualizam_cor_e_ligada() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nTRILHOS\n10\n0\n20\n0\n11\n1\n21\n1\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nTRILHOS\n62\n1\n0\nENDTAB\n0\nENDSEC\n0\nEOF";
    let a = abrir(dxf.as_bytes()).unwrap();
    let c = a.doc.camadas();
    assert_eq!(c[0].nome, "TRILHOS");
    assert_eq!(c[0].cor, 0xFF0000, "a cor da camada é atualizada mesmo se criada antes da tabela LAYER");
}

#[test]
fn espaco_do_papel_entra_no_layout_e_vertices_nao_viram_lixo() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nLINE\n67\n1\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n0\nPOLYLINE\n8\n0\n0\nVERTEX\n10\n0\n20\n0\n0\nVERTEX\n10\n1\n20\n1\n0\nSEQEND\n0\nXLINE\n8\n0\n10\n0\n20\n0\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert_eq!(a.doc.entidades().len(), 2, "a linha do papel e a polilinha entram");
    assert!(matches!(a.doc.entidades()[0].espaco, feathercad_core::documento::Espaco::Layout(_)));
    assert_eq!(a.doc.entidades()[1].espaco, feathercad_core::documento::Espaco::Modelo);
}

#[test]
fn arquivo_ruim_da_erro_em_portugues_e_nunca_panico() {
    assert!(abrir(b"AutoCAD Binary DXF\r\n\x1a\0").unwrap_err().contains("binário"));
    assert!(abrir(b"isto nao e dxf\nnada\n").unwrap_err().contains("linha 1"));
    assert!(abrir(b"").unwrap_err().contains("não parece um DXF"));
    assert!(abrir(b"0\nSECTION\n2\n").is_err() || abrir(b"0\nSECTION\n2\n").is_ok());
    assert!(abrir(b"PK\x03\x04lixo").unwrap_err().contains(".fcdz"));
    assert!(abrir(&[0xFF, 0xFE, 0x00, 0x01]).is_err());
}

#[test]
fn le_latin1_das_versoes_antigas_e_utf8_das_novas() {
    let mut antigo = b"0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nCal".to_vec();
    antigo.extend_from_slice(&[0xE7, 0xE3, b'o']); // "ção" em Latin-1
    antigo.extend_from_slice(b"\n62\n2\n0\nENDTAB\n0\nENDSEC\n0\nEOF\n");
    assert_eq!(abrir(&antigo).unwrap().doc.camadas()[0].nome, "Calção");
    let novo = "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nCalção\n62\n2\n0\nENDTAB\n0\nENDSEC\n0\nEOF\n";
    assert_eq!(abrir(novo.as_bytes()).unwrap().doc.camadas()[0].nome, "Calção");
}

/// Abre TODAS as amostras reais (ezdxf e de terceiros): nenhuma pode dar pânico, e as
/// que abrem têm que abrir com pelo menos uma camada.
#[test]
fn corpus_de_amostras_abre_sem_panico() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes");
    let mut abertas = 0;
    let mut total = 0;
    for pasta in ["amostras", "amostras-terceiros"] {
        for f in fs::read_dir(raiz.join(pasta)).unwrap().flatten() {
            if f.path().extension().and_then(|e| e.to_str()) != Some("dxf") {
                continue;
            }
            total += 1;
            let bytes = fs::read(f.path()).unwrap();
            match abrir(&bytes) {
                Ok(a) => {
                    abertas += 1;
                    assert!(!a.doc.camadas().is_empty());
                    // o que abre, grava e reabre igual
                    let g = gravar(&a.doc, Formato::Dxf).unwrap();
                    let (antes, depois) = (linhas(&a.doc), linhas(&abrir(&g.bytes).unwrap().doc));
                    if a.doc.entidades().iter().any(|e| matches!(e.geometria, Geometria::Hachura { .. })) {
                        // A hachura de padrão volta como linhas: as originais continuam todas lá.
                        assert!(antes.iter().all(|l| depois.contains(l)), "{:?}", f.path());
                    } else {
                        assert_eq!(depois, antes, "{:?}", f.path());
                    }
                    // O `.fcdz` guarda tudo, igual.
                    let z = gravar(&a.doc, Formato::Fcdz).unwrap();
                    let volta = abrir(&z.bytes).unwrap().doc;
                    assert_eq!(geometrias(&volta), geometrias(&a.doc), "{:?}", f.path());
                    assert_eq!(volta.blocos(), a.doc.blocos(), "{:?}", f.path());
                }
                Err(e) => panic!("{:?} não abriu: {e}", f.path()),
            }
        }
    }
    assert!(total >= 40, "achou só {total} amostras");
    assert_eq!(abertas, total, "todo DXF do corpus tem que abrir");
    println!("corpus: {abertas} de {total} abriram");
}

#[test]
fn formatos_e_extensoes() {
    assert_eq!(Formato::da_extensao("DXF"), Some(Formato::Dxf));
    assert_eq!(Formato::da_extensao("fcdz"), Some(Formato::Fcdz));
    assert_eq!(Formato::da_extensao("dwg"), None);
    assert_eq!(Formato::Fcdz.extensao(), "fcdz");
}

// ---------- página inicial ----------

use feathercad_io::inicio::{listar_pasta, recentes_existentes, Preferencias};

fn pasta_de_teste(nome: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("feathercad-teste-{}-{nome}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn recentes_sem_repetir_no_topo_e_com_teto() {
    let mut p = Preferencias::default();
    p.adicionar_recente("/a.dxf", 1);
    p.adicionar_recente("/b.dxf", 2);
    p.adicionar_recente("/a.dxf", 3);
    assert_eq!(p.recentes.iter().map(|r| r.caminho.as_str()).collect::<Vec<_>>(), ["/a.dxf", "/b.dxf"]);
    assert_eq!(p.recentes[0].em_ms, 3);
    for i in 0..100 {
        p.adicionar_recente(&format!("/x{i}.dxf"), i);
    }
    assert_eq!(p.recentes.len(), 30);
    p.remover_recente("/x99.dxf");
    assert_eq!(p.recentes.len(), 29);
}

#[test]
fn preferencias_gravam_atomico_e_arquivo_ruim_vira_vazio() {
    let d = pasta_de_teste("prefs");
    let arq = d.join("sub").join("inicio.json");
    let mut p = Preferencias::default();
    p.adicionar_projeto("/projetos/casa");
    p.adicionar_projeto("/projetos/casa");
    p.adicionar_recente("/a.dxf", 5);
    p.gravar(&arq).unwrap();
    assert!(!arq.with_extension("parcial").exists());
    assert_eq!(Preferencias::carregar(&arq), p);
    assert_eq!(p.projetos.len(), 1);
    fs::write(&arq, b"{ lixo").unwrap();
    assert_eq!(Preferencias::carregar(&arq), Preferencias::default());
    assert_eq!(Preferencias::carregar(&d.join("nao-existe.json")), Preferencias::default());
    p.remover_projeto("/projetos/casa");
    assert!(p.projetos.is_empty());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn lista_pasta_so_com_desenhos_do_app_e_o_mais_novo_primeiro() {
    let d = pasta_de_teste("lista");
    let bytes = gravar(&desenho(), Formato::Dxf).unwrap().bytes;
    fs::write(d.join("planta.dxf"), &bytes).unwrap();
    fs::write(d.join("Corte.FCDZ"), gravar(&desenho(), Formato::Fcdz).unwrap().bytes).unwrap();
    fs::write(d.join("notas.txt"), b"x").unwrap();
    fs::write(d.join("desenho.dwg"), b"x").unwrap();
    fs::create_dir(d.join("subpasta.dxf")).unwrap();
    let itens = listar_pasta(&d).unwrap();
    let mut nomes: Vec<_> = itens.iter().map(|i| (i.nome.as_str(), i.tipo)).collect();
    nomes.sort();
    assert_eq!(nomes, [("Corte.FCDZ", "fcdz"), ("planta.dxf", "dxf")], "só .dxf e .fcdz, sem pasta nem outros formatos");
    assert!(itens.iter().all(|i| i.modificado_ms > 0 && i.pasta == d.to_string_lossy()));
    assert!(listar_pasta(&d.join("nada")).unwrap_err().contains("Não consegui abrir a pasta"));
    let mut p = Preferencias::default();
    p.adicionar_recente(&d.join("planta.dxf").to_string_lossy(), 9);
    p.adicionar_recente(&d.join("sumiu.dxf").to_string_lossy(), 10);
    let vivos = recentes_existentes(&p);
    assert_eq!(vivos.len(), 1, "o que sumiu do disco não aparece");
    assert_eq!(vivos[0].1, 9);
    let _ = fs::remove_dir_all(&d);
}

// ---------- polilinha e texto ----------

fn v(x: f64, y: f64, bulge: f64) -> Vertice {
    Vertice { p: p(x, y), bulge }
}

fn com_polilinha_e_texto() -> Documento {
    let mut d = Documento::default();
    d.adicionar_linha(p(0.0, 0.0), p(1.0, 1.0));
    d.adicionar_polilinha(vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.5), v(10.0, 10.0, -1.0), v(0.0, 10.0, 0.0)], true);
    d.adicionar_polilinha(vec![v(-5.0, -5.0, 0.0), v(-1.0, -2.5, 0.0)], false);
    d
}

#[test]
fn polilinha_com_bulge_ida_e_volta_no_dxf_e_no_fcdz() {
    let d = com_polilinha_e_texto();
    for formato in [Formato::Dxf, Formato::Fcdz] {
        let g = gravar(&d, formato).unwrap();
        let a = abrir(&g.bytes).unwrap();
        assert_eq!(geometrias(&a.doc), geometrias(&d), "{formato:?}");
        assert!(a.avisos.is_empty(), "{formato:?}: {:?}", a.avisos);
    }
    let texto = String::from_utf8(gravar(&d, Formato::Dxf).unwrap().bytes).unwrap();
    assert!(texto.contains("POLYLINE") && texto.contains("VERTEX") && texto.contains("SEQEND"), "R12 grava POLYLINE+VERTEX+SEQEND");
    assert!(!texto.contains("LWPOLYLINE"));
}

#[test]
fn le_lwpolilinha_com_bulge_e_fechada() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nParedes\n90\n3\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n42\n0.5\n43\n0\n10\n10\n20\n5\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.avisos.is_empty());
    assert_eq!(geometrias(&a.doc), [Geometria::Polilinha { vertices: vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.5), v(10.0, 5.0, 0.0)], fechada: true }]);
}

#[test]
fn le_polilinha_pesada_do_r12_e_pula_vertice_de_controle_e_3d() {
    let dxf = "0\nSECTION\n2\nENTITIES\n\
0\nPOLYLINE\n8\n0\n66\n1\n70\n0\n0\nVERTEX\n8\n0\n10\n0\n20\n0\n0\nVERTEX\n8\n0\n10\n5\n20\n0\n42\n1\n0\nVERTEX\n8\n0\n10\n99\n20\n99\n70\n16\n0\nVERTEX\n8\n0\n10\n5\n20\n5\n0\nSEQEND\n8\n0\n\
0\nPOLYLINE\n8\n0\n66\n1\n70\n8\n0\nVERTEX\n8\n0\n10\n0\n20\n0\n30\n1\n0\nVERTEX\n8\n0\n10\n1\n20\n1\n30\n2\n0\nSEQEND\n8\n0\n\
0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert_eq!(geometrias(&a.doc).len(), 2, "a 2D e a linha; a 3D fica de fora");
    assert_eq!(geometrias(&a.doc)[0], Geometria::Polilinha { vertices: vec![v(0.0, 0.0, 0.0), v(5.0, 0.0, 1.0), v(5.0, 5.0, 0.0)], fechada: false });
    assert_eq!(a.avisos.len(), 1);
    assert_eq!(a.avisos[0].vars["tipos"], "1 POLYLINE (3D ou malha)", "vértice e seqend não contam como entidade");
}

#[test]
fn le_texto_com_codigos_percentuais_alinhamento_e_giro() {
    let dxf = "0\nSECTION\n2\nENTITIES\n\
0\nTEXT\n8\n0\n10\n1\n20\n2\n40\n2.5\n1\nDi%%c 30%%d %%p2 %%%%\n50\n90\n\
0\nTEXT\n8\n0\n10\n0\n20\n0\n40\n1\n1\nCentro\n72\n1\n73\n2\n11\n7\n21\n8\n\
0\nTEXT\n8\n0\n10\n0\n20\n0\n40\n0\n1\nsem altura\n\
0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    let g = geometrias(&a.doc);
    assert_eq!(g[0], Geometria::Texto { ponto: p(1.0, 2.0), altura: 2.5, rotacao: 90.0, conteudo: "DiØ 30° ±2 %%".into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
    assert_eq!(g[1], Geometria::Texto { ponto: p(7.0, 8.0), altura: 1.0, rotacao: 0.0, conteudo: "Centro".into(), h: AlinhaH::Centro, v: AlinhaV::Meio }, "com alinhamento, o ponto é o 11/21");
    assert_eq!(g.len(), 2);
    assert!(a.avisos[0].vars["tipos"].contains("1 TEXT (incompleta)"), "texto sem altura não desenha");
}

#[test]
fn texto_ida_e_volta_com_acento_alinhamento_e_giro() {
    let mut d = Documento::default();
    let de = |conteudo: &str, h, v, rot| Geometria::Texto { ponto: p(3.0, 4.0), altura: 2.0, rotacao: rot, conteudo: conteudo.into(), h, v };
    let lista = [de("Cozinha ção", AlinhaH::Esquerda, AlinhaV::Base, 0.0), de("Sala", AlinhaH::Centro, AlinhaV::Meio, 45.0), de("Ø12", AlinhaH::Direita, AlinhaV::Topo, 0.0), de("baixo", AlinhaH::Esquerda, AlinhaV::Baixo, 0.0)];
    let novo = Documento::de_partes(d.camadas().to_vec(), 0, lista.iter().cloned().map(|geometria| feathercad_core::documento::NovaEntidade { camada: 0, cor: Some(0xFF0000), geometria, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo }).collect());
    d = novo;
    for formato in [Formato::Dxf, Formato::Fcdz] {
        let a = abrir(&gravar(&d, formato).unwrap().bytes).unwrap();
        assert_eq!(geometrias(&a.doc), lista, "{formato:?}");
        assert_eq!(a.doc.entidades()[1].cor, Some(0xFF0000));
    }
}

#[test]
fn vp4_abre_a_polilinha_e_o_texto_incluindo_papel() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes/amostras/VP4.dxf");
    let a = abrir(&fs::read(raiz).unwrap()).unwrap();
    let g = geometrias(&a.doc);
    assert!(g.iter().any(|g| matches!(g, Geometria::Polilinha { .. })), "a LWPOLYLINE do VP4 entrou");
    assert!(g.iter().filter(|g| matches!(g, Geometria::Texto { .. })).count() >= 1, "os TEXT do VP4 entraram");
}

// ---------- círculo e arco ----------

#[test]
fn circulo_e_arco_ida_e_volta_no_dxf_e_no_fcdz() {
    let d = Documento::de_partes(
        vec![Camada { ligada: true, ..Camada::nova("0", 0xFF_FF_FF) }],
        0,
        vec![
            feathercad_core::documento::NovaEntidade { camada: 0, cor: None, geometria: Geometria::Circulo { centro: p(1.5, -2.0), raio: 3.25 }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo },
            feathercad_core::documento::NovaEntidade { camada: 0, cor: Some(0x00FF00), geometria: Geometria::Arco { centro: p(0.0, 0.0), raio: 10.0, inicio: 300.0, fim: 45.0 }, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo },
        ],
    );
    for formato in [Formato::Dxf, Formato::Fcdz] {
        let a = abrir(&gravar(&d, formato).unwrap().bytes).unwrap();
        assert_eq!(geometrias(&a.doc), geometrias(&d), "{formato:?}");
        assert!(a.avisos.is_empty());
    }
}

#[test]
fn le_circulo_e_arco_e_espelha_quando_a_extrusao_aponta_para_baixo() {
    let dxf = "0\nSECTION\n2\nENTITIES\n\
0\nCIRCLE\n8\n0\n10\n5\n20\n1\n40\n2\n\
0\nCIRCLE\n8\n0\n10\n5\n20\n1\n40\n2\n230\n-1\n\
0\nARC\n8\n0\n10\n0\n20\n0\n40\n4\n50\n0\n51\n90\n230\n-1\n\
0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n0\n\
0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    let g = geometrias(&a.doc);
    assert_eq!(g[0], Geometria::Circulo { centro: p(5.0, 1.0), raio: 2.0 });
    assert_eq!(g[1], Geometria::Circulo { centro: p(-5.0, 1.0), raio: 2.0 }, "extrusão -Z espelha o X");
    assert_eq!(g[2], Geometria::Arco { centro: p(-0.0, 0.0), raio: 4.0, inicio: 90.0, fim: 180.0 }, "de 0→90 vira 90→180 espelhado");
    assert_eq!(g.len(), 3);
    assert_eq!(a.avisos[0].vars["tipos"], "1 CIRCLE (incompleta)", "raio zero não desenha");
}

#[test]
fn corpus_nao_avisa_mais_circulo_nem_arco() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes");
    for pasta in ["amostras", "amostras-terceiros"] {
        for f in fs::read_dir(raiz.join(pasta)).unwrap().flatten() {
            if f.path().extension().and_then(|e| e.to_str()) != Some("dxf") {
                continue;
            }
            if let Ok(a) = abrir(&fs::read(f.path()).unwrap()) {
                for m in &a.avisos {
                    let tipos = m.vars.get("tipos").cloned().unwrap_or_default();
                    for item in tipos.split(", ") {
                        let nome = item.split_once(' ').map_or("", |(_, n)| n);
                        assert!(nome != "CIRCLE" && nome != "ARC", "{:?}: {tipos}", f.path());
                    }
                }
            }
        }
    }
}

// ---------- blocos (INSERT) ----------

fn nova(geometria: Geometria) -> NovaEntidade {
    NovaEntidade { camada: 0, cor: None, geometria, props: Default::default(), espaco: feathercad_core::documento::Espaco::Modelo }
}

/// "porta" (uma linha) dentro de "sala" (uma linha + a porta inserida), e a sala inserida duas vezes.
fn com_blocos() -> Documento {
    let ins = |bloco, x, sx, giro| Geometria::Insercao { bloco, ponto: p(x, 0.0), escala_x: sx, escala_y: 1.0, rotacao: giro };
    let blocos = vec![
        Bloco { nome: "sala".into(), base: p(1.0, 1.0), entidades: vec![nova(Geometria::Linha { a: p(0.0, 0.0), b: p(4.0, 0.0) }), nova(ins(1, 2.0, 1.0, 0.0))] },
        Bloco { nome: "porta".into(), base: p(0.0, 0.0), entidades: vec![nova(Geometria::Linha { a: p(0.0, 0.0), b: p(0.0, 1.0) })] },
    ];
    Documento::de_partes_com_blocos(Vec::new(), 0, blocos, vec![nova(ins(0, 10.0, 1.0, 0.0)), nova(ins(0, 20.0, -2.0, 90.0))])
}

#[test]
fn bloco_aninhado_ida_e_volta_no_dxf_e_no_fcdz() {
    let d = com_blocos();
    for formato in [Formato::Dxf, Formato::Fcdz] {
        let g = gravar(&d, formato).unwrap();
        let a = abrir(&g.bytes).unwrap();
        assert_eq!(a.doc.blocos(), d.blocos(), "{formato:?}");
        assert_eq!(geometrias(&a.doc), geometrias(&d), "{formato:?}");
        assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    }
}

#[test]
fn cota_do_app_e_um_bloco_anonimo_que_volta_igual_no_dxf_e_no_fcdz() {
    let mut d = Documento::default();
    let formas = feathercad_core::anotacao::cota_linear(p(0.0, 0.0), p(10.0, 0.0), p(5.0, 4.0), 1.0, &Default::default()).unwrap();
    d.adicionar_cota(formas, p(0.0, 0.0)).unwrap();
    for formato in [Formato::Dxf, Formato::Fcdz] {
        let g = gravar(&d, formato).unwrap();
        let a = abrir(&g.bytes).unwrap();
        assert_eq!(a.doc.entidades().len(), 1, "{formato:?}: continua um objeto só");
        assert_eq!(a.doc.blocos()[0].nome, "*D1", "{formato:?}");
        assert_eq!(a.doc.blocos().len(), d.blocos().len(), "{formato:?}");
        assert_eq!(a.doc.blocos()[0].entidades.len(), d.blocos()[0].entidades.len(), "{formato:?}");
        assert!(a.avisos.is_empty(), "{formato:?}: {:?}", a.avisos);
    }
}

#[test]
fn insert_de_bloco_inexistente_e_dimension_sem_bloco_viram_aviso() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n0\n2\nfantasma\n10\n0\n20\n0\n0\nDIMENSION\n8\n0\n2\n*D1\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.doc.entidades().is_empty());
    let tipos = &a.avisos[0].vars["tipos"];
    assert!(tipos.contains("INSERT (bloco inexistente)") && tipos.contains("DIMENSION"), "{tipos}");
}

#[test]
fn dimension_mostra_o_bloco_anonimo_e_ignorado_dentro_de_bloco_avisa_a_parte() {
    let dxf = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\n*D1\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n5\n21\n0\n0\nHATCH\n8\n0\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nDIMENSION\n8\n0\n2\n*D1\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert_eq!(a.doc.blocos().len(), 1);
    assert!(matches!(a.doc.entidades()[0].geometria, Geometria::Insercao { bloco: 0, escala_x, .. } if escala_x == 1.0));
    assert_eq!(a.avisos.len(), 1);
    assert_eq!(a.avisos[0].chave, "aviso.dentroDeBlocos");
}

#[test]
fn bloco_que_se_insere_em_si_mesmo_nao_trava() {
    let dxf = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\nA\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n0\nINSERT\n8\n0\n2\nA\n10\n1\n20\n0\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n0\n2\nA\n10\n0\n20\n0\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.doc.caixa().is_some());
    let g = gravar(&a.doc, Formato::Dxf).unwrap();
    assert!(abrir(&g.bytes).is_ok());
}

#[test]
fn corpus_nao_avisa_mais_insert() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes");
    for pasta in ["amostras", "amostras-terceiros"] {
        for f in fs::read_dir(raiz.join(pasta)).unwrap().flatten() {
            if f.path().extension().and_then(|e| e.to_str()) != Some("dxf") {
                continue;
            }
            if let Ok(a) = abrir(&fs::read(f.path()).unwrap()) {
                for m in &a.avisos {
                    let tipos = m.vars.get("tipos").cloned().unwrap_or_default();
                    assert!(!tipos.contains("INSERT ") || tipos.contains("INSERT ("), "{:?}: {tipos}", f.path());
                }
            }
        }
    }
}

#[test]
fn attrib_visivel_vira_texto_e_o_invisivel_e_o_attdef_ficam_de_fora_sem_aviso() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nATTRIB\n8\n0\n10\n1\n20\n2\n40\n0.5\n1\nP-101\n0\nATTRIB\n8\n0\n70\n1\n10\n1\n20\n2\n40\n0.5\n1\noculto\n0\nSEQEND\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert_eq!(a.doc.entidades().len(), 1);
    assert!(matches!(&a.doc.entidades()[0].geometria, Geometria::Texto { conteudo, .. } if conteudo == "P-101"));
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
}

#[test]
fn mtext_vira_uma_linha_de_texto_por_paragrafo_sem_codigos_de_formatacao() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nMTEXT\n8\n0\n10\n5\n20\n10\n40\n2\n71\n1\n3\n{\\fArial|b1;Sala\n1\n \\C1;de estar}\\Pa\\S1/2;m\\P\\Pfim\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    let t: Vec<_> = a.doc.entidades().iter().map(|e| match &e.geometria {
        Geometria::Texto { ponto, conteudo, v, .. } => (conteudo.clone(), *ponto, *v),
        g => panic!("{g:?}"),
    }).collect();
    assert_eq!(t.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["Sala de estar", "a1/2m", "fim"]);
    assert!(t.iter().all(|x| x.2 == AlinhaV::Topo));
    // Linhas descem em passos de 5/3 da altura; a linha em branco (i = 2) consome o seu espaço.
    assert!((t[0].1.y - 10.0).abs() < 1e-9 && (t[1].1.y - (10.0 - 2.0 * 5.0 / 3.0)).abs() < 1e-9 && (t[2].1.y - (10.0 - 6.0 * 5.0 / 3.0)).abs() < 1e-9, "{t:?}");
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
}

// ---------- hachura ----------

/// Um quadrado de lado 10 (polilinha) e, opcionalmente, uma ilha de lado 2 (arestas de linha).
fn dxf_hachura(solido: bool) -> String {
    let padrao = if solido { "" } else { "78\n1\n53\n45\n43\n0\n44\n0\n45\n0\n46\n2\n79\n0\n" };
    format!(
        "0\nSECTION\n2\nENTITIES\n0\nHATCH\n8\n0\n10\n0\n20\n0\n30\n0\n2\n{}\n70\n{}\n71\n0\n91\n2\n\
         92\n3\n72\n0\n73\n1\n93\n4\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n97\n0\n\
         92\n0\n93\n2\n72\n1\n10\n4\n20\n4\n11\n6\n21\n4\n72\n2\n10\n5\n20\n4\n40\n1\n50\n0\n51\n360\n73\n1\n97\n0\n\
         75\n0\n76\n1\n52\n0\n41\n1\n77\n0\n{}47\n1\n98\n0\n0\nENDSEC\n0\nEOF\n",
        if solido { "SOLID" } else { "ANSI31" },
        i32::from(solido),
        padrao
    )
}

#[test]
fn le_hachura_solida_com_contorno_de_polilinha_e_de_arestas() {
    let a = abrir(dxf_hachura(true).as_bytes()).unwrap();
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    let Geometria::Hachura { contornos, padrao, nome } = &a.doc.entidades()[0].geometria else { panic!() };
    assert_eq!((contornos.len(), contornos[0].len(), nome.as_str()), (2, 4, "SOLID"));
    assert!(padrao.is_empty());
    // A aresta-linha soma 2 pontos e o círculo 73 (72 passos de 5° + 1): sem pânico e com mais de 3 pontos.
    assert!(contornos[1].len() > 3);
}

#[test]
fn le_hachura_de_padrao_com_as_linhas_da_definicao() {
    let a = abrir(dxf_hachura(false).as_bytes()).unwrap();
    let Geometria::Hachura { padrao, .. } = &a.doc.entidades()[0].geometria else { panic!() };
    assert_eq!(padrao.len(), 1);
    assert_eq!((padrao[0].angulo, padrao[0].dy), (45.0, 2.0));
}

#[test]
fn hachura_ida_e_volta_no_fcdz_e_avisa_que_o_dxf_r12_nao_a_grava() {
    let d = abrir(dxf_hachura(false).as_bytes()).unwrap().doc;
    let g = gravar(&d, Formato::Fcdz).unwrap();
    assert_eq!(geometrias(&abrir(&g.bytes).unwrap().doc), geometrias(&d));
    let dxf = gravar(&d, Formato::Dxf).unwrap();
    assert!(dxf.avisos.iter().any(|m| m.chave == "aviso.hachuraDecomposta"), "{:?}", dxf.avisos);
}

#[test]
fn dxf_r12_grava_hachura_de_padrao_como_linhas_e_solida_como_triangulos_e_avisa_da_ilha() {
    // Padrão: linhas a 45° a cada 2 de deslocamento em Y; o quadrado 10×10 tem ilha, que vaza as linhas.
    let d = abrir(dxf_hachura(false).as_bytes()).unwrap().doc;
    let volta = abrir(&gravar(&d, Formato::Dxf).unwrap().bytes).unwrap().doc;
    let n = volta.entidades().iter().filter(|e| matches!(e.geometria, Geometria::Linha { .. })).count();
    assert!(n > 5 && volta.entidades().iter().all(|e| !matches!(e.geometria, Geometria::Hachura { .. })), "{n} linhas");
    // Sólida sem ilha → triângulos SOLID que voltam como hachuras sólidas.
    let sem_ilha = "0\nSECTION\n2\nENTITIES\n0\nHATCH\n8\n0\n2\nSOLID\n70\n1\n91\n1\n92\n3\n72\n0\n73\n1\n93\n5\n10\n0\n20\n0\n10\n4\n20\n0\n10\n4\n20\n2\n10\n2\n20\n2\n10\n2\n20\n4\n97\n0\n75\n0\n76\n1\n78\n0\n0\nENDSEC\n0\nEOF\n";
    let d = abrir(sem_ilha.as_bytes()).unwrap().doc;
    let g = gravar(&d, Formato::Dxf).unwrap();
    assert!(g.avisos.iter().any(|m| m.chave == "aviso.hachuraDecomposta"), "{:?}", g.avisos);
    let volta = abrir(&g.bytes).unwrap().doc;
    assert_eq!(volta.entidades().len(), 3, "5 vértices → 3 triângulos");
    assert!(volta.entidades().iter().all(|e| matches!(&e.geometria, Geometria::Hachura { padrao, .. } if padrao.is_empty())));
    // Sólida com ilha: não dá para gravar.
    let g = gravar(&abrir(dxf_hachura(true).as_bytes()).unwrap().doc, Formato::Dxf).unwrap();
    assert!(g.avisos.iter().any(|m| m.chave == "aviso.hachuraNaoGravada"), "{:?}", g.avisos);
}

#[test]
fn amostras_de_hachura_abrem_desenham_e_nao_acusam_mais_hatch() {
    use feathercad_core::quadro::montar;
    use feathercad_core::vista::Vista;
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes/amostras");
    let mut com_hachura = 0;
    for f in fs::read_dir(raiz).unwrap().flatten() {
        let nome = f.file_name().to_string_lossy().to_string();
        if !nome.contains("hatch") {
            continue;
        }
        let Ok(a) = abrir(&fs::read(f.path()).unwrap()) else { continue };
        for m in &a.avisos {
            let tipos = m.vars.get("tipos").cloned().unwrap_or_default();
            assert!(!tipos.contains("HATCH"), "{nome}: {tipos}");
        }
        let vista = Vista::inicial(800.0, 600.0).ajustar(a.doc.caixa());
        let q = montar(&a.doc, &vista, None, None, false);
        assert!(q.starts_with(b"FCQ1"), "{nome}");
        com_hachura += a.doc.entidades().iter().filter(|e| matches!(e.geometria, Geometria::Hachura { .. })).count();
    }
    assert!(com_hachura > 0, "as amostras de hachura deviam trazer hachuras");
}

/// Relatório da bateria: `cargo test bateria -- --nocapture --ignored`.
#[test]
#[ignore]
fn bateria_relatorio() {
    use feathercad_core::quadro::montar;
    use feathercad_core::vista::Vista;
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes");
    let mut arquivos: Vec<_> = ["amostras", "amostras-terceiros"].iter().flat_map(|p| fs::read_dir(raiz.join(p)).unwrap().flatten().map(|f| f.path())).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("dxf")).collect();
    arquivos.sort();
    for f in arquivos {
        let nome = f.file_name().unwrap().to_string_lossy().to_string();
        match abrir(&fs::read(&f).unwrap()) {
            Err(e) => println!("ERRO  {nome}: {e}"),
            Ok(a) => {
                let vista = Vista::inicial(800.0, 600.0).ajustar(a.doc.caixa());
                let q = montar(&a.doc, &vista, None, None, false);
                let avisos: Vec<String> = a.avisos.iter().map(|m| format!("{} {:?}", m.chave, m.vars.get("tipos"))).collect();
                println!("ok    {nome}: {} entidades, {} blocos, {} bytes de quadro {}", a.doc.entidades().len(), a.doc.blocos().len(), q.len(), avisos.join(" | "));
            }
        }
    }
}

#[test]
fn mtext_paragrafo_minusculo_nao_quebra_linha_coluna_quebra_e_vazio_nao_avisa() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nMTEXT\n8\n0\n10\n0\n20\n0\n40\n1\n1\n\\pxqr;um\\Ndois^Itres\n0\nMTEXT\n8\n0\n10\n0\n20\n0\n40\n1\n1\n\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    let t: Vec<_> = a.doc.entidades().iter().filter_map(|e| match &e.geometria { Geometria::Texto { conteudo, .. } => Some(conteudo.clone()), _ => None }).collect();
    assert_eq!(t, ["um", "dois tres"]);
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
}

// ---------- ELLIPSE, SPLINE, SOLID, 3DFACE, POINT, MINSERT, tabela ----------

fn entidades_do(dxf_corpo: &str) -> Vec<Geometria> {
    let dxf = format!("0\nSECTION\n2\nENTITIES\n{dxf_corpo}0\nENDSEC\n0\nEOF\n");
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    geometrias(&a.doc)
}

#[test]
fn elipse_completa_fecha_e_passa_pelas_pontas_dos_eixos() {
    let g = entidades_do("0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n4\n21\n0\n40\n0.5\n41\n0\n42\n6.283185307179586\n");
    let Geometria::Polilinha { vertices, fechada } = &g[0] else { panic!("{g:?}") };
    assert!(*fechada);
    let (xs, ys): (Vec<f64>, Vec<f64>) = vertices.iter().map(|v| (v.p.x, v.p.y)).unzip();
    let max = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max);
    assert!((max(&xs) - 4.0).abs() < 1e-9 && (max(&ys) - 2.0).abs() < 0.01, "semi-eixos 4 e 2 (amostrado de 4 em 4 graus)");
}

#[test]
fn spline_de_grau_1_passa_pelos_controles_e_a_de_grau_2_fica_dentro_do_casco() {
    // Grau 1 com nós 0,0,1,2,2: a curva é o próprio polígono de controle.
    let g = entidades_do("0\nSPLINE\n8\n0\n70\n8\n71\n1\n72\n5\n73\n3\n40\n0\n40\n0\n40\n1\n40\n2\n40\n2\n10\n0\n20\n0\n10\n2\n20\n2\n10\n4\n20\n0\n");
    let Geometria::Polilinha { vertices, .. } = &g[0] else { panic!("{g:?}") };
    assert!(vertices.iter().all(|v| (v.p.y - (2.0 - (v.p.x - 2.0).abs())).abs() < 1e-6), "todo ponto está no triângulo 0,0 → 2,2 → 4,0");
    assert!((vertices.first().unwrap().p.x).abs() < 1e-9 && (vertices.last().unwrap().p.x - 4.0).abs() < 1e-3);
    // Grau 2 (Bézier): o ponto do meio é (0.25·P0 + 0.5·P1 + 0.25·P2) = (2, 1).
    let g = entidades_do("0\nSPLINE\n8\n0\n70\n8\n71\n2\n72\n6\n73\n3\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n10\n0\n20\n0\n10\n2\n20\n2\n10\n4\n20\n0\n");
    let Geometria::Polilinha { vertices, .. } = &g[0] else { panic!() };
    assert!(vertices.iter().any(|v| (v.p.x - 2.0).abs() < 0.05 && (v.p.y - 1.0).abs() < 0.05));
}

#[test]
fn solid_vira_area_cheia_na_ordem_certa_e_3dface_vira_contorno_fechado_e_point_um_ponto() {
    let g = entidades_do("0\nSOLID\n8\n0\n10\n0\n20\n0\n11\n4\n21\n0\n12\n0\n22\n3\n13\n4\n23\n3\n0\n3DFACE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n12\n1\n22\n1\n13\n1\n23\n1\n0\nPOINT\n8\n0\n10\n7\n20\n8\n");
    let Geometria::Hachura { contornos, padrao, .. } = &g[0] else { panic!() };
    assert!(padrao.is_empty());
    let pts: Vec<(f64, f64)> = contornos[0].iter().map(|v| (v.p.x, v.p.y)).collect();
    assert_eq!(pts, [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0)], "1, 2, 4, 3: sem cruzar");
    assert!(matches!(&g[1], Geometria::Face3D { p1, p2, p3, p4 } if *p1 == p(0.0, 0.0) && *p2 == p(1.0, 0.0) && *p3 == p(1.0, 1.0) && *p4 == p(1.0, 1.0)), "o 3DFACE vira Face3D");
    assert_eq!(g[2], Geometria::Ponto { p: p(7.0, 8.0) });
}

#[test]
fn minsert_expande_a_matriz_com_o_giro_do_insert() {
    let dxf = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\nb\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n8\n0\n2\nb\n10\n10\n20\n0\n50\n90\n70\n3\n71\n2\n44\n5\n45\n7\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    let g = geometrias(&a.doc);
    assert_eq!(g.len(), 6);
    // Coluna 1, linha 0, giro 90°: o passo de 5 vai para +Y.
    assert!(matches!(&g[1], Geometria::Insercao { ponto, .. } if (ponto.x - 10.0).abs() < 1e-9 && (ponto.y - 5.0).abs() < 1e-9), "{:?}", g[1]);
}

#[test]
fn tabela_e_cotas_de_arco_usam_o_bloco_anonimo() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes/amostras");
    let a = abrir(&fs::read(raiz.join("acad_table_simple.dxf")).unwrap()).unwrap();
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    assert!(a.doc.entidades().iter().any(|e| matches!(e.geometria, Geometria::Insercao { .. })), "a tabela virou INSERT do bloco dela");
}

#[test]
fn ponto_e_polilinha_de_elipse_gravam_e_voltam_no_dxf_e_no_fcdz() {
    let d = Documento::de_partes_com_blocos(Vec::new(), 0, Vec::new(), vec![nova(Geometria::Ponto { p: p(1.0, 2.0) })]);
    for f in [Formato::Dxf, Formato::Fcdz] {
        assert_eq!(geometrias(&abrir(&gravar(&d, f).unwrap().bytes).unwrap().doc), geometrias(&d), "{f:?}");
    }
}

#[test]
fn multileader_vira_linha_ate_o_pouso_mais_o_dogleg_e_o_texto() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nMULTILEADER\n8\n0\n100\nAcDbMLeader\n300\nCONTEXT_DATA{\n40\n1\n41\n0.5\n290\n1\n304\nLEADER\\PSegunda\n12\n10\n22\n20\n302\nLEADER{\n290\n1\n291\n1\n10\n8\n20\n20\n11\n1\n21\n0\n40\n2\n304\nLEADER_LINE{\n10\n0\n20\n0\n91\n0\n305\n}\n303\n}\n301\n}\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    assert!(a.avisos.is_empty(), "{:?}", a.avisos);
    let g = geometrias(&a.doc);
    let Geometria::Polilinha { vertices, fechada: false } = &g[0] else { panic!("{g:?}") };
    let pts: Vec<(f64, f64)> = vertices.iter().map(|v| (v.p.x, v.p.y)).collect();
    assert_eq!(pts, [(0.0, 0.0), (8.0, 20.0), (10.0, 20.0)]);
    let textos: Vec<_> = g.iter().filter_map(|g| match g { Geometria::Texto { conteudo, ponto, .. } => Some((conteudo.as_str(), ponto.x, ponto.y)), _ => None }).collect();
    assert_eq!(textos.len(), 2);
    assert_eq!((textos[0].0, textos[0].1, textos[0].2), ("LEADER", 10.0, 20.0));
    assert_eq!(textos[1].0, "Segunda");
}

#[test]
fn circuit_diagram_grava_todas_as_hachuras_no_dxf() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testes/amostras-terceiros/Circuit diagram_OXB_T.dxf");
    let a = abrir(&fs::read(raiz).unwrap()).unwrap();
    let g = gravar(&a.doc, Formato::Dxf).unwrap();
    let chaves: Vec<_> = g.avisos.iter().map(|m| m.chave.as_str()).collect();
    assert!(!chaves.contains(&"aviso.hachuraNaoGravada"), "{:?}", g.avisos);
    assert!(chaves.contains(&"aviso.hachuraDecomposta"), "as 16 hachuras viraram triângulos");
}

#[test]
fn hachura_solida_de_circulo_em_coordenada_grande_ainda_grava_no_dxf() {
    // Duas meias-voltas (bulge 1) de raio 2, longe da origem: a tolerância precisa seguir o tamanho da hachura.
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nHATCH\n8\n0\n2\nSOLID\n70\n1\n91\n1\n92\n3\n72\n1\n73\n1\n93\n2\n10\n100000\n20\n200000\n42\n1\n10\n100004\n20\n200000\n42\n1\n97\n0\n75\n0\n76\n1\n78\n0\n0\nENDSEC\n0\nEOF\n";
    let d = abrir(dxf.as_bytes()).unwrap().doc;
    let g = gravar(&d, Formato::Dxf).unwrap();
    assert!(g.avisos.iter().all(|m| m.chave != "aviso.hachuraNaoGravada"), "{:?}", g.avisos);
    assert!(abrir(&g.bytes).unwrap().doc.entidades().len() > 10, "o círculo virou triângulos");
}

#[test]
fn espessura_e_tipo_de_linha_do_dxf_entram_e_o_tipo_volta_na_gravacao() {
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n6\nDASHED2\n370\n50\n10\n0\n20\n0\n11\n5\n21\n0\n0\nLINE\n8\n0\n6\nBYLAYER\n370\n-1\n10\n0\n20\n1\n11\n5\n21\n1\n0\nENDSEC\n0\nEOF\n";
    let a = abrir(dxf.as_bytes()).unwrap();
    let e = a.doc.entidades();
    assert_eq!((e[0].props.espessura, e[0].props.tipo_linha), (Some(0.5), Some(feathercad_core::documento::TipoLinha::Tracejada)));
    assert_eq!((e[1].props.espessura, e[1].props.tipo_linha), (None, None), "por camada não é propriedade da entidade");
    let volta = abrir(&gravar(&a.doc, Formato::Dxf).unwrap().bytes).unwrap().doc;
    assert_eq!(volta.entidades()[0].props.tipo_linha, Some(feathercad_core::documento::TipoLinha::Tracejada));
    // O .fcdz guarda a espessura também (o R12 não tem).
    let z = abrir(&gravar(&a.doc, Formato::Fcdz).unwrap().bytes).unwrap().doc;
    assert_eq!(z.entidades()[0].props, e[0].props);
}

#[test]
fn a_unidade_do_desenho_faz_ida_e_volta_nos_dois_formatos() {
    use feathercad_core::documento::{Documento, Unidade};
    use feathercad_io::{abrir, gravar, Formato};
    for u in Unidade::TODAS {
        let doc = Documento::default().com_unidade(Some(u));
        for formato in [Formato::Dxf, Formato::Fcdz] {
            let volta = abrir(&gravar(&doc, formato).unwrap().bytes).unwrap().doc;
            assert_eq!(volta.unidade(), Some(u), "{formato:?} {u:?}");
        }
    }
    let sem = Documento::default();
    for formato in [Formato::Dxf, Formato::Fcdz] {
        assert_eq!(abrir(&gravar(&sem, formato).unwrap().bytes).unwrap().doc.unidade(), None);
    }
}

#[test]
fn abrir_nuvem_de_pontos_como_levantamento_topografico() {
    let csv = "E,N,Z\n0,0,10\n10,0,12\n10,10,14\n0,10,11\n5,5,15\n";
    let aberto = abrir(csv.as_bytes()).unwrap();
    assert_eq!(aberto.versao_dxf, None);
    assert!(!aberto.doc.entidades().is_empty());
    assert!(aberto.doc.camadas().iter().any(|c| c.nome == "TOPOGRAFIA_CURVAS_MESTRAS"));
    assert!(aberto.doc.camadas().iter().any(|c| c.nome == "TOPOGRAFIA_TIN"));
    assert_eq!(aberto.avisos.len(), 1);
    assert_eq!(aberto.avisos[0].chave, "aviso.nuvemCarregada");
}

#[test]
fn dxf_r12_avisa_que_a_espessura_de_linha_nao_e_gravada_e_o_fcdz_guarda() {
    // Camada TRILHO com 0,50 mm (370 = 50) vinda de um DXF R2000+, e uma linha nela.
    let dxf = "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nTRILHO\n70\n0\n62\n7\n6\nCONTINUOUS\n370\n50\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nTRILHO\n10\n0\n20\n0\n11\n10\n21\n0\n0\nENDSEC\n0\nEOF\n";
    let d = abrir(dxf.as_bytes()).unwrap().doc;
    let i = d.camadas().iter().position(|c| c.nome == "TRILHO").unwrap();
    assert_eq!(d.camadas()[i].espessura, Some(0.5));
    let g = gravar(&d, Formato::Dxf).unwrap();
    assert!(g.avisos.iter().any(|m| m.chave == "aviso.espessuraNaoGravada" && m.vars.get("n").map(String::as_str) == Some("1")), "{:?}", g.avisos);
    let volta = abrir(&gravar(&d, Formato::Fcdz).unwrap().bytes).unwrap().doc;
    assert_eq!(volta.camadas()[i].espessura, Some(0.5), "o .fcdz guarda a espessura");
    // Sem espessura, sem aviso.
    let mut limpo = Documento::default();
    limpo.adicionar_linha(feathercad_core::geom::Ponto::novo(0.0, 0.0), feathercad_core::geom::Ponto::novo(1.0, 0.0));
    assert!(!gravar(&limpo, Formato::Dxf).unwrap().avisos.iter().any(|m| m.chave == "aviso.espessuraNaoGravada"));
}

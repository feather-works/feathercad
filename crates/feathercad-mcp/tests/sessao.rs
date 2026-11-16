//! Uma sessão MCP de verdade: sobe o binário, conversa em JSON-RPC pelo stdin/stdout e confere o que volta.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

struct Servidor {
    filho: std::process::Child,
    leitor: BufReader<std::process::ChildStdout>,
    proximo: u64,
}

impl Servidor {
    fn novo() -> Self {
        let mut filho = Command::new(env!("CARGO_BIN_EXE_feathercad-mcp")).env("FEATHERCAD_VIVO", "0").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let leitor = BufReader::new(filho.stdout.take().unwrap());
        Self { filho, leitor, proximo: 1 }
    }

    fn pedir(&mut self, metodo: &str, params: Value) -> Value {
        let id = self.proximo;
        self.proximo += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": metodo, "params": params });
        writeln!(self.filho.stdin.as_mut().unwrap(), "{msg}").unwrap();
        let mut linha = String::new();
        self.leitor.read_line(&mut linha).unwrap();
        let r: Value = serde_json::from_str(&linha).unwrap();
        assert_eq!(r["id"], json!(id));
        r
    }

    /// Chama uma ferramenta; devolve os blocos de conteúdo e se deu erro.
    fn ferramenta(&mut self, nome: &str, args: Value) -> (Vec<Value>, bool) {
        let r = self.pedir("tools/call", json!({ "name": nome, "arguments": args }));
        (r["result"]["content"].as_array().unwrap().clone(), r["result"]["isError"].as_bool().unwrap())
    }

    /// O primeiro bloco de texto, lido como JSON.
    fn json(&mut self, nome: &str, args: Value) -> Value {
        let (c, erro) = self.ferramenta(nome, args);
        assert!(!erro, "{nome} falhou: {}", c[0]["text"]);
        serde_json::from_str(c[0]["text"].as_str().unwrap()).unwrap()
    }
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.filho.kill();
    }
}

#[test]
fn handshake_e_lista_de_ferramentas() {
    let mut s = Servidor::novo();
    let r = s.pedir("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "teste", "version": "1" } }));
    assert_eq!(r["result"]["serverInfo"]["name"], "feathercad");
    assert!(r["result"]["capabilities"]["tools"].is_object());
    let t = s.pedir("tools/list", json!({}));
    let nomes: Vec<&str> = t["result"]["tools"].as_array().unwrap().iter().map(|x| x["name"].as_str().unwrap()).collect();
    for esperado in ["cad_novo", "cad_ver", "cad_entidades", "cad_exportar", "cad_importar", "cad_medir", "cad_regioes", "cad_verificar", "cad_criar", "cad_editar", "cad_hachurar", "cad_converter"] {
        assert!(nomes.contains(&esperado), "falta {esperado}");
    }
    assert!(t["result"]["tools"].as_array().unwrap().iter().all(|x| x["inputSchema"]["type"] == "object"));
    assert_eq!(s.pedir("nao_existe", json!({}))["error"]["code"], -32601);
}

#[test]
fn desenhar_medir_ver_e_verificar() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "cm" }));
    // Um quadrado de 100 por 100 feito de quatro linhas, e um círculo dentro.
    let c = s.json("cad_criar", json!({ "formas": [
        { "tipo": "linha", "a": [0, 0], "b": [100, 0] }, { "tipo": "linha", "de": [100, 0], "distancia": 100, "angulo": 90 },
        { "tipo": "linha", "a": [100, 100], "b": [0, 100] }, { "tipo": "linha", "a": [0, 100], "b": [0, 0] },
        { "tipo": "circulo", "centro": [50, 50], "raio": 10 } ], "camada": "Paredes", "cor": "#FF0000" }));
    assert_eq!(c["criadas"].as_array().unwrap().len(), 5);
    let r = s.json("cad_resumo", json!({}));
    assert_eq!(r["unidade"], "cm");
    assert_eq!(r["por_tipo"]["linha"], 4);
    assert_eq!(r["extensao"]["largura"], 100.0);
    // Noção de espaço em números.
    let e = s.json("cad_entidades", json!({ "tipo": "circulo" }));
    assert_eq!(e["entidades"][0]["diametro"], 20.0);
    let perto = s.json("cad_entidades", json!({ "perto": { "ponto": [50, 0], "raio": 5 } }));
    assert_eq!(perto["total"], 1, "só a linha de baixo está a menos de 5 do ponto (50,0)");
    let m = s.json("cad_medir", json!({ "de": [0, 0], "para": [30, 40] }));
    assert_eq!(m["distancia"], 50.0);
    let reg = s.json("cad_regioes", json!({}));
    // Duas regiões: o círculo (≈ 314) e o quadrado de linhas soltas (10000), cuja área útil desconta o círculo.
    let lista = reg["regioes"].as_array().unwrap();
    assert_eq!(lista.len(), 2, "{reg}");
    assert!((lista[0]["area"].as_f64().unwrap() - 314.0).abs() < 3.0);
    assert_eq!(lista[1]["area"], 10000.0);
    assert!((lista[1]["area_util"].as_f64().unwrap() - (10000.0 - lista[0]["area"].as_f64().unwrap())).abs() < 1e-6);
    let em_ponto = s.json("cad_regioes", json!({ "ponto": [10, 10] }));
    assert_eq!(em_ponto["encontrada"], true);
    // Hachura e a imagem.
    assert_eq!(s.json("cad_hachurar", json!({ "ponto": [10, 10] }))["criada"]["tipo"], "hachura");
    let (ver, erro) = s.ferramenta("cad_ver", json!({ "largura": 600 }));
    assert!(!erro && ver[0]["type"] == "image" && ver[0]["mimeType"] == "image/png");
    let png = base64_decodifica(ver[0]["data"].as_str().unwrap());
    assert_eq!(&png[1..4], b"PNG");
    assert_eq!(s.json("cad_verificar", json!({}))["ok"], true);
    // Desfazer a hachura.
    assert_eq!(s.json("cad_desfazer", json!({}))["feito"], true);
}

fn base64_decodifica(s: &str) -> Vec<u8> {
    let tabela = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let (mut fora, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for b in s.bytes().filter(|b| *b != b'=') {
        acc = (acc << 6) | tabela.iter().position(|t| *t == b).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            fora.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    fora
}

#[test]
fn verificar_acha_folga_e_editar_move_num_passo_so() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "mm" }));
    s.json("cad_criar", json!({ "formas": [{ "tipo": "linha", "a": [0, 0], "b": [100, 0] }, { "tipo": "linha", "a": [100.4, 0], "b": [100, 60] }, { "tipo": "linha", "a": [0, 0], "b": [0, 60] }] }));
    let v = s.json("cad_verificar", json!({ "tolerancia": 1 }));
    assert_eq!(v["ok"], false);
    assert!(v["problemas"].as_array().unwrap().iter().any(|p| p["tipo"] == "folga"), "{v}");
    let ed = s.json("cad_editar", json!({ "operacao": "mover", "tipo": "linha", "dx": 10, "dy": 5 }));
    assert_eq!(ed["entidades_afetadas"], 3);
    let l = s.json("cad_entidades", json!({ "ids": [1] }));
    assert_eq!(l["entidades"][0]["de"], json!([10.0, 5.0]));
    s.json("cad_desfazer", json!({}));
    assert_eq!(s.json("cad_entidades", json!({ "ids": [1] }))["entidades"][0]["de"], json!([0.0, 0.0]), "mover 3 linhas = um Ctrl+Z");
    // Erro de ferramenta volta como isError, com mensagem clara.
    let (c, erro) = s.ferramenta("cad_criar", json!({ "formas": [{ "tipo": "circulo", "centro": [0, 0], "raio": -1 }] }));
    assert!(erro && c[0]["text"].as_str().unwrap().contains("raio"));
}

#[test]
fn texto_compacto_ida_e_volta_e_conversao_de_arquivo() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "mm" }));
    let importado = s.json("cad_importar", json!({ "texto": "camada Paredes #FF0000\nR 0 0 400 300\nC 200 150 25\nT 10 10 12 0 \"Sala 1\"\n" }));
    assert_eq!(importado["criadas"], 3);
    let (exp, erro) = s.ferramenta("cad_exportar", json!({}));
    assert!(!erro);
    let texto = exp[0]["text"].as_str().unwrap();
    assert!(texto.contains("R 0 0 400 300") && texto.contains("C 200 150 25") && texto.contains("\"Sala 1\""), "{texto}");
    // Converter um DXF para compacto pelo disco e ver o quanto encolhe.
    let dir = std::env::temp_dir().join(format!("fcmcp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dxf = dir.join("a.dxf");
    let fct = dir.join("a.fct");
    s.json("cad_salvar", json!({ "caminho": dxf.to_str().unwrap() }));
    let conv = s.json("cad_converter", json!({ "origem": dxf.to_str().unwrap(), "destino": fct.to_str().unwrap() }));
    assert!(conv["bytes_destino"].as_u64().unwrap() * 3 < conv["bytes_origem"].as_u64().unwrap(), "{conv}");
    // Não sobrescreve sem pedir; abrir o .fct traz o mesmo desenho de volta.
    let (c, erro) = s.ferramenta("cad_converter", json!({ "origem": dxf.to_str().unwrap(), "destino": fct.to_str().unwrap() }));
    assert!(erro && c[0]["text"].as_str().unwrap().contains("sobrescrever"));
    s.json("cad_novo", json!({ "unidade": "m" }));
    assert_eq!(s.json("cad_abrir", json!({ "caminho": fct.to_str().unwrap() }))["entidades"], 3);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn prompts_so_para_projeto_eletrico() {
    let mut s = Servidor::novo();
    let i = s.pedir("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "teste", "version": "1" } }));
    assert!(i["result"]["capabilities"]["prompts"].is_object());
    assert!(i["result"]["instructions"].as_str().unwrap().contains("PROJETO ELÉTRICO: só quando"));
    let l = s.pedir("prompts/list", json!({}));
    let p = &l["result"]["prompts"][0];
    assert_eq!(p["name"], "projeto_eletrico");
    assert!(p["description"].as_str().unwrap().contains("SOMENTE"));
    let g = s.pedir("prompts/get", json!({ "name": "projeto_eletrico" }));
    assert!(g["result"]["messages"][0]["content"]["text"].as_str().unwrap().contains("FOLHA"));
    assert_eq!(s.pedir("prompts/get", json!({ "name": "nao_existe" }))["error"]["code"], -32602);
    let grande = s.pedir("prompts/get", json!({ "name": "projeto_grande" }));
    assert!(grande["result"]["messages"][0]["content"]["text"].as_str().unwrap().contains("cad_verificar {leiaute:true}"));
    assert_eq!(l["result"]["prompts"][1]["name"], "projeto_grande");
}

#[test]
fn simbolos_e_eletroduto_da_nbr_5444() {
    let mut s = Servidor::novo();
    let lista = s.json("cad_simbolos", json!({}));
    let nomes: Vec<&str> = lista["simbolos"].as_array().unwrap().iter().map(|x| x["nome"].as_str().unwrap()).collect();
    for n in ["tomada_baixa", "tomada_media", "tomada_alta", "tomada_piso", "ponto_luz_teto", "interruptor_paralelo", "interruptor_intermediario", "quadro_geral_embutido"] {
        assert!(nomes.contains(&n), "falta {n}");
    }
    // Sem unidade não dá para saber o tamanho de papel.
    let (_, erro) = s.ferramenta("cad_simbolo", json!({ "nome": "tomada_baixa", "ponto": [0, 0] }));
    assert!(erro);
    s.json("cad_novo", json!({ "unidade": "cm" }));
    // Tomada média: barra, haste, triângulo, metade cheia e rótulo. 1 mm de papel a 1:50 em cm = 5 cm.
    let t = s.json("cad_simbolo", json!({ "nome": "tomada_media", "ponto": [100, 50], "parede": "norte", "circuito": 3, "bloco": false }));
    let ids = t["ids"].as_array().unwrap().len();
    assert_eq!(ids, 5, "{t}");
    let barra = s.json("cad_entidades", json!({ "tipo": "linha" }));
    let comprimentos: Vec<f64> = barra["entidades"].as_array().unwrap().iter().map(|e| e["comprimento"].as_f64().unwrap()).collect();
    assert!(comprimentos.iter().any(|c| (c - 25.0).abs() < 1e-6), "a barra tem 5 mm de papel = 25 cm: {comprimentos:?}");
    // O terceiro nível: dupla tem dois triângulos; o texto do circuito fica na horizontal.
    let d = s.json("cad_simbolo", json!({ "nome": "tomada_alta", "ponto": [300, 50], "parede": "oeste", "quantidade": 2, "bloco": false }));
    assert!(d["ids"].as_array().unwrap().len() >= 6);
    let l = s.json("cad_simbolo", json!({ "nome": "ponto_luz_teto", "ponto": [500, 200], "comando": "a", "circuito": 4, "potencia": 100, "quantidade": 2, "bloco": false }));
    assert!(l["ids"].as_array().unwrap().len() >= 4);
    let textos = s.json("cad_entidades", json!({ "tipo": "texto" }));
    let conteudos: Vec<&str> = textos["entidades"].as_array().unwrap().iter().filter_map(|e| e["conteudo"].as_str()).collect();
    for esperado in ["-4-", "2x100W", "a"] {
        assert!(conteudos.contains(&esperado), "falta o texto {esperado}: {conteudos:?}");
    }
    // Por padrão o símbolo é UM bloco, e a contagem para a lista de materiais vem pelo nome dele.
    for x in [700, 800] {
        let b = s.json("cad_simbolo", json!({ "nome": "tomada_baixa", "ponto": [x, 50], "parede": "norte", "circuito": 5 }));
        assert_eq!(b["ids"].as_array().unwrap().len(), 1, "{b}");
    }
    let c = s.json("cad_contar", json!({ "simbolos": true }));
    assert_eq!(c["por_simbolo"]["tomada_baixa"], 2, "{c}");
    // Eletroduto com condutores: 3 circuitos + terra; a bandeirinha traz o Ø automático.
    let e = s.json("cad_eletroduto", json!({ "pontos": [[0, 0], [400, 0], [400, 300]], "condutores": [
        { "circuito": 1, "tipo": "fase" }, { "circuito": 1, "tipo": "neutro" },
        { "circuito": 3, "tipo": "fase", "secao": 2.5 }, { "circuito": 3, "tipo": "neutro", "secao": 2.5 } ] }));
    assert_eq!(e["condutores"], 5);
    assert_eq!(e["diametro_mm"], 20, "50,5 mm² de condutores não cabem em Ø16 (40 % = 49 mm²)");
    let textos = s.json("cad_entidades", json!({ "tipo": "texto" }));
    let conteudos: Vec<&str> = textos["entidades"].as_array().unwrap().iter().filter_map(|e| e["conteudo"].as_str()).collect();
    assert!(conteudos.contains(&"1 3") && conteudos.contains(&"#2,5") && conteudos.contains(&"Ø20"), "{conteudos:?}");
    // No piso o eletroduto é tracejado (só a linha dele).
    let p = s.json("cad_eletroduto", json!({ "pontos": [[0, 100], [200, 100]], "tipo": "piso", "condutores": [{ "circuito": 2, "tipo": "fase" }], "diametro": 25 }));
    assert_eq!(p["tipo"], "piso");
    assert_eq!(p["diametro_mm"], 25);
    assert_eq!(s.json("cad_desfazer", json!({}))["feito"], true);
    let (_, erro) = s.ferramenta("cad_simbolo", json!({ "nome": "nao_existe", "ponto": [0, 0] }));
    assert!(erro);
}

#[test]
fn eletroduto_curvo_espessura_e_estilo_de_camada() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "cm" }));
    // Espessura e tipo de linha por camada.
    s.json("cad_camadas", json!({ "acao": "criar", "nome": "PAREDES", "cor": "#999999", "espessura": 0.5 }));
    s.json("cad_camadas", json!({ "acao": "propriedade", "nome": "PAREDES", "tipo_linha": "tracejada" }));
    let (_, erro) = s.ferramenta("cad_camadas", json!({ "acao": "propriedade", "nome": "PAREDES", "tipo_linha": "zigzag" }));
    assert!(erro, "tipo de linha desconhecido tem que dar erro");
    // Eletroduto curvo de 2 pontos: vira polilinha amostrada, com espessura própria; o texto do circuito segue na horizontal.
    let e = s.json("cad_eletroduto", json!({ "pontos": [[0, 0], [400, 0]], "curva": 0.2, "espessura": 0.35, "condutores": [{ "circuito": 1, "tipo": "fase" }, { "circuito": 1, "tipo": "neutro" }] }));
    assert_eq!(e["condutores"], 3);
    let l = s.json("cad_entidades", json!({ "ids": [e["eletroduto"].clone()] }));
    let caixa = &l["entidades"][0];
    assert!(caixa["caixa"]["max"][1].as_f64().unwrap() > 30.0, "a curva sai da reta y = 0: {caixa}");
}

#[test]
fn lote_desenha_com_dados_e_lacos_e_e_um_passo_so() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "cm" }));
    let r = s.json("cad_lote", json!({
        "dados": { "X": 100, "Y": 50, "pontos": [[0, 0], [30, 0], [60, 20]], "n": 3, "cs": [{ "v": 2 }, { "v": 3 }] },
        "passos": [
            { "para_cada": "pontos", "como": "p", "indice": "i", "faca": [
                { "ferramenta": "cad_criar", "argumentos": { "formas": [
                    { "tipo": "circulo", "centro": "=[X + p[0], Y + p[1]]", "raio": "=5 + i" },
                    { "tipo": "texto", "ponto": "=[X + p[0], Y + p[1] + 10]", "altura": 3, "conteudo": "P${i + 1}" } ] } } ] },
            { "repetir": "=n", "como": "k", "faca": [
                { "ferramenta": "cad_criar", "argumentos": { "formas": [
                    { "tipo": "linha", "a": [0, "=k * 10"], "b": [50, "=k * 10"] } ] } } ] },
            { "definir": "total", "valor": "=soma_campo(cs, 'v')" }
        ],
        "retornar": ["total"] }));
    assert_eq!((r["chamadas"].as_u64(), r["voltas"].as_u64(), r["entidades_novas"].as_u64()), (Some(6), Some(6), Some(9)));
    assert_eq!(r["variaveis"]["total"], 5);
    let e = s.json("cad_entidades", json!({ "tipo": "circulo" }));
    let raios: Vec<f64> = e["entidades"].as_array().unwrap().iter().map(|x| x["raio"].as_f64().unwrap()).collect();
    assert_eq!(raios, vec![5.0, 6.0, 7.0]);
    assert_eq!(s.json("cad_resumo", json!({}))["por_tipo"]["texto"], 3);
    s.json("cad_desfazer", json!({}));
    assert_eq!(s.json("cad_resumo", json!({}))["total_de_entidades"], 0, "o lote inteiro é um passo de desfazer");
}

#[test]
fn lote_com_erro_nao_aplica_nada_e_diz_onde() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "mm" }));
    s.json("cad_criar", json!({ "formas": [{ "tipo": "linha", "a": [0, 0], "b": [1, 1] }] }));
    let (c, erro) = s.ferramenta("cad_lote", json!({
        "dados": { "itens": [[0, 0], [10, 10], "quebrado"] },
        "passos": [ { "para_cada": "itens", "como": "p", "faca": [
            { "ferramenta": "cad_criar", "argumentos": { "formas": [{ "tipo": "circulo", "centro": "=p", "raio": 2 }] } } ] } ] }));
    assert!(erro);
    let msg = c[0]["text"].as_str().unwrap();
    assert!(msg.contains("passo [0](volta 2)[0]") && msg.contains("nada foi aplicado"), "{msg}");
    assert_eq!(s.json("cad_resumo", json!({}))["total_de_entidades"], 1, "os dois círculos que deram certo foram desfeitos");
    // O desfazer continua sendo o da linha, e o refazer não guarda lixo do lote cancelado.
    s.json("cad_desfazer", json!({}));
    assert_eq!(s.json("cad_resumo", json!({}))["total_de_entidades"], 0);
}

#[test]
fn lote_valida_com_exigir_guarda_resultados_e_barra_ferramentas_proibidas() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "cm" }));
    let (c, erro) = s.ferramenta("cad_lote", json!({ "dados": { "circuitos": [{ "n": 1 }] },
        "passos": [ { "exigir": "=tamanho(circuitos) >= 2", "mensagem": "faltam circuitos (tem ${tamanho(circuitos)})" } ] }));
    assert!(erro && c[0]["text"].as_str().unwrap().contains("faltam circuitos (tem 1)"));
    let (c, erro) = s.ferramenta("cad_lote", json!({ "passos": [ { "ferramenta": "cad_novo", "argumentos": { "unidade": "mm" } } ] }));
    assert!(erro && c[0]["text"].as_str().unwrap().contains("não pode ser usada"));
    let (_, erro) = s.ferramenta("cad_lote", json!({ "passos": [ { "ferramenta": "cad_importar", "argumentos": { "texto": "L 0 0 1 1", "substituir": true } } ] }));
    assert!(erro);
    // `guardar` põe a resposta de uma ferramenta numa variável que os passos seguintes usam.
    let r = s.json("cad_lote", json!({ "passos": [
        { "ferramenta": "cad_criar", "argumentos": { "formas": [{ "tipo": "linha", "a": [0, 0], "b": [30, 40] }] }, "guardar": "feito" },
        { "ferramenta": "cad_medir", "argumentos": { "de": [0, 0], "para": "=[30, 40]" }, "guardar": "medida" } ],
        "retornar": ["feito", "medida"] }));
    assert_eq!(r["variaveis"]["feito"]["criadas"].as_array().unwrap().len(), 1);
    assert_eq!(r["variaveis"]["medida"]["distancia"].as_f64(), Some(50.0), "a resposta de uma ferramenta vira variável para os passos seguintes");
}

#[test]
fn lote_e_anunciado_com_a_linguagem_de_expressoes() {
    let mut s = Servidor::novo();
    let t = s.pedir("tools/list", json!({}));
    let lote = t["result"]["tools"].as_array().unwrap().iter().find(|x| x["name"] == "cad_lote").unwrap();
    let d = lote["description"].as_str().unwrap();
    assert!(d.contains("para_cada") && d.contains("deslocar(caminho,d)") && d.contains("tudo ou nada") || d.contains("Tudo ou nada"));
}

#[test]
fn folha_pdf_contar_e_hachura_com_padrao_funcionam_pelo_mcp() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "m" }));
    s.json("cad_criar", json!({ "formas": [{ "tipo": "linha", "a": [0, 0], "b": [500, 250] }] }));
    for i in 0..6 {
        s.json("cad_criar", json!({ "camada": "DORMENTES", "formas": [{ "tipo": "retangulo", "canto": [f64::from(i) * 2.0, 10], "largura": 0.24, "altura": 2.0, "rotacao": 30 }] }));
    }
    // folha: sem escala pedida escolhe a que cabe; origem põe o canto em (0,0)
    let f = s.json("cad_folha", json!({ "acao": "inserir", "papel": "A1", "origem": true, "campos": { "titulo": "TÚNEL" } }));
    assert_eq!(f["folha"]["papel"], "A1");
    assert_eq!(f["folha"]["min"], json!({ "x": 0.0, "y": 0.0, "z": 0.0 }));
    let escala = f["folha"]["escala"].as_f64().unwrap();
    assert!(escala >= 500.0, "500 m em A1 pedem 1:750 ou mais: {escala}");
    let c = s.json("cad_folha", json!({ "acao": "campos", "indice": 0 }));
    assert_eq!(c["campos"]["titulo"], "TÚNEL");
    let e = s.json("cad_folha", json!({ "acao": "escala", "indice": 0, "escala": 1000 }));
    assert_eq!(e["folha"]["escala_texto"], "1:1000");
    // contar dormentes pela medida e pelo modelo, mesmo girados
    let n = s.json("cad_contar", json!({ "tipo": "retangulo", "medida": { "largura": 0.24, "altura": 2 }, "camada": "DORMENTES" }));
    assert_eq!(n["total"], 6);
    let (ent, _) = s.ferramenta("cad_entidades", json!({ "camada": "DORMENTES", "limite": 1 }));
    let id = serde_json::from_str::<Value>(ent[0]["text"].as_str().unwrap()).unwrap()["entidades"][0]["id"].as_u64().unwrap();
    assert_eq!(s.json("cad_contar", json!({ "igual_a": id }))["total"], 6);
    // hachura com padrão e como forma
    let h = s.json("cad_criar", json!({ "formas": [{ "tipo": "hachura", "pontos": [[600, 0], [700, 0], [700, 100], [600, 100]], "padrao": "concreto" }] }));
    assert_eq!(h["criadas"][0]["tipo"], "hachura");
    let dir = std::env::temp_dir().join(format!("fcad_pdf_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = dir.join("t.pdf");
    let r = s.json("cad_pdf", json!({ "caminho": pdf.to_str().unwrap(), "papel": "A1" }));
    assert_eq!(r["paginas"], 1);
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF") && String::from_utf8_lossy(&bytes).contains("MediaBox [0 0 2383.937 1683.78]"), "A1 deitada");
    let (_, erro) = s.ferramenta("cad_pdf", json!({ "caminho": pdf.to_str().unwrap() }));
    assert!(erro, "não sobrescreve sem pedir");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn checklist_e_validacao_de_dominio_no_mcp() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "m" }));
    
    // Testar cad_verificar com dominio eletrica no desenho vazio -> deve falhar no checklist
    let ver = s.json("cad_verificar", json!({ "dominio": "eletrica" }));
    assert_eq!(ver["checklist_dominio"]["aprovado"], false);

    // Salvar com verificar_checklist: true e dominio: eletrica -> deve dar erro e BLOQUEAR o salvamento
    let tmp = std::env::temp_dir().join(format!("teste_chk_{}.dxf", std::process::id()));
    let (_, erro) = s.ferramenta("cad_salvar", json!({ "caminho": tmp.to_str().unwrap(), "verificar_checklist": true, "dominio": "eletrica" }));
    assert!(erro, "cad_salvar deve bloquear o salvamento quando o checklist do domínio falha");

    // Adiciona o QDC, pontos de luz e eletroduto
    s.json("cad_criar", json!({ "camada": "ELE_PONTOS", "formas": [
        { "tipo": "texto", "ponto": [1, 1], "altura": 0.2, "conteudo": "QDC GERAL" },
        { "tipo": "circulo", "centro": [5, 5], "raio": 0.2 }
    ] }));
    s.json("cad_criar", json!({ "camada": "ELE_ELETRODUTOS", "formas": [
        { "tipo": "polilinha", "pontos": [[1, 1], [5, 5]] }
    ] }));

    // Agora o checklist do domínio elétrica passa!
    let ver_ok = s.json("cad_verificar", json!({ "dominio": "eletrica" }));
    assert_eq!(ver_ok["checklist_dominio"]["aprovado"], true);

    // Salvar com verificar_checklist: true -> deve salvar com sucesso!
    let salvou = s.json("cad_salvar", json!({ "caminho": tmp.to_str().unwrap(), "verificar_checklist": true, "dominio": "eletrica" }));
    assert_eq!(salvou["ok"], true);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn topografia_mcp_gera_levantamento_e_abre_arquivo() {
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "m" }));

    let csv = "E,N,Z\n0,0,10\n10,0,12\n10,10,15\n0,10,11\n5,5,18\n";
    let res = s.json("cad_topografia", json!({ "texto": csv, "equidistancia": 1.0 }));
    assert_eq!(res["total_pontos"], 5);
    assert!(res["total_curvas"].as_u64().unwrap() > 0);
    assert!(res["entidades_criadas"].as_u64().unwrap() > 10);

    let desfeito = s.json("cad_desfazer", json!({}));
    assert_eq!(desfeito["feito"], true);

    let tmp = std::env::temp_dir().join(format!("teste_topo_{}.csv", std::process::id()));
    std::fs::write(&tmp, csv).unwrap();
    let abrir = s.json("cad_abrir", json!({ "caminho": tmp.to_str().unwrap() }));
    assert!(abrir["avisos"].as_array().unwrap().iter().any(|a| a.as_str().unwrap().contains("aviso.nuvemCarregada")));

    let resumo = s.json("cad_resumo", json!({}));
    assert!(resumo["camadas"].as_array().unwrap().iter().any(|c| c["nome"] == "TOPOGRAFIA_CURVAS_MESTRAS"));
    std::fs::remove_file(&tmp).ok();
}


#[test]
fn importar_do_disco_e_modo_projeto_com_ver_em_arquivo() {
    let pasta = std::env::temp_dir().join(format!("fcad_modo_{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    let fct = pasta.join("b.fct");
    std::fs::write(&fct, "camada EIXO #FF0000 esp=0.5 linha=eixo\nL 0 0 10 0\nT 1 1 2 0 \"x\" cm\n").unwrap();
    let mut s = Servidor::novo();
    s.json("cad_novo", json!({ "unidade": "m" }));
    s.json("cad_importar", json!({ "texto": "L 0 0 1 1\n" }));
    let r = s.json("cad_importar", json!({ "caminho": fct.to_str().unwrap() }));
    assert_eq!(r["criadas"], 2);
    assert_eq!(s.json("cad_resumo", json!({}))["total_de_entidades"], 3, "acrescenta, não substitui");
    s.json("cad_desfazer", json!({}));
    assert_eq!(s.json("cad_resumo", json!({}))["total_de_entidades"], 1, "um Ctrl+Z desfaz só o arquivo");
    let (_, erro) = s.ferramenta("cad_importar", json!({ "caminho": "/tmp/nao.dxf" }));
    assert!(erro);

    assert_eq!(s.json("cad_modo", json!({ "projeto": true }))["projeto"], true);
    let png = pasta.join("v.png");
    let l = s.json("cad_ver", json!({ "arquivo": png.to_str().unwrap() }));
    assert_eq!(l["pixels"][0], 800, "modo projeto: 800 px");
    assert!(std::fs::metadata(&png).unwrap().len() > 100);
    let (c, _) = s.ferramenta("cad_ver", json!({}));
    assert_eq!(c[0]["type"], "image", "sem `arquivo` a imagem segue na conversa");
    s.json("cad_modo", json!({ "projeto": false }));
    assert_eq!(s.json("cad_ver", json!({ "arquivo": png.to_str().unwrap() }))["pixels"][0], 1000);
    let _ = std::fs::remove_dir_all(&pasta);
}

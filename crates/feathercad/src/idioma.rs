//! Os textos da interface na língua escolhida, dos mesmos arquivos que o TypeScript usava (`src/idiomas/<código>.json`),
//! com o pt-BR (a referência) preenchendo o que a língua não traduziu. Nenhum texto de interface no código.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::Value;

const REFERENCIA: &str = "pt-BR";
const TEXTOS: [(&str, &str); 2] = [("pt-BR", include_str!("../../../src/idiomas/pt-BR.json")), ("en-US", include_str!("../../../src/idiomas/en-US.json"))];
const COMANDOS: [(&str, &str); 2] = [("pt-BR", include_str!("../../../src/idiomas/comandos.pt-BR.json")), ("en-US", include_str!("../../../src/idiomas/comandos.en-US.json"))];

static ATUAL: OnceLock<(String, Value)> = OnceLock::new();
static ESCOLHIDA: OnceLock<String> = OnceLock::new();

/// As línguas que existem: `(código, nome)` para a caixa das Configurações.
pub fn linguas() -> Vec<(&'static str, String)> {
    TEXTOS.iter().map(|(c, t)| (*c, serde_json::from_str::<Value>(t).ok().and_then(|v| v["idioma"]["nome"].as_str().map(str::to_string)).unwrap_or_else(|| c.to_string()))).collect()
}

/// A língua escolhida nas Configurações (vale na largada; trocar pede reabrir, como no TypeScript).
pub fn escolher(codigo: Option<&str>) {
    if let Some(c) = codigo.filter(|c| TEXTOS.iter().any(|(k, _)| k == c)) {
        let _ = ESCOLHIDA.set(c.to_string());
    }
}

pub fn codigo() -> &'static str {
    &atual().0
}

fn mesclar(base: &mut Value, por: Value) {
    match (base, por) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                mesclar(b.entry(k).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p,
    }
}

fn arquivo(lista: &[(&str, &'static str)], codigo: &str) -> Value {
    let achar = |c: &str| lista.iter().find(|(k, _)| *k == c).and_then(|(_, t)| serde_json::from_str(t).ok());
    achar(codigo).or_else(|| achar(REFERENCIA)).unwrap_or(Value::Null)
}

/// A língua do sistema (`LANG=en_US.UTF-8` → `en-US`), se houver arquivo para ela; senão a referência.
fn codigo_do_sistema() -> String {
    let lang = std::env::var("FEATHERCAD_IDIOMA").or_else(|_| std::env::var("LANG")).unwrap_or_default();
    let codigo = lang.split('.').next().unwrap_or("").replace('_', "-");
    if TEXTOS.iter().any(|(k, _)| *k == codigo) {
        return codigo;
    }
    let lingua = codigo.split('-').next().unwrap_or("");
    TEXTOS.iter().map(|(k, _)| *k).find(|k| k.starts_with(lingua) && !lingua.is_empty()).unwrap_or(REFERENCIA).to_string()
}

fn atual() -> &'static (String, Value) {
    ATUAL.get_or_init(|| {
        let codigo = ESCOLHIDA.get().cloned().unwrap_or_else(codigo_do_sistema);
        let mut t = arquivo(&TEXTOS, REFERENCIA);
        if codigo != REFERENCIA {
            mesclar(&mut t, arquivo(&TEXTOS, &codigo));
        }
        (codigo, t)
    })
}

/// O texto de uma chave com ponto (`rodape.orto`), com `{variáveis}` trocadas; a própria chave se não existir.
pub fn texto_com(chave: &str, vars: &BTreeMap<String, String>) -> String {
    let mut no = &atual().1;
    for parte in chave.split('.') {
        no = &no[parte];
    }
    let Some(s) = no.as_str() else { return chave.to_string() };
    let mut saida = s.to_string();
    for (k, v) in vars {
        saida = saida.replace(&format!("{{{k}}}"), v);
    }
    saida
}

pub fn t(chave: &str) -> String {
    texto_com(chave, &BTreeMap::new())
}

/// `t` com variáveis em linha: `tv("erro.abrir", &[("motivo", &e)])`.
pub fn tv(chave: &str, vars: &[(&str, &str)]) -> String {
    texto_com(chave, &vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
}

/// A tabela de apelidos da língua (sem os do usuário): `comando → [apelidos]`.
pub fn apelidos_padrao() -> BTreeMap<String, Vec<String>> {
    serde_json::from_value(arquivo(&COMANDOS, &atual().0)["comandos"].clone()).unwrap_or_default()
}

/// O que o motor precisa da língua: separador decimal, de coordenadas, casas e a tabela de comandos (`idioma_definir`),
/// com os apelidos do usuário somados aos da língua.
pub fn para_o_motor(extras: &BTreeMap<String, Vec<String>>) -> Value {
    let i = &atual().1["idioma"];
    let regional = serde_json::json!({ "decimal": i["decimal"], "separador_de_coordenadas": i["separadorDeCoordenadas"], "casas": i["casas"] });
    let tabela = arquivo(&COMANDOS, &atual().0);
    let mut comandos = apelidos_padrao();
    for (id, nomes) in comandos.iter_mut() {
        for a in extras.get(id).into_iter().flatten() {
            let a = a.trim().to_uppercase();
            if !a.is_empty() && !nomes.contains(&a) {
                nomes.push(a);
            }
        }
    }
    serde_json::json!({ "regional": regional, "comandos": comandos, "opcoes": tabela["opcoes"] })
}

#[cfg(test)]
mod testes {
    #[test]
    fn chaves_variaveis_e_falta() {
        assert!(super::tv("erro.abrir", &[("motivo", "X")]).contains('X'));
        assert_eq!(super::t("nao.existe"), "nao.existe");
        let m = super::para_o_motor(&Default::default());
        assert!(m["comandos"]["linha"].is_array(), "a tabela de comandos vai ao motor");
    }

    #[test]
    fn o_motor_aceita_a_lingua() {
        let mut p = feathercad_ponte::Ponte::default();
        let mut extras = std::collections::BTreeMap::new();
        extras.insert("mover".to_string(), vec!["g".to_string()]);
        p.chamar("idioma_definir", super::para_o_motor(&extras).to_string().as_bytes()).unwrap();
        p.chamar("digitar", br#"{"texto":"G"}"#).unwrap();
        let e: serde_json::Value = serde_json::from_slice(&p.chamar("estado", b"").unwrap()).unwrap();
        assert_eq!(e["ferramenta"], "mover", "o apelido do usuário vale");
    }
}

//! `cad_lote`: muitas ferramentas numa chamada só, com **dados** e **laços declarativos**.
//!
//! ```json
//! { "dados": { "luzes": [[250,200,"a"]], "X": 300 },
//!   "passos": [ { "para_cada": "luzes", "como": "l",
//!                 "faca": [ { "ferramenta": "cad_simbolo",
//!                             "argumentos": { "nome": "ponto_luz_teto", "ponto": "=[X + l[0], l[1]]", "comando": "=l[2]" } } ] } ] }
//! ```
//!
//! Passos: `ferramenta` (+ `argumentos`, `guardar`), `definir` (+ `valor`), `para_cada` (+ `como`, `indice`, `faca`),
//! `repetir` (+ `como`, `faca`), `se` (+ `entao`, `senao`) e `exigir` (+ `mensagem`). Texto que começa com `=` é uma
//! expressão (ver `expressao.rs`); `${expr}` dentro de um texto é trocado pelo valor. Tudo ou nada: se um passo falha,
//! o desenho volta ao que era, e o que deu certo vira **um** passo de desfazer.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::agente::Agente;
use crate::expressao::{avaliar, mostrar, numero, verdadeiro, Escopo};
use crate::ferramentas::chamar;
use crate::fonte::Fonte;
use crate::json::Res;

/// Ferramentas que não cabem num lote (imagem, recursão, ou que troca/zera o desenho e o histórico).
const PROIBIDAS: [&str; 5] = ["cad_lote", "cad_ver", "cad_desfazer", "cad_novo", "cad_abrir"];
const PROFUNDIDADE_MAX: usize = 16;
const ITERACOES_MAX: usize = 200_000;
const CHAMADAS_MAX: usize = 50_000;
/// Toda mensagem de erro de passo começa assim (para não ser prefixada duas vezes).
const MARCA: &str = "passo ";

struct Contexto<'a> {
    ag: &'a mut Agente,
    fonte: &'a Fonte,
    esc: Escopo,
    chamadas: usize,
    iteracoes: usize,
    por_ferramenta: BTreeMap<String, usize>,
    /// Onde estamos (`[3].faca[1]`), para as mensagens de erro.
    trilha: Vec<String>,
}

/// Troca expressões (`=…`) e interpolações (`${…}`) pelos valores, em qualquer profundidade.
fn resolver(v: &Value, esc: &Escopo) -> Res<Value> {
    match v {
        Value::String(s) if s.starts_with("==") => Ok(Value::String(s[1..].to_string())),
        Value::String(s) if s.starts_with('=') => avaliar(&s[1..], esc),
        Value::String(s) if s.contains("${") => interpolar(s, esc),
        Value::Array(l) => l.iter().map(|x| resolver(x, esc)).collect::<Res<Vec<_>>>().map(Value::Array),
        Value::Object(o) => o.iter().map(|(k, x)| resolver(x, esc).map(|r| (k.clone(), r))).collect::<Res<Map<_, _>>>().map(Value::Object),
        outro => Ok(outro.clone()),
    }
}

fn interpolar(s: &str, esc: &Escopo) -> Res<Value> {
    let mut saida = String::new();
    let mut resto = s;
    while let Some(ini) = resto.find("${") {
        saida.push_str(&resto[..ini]);
        let fim = resto[ini..].find('}').ok_or_else(|| format!("`${{` sem `}}` em `{s}`"))? + ini;
        saida.push_str(&mostrar(&avaliar(&resto[ini + 2..fim], esc)?));
        resto = &resto[fim + 1..];
    }
    saida.push_str(resto);
    Ok(Value::String(saida))
}

fn texto_do_passo<'a>(p: &'a Value, campo: &str) -> Option<&'a str> {
    p.get(campo).and_then(Value::as_str)
}

fn sublista<'a>(p: &'a Value, campo: &str) -> Res<&'a [Value]> {
    match p.get(campo) {
        None => Ok(&[]),
        Some(Value::Array(l)) => Ok(l),
        Some(_) => Err(format!("`{campo}` precisa ser uma lista de passos")),
    }
}

impl Contexto<'_> {
    fn local(&self) -> String {
        format!("{MARCA}{}", self.trilha.join(""))
    }

    fn contar_iteracao(&mut self) -> Res<()> {
        self.iteracoes += 1;
        if self.iteracoes > ITERACOES_MAX {
            return Err(format!("laço demais (máximo {ITERACOES_MAX} voltas no lote)"));
        }
        Ok(())
    }

    fn lista_de_passos(&mut self, passos: &[Value], prof: usize) -> Res<()> {
        if prof > PROFUNDIDADE_MAX {
            return Err(format!("laços aninhados demais (máximo {PROFUNDIDADE_MAX})"));
        }
        for (i, p) in passos.iter().enumerate() {
            self.trilha.push(format!("[{i}]"));
            self.passo(p, prof).map_err(|e| if e.starts_with(MARCA) { e } else { format!("{}: {e}", self.local()) })?;
            self.trilha.pop();
        }
        Ok(())
    }

    fn passo(&mut self, p: &Value, prof: usize) -> Res<()> {
        if !p.is_object() {
            return Err("cada passo é um objeto".into());
        }
        if p.get("ferramenta").is_some() {
            self.ferramenta(p)
        } else if let Some(nome) = texto_do_passo(p, "definir") {
            let v = resolver(p.get("valor").ok_or("`definir` pede `valor`")?, &self.esc)?;
            self.esc.atribuir(nome, v);
            Ok(())
        } else if p.get("para_cada").is_some() {
            self.para_cada(p, prof)
        } else if p.get("repetir").is_some() {
            self.repetir(p, prof)
        } else if p.get("se").is_some() {
            let cond = resolver(&p["se"], &self.esc)?;
            self.lista_de_passos(sublista(p, if verdadeiro(&cond) { "entao" } else { "senao" })?, prof + 1)
        } else if p.get("exigir").is_some() {
            self.exigir(p)
        } else {
            Err("passo sem `ferramenta`, `definir`, `para_cada`, `repetir`, `se` ou `exigir`".into())
        }
    }

    fn exigir(&self, p: &Value) -> Res<()> {
        if verdadeiro(&resolver(&p["exigir"], &self.esc)?) {
            return Ok(());
        }
        let msg = p.get("mensagem").map(|m| resolver(m, &self.esc)).transpose()?.map_or_else(|| "condição de `exigir` não vale".to_string(), |m| mostrar(&m));
        Err(msg)
    }

    fn ferramenta(&mut self, p: &Value) -> Res<()> {
        let nome = texto_do_passo(p, "ferramenta").ok_or("`ferramenta` precisa ser o nome (texto)")?;
        if PROIBIDAS.contains(&nome) {
            return Err(format!("`{nome}` não pode ser usada dentro de um lote"));
        }
        let args = resolver(p.get("argumentos").unwrap_or(&json!({})), &self.esc)?;
        if nome == "cad_importar" && args.get("substituir").and_then(Value::as_bool).unwrap_or(false) {
            return Err("`cad_importar` com `substituir` apaga o desenho e o histórico: não cabe num lote".into());
        }
        self.chamadas += 1;
        if self.chamadas > CHAMADAS_MAX {
            return Err(format!("chamadas demais (máximo {CHAMADAS_MAX} por lote)"));
        }
        *self.por_ferramenta.entry(nome.to_string()).or_default() += 1;
        let conteudo = chamar(self.ag, self.fonte, nome, &args).map_err(|e| format!("{nome}: {e}"))?;
        if let Some(guardar) = texto_do_passo(p, "guardar") {
            let texto = conteudo.iter().find_map(|b| b.get("text").and_then(Value::as_str)).unwrap_or("");
            self.esc.atribuir(guardar, serde_json::from_str(texto).unwrap_or_else(|_| Value::String(texto.to_string())));
        }
        Ok(())
    }

    /// A lista de um `para_cada`: uma expressão (`=…`), o nome de uma variável, ou a lista escrita ali.
    fn itens(&self, alvo: &Value) -> Res<Vec<Value>> {
        let v = match alvo {
            Value::String(s) if !s.starts_with('=') => self.esc.obter(s).cloned().ok_or_else(|| format!("a variável `{s}` não existe"))?,
            outro => resolver(outro, &self.esc)?,
        };
        match v {
            Value::Array(l) => Ok(l),
            Value::Object(o) => Ok(o.into_iter().map(|(k, x)| json!([k, x])).collect()),
            outro => Err(format!("`para_cada` precisa de uma lista, veio {}", mostrar(&outro))),
        }
    }

    fn volta(&mut self, p: &Value, como: &str, item: Value, i: usize, prof: usize) -> Res<()> {
        self.contar_iteracao()?;
        self.esc.empilhar();
        self.esc.declarar(como, item);
        self.esc.declarar(texto_do_passo(p, "indice").unwrap_or("indice"), Value::from(i));
        self.trilha.push(format!("(volta {i})"));
        let r = self.lista_de_passos(sublista(p, "faca")?, prof + 1);
        if r.is_ok() {
            self.trilha.pop();
        }
        self.esc.desempilhar();
        r
    }

    fn para_cada(&mut self, p: &Value, prof: usize) -> Res<()> {
        let como = texto_do_passo(p, "como").unwrap_or("item").to_string();
        for (i, item) in self.itens(&p["para_cada"])?.into_iter().enumerate() {
            self.volta(p, &como, item, i, prof)?;
        }
        Ok(())
    }

    fn repetir(&mut self, p: &Value, prof: usize) -> Res<()> {
        let n = resolver(&p["repetir"], &self.esc)?.as_f64().ok_or("`repetir` precisa de um número")?;
        if !(0.0..=100_000.0).contains(&n) {
            return Err("`repetir` vai de 0 a 100000".into());
        }
        let como = texto_do_passo(p, "como").unwrap_or("i").to_string();
        for i in 0..n as usize {
            self.volta(p, &como, Value::from(i), i, prof)?;
        }
        Ok(())
    }
}

/// Executa o lote. Erro = nada foi aplicado (o desenho volta ao que era; arquivos já gravados por `cad_salvar` ficam).
pub fn lote(ag: &mut Agente, fonte: &Fonte, args: &Value) -> Res<Value> {
    let passos = args.get("passos").and_then(Value::as_array).ok_or("falta `passos` (lista de passos)")?;
    let dados = args.get("dados").cloned().unwrap_or_else(|| json!({}));
    if !dados.is_object() {
        return Err("`dados` precisa ser um objeto {nome: valor}".into());
    }
    let (marca, antes) = (ag.doc.marca_do_historico(), ag.doc.entidades().len());
    let mut ctx = Contexto { ag, fonte, esc: Escopo::novo(&dados), chamadas: 0, iteracoes: 0, por_ferramenta: BTreeMap::new(), trilha: Vec::new() };
    if let Err(e) = ctx.lista_de_passos(passos, 0) {
        ctx.ag.doc.desfazer_ate(marca);
        return Err(format!("{e} — nada foi aplicado"));
    }
    let mut resposta = json!({ "ok": true, "chamadas": ctx.chamadas, "voltas": ctx.iteracoes, "por_ferramenta": ctx.por_ferramenta });
    if let Some(nomes) = args.get("retornar").and_then(Value::as_array) {
        let mut saida = Map::new();
        for n in nomes.iter().filter_map(Value::as_str) {
            saida.insert(n.to_string(), ctx.esc.obter(n).cloned().ok_or_else(|| format!("`retornar`: a variável `{n}` não existe"))?);
        }
        resposta["variaveis"] = Value::Object(saida);
    }
    ctx.ag.doc.juntar_historico(marca);
    let depois = ctx.ag.doc.entidades().len();
    resposta["entidades_novas"] = numero(depois as f64 - antes as f64)?;
    Ok(resposta)
}

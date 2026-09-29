//! As expressões do `cad_lote`: uma calculadora pequena e **sem laço**, para os dados virarem coordenadas.
//!
//! Valores são JSON (número, texto, lista, objeto, verdadeiro/falso, nulo). Operadores: `+ - * / %`, comparações,
//! `&& || !`, `a ? b : c`, índice `l[0]`, campo `c.nome`, lista literal `[x, y]`. Aritmética com listas de números
//! vale elemento a elemento (`[1,2] + [10,20]`, `[3,4] * 2`), então um ponto se soma a outro. `+` com texto junta.
//! As funções estão em [`FUNCOES`] (em português; os ângulos são em graus).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::json::Res;

/// As variáveis visíveis, em camadas: cada laço abre uma e fecha ao terminar.
pub struct Escopo {
    camadas: Vec<BTreeMap<String, Value>>,
}

impl Escopo {
    /// A camada de base já traz os `dados` (um objeto: cada chave vira uma variável).
    pub fn novo(dados: &Value) -> Self {
        let base = dados.as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        Self { camadas: vec![base] }
    }

    pub fn empilhar(&mut self) {
        self.camadas.push(BTreeMap::new());
    }

    pub fn desempilhar(&mut self) {
        if self.camadas.len() > 1 {
            self.camadas.pop();
        }
    }

    /// Cria a variável na camada de cima (a variável do laço, que some quando ele acaba).
    pub fn declarar(&mut self, nome: &str, valor: Value) {
        if let Some(topo) = self.camadas.last_mut() {
            topo.insert(nome.to_string(), valor);
        }
    }

    /// Muda a variável onde ela já existe; se não existe, cria na camada de cima.
    pub fn atribuir(&mut self, nome: &str, valor: Value) {
        match self.camadas.iter_mut().rev().find(|c| c.contains_key(nome)) {
            Some(c) => {
                c.insert(nome.to_string(), valor);
            }
            None => self.declarar(nome, valor),
        }
    }

    pub fn obter(&self, nome: &str) -> Option<&Value> {
        self.camadas.iter().rev().find_map(|c| c.get(nome))
    }
}

// ---------------------------------------------------------------- valores

/// Um número como JSON: inteiro quando é inteiro (as ferramentas leem `circuito: 3` com `as_u64`).
pub fn numero(v: f64) -> Res<Value> {
    if !v.is_finite() {
        return Err("a conta deu um número infinito ou indefinido".into());
    }
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        return Ok(Value::from(v as i64));
    }
    Ok(Value::from(v))
}

fn num(v: &Value) -> Res<f64> {
    v.as_f64().ok_or_else(|| format!("esperava um número, veio {}", mostrar(v)))
}

fn lista(v: &Value) -> Res<&Vec<Value>> {
    v.as_array().ok_or_else(|| format!("esperava uma lista, veio {}", mostrar(v)))
}

fn ponto(v: &Value) -> Res<(f64, f64)> {
    match lista(v)?.as_slice() {
        [x, y, ..] => Ok((num(x)?, num(y)?)),
        _ => Err(format!("esperava um ponto [x, y], veio {}", mostrar(v))),
    }
}

fn caminho(v: &Value) -> Res<Vec<(f64, f64)>> {
    lista(v)?.iter().map(ponto).collect()
}

fn de_ponto((x, y): (f64, f64)) -> Res<Value> {
    Ok(Value::Array(vec![numero(x)?, numero(y)?]))
}

/// O valor como texto de gente: `5`, `1.5`, `Sala`; lista e objeto saem como JSON.
pub fn mostrar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() != 0.0 => format!("{f:.6}").trim_end_matches('0').trim_end_matches('.').to_string(),
            Some(f) => format!("{f:.0}"),
            None => n.to_string(),
        },
        outro => outro.to_string(),
    }
}

/// Falso: `false`, nulo, `0`, texto vazio e lista vazia.
pub fn verdadeiro(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

// ---------------------------------------------------------------- análise

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Texto(String),
    Nome(String),
    Op(&'static str),
}

const OPERADORES: [&str; 22] = ["==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "(", ")", "[", "]", ",", ".", "!", "<", ">", "?", ":"];

fn ler_numero(c: &[char], i: &mut usize) -> Res<Tok> {
    let ini = *i;
    while *i < c.len() && (c[*i].is_ascii_digit() || (c[*i] == '.' && c.get(*i + 1).is_some_and(char::is_ascii_digit))) {
        *i += 1;
    }
    let t: String = c[ini..*i].iter().collect();
    t.parse().map(Tok::Num).map_err(|_| format!("número inválido `{t}`"))
}

fn ler_texto(c: &[char], i: &mut usize) -> Res<Tok> {
    let aspas = c[*i];
    *i += 1;
    let mut s = String::new();
    while *i < c.len() && c[*i] != aspas {
        if c[*i] == '\\' && *i + 1 < c.len() {
            *i += 1;
        }
        s.push(c[*i]);
        *i += 1;
    }
    if *i >= c.len() {
        return Err("texto sem aspas de fechamento".into());
    }
    *i += 1;
    Ok(Tok::Texto(s))
}

fn lexar(fonte: &str) -> Res<Vec<Tok>> {
    let c: Vec<char> = fonte.chars().collect();
    let (mut i, mut toks) = (0, Vec::new());
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() {
            toks.push(ler_numero(&c, &mut i)?);
        } else if ch == '"' || ch == '\'' {
            toks.push(ler_texto(&c, &mut i)?);
        } else if ch.is_alphabetic() || ch == '_' {
            let ini = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            toks.push(Tok::Nome(c[ini..i].iter().collect()));
        } else {
            let resto: String = c[i..c.len().min(i + 2)].iter().collect();
            let op = OPERADORES.iter().find(|o| resto.starts_with(**o)).ok_or_else(|| format!("caractere inesperado `{ch}`"))?;
            toks.push(Tok::Op(op));
            i += op.len();
        }
    }
    Ok(toks)
}

#[derive(Debug)]
enum Ex {
    Lit(Value),
    Var(String),
    Unario(&'static str, Box<Ex>),
    Binario(&'static str, Box<Ex>, Box<Ex>),
    Ternario(Box<Ex>, Box<Ex>, Box<Ex>),
    Indice(Box<Ex>, Box<Ex>),
    Campo(Box<Ex>, String),
    Chama(String, Vec<Ex>),
    Lista(Vec<Ex>),
}

struct Analisador {
    toks: Vec<Tok>,
    pos: usize,
}

/// Os níveis de precedência binária, do que amarra menos ao que amarra mais.
const NIVEIS: [&[&str]; 5] = [&["||"], &["&&"], &["==", "!="], &["<", "<=", ">", ">="], &["+", "-"]];

impl Analisador {
    fn espiar(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn e_op(&self, op: &str) -> bool {
        matches!(self.espiar(), Some(Tok::Op(o)) if *o == op)
    }

    fn aceitar(&mut self, op: &str) -> bool {
        let e = self.e_op(op);
        if e {
            self.pos += 1;
        }
        e
    }

    fn exigir(&mut self, op: &str) -> Res<()> {
        if self.aceitar(op) { Ok(()) } else { Err(format!("esperava `{op}`")) }
    }

    fn expressao(&mut self) -> Res<Ex> {
        let cond = self.binario(0)?;
        if !self.aceitar("?") {
            return Ok(cond);
        }
        let (sim, nao) = (self.expressao()?, {
            self.exigir(":")?;
            self.expressao()?
        });
        Ok(Ex::Ternario(Box::new(cond), Box::new(sim), Box::new(nao)))
    }

    /// Um nível da tabela `NIVEIS`; depois do último vêm `* / %`.
    fn binario(&mut self, nivel: usize) -> Res<Ex> {
        if nivel == NIVEIS.len() {
            return self.multiplicativo();
        }
        let mut esq = self.binario(nivel + 1)?;
        while let Some(op) = NIVEIS[nivel].iter().find(|o| self.e_op(o)).copied() {
            self.pos += 1;
            esq = Ex::Binario(op, Box::new(esq), Box::new(self.binario(nivel + 1)?));
        }
        Ok(esq)
    }

    fn multiplicativo(&mut self) -> Res<Ex> {
        let mut esq = self.unario()?;
        while let Some(op) = ["*", "/", "%"].into_iter().find(|o| self.e_op(o)) {
            self.pos += 1;
            esq = Ex::Binario(op, Box::new(esq), Box::new(self.unario()?));
        }
        Ok(esq)
    }

    fn unario(&mut self) -> Res<Ex> {
        for op in ["-", "!"] {
            if self.aceitar(op) {
                return Ok(Ex::Unario(if op == "-" { "-" } else { "!" }, Box::new(self.unario()?)));
            }
        }
        self.posfixo()
    }

    fn posfixo(&mut self) -> Res<Ex> {
        let mut e = self.primario()?;
        loop {
            if self.aceitar("[") {
                let i = self.expressao()?;
                self.exigir("]")?;
                e = Ex::Indice(Box::new(e), Box::new(i));
            } else if self.aceitar(".") {
                match self.toks.get(self.pos).cloned() {
                    Some(Tok::Nome(n)) => {
                        self.pos += 1;
                        e = Ex::Campo(Box::new(e), n);
                    }
                    _ => return Err("depois do `.` vem o nome de um campo".into()),
                }
            } else {
                return Ok(e);
            }
        }
    }

    fn argumentos(&mut self, fim: &str) -> Res<Vec<Ex>> {
        let mut v = Vec::new();
        if self.aceitar(fim) {
            return Ok(v);
        }
        loop {
            v.push(self.expressao()?);
            if self.aceitar(fim) {
                return Ok(v);
            }
            self.exigir(",")?;
        }
    }

    fn primario(&mut self) -> Res<Ex> {
        let Some(t) = self.toks.get(self.pos).cloned() else { return Err("a expressão terminou antes da hora".into()) };
        self.pos += 1;
        match t {
            Tok::Num(n) => Ok(Ex::Lit(numero(n)?)),
            Tok::Texto(s) => Ok(Ex::Lit(Value::String(s))),
            Tok::Nome(n) if self.aceitar("(") => Ok(Ex::Chama(n, self.argumentos(")")?)),
            Tok::Nome(n) => Ok(match n.as_str() {
                "verdadeiro" => Ex::Lit(Value::Bool(true)),
                "falso" => Ex::Lit(Value::Bool(false)),
                "nulo" => Ex::Lit(Value::Null),
                "pi" => Ex::Lit(Value::from(std::f64::consts::PI)),
                _ => Ex::Var(n),
            }),
            Tok::Op("(") => {
                let e = self.expressao()?;
                self.exigir(")")?;
                Ok(e)
            }
            Tok::Op("[") => Ok(Ex::Lista(self.argumentos("]")?)),
            Tok::Op(o) => Err(format!("`{o}` não cabe aqui")),
        }
    }
}

fn analisar(fonte: &str) -> Res<Ex> {
    let mut a = Analisador { toks: lexar(fonte)?, pos: 0 };
    let e = a.expressao()?;
    match a.espiar() {
        None => Ok(e),
        Some(t) => Err(format!("sobrou `{}` no fim da expressão", if let Tok::Op(o) = t { (*o).to_string() } else { format!("{t:?}") })),
    }
}

// ---------------------------------------------------------------- conta

/// Aritmética com número ou lista de números (elemento a elemento; número com lista espalha).
fn aritmetica(a: &Value, b: &Value, f: fn(f64, f64) -> Res<f64>) -> Res<Value> {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x.iter().zip(y).map(|(p, q)| aritmetica(p, q, f)).collect::<Res<Vec<_>>>().map(Value::Array),
        (Value::Array(x), Value::Array(y)) => Err(format!("listas de tamanhos diferentes ({} e {})", x.len(), y.len())),
        (Value::Array(x), n) => x.iter().map(|p| aritmetica(p, n, f)).collect::<Res<Vec<_>>>().map(Value::Array),
        (n, Value::Array(y)) => y.iter().map(|q| aritmetica(n, q, f)).collect::<Res<Vec<_>>>().map(Value::Array),
        (p, q) => numero(f(num(p)?, num(q)?)?),
    }
}

fn dividir(a: f64, b: f64) -> Res<f64> {
    if b == 0.0 { Err("divisão por zero".into()) } else { Ok(a / b) }
}

fn ordem(a: &Value, b: &Value) -> Res<std::cmp::Ordering> {
    match (a, b) {
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        _ => num(a)?.partial_cmp(&num(b)?).ok_or_else(|| "não dá para comparar".to_string()),
    }
}

fn igual(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

fn binaria(op: &str, a: &Value, b: &Value) -> Res<Value> {
    use std::cmp::Ordering::{Greater, Less};
    Ok(match op {
        "+" if matches!(a, Value::String(_)) || matches!(b, Value::String(_)) => Value::String(format!("{}{}", mostrar(a), mostrar(b))),
        "+" => aritmetica(a, b, |x, y| Ok(x + y))?,
        "-" => aritmetica(a, b, |x, y| Ok(x - y))?,
        "*" => aritmetica(a, b, |x, y| Ok(x * y))?,
        "/" => aritmetica(a, b, dividir)?,
        "%" => aritmetica(a, b, |x, y| dividir(x, y).map(|_| x % y))?,
        "==" => Value::Bool(igual(a, b)),
        "!=" => Value::Bool(!igual(a, b)),
        "<" => Value::Bool(ordem(a, b)? == Less),
        "<=" => Value::Bool(ordem(a, b)? != Greater),
        ">" => Value::Bool(ordem(a, b)? == Greater),
        ">=" => Value::Bool(ordem(a, b)? != Less),
        _ => return Err(format!("operador `{op}` desconhecido")),
    })
}

fn indexar(base: &Value, i: &Value) -> Res<Value> {
    match (base, i) {
        (Value::Array(l), n) if n.is_number() => {
            let k = num(n)? as i64;
            let pos = if k < 0 { k + l.len() as i64 } else { k };
            usize::try_from(pos).ok().and_then(|p| l.get(p)).cloned().ok_or_else(|| format!("posição {k} fora da lista (tem {} itens)", l.len()))
        }
        (Value::Object(o), Value::String(k)) => o.get(k).cloned().ok_or_else(|| format!("não existe o campo `{k}`")),
        _ => Err(format!("não dá para indexar {} com {}", mostrar(base), mostrar(i))),
    }
}

fn calcular(e: &Ex, esc: &Escopo) -> Res<Value> {
    match e {
        Ex::Lit(v) => Ok(v.clone()),
        Ex::Var(n) => esc.obter(n).cloned().ok_or_else(|| format!("a variável `{n}` não existe")),
        Ex::Lista(itens) => itens.iter().map(|i| calcular(i, esc)).collect::<Res<Vec<_>>>().map(Value::Array),
        Ex::Unario(op, x) => {
            let v = calcular(x, esc)?;
            if *op == "!" { Ok(Value::Bool(!verdadeiro(&v))) } else { numero(-num(&v)?) }
        }
        Ex::Binario("&&", a, b) => {
            let va = calcular(a, esc)?;
            if verdadeiro(&va) { calcular(b, esc) } else { Ok(va) }
        }
        Ex::Binario("||", a, b) => {
            let va = calcular(a, esc)?;
            if verdadeiro(&va) { Ok(va) } else { calcular(b, esc) }
        }
        Ex::Binario(op, a, b) => binaria(op, &calcular(a, esc)?, &calcular(b, esc)?),
        Ex::Ternario(c, a, b) => calcular(if verdadeiro(&calcular(c, esc)?) { a } else { b }, esc),
        Ex::Indice(b, i) => indexar(&calcular(b, esc)?, &calcular(i, esc)?),
        Ex::Campo(b, n) => indexar(&calcular(b, esc)?, &Value::String(n.clone())),
        Ex::Chama(nome, args) => {
            let f = FUNCOES.iter().find(|(n, _)| n == nome).ok_or_else(|| format!("a função `{nome}` não existe"))?.1;
            let valores = args.iter().map(|a| calcular(a, esc)).collect::<Res<Vec<_>>>()?;
            f(&valores).map_err(|m| format!("{nome}(): {m}"))
        }
    }
}

/// Calcula uma expressão no escopo dado.
pub fn avaliar(fonte: &str, esc: &Escopo) -> Res<Value> {
    calcular(&analisar(fonte).map_err(|m| format!("expressão `{fonte}`: {m}"))?, esc).map_err(|m| format!("expressão `{fonte}`: {m}"))
}

// ---------------------------------------------------------------- funções

type Funcao = fn(&[Value]) -> Res<Value>;

fn exatamente(a: &[Value], n: usize) -> Res<()> {
    if a.len() == n { Ok(()) } else { Err(format!("pede {n} argumento(s), vieram {}", a.len())) }
}

fn um_numero(a: &[Value], f: fn(f64) -> f64) -> Res<Value> {
    exatamente(a, 1)?;
    numero(f(num(&a[0])?))
}

/// Números soltos ou uma lista deles.
fn numeros(a: &[Value]) -> Res<Vec<f64>> {
    let itens: &[Value] = match a {
        [Value::Array(l)] => l,
        _ => a,
    };
    if itens.is_empty() {
        return Err("precisa de pelo menos um número".into());
    }
    itens.iter().map(num).collect()
}

fn f_min(a: &[Value]) -> Res<Value> {
    numero(numeros(a)?.into_iter().fold(f64::INFINITY, f64::min))
}

fn f_max(a: &[Value]) -> Res<Value> {
    numero(numeros(a)?.into_iter().fold(f64::NEG_INFINITY, f64::max))
}

fn f_soma(a: &[Value]) -> Res<Value> {
    numero(match a {
        [Value::Array(l)] if l.is_empty() => 0.0,
        _ => numeros(a)?.iter().sum(),
    })
}

fn f_media(a: &[Value]) -> Res<Value> {
    let n = numeros(a)?;
    numero(n.iter().sum::<f64>() / n.len() as f64)
}

fn f_arredondar(a: &[Value]) -> Res<Value> {
    let casas = a.get(1).map_or(Ok(0.0), num)?.clamp(0.0, 12.0) as i32;
    let f = 10f64.powi(casas);
    numero((num(a.first().ok_or("falta o número")?)? * f).round() / f)
}

fn f_potencia(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    numero(num(&a[0])?.powf(num(&a[1])?))
}

fn f_atan2(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    numero(num(&a[0])?.atan2(num(&a[1])?).to_degrees())
}

fn f_dist(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    let (p, q) = (ponto(&a[0])?, ponto(&a[1])?);
    numero((p.0 - q.0).hypot(p.1 - q.1))
}

fn f_angulo(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    let (p, q) = (ponto(&a[0])?, ponto(&a[1])?);
    numero((q.1 - p.1).atan2(q.0 - p.0).to_degrees())
}

fn f_ponto_polar(a: &[Value]) -> Res<Value> {
    exatamente(a, 3)?;
    let (o, ang, d) = (ponto(&a[0])?, num(&a[1])?.to_radians(), num(&a[2])?);
    de_ponto((o.0 + d * ang.cos(), o.1 + d * ang.sin()))
}

fn f_ponto_medio(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    let (p, q) = (ponto(&a[0])?, ponto(&a[1])?);
    de_ponto(((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0))
}

fn f_comprimento(a: &[Value]) -> Res<Value> {
    exatamente(a, 1)?;
    let c = caminho(&a[0])?;
    numero(c.windows(2).map(|s| (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1)).sum())
}

/// A polilinha deslocada `d` para a esquerda de quem anda (`d` negativo = direita), com quina em esquadro.
fn f_deslocar(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    let (c, d) = (caminho(&a[0])?, num(&a[1])?);
    if c.len() < 2 {
        return Err("o caminho precisa de pelo menos 2 pontos".into());
    }
    let mut trechos = Vec::new();
    for s in c.windows(2) {
        let (dx, dy) = (s[1].0 - s[0].0, s[1].1 - s[0].1);
        let l = dx.hypot(dy);
        if l < 1e-12 {
            return Err("há dois pontos iguais em sequência".into());
        }
        let (nx, ny) = (-dy / l * d, dx / l * d);
        trechos.push(((s[0].0 + nx, s[0].1 + ny), (s[1].0 + nx, s[1].1 + ny)));
    }
    let mut saida = vec![trechos[0].0];
    for par in trechos.windows(2) {
        saida.push(quina(par[0], par[1]));
    }
    saida.push(trechos[trechos.len() - 1].1);
    saida.into_iter().map(de_ponto).collect::<Res<Vec<_>>>().map(Value::Array)
}

/// Onde se encontram as retas dos dois trechos (se são paralelas, o fim do primeiro).
fn quina(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> (f64, f64) {
    let ((x1, y1), (x2, y2), (x3, y3), (x4, y4)) = (a.0, a.1, b.0, b.1);
    let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if den.abs() < 1e-9 {
        return a.1;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den;
    (x1 + t * (x2 - x1), y1 + t * (y2 - y1))
}

fn f_intervalo(a: &[Value]) -> Res<Value> {
    let (de, ate) = match a {
        [n] => (0, num(n)? as i64),
        [x, y] => (num(x)? as i64, num(y)? as i64),
        _ => return Err("pede intervalo(n) ou intervalo(de, ate)".into()),
    };
    if ate - de > 100_000 {
        return Err("intervalo grande demais (máximo 100000 itens)".into());
    }
    Ok(Value::Array((de..ate).map(Value::from).collect()))
}

fn f_tamanho(a: &[Value]) -> Res<Value> {
    exatamente(a, 1)?;
    numero(match &a[0] {
        Value::Array(l) => l.len(),
        Value::String(s) => s.chars().count(),
        Value::Object(o) => o.len(),
        outro => return Err(format!("{} não tem tamanho", mostrar(outro))),
    } as f64)
}

fn f_texto(a: &[Value]) -> Res<Value> {
    exatamente(a, 1)?;
    Ok(Value::String(mostrar(&a[0])))
}

fn f_numero(a: &[Value]) -> Res<Value> {
    exatamente(a, 1)?;
    match &a[0] {
        Value::String(s) => numero(s.trim().replace(',', ".").parse().map_err(|_| format!("`{s}` não é um número"))?),
        v => numero(num(v)?),
    }
}

/// O número com `casas` decimais e vírgula (`1234.5` → `1234,50`).
fn f_virgula(a: &[Value]) -> Res<Value> {
    let casas = a.get(1).map_or(Ok(1.0), num)?.clamp(0.0, 12.0) as usize;
    Ok(Value::String(format!("{:.casas$}", num(a.first().ok_or("falta o número")?)?).replace('.', ",")))
}

fn f_se(a: &[Value]) -> Res<Value> {
    exatamente(a, 3)?;
    Ok(if verdadeiro(&a[0]) { a[1].clone() } else { a[2].clone() })
}

fn f_juntar(a: &[Value]) -> Res<Value> {
    let mut saida = Vec::new();
    for l in a {
        saida.extend(lista(l)?.iter().cloned());
    }
    Ok(Value::Array(saida))
}

fn f_contem(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    Ok(Value::Bool(lista(&a[0])?.iter().any(|x| igual(x, &a[1]))))
}

fn f_inverter(a: &[Value]) -> Res<Value> {
    exatamente(a, 1)?;
    Ok(Value::Array(lista(&a[0])?.iter().rev().cloned().collect()))
}

fn f_soma_campo(a: &[Value]) -> Res<Value> {
    exatamente(a, 2)?;
    let campo = a[1].as_str().ok_or("o segundo argumento é o nome do campo")?;
    let mut total = 0.0;
    for item in lista(&a[0])? {
        total += num(&indexar(item, &Value::String(campo.to_string()))?)?;
    }
    numero(total)
}

/// Os itens da lista cujo `campo` vale `valor`.
fn f_onde(a: &[Value]) -> Res<Value> {
    exatamente(a, 3)?;
    let campo = Value::String(a[1].as_str().ok_or("o segundo argumento é o nome do campo")?.to_string());
    Ok(Value::Array(lista(&a[0])?.iter().filter(|i| indexar(i, &campo).is_ok_and(|v| igual(&v, &a[2]))).cloned().collect()))
}

/// As funções que as expressões conhecem.
const FUNCOES: &[(&str, Funcao)] = &[
    ("abs", |a| um_numero(a, f64::abs)),
    ("min", f_min),
    ("max", f_max),
    ("soma", f_soma),
    ("media", f_media),
    ("arredondar", f_arredondar),
    ("piso", |a| um_numero(a, f64::floor)),
    ("teto", |a| um_numero(a, f64::ceil)),
    ("raiz", |a| um_numero(a, f64::sqrt)),
    ("potencia", f_potencia),
    ("sen", |a| um_numero(a, |x| x.to_radians().sin())),
    ("cos", |a| um_numero(a, |x| x.to_radians().cos())),
    ("tan", |a| um_numero(a, |x| x.to_radians().tan())),
    ("atan2", f_atan2),
    ("dist", f_dist),
    ("angulo", f_angulo),
    ("ponto_polar", f_ponto_polar),
    ("ponto_medio", f_ponto_medio),
    ("comprimento", f_comprimento),
    ("deslocar", f_deslocar),
    ("intervalo", f_intervalo),
    ("tamanho", f_tamanho),
    ("texto", f_texto),
    ("numero", f_numero),
    ("virgula", f_virgula),
    ("se", f_se),
    ("juntar", f_juntar),
    ("contem", f_contem),
    ("inverter", f_inverter),
    ("soma_campo", f_soma_campo),
    ("onde", f_onde),
];

#[cfg(test)]
mod testes {
    use serde_json::json;

    use super::*;

    fn calc(fonte: &str, dados: &Value) -> Res<Value> {
        avaliar(fonte, &Escopo::novo(dados))
    }

    fn ok(fonte: &str) -> Value {
        calc(fonte, &json!({})).unwrap()
    }

    #[test]
    fn aritmetica_e_precedencia() {
        assert_eq!(ok("1 + 2 * 3"), json!(7));
        assert_eq!(ok("(1 + 2) * 3"), json!(9));
        assert_eq!(ok("10 / 4"), json!(2.5));
        assert_eq!(ok("-3 + 5 % 3"), json!(-1));
        assert_eq!(ok("2 * 3 > 5 && 1 < 2"), json!(true));
        assert_eq!(ok("1 == 1.0 ? 'sim' : 'nao'"), json!("sim"));
    }

    #[test]
    fn numero_inteiro_sai_como_inteiro_para_as_ferramentas() {
        assert!(ok("6 / 2").as_u64().is_some(), "as ferramentas leem circuito com as_u64");
    }

    #[test]
    fn listas_somam_elemento_a_elemento_e_pontos_se_combinam() {
        assert_eq!(ok("[1, 2] + [10, 20]"), json!([11, 22]));
        assert_eq!(ok("[3, 4] * 2"), json!([6, 8]));
        let d = json!({ "origem": [300, 420], "l": [250, 200, "a"] });
        assert_eq!(calc("origem + [l[0], l[1]]", &d).unwrap(), json!([550, 620]));
        assert!(calc("[1] + [1, 2]", &json!({})).is_err());
    }

    #[test]
    fn texto_junta_e_indice_negativo_pega_do_fim() {
        assert_eq!(ok("'C' + 3 + ' - ' + 1.5"), json!("C3 - 1.5"));
        assert_eq!(calc("l[-1]", &json!({ "l": [1, 2, 3] })).unwrap(), json!(3));
        assert!(calc("l[5]", &json!({ "l": [1] })).unwrap_err().contains("fora da lista"));
    }

    #[test]
    fn campos_de_objeto_e_funcoes_de_lista() {
        let d = json!({ "cs": [{ "fase": "F1", "va": 500 }, { "fase": "F2", "va": 800 }, { "fase": "F1", "va": 2000 }] });
        assert_eq!(calc("cs[1].va", &d).unwrap(), json!(800));
        assert_eq!(calc("soma_campo(onde(cs, 'fase', 'F1'), 'va')", &d).unwrap(), json!(2500));
        assert_eq!(calc("tamanho(cs)", &d).unwrap(), json!(3));
    }

    #[test]
    fn geometria() {
        assert_eq!(ok("dist([0, 0], [3, 4])"), json!(5));
        assert_eq!(ok("comprimento([[0, 0], [10, 0], [10, 5]])"), json!(15));
        assert_eq!(ok("angulo([0, 0], [0, 5])"), json!(90));
        assert_eq!(ok("arredondar(ponto_polar([0, 0], 90, 10)[1])"), json!(10));
        // Deslocar para a esquerda de quem anda para leste: sobe. A quina em L acompanha.
        assert_eq!(ok("deslocar([[0, 0], [10, 0], [10, 10]], 2)"), json!([[0, 2], [8, 2], [8, 10]]));
        assert_eq!(ok("deslocar([[0, 0], [10, 0]], -1)"), json!([[0, -1], [10, -1]]));
    }

    #[test]
    fn formatacao_com_virgula_e_intervalo() {
        assert_eq!(ok("virgula(3.456, 2)"), json!("3,46"));
        assert_eq!(ok("intervalo(3)"), json!([0, 1, 2]));
        assert_eq!(ok("numero('2,5') * 2"), json!(5));
    }

    #[test]
    fn erros_dizem_o_que_deu_errado() {
        assert!(calc("x + 1", &json!({})).unwrap_err().contains("a variável `x` não existe"));
        assert!(calc("1 / 0", &json!({})).unwrap_err().contains("divisão por zero"));
        assert!(calc("foo(1)", &json!({})).unwrap_err().contains("a função `foo` não existe"));
        assert!(calc("1 +", &json!({})).is_err());
        assert!(calc("1 2", &json!({})).unwrap_err().contains("sobrou"));
        assert!(calc("'aberto", &json!({})).is_err());
    }

    #[test]
    fn ternario_so_calcula_o_ramo_escolhido() {
        assert_eq!(calc("n > 0 ? 10 / n : 0", &json!({ "n": 0 })).unwrap(), json!(0));
    }
}

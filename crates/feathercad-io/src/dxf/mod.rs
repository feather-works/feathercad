//! DXF de texto (ASCII), como descrito em `docs/DXF-REFERENCIA.md`.
//!
//! O leitor **não confia** na ordem das tags nem nas contagens (`70`), ignora o
//! que não conhece (XDATA ≥ 1000, `102 {…}`, comentários `999`) e conta o que
//! não sabe representar para avisar o usuário. O escritor grava R12 (AC1009);
//! o R2000+ (handles, BLOCK_RECORD, LAYOUT) ainda não entrou porque não há como
//! validá-lo contra o AutoCAD aqui — ver `feathercad/CLAUDE.md`.

pub mod escritor;
pub mod leitor;

/// Um objeto do arquivo: o que começa numa tag `0` e vai até a próxima.
pub(crate) struct Objeto<'a> {
    pub tipo: &'a str,
    pub tags: Vec<(i32, &'a str)>,
}

impl<'a> Objeto<'a> {
    /// O valor da primeira tag com o código dado.
    pub(crate) fn primeiro(&self, codigo: i32) -> Option<&'a str> {
        self.tags.iter().find(|(c, _)| *c == codigo).map(|(_, v)| *v)
    }

    /// O primeiro valor numérico com o código dado.
    pub(crate) fn numero(&self, codigo: i32) -> Option<f64> {
        self.primeiro(codigo).and_then(|v| v.trim().parse::<f64>().ok()).filter(|n| n.is_finite())
    }
}

/// `\U+00E7` → `ç` (o DXF escreve assim o que a codificação não cobre).
pub(crate) fn decodificar_u(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(pos) = resto.find("\\U+") {
        saida.push_str(&resto[..pos]);
        let hex = resto.get(pos + 3..pos + 7).unwrap_or("");
        match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
            Some(c) if hex.len() == 4 => {
                saida.push(c);
                resto = &resto[pos + 7..];
            }
            _ => {
                saida.push_str("\\U+");
                resto = &resto[pos + 3..];
            }
        }
    }
    saida.push_str(resto);
    saida
}

/// O inverso: o que não é ASCII vira `\U+XXXX` (vale em todas as versões).
pub(crate) fn codificar_u(texto: &str) -> String {
    texto.chars().fold(String::with_capacity(texto.len()), |mut s, c| {
        if c.is_ascii() {
            s.push(c);
        } else {
            for unidade in c.encode_utf16(&mut [0u16; 2]) {
                s.push_str(&format!("\\U+{unidade:04X}"));
            }
        }
        s
    })
}

/// Os códigos `%%` do texto DXF: `%%c` Ø, `%%d` °, `%%p` ±, `%%%` %, `%%nnn` (caractere de
/// código decimal); `%%u` e `%%o` (sublinhado/sobrelinha) somem.
pub(crate) fn decodificar_percentual(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(pos) = resto.find("%%") {
        saida.push_str(&resto[..pos]);
        let depois = &resto[pos + 2..];
        let mut chars = depois.chars();
        match chars.next() {
            Some('c' | 'C') => saida.push('Ø'),
            Some('d' | 'D') => saida.push('°'),
            Some('p' | 'P') => saida.push('±'),
            Some('%') => saida.push('%'),
            Some('u' | 'U' | 'o' | 'O') => {}
            Some(d) if d.is_ascii_digit() => {
                let numero: String = depois.chars().take(3).take_while(char::is_ascii_digit).collect();
                if let Some(c) = numero.parse::<u8>().ok().map(char::from) {
                    saida.push(c);
                }
                resto = &depois[numero.len()..];
                continue;
            }
            Some(outro) => {
                saida.push_str("%%");
                saida.push(outro);
            }
            None => saida.push_str("%%"),
        }
        resto = chars.as_str();
    }
    saida.push_str(resto);
    saida
}

/// O texto de um `TEXT`/`MTEXT` como o usuário lê: `%%` e `\U+` decodificados.
pub(crate) fn decodificar_texto(texto: &str) -> String {
    decodificar_u(&decodificar_percentual(texto))
}

//! Ler os bytes do quadro (formato em `crates/feathercad-core/src/quadro.rs`): `FCQ0` = nada mudou, `FCQ1` = segmentos,
//! textos, preenchimentos, marcas do fundo e imagens, tudo já em pixels da área de desenho e com a cor resolvida.

/// Um texto, na posição, tamanho e giro (radianos) que o motor mandou.
#[derive(Debug, Clone, PartialEq)]
pub struct Texto {
    pub x: f32,
    pub y: f32,
    pub altura: f32,
    pub giro: f32,
    /// 0 = esquerda, 1 = centro, 2 = direita.
    pub h: u8,
    /// 0 = linha de base, 1 = embaixo, 2 = meio, 3 = em cima.
    pub base: u8,
    pub cor: u32,
    pub conteudo: String,
}

/// Uma área cheia: laços de pontos (par-ímpar).
#[derive(Debug, Clone, PartialEq)]
pub struct Preenchimento {
    pub cor: u32,
    pub lacos: Vec<Vec<(f32, f32)>>,
}

/// Uma imagem esticada no paralelogramo `(x, y) + u + v`.
#[derive(Debug, Clone, PartialEq)]
pub struct Imagem {
    pub x: f32,
    pub y: f32,
    pub u: (f32, f32),
    pub v: (f32, f32),
    pub caminho: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Quadro {
    pub versao: u64,
    /// `(x1, y1, x2, y2, cor, espessura)`.
    pub segmentos: Vec<([f32; 4], u32, f32)>,
    pub textos: Vec<Texto>,
    pub preenchimentos: Vec<Preenchimento>,
    /// Marcas do fundo: posição e se é principal.
    pub fundo_x: Vec<(f32, bool)>,
    pub fundo_y: Vec<(f32, bool)>,
    pub imagens: Vec<Imagem>,
}

struct Leitor<'a> {
    b: &'a [u8],
    o: usize,
}

impl Leitor<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self.b.get(self.o..self.o + n).ok_or_else(|| format!("quadro truncado no byte {}", self.o))?;
        self.o += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().map_err(|_| "u16")?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().map_err(|_| "u32")?))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().map_err(|_| "f32")?))
    }
    fn texto(&mut self, n: usize) -> Result<String, String> {
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
    /// Uma contagem, conferida contra o que resta (bytes corrompidos não alocam gigabytes).
    fn contagem(&mut self, minimo_por_item: usize) -> Result<usize, String> {
        let n = self.u32()? as usize;
        if n.saturating_mul(minimo_por_item) > self.b.len().saturating_sub(self.o) {
            return Err(format!("quadro com contagem impossível ({n}) no byte {}", self.o));
        }
        Ok(n)
    }
    fn marcas(&mut self, n: usize) -> Result<Vec<(f32, bool)>, String> {
        (0..n).map(|_| Ok((self.f32()?, self.u8()? == 1))).collect()
    }
}

/// `None` = `FCQ0` (o quadro que já está na tela continua valendo).
pub fn ler(bytes: &[u8]) -> Result<Option<Quadro>, String> {
    let mut l = Leitor { b: bytes, o: 0 };
    match l.bytes(4)? {
        b"FCQ0" => return Ok(None),
        b"FCQ1" => {}
        m => return Err(format!("quadro desconhecido: {}", String::from_utf8_lossy(m))),
    }
    let mut q = Quadro { versao: u64::from_le_bytes(l.bytes(8)?.try_into().map_err(|_| "versão")?), ..Quadro::default() };
    let n = l.contagem(24)?;
    q.segmentos.reserve(n);
    for _ in 0..n {
        q.segmentos.push(([l.f32()?, l.f32()?, l.f32()?, l.f32()?], l.u32()?, l.f32()?));
    }
    for _ in 0..l.contagem(24)? {
        let (x, y, altura, giro) = (l.f32()?, l.f32()?, l.f32()?, l.f32()?);
        let (h, base, cor) = (l.u8()?, l.u8()?, l.u32()?);
        let tamanho = l.u16()? as usize;
        q.textos.push(Texto { x, y, altura, giro, h, base, cor, conteudo: l.texto(tamanho)? });
    }
    for _ in 0..l.contagem(8)? {
        let cor = l.u32()?;
        let lacos = (0..l.contagem(4)?)
            .map(|_| {
                let nv = l.contagem(8)?;
                (0..nv).map(|_| Ok((l.f32()?, l.f32()?))).collect::<Result<Vec<_>, String>>()
            })
            .collect::<Result<_, String>>()?;
        q.preenchimentos.push(Preenchimento { cor, lacos });
    }
    let (nx, ny) = (l.u32()? as usize, l.u32()? as usize);
    if (nx + ny) * 5 > bytes.len() - l.o {
        return Err("quadro com marcas do fundo impossíveis".into());
    }
    q.fundo_x = l.marcas(nx)?;
    q.fundo_y = l.marcas(ny)?;
    // A seção de imagens é a última e pode faltar (quadro mais antigo).
    if l.o + 4 <= bytes.len() {
        for _ in 0..l.contagem(26)? {
            let (x, y, ux, uy, vx, vy) = (l.f32()?, l.f32()?, l.f32()?, l.f32()?, l.f32()?, l.f32()?);
            let tamanho = l.u16()? as usize;
            q.imagens.push(Imagem { x, y, u: (ux, uy), v: (vx, vy), caminho: l.texto(tamanho)? });
        }
    }
    Ok(Some(q))
}

#[cfg(test)]
mod testes {
    use feathercad_ponte::Ponte;

    #[test]
    fn le_o_quadro_que_o_motor_monta() {
        let mut p = Ponte::default();
        p.chamar("documento_novo", br#"{"unidade":"mm"}"#).unwrap();
        p.chamar("linha_criar", br#"{"a":{"x":0,"y":0},"b":{"x":100,"y":50}}"#).unwrap();
        p.chamar("fundo_definir", br#"{"modo":"grade"}"#).unwrap();
        let vista = r#"{"centro":{"x":50,"y":25},"escala":2,"largura":400,"altura":300}"#;
        let bytes = p.chamar("quadro", format!(r#"{{"vista":{vista},"versao_conhecida":null,"fundo_claro":false}}"#).as_bytes()).unwrap();
        let q = super::ler(&bytes).unwrap().expect("quadro novo");
        assert_eq!(q.segmentos.len(), 1);
        let ([x1, y1, x2, y2], _, _) = q.segmentos[0];
        // (0,0) e (100,50) com centro (50,25), 2 px/unidade e 400×300: (100,200) e (300,100) na tela (y para baixo).
        assert_eq!((x1, y1, x2, y2), (100.0, 200.0, 300.0, 100.0));
        assert!(!q.fundo_x.is_empty() && !q.fundo_y.is_empty(), "a grade vem do motor");
        let igual = p.chamar("quadro", format!(r#"{{"vista":{vista},"versao_conhecida":{},"fundo_claro":false}}"#, q.versao).as_bytes()).unwrap();
        assert_eq!(super::ler(&igual).unwrap(), None, "mesma versão e vista = FCQ0");
    }

    #[test]
    fn bytes_quebrados_dao_erro_e_nao_panico() {
        assert!(super::ler(b"FCQ1\0\0").is_err());
        assert!(super::ler(b"XXXX").is_err());
        let mut b = b"FCQ1".to_vec();
        b.extend(0u64.to_le_bytes());
        b.extend(u32::MAX.to_le_bytes());
        assert!(super::ler(&b).is_err());
    }
}

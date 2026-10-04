//! As camadas: criar, renomear, excluir, mudar propriedades (ligada, congelada, bloqueada, plotar, cor,
//! tipo de linha, espessura, transparência) e a **consulta da tabela** do gerenciador (filtro, busca,
//! ordenação). Tudo aqui é do Rust; a interface só mostra as linhas que voltam.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::documento::{Camada, Captura, Documento, Passo, TipoLinha};

/// Por que uma operação de camada não aconteceu (a interface traduz a chave em mensagem).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErroCamada {
    NomeVazio,
    /// O nome tem caractere que o DXF proíbe (`< > / \ " : ; ? * | = ,`) ou passa de 255.
    NomeInvalido,
    NomeRepetido,
    /// A camada `0` não se renomeia nem se exclui.
    CamadaZero,
    /// A camada atual não se congela nem se exclui.
    CamadaAtual,
    /// A camada tem objetos (ou é usada por um bloco).
    EmUso,
    SemCamada,
}

impl ErroCamada {
    /// A chave da mensagem no arquivo de língua (`camadas.erro.<chave>`).
    pub fn chave(self) -> &'static str {
        match self {
            ErroCamada::NomeVazio => "camadas.erro.nomeVazio",
            ErroCamada::NomeInvalido => "camadas.erro.nomeInvalido",
            ErroCamada::NomeRepetido => "camadas.erro.nomeRepetido",
            ErroCamada::CamadaZero => "camadas.erro.camadaZero",
            ErroCamada::CamadaAtual => "camadas.erro.camadaAtual",
            ErroCamada::EmUso => "camadas.erro.emUso",
            ErroCamada::SemCamada => "camadas.erro.semCamada",
        }
    }
}

/// Uma propriedade de camada e o valor novo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Propriedade {
    Ligada(bool),
    Congelada(bool),
    Bloqueada(bool),
    Plotar(bool),
    Cor(u32),
    TipoLinha(TipoLinha),
    /// Espessura em mm; `None` = padrão.
    Espessura(Option<f32>),
    /// De 0 a 90.
    Transparencia(u8),
}

const NOME_MAXIMO: usize = 255;
const CARACTERES_PROIBIDOS: &str = "<>/\\\":;?*|=,`";

/// O filtro da árvore da esquerda do gerenciador.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filtro {
    #[default]
    Todas,
    /// Só as que têm objetos.
    Usadas,
}

/// A coluna pela qual a tabela se ordena.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ordem {
    Status,
    #[default]
    Nome,
    Ligada,
    Congelada,
    Bloqueada,
    Plotar,
    Cor,
    TipoLinha,
    Espessura,
    Transparencia,
}

/// O que a tabela pede: filtro, texto de busca (`*` e `?` valem), coluna e sentido.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consulta {
    #[serde(default)]
    pub filtro: Filtro,
    #[serde(default)]
    pub busca: String,
    #[serde(default)]
    pub ordem: Ordem,
    #[serde(default = "crescente_padrao")]
    pub crescente: bool,
}

fn crescente_padrao() -> bool {
    true
}

impl Default for Consulta {
    fn default() -> Self {
        Self { filtro: Filtro::Todas, busca: String::new(), ordem: Ordem::Nome, crescente: true }
    }
}

/// Uma linha da tabela do gerenciador.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LinhaCamada {
    /// O índice da camada no documento (é com ele que se pede uma mudança).
    pub indice: usize,
    pub nome: String,
    pub atual: bool,
    /// Tem objetos.
    pub usada: bool,
    pub ligada: bool,
    pub congelada: bool,
    pub bloqueada: bool,
    pub plotar: bool,
    pub cor: String,
    pub tipo_linha: TipoLinha,
    /// Em mm; `None` = padrão.
    pub espessura: Option<f32>,
    pub transparencia: u8,
}

/// A resposta da consulta: as linhas e as contagens ("9 camadas exibidas de 9 totais").
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Tabela {
    pub linhas: Vec<LinhaCamada>,
    pub exibidas: usize,
    pub totais: usize,
    /// O nome da camada atual.
    pub atual: String,
}

/// Comparação "natural": `Camada2` vem antes de `Camada10`; ignora maiúsculas.
fn comparar_nomes(a: &str, b: &str) -> Ordering {
    let (mut ia, mut ib) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (mut na, mut nb) = (String::new(), String::new());
                while let Some(c) = ia.next_if(char::is_ascii_digit) {
                    na.push(c);
                }
                while let Some(c) = ib.next_if(char::is_ascii_digit) {
                    nb.push(c);
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                let o = x.to_lowercase().to_string().cmp(&y.to_lowercase().to_string());
                if o != Ordering::Equal {
                    return o;
                }
                ia.next();
                ib.next();
            }
        }
    }
}

/// `texto` casa com `padrao` (`*` = qualquer trecho, `?` = uma letra; sem curinga, "contém"), sem diferenciar maiúscula.
fn casa(padrao: &str, texto: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (padrao.to_lowercase().chars().collect(), texto.to_lowercase().chars().collect());
    if !p.iter().any(|c| *c == '*' || *c == '?') {
        return t.windows(p.len().max(1)).any(|w| w == p.as_slice()) || p.is_empty();
    }
    fn rec(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => (0..=t.len()).any(|i| rec(&p[1..], &t[i..])),
            Some('?') => !t.is_empty() && rec(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && rec(&p[1..], &t[1..]),
        }
    }
    rec(&p, &t)
}

/// Por que um bloco não foi criado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErroBloco {
    NomeVazio,
    /// Nome com caractere proibido, ou que começa por `*` (reservado aos anônimos).
    NomeInvalido,
    NomeRepetido,
    SemObjetos,
}

impl ErroBloco {
    /// A chave da mensagem (`bloco.erro.<chave>`).
    pub fn chave(self) -> &'static str {
        match self {
            ErroBloco::NomeVazio => "bloco.erro.nomeVazio",
            ErroBloco::NomeInvalido => "bloco.erro.nomeInvalido",
            ErroBloco::NomeRepetido => "bloco.erro.nomeRepetido",
            ErroBloco::SemObjetos => "bloco.erro.semObjetos",
        }
    }
}

impl Documento {
    /// Cria um bloco com as entidades `ids` (ponto-base `base`) e as troca por **uma** inserção dele, no mesmo
    /// lugar (o "converter em bloco" do AutoCAD). Um Ctrl+Z desfaz tudo. Devolve o índice do bloco.
    pub fn criar_bloco_da_selecao(&mut self, nome: &str, base: crate::geom::Ponto, ids: &std::collections::BTreeSet<crate::documento::Id>) -> Result<usize, ErroBloco> {
        let nome = nome.trim();
        if nome.is_empty() {
            return Err(ErroBloco::NomeVazio);
        }
        if nome.starts_with('*') || nome.chars().count() > NOME_MAXIMO || nome.chars().any(|c| CARACTERES_PROIBIDOS.contains(c) || c.is_control()) {
            return Err(ErroBloco::NomeInvalido);
        }
        if self.bloco_por_nome(nome).is_some() {
            return Err(ErroBloco::NomeRepetido);
        }
        let escolhidas: Vec<_> = self.entidades.iter().filter(|e| ids.contains(&e.id) && !self.bloqueada(e)).cloned().collect();
        let Some(primeira) = escolhidas.first() else { return Err(ErroBloco::SemObjetos) };
        let (camada, cor) = (primeira.camada, primeira.cor);
        self.desfazer.push(Passo::Tudo(Box::new(Captura { camadas: self.camadas.clone(), atual: self.camada_atual, entidades: self.entidades.clone(), blocos: self.blocos.clone() })));
        self.refazer.clear();
        let entidades = escolhidas.iter().map(|e| crate::documento::NovaEntidade { camada: e.camada, cor: e.cor, geometria: e.geometria.clone(), props: e.props.clone(), espaco: e.espaco.clone() }).collect();
        self.blocos.push(crate::documento::Bloco { nome: nome.to_string(), base, entidades });
        let indice = self.blocos.len() - 1;
        self.caixas_dos_blocos.push(self.caixa_do_bloco(indice, 0));
        let sair: std::collections::BTreeSet<_> = escolhidas.iter().map(|e| e.id).collect();
        self.entidades.retain(|e| !sair.contains(&e.id));
        let id = self.proximo_id;
        self.proximo_id += 1;
        self.entidades.push(crate::documento::Entidade { id, camada, cor, geometria: crate::documento::Geometria::Insercao { bloco: indice, ponto: base, escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: Default::default(), espaco: self.espaco_ativo.clone() });
        self.mudou();
        Ok(indice)
    }

    /// Uma cota (ou outra anotação que deve valer por um objeto só): as `formas`, em coordenadas do desenho, vão para
    /// um bloco anônimo `*D<n>` e entra **uma** inserção dele, na camada atual, com a base em `base`. Selecionar,
    /// mover, copiar e apagar valem para o conjunto; o DXF grava como bloco `*D`, o que o AutoCAD lê como cota.
    /// Um Ctrl+Z desfaz tudo. `None` se não há formas.
    pub fn adicionar_cota(&mut self, formas: Vec<crate::documento::Geometria>, base: crate::geom::Ponto) -> Option<crate::documento::Id> {
        if formas.is_empty() {
            return None;
        }
        self.desfazer.push(Passo::Tudo(Box::new(Captura { camadas: self.camadas.clone(), atual: self.camada_atual, entidades: self.entidades.clone(), blocos: self.blocos.clone() })));
        self.refazer.clear();
        let mut n = 1;
        while self.bloco_por_nome(&format!("*D{n}")).is_some() {
            n += 1;
        }
        // camada 0 dentro do bloco: as formas tomam a camada e a cor da inserção
        let entidades = formas.into_iter().map(|g| crate::documento::NovaEntidade { camada: 0, cor: None, geometria: g, props: Default::default(), espaco: crate::documento::Espaco::Modelo }).collect();
        self.blocos.push(crate::documento::Bloco { nome: format!("*D{n}"), base, entidades });
        let indice = self.blocos.len() - 1;
        self.caixas_dos_blocos.push(self.caixa_do_bloco(indice, 0));
        let id = self.proximo_id;
        self.proximo_id += 1;
        let camada = self.camada_atual;
        self.entidades.push(crate::documento::Entidade { id, camada, cor: None, geometria: crate::documento::Geometria::Insercao { bloco: indice, ponto: base, escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: Default::default(), espaco: self.espaco_ativo.clone() });
        self.mudou();
        Some(id)
    }

    /// Cria o bloco `nome` com `entidades` (camadas do próprio desenho; camada `0` herda a da inserção) e o insere em
    /// `ponto` com a escala dada, na camada `camada`. Um passo de desfazer. `None` se não há entidades ou o nome já existe.
    pub fn adicionar_bloco_inserido(&mut self, nome: &str, entidades: Vec<crate::documento::NovaEntidade>, ponto: crate::geom::Ponto, escala: f64, camada: usize) -> Option<crate::documento::Id> {
        if entidades.is_empty() || self.bloco_por_nome(nome).is_some() {
            return None;
        }
        self.desfazer.push(Passo::Tudo(Box::new(Captura { camadas: self.camadas.clone(), atual: self.camada_atual, entidades: self.entidades.clone(), blocos: self.blocos.clone() })));
        self.refazer.clear();
        self.blocos.push(crate::documento::Bloco { nome: nome.to_string(), base: crate::geom::Ponto::novo(0.0, 0.0), entidades });
        let indice = self.blocos.len() - 1;
        self.caixas_dos_blocos.push(self.caixa_do_bloco(indice, 0));
        let id = self.proximo_id;
        self.proximo_id += 1;
        self.entidades.push(crate::documento::Entidade { id, camada, cor: None, geometria: crate::documento::Geometria::Insercao { bloco: indice, ponto, escala_x: escala, escala_y: escala, rotacao: 0.0 }, props: Default::default(), espaco: self.espaco_ativo.clone() });
        self.mudou();
        Some(id)
    }

    /// A camada `indice` tem objetos (no desenho ou dentro de um bloco)?
    pub fn camada_em_uso(&self, indice: usize) -> bool {
        self.entidades.iter().any(|e| e.camada == indice) || self.blocos.iter().any(|b| b.entidades.iter().any(|n| n.camada == indice))
    }

    fn validar_nome(&self, nome: &str, exceto: Option<usize>) -> Result<String, ErroCamada> {
        let nome = nome.trim();
        if nome.is_empty() {
            return Err(ErroCamada::NomeVazio);
        }
        if nome.chars().count() > NOME_MAXIMO || nome.chars().any(|c| CARACTERES_PROIBIDOS.contains(c) || c.is_control()) {
            return Err(ErroCamada::NomeInvalido);
        }
        if self.camadas.iter().enumerate().any(|(i, c)| Some(i) != exceto && c.nome.to_lowercase() == nome.to_lowercase()) {
            return Err(ErroCamada::NomeRepetido);
        }
        Ok(nome.to_string())
    }

    /// Empilha a tabela de camadas atual no histórico (o que desfazer devolve) e marca o desenho alterado.
    fn guardar_camadas(&mut self) {
        self.desfazer.push(Passo::Camadas { camadas: self.camadas.clone(), atual: self.camada_atual });
        self.refazer.clear();
        self.mudou();
    }

    /// Cria uma camada. Com `modelo`, herda as propriedades dela (como o "Nova camada" do AutoCAD). Devolve o índice.
    pub fn nova_camada(&mut self, nome: &str, modelo: Option<usize>) -> Result<usize, ErroCamada> {
        let nome = self.validar_nome(nome, None)?;
        self.guardar_camadas();
        let base = modelo.and_then(|m| self.camadas.get(m)).cloned().unwrap_or_else(|| Camada::nova("", 0xFF_FF_FF));
        self.camadas.push(Camada { nome, ..base });
        Ok(self.camadas.len() - 1)
    }

    /// Um nome livre que começa por `prefixo` e termina num número (`Camada1`, `Camada2`…).
    pub fn nome_livre_de_camada(&self, prefixo: &str) -> String {
        (1..).map(|n| format!("{prefixo}{n}")).find(|n| !self.camadas.iter().any(|c| c.nome.eq_ignore_ascii_case(n))).unwrap_or_default()
    }

    pub fn renomear_camada(&mut self, indice: usize, nome: &str) -> Result<(), ErroCamada> {
        let atual = self.camadas.get(indice).ok_or(ErroCamada::SemCamada)?;
        if atual.nome == "0" {
            return Err(ErroCamada::CamadaZero);
        }
        let nome = self.validar_nome(nome, Some(indice))?;
        if nome != self.camadas[indice].nome {
            self.guardar_camadas();
            self.camadas[indice].nome = nome;
        }
        Ok(())
    }

    /// Muda uma propriedade de várias camadas de uma vez (um Ctrl+Z desfaz todas). Congelar a atual é recusado.
    pub fn definir_propriedade_de_camadas(&mut self, indices: &[usize], p: Propriedade) -> Result<(), ErroCamada> {
        if indices.iter().any(|i| *i >= self.camadas.len()) {
            return Err(ErroCamada::SemCamada);
        }
        if matches!(p, Propriedade::Congelada(true)) && indices.contains(&self.camada_atual) {
            return Err(ErroCamada::CamadaAtual);
        }
        let mudaria = indices.iter().any(|i| {
            let c = &self.camadas[*i];
            match p {
                Propriedade::Ligada(v) => c.ligada != v,
                Propriedade::Congelada(v) => c.congelada != v,
                Propriedade::Bloqueada(v) => c.bloqueada != v,
                Propriedade::Plotar(v) => c.plotar != v,
                Propriedade::Cor(v) => c.cor != v,
                Propriedade::TipoLinha(v) => c.tipo_linha != v,
                Propriedade::Espessura(v) => c.espessura != v,
                Propriedade::Transparencia(v) => c.transparencia != v.min(90),
            }
        });
        if !mudaria {
            return Ok(());
        }
        self.guardar_camadas();
        for i in indices {
            let c = &mut self.camadas[*i];
            match p {
                Propriedade::Ligada(v) => c.ligada = v,
                Propriedade::Congelada(v) => c.congelada = v,
                Propriedade::Bloqueada(v) => c.bloqueada = v,
                Propriedade::Plotar(v) => c.plotar = v,
                Propriedade::Cor(v) => c.cor = v & 0xFF_FF_FF,
                Propriedade::TipoLinha(v) => c.tipo_linha = v,
                Propriedade::Espessura(v) => c.espessura = v.map(|e| e.clamp(0.0, 2.11)),
                Propriedade::Transparencia(v) => c.transparencia = v.min(90),
            }
        }
        Ok(())
    }

    /// Torna atual a camada `indice` (não entra no histórico: não muda o desenho).
    pub fn tornar_camada_atual(&mut self, indice: usize) -> Result<(), ErroCamada> {
        if indice >= self.camadas.len() {
            return Err(ErroCamada::SemCamada);
        }
        self.camada_atual = indice;
        Ok(())
    }

    /// Exclui as camadas que podem sair (a `0`, a atual e as que têm objetos ficam). Devolve quantas saíram e
    /// o primeiro motivo de recusa, se alguma ficou.
    pub fn excluir_camadas(&mut self, indices: &[usize]) -> (usize, Option<ErroCamada>) {
        let (mut sair, mut motivo) = (Vec::new(), None);
        for i in indices.iter().copied().collect::<std::collections::BTreeSet<_>>() {
            let erro = match self.camadas.get(i) {
                None => Some(ErroCamada::SemCamada),
                Some(c) if c.nome == "0" => Some(ErroCamada::CamadaZero),
                Some(_) if i == self.camada_atual => Some(ErroCamada::CamadaAtual),
                Some(_) if self.camada_em_uso(i) => Some(ErroCamada::EmUso),
                Some(_) => None,
            };
            match erro {
                None => sair.push(i),
                Some(e) => motivo = motivo.or(Some(e)),
            }
        }
        if sair.is_empty() {
            return (0, motivo);
        }
        self.desfazer.push(Passo::Tudo(Box::new(Captura { camadas: self.camadas.clone(), atual: self.camada_atual, entidades: self.entidades.clone(), blocos: self.blocos.clone() })));
        self.refazer.clear();
        // Índice novo de cada camada que fica.
        let novo = |i: usize| i - sair.iter().filter(|s| **s < i).count();
        let atual = novo(self.camada_atual);
        self.camadas = self.camadas.iter().enumerate().filter(|(i, _)| !sair.contains(i)).map(|(_, c)| c.clone()).collect();
        self.camada_atual = atual;
        for e in &mut self.entidades {
            e.camada = novo(e.camada);
        }
        for b in &mut self.blocos {
            for n in &mut b.entidades {
                n.camada = novo(n.camada);
            }
        }
        self.mudou();
        (sair.len(), motivo)
    }

    /// A tabela do gerenciador: filtrada, buscada e ordenada.
    pub fn tabela_de_camadas(&self, q: &Consulta) -> Tabela {
        let em_uso: Vec<bool> = {
            let mut v = vec![false; self.camadas.len()];
            for e in &self.entidades {
                if let Some(x) = v.get_mut(e.camada) {
                    *x = true;
                }
            }
            for b in &self.blocos {
                for n in &b.entidades {
                    if let Some(x) = v.get_mut(n.camada) {
                        *x = true;
                    }
                }
            }
            v
        };
        let mut linhas: Vec<LinhaCamada> = self
            .camadas
            .iter()
            .enumerate()
            .filter(|(i, c)| (q.filtro == Filtro::Todas || em_uso[*i]) && (q.busca.trim().is_empty() || casa(q.busca.trim(), &c.nome)))
            .map(|(i, c)| LinhaCamada {
                indice: i,
                nome: c.nome.clone(),
                atual: i == self.camada_atual,
                usada: em_uso[i],
                ligada: c.ligada,
                congelada: c.congelada,
                bloqueada: c.bloqueada,
                plotar: c.plotar,
                cor: format!("#{:06x}", c.cor),
                tipo_linha: c.tipo_linha,
                espessura: c.espessura,
                transparencia: c.transparencia,
            })
            .collect();
        let bool_ord = |a: bool, b: bool| b.cmp(&a);
        linhas.sort_by(|a, b| {
            let o = match q.ordem {
                Ordem::Nome => comparar_nomes(&a.nome, &b.nome),
                Ordem::Status => bool_ord(a.atual, b.atual).then(bool_ord(a.usada, b.usada)),
                Ordem::Ligada => bool_ord(a.ligada, b.ligada),
                Ordem::Congelada => bool_ord(a.congelada, b.congelada),
                Ordem::Bloqueada => bool_ord(a.bloqueada, b.bloqueada),
                Ordem::Plotar => bool_ord(a.plotar, b.plotar),
                Ordem::Cor => a.cor.cmp(&b.cor),
                Ordem::TipoLinha => a.tipo_linha.nome_dxf().cmp(b.tipo_linha.nome_dxf()),
                Ordem::Espessura => a.espessura.unwrap_or(-1.0).total_cmp(&b.espessura.unwrap_or(-1.0)),
                Ordem::Transparencia => a.transparencia.cmp(&b.transparencia),
            }
            .then_with(|| comparar_nomes(&a.nome, &b.nome));
            if q.crescente { o } else { o.reverse() }
        });
        Tabela { exibidas: linhas.len(), totais: self.camadas.len(), atual: self.camadas.get(self.camada_atual).map_or_else(String::new, |c| c.nome.clone()), linhas }
    }
}

#[cfg(test)]
mod testes_bloco {
    use super::*;
    use crate::geom::Ponto;

    #[test]
    fn criar_bloco_troca_a_selecao_por_uma_insercao_e_desfaz_de_uma_vez() {
        let mut d = Documento::default();
        let a = d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(10.0, 0.0));
        let b = d.adicionar_linha(Ponto::novo(0.0, 5.0), Ponto::novo(10.0, 5.0));
        let ids = std::collections::BTreeSet::from([a, b]);
        assert_eq!(d.criar_bloco_da_selecao(" ", Ponto::novo(0.0, 0.0), &ids), Err(ErroBloco::NomeVazio));
        assert_eq!(d.criar_bloco_da_selecao("*X", Ponto::novo(0.0, 0.0), &ids), Err(ErroBloco::NomeInvalido));
        assert_eq!(d.criar_bloco_da_selecao("Mesa", Ponto::novo(0.0, 0.0), &std::collections::BTreeSet::new()), Err(ErroBloco::SemObjetos));
        let i = d.criar_bloco_da_selecao("Mesa", Ponto::novo(0.0, 0.0), &ids).unwrap();
        assert_eq!((d.blocos().len(), d.blocos()[i].entidades.len(), d.entidades().len()), (1, 2, 1));
        assert!(matches!(d.entidades()[0].geometria, crate::documento::Geometria::Insercao { bloco, .. } if bloco == i));
        assert_eq!(d.criar_bloco_da_selecao("MESA", Ponto::novo(0.0, 0.0), &ids), Err(ErroBloco::NomeRepetido), "sem diferenciar maiúscula");
        assert!(d.caixa().is_some(), "a caixa do bloco entrou");
        assert!(d.desfazer());
        assert_eq!((d.blocos().len(), d.entidades().len()), (0, 2));
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn com_camadas(nomes: &[&str]) -> Documento {
        let mut d = Documento::default();
        for n in nomes {
            d.nova_camada(n, None).unwrap();
        }
        d
    }

    #[test]
    fn nome_natural_curinga_e_ordenacao() {
        let d = com_camadas(&["Camada10", "Camada2", "Paredes", "cotas"]);
        let nomes = |q: &Consulta| d.tabela_de_camadas(q).linhas.into_iter().map(|l| l.nome).collect::<Vec<_>>();
        assert_eq!(nomes(&Consulta::default()), ["0", "Camada2", "Camada10", "cotas", "Paredes"]);
        assert_eq!(nomes(&Consulta { crescente: false, ..Consulta::default() })[0], "Paredes");
        assert_eq!(nomes(&Consulta { busca: "cam*10".into(), ..Consulta::default() }), ["Camada10"]);
        assert_eq!(nomes(&Consulta { busca: "PAR".into(), ..Consulta::default() }), ["Paredes"]);
        assert_eq!(nomes(&Consulta { busca: "cotas?".into(), ..Consulta::default() }), Vec::<String>::new());
        let t = d.tabela_de_camadas(&Consulta { busca: "camada".into(), ..Consulta::default() });
        assert_eq!((t.exibidas, t.totais), (2, 5));
    }

    #[test]
    fn nova_renomear_e_regras_de_nome() {
        let mut d = Documento::default();
        assert_eq!(d.nova_camada("  ", None), Err(ErroCamada::NomeVazio));
        assert_eq!(d.nova_camada("a/b", None), Err(ErroCamada::NomeInvalido));
        assert_eq!(d.nova_camada("0", None), Err(ErroCamada::NomeRepetido));
        let i = d.nova_camada("Paredes", None).unwrap();
        assert_eq!(d.nova_camada("PAREDES", None), Err(ErroCamada::NomeRepetido), "sem diferenciar maiúscula");
        assert_eq!(d.renomear_camada(0, "X"), Err(ErroCamada::CamadaZero));
        d.renomear_camada(i, "Muros").unwrap();
        assert_eq!(d.camadas()[i].nome, "Muros");
        assert_eq!(d.nome_livre_de_camada("Camada"), "Camada1");
        assert!(d.desfazer() && d.camadas()[i].nome == "Paredes");
    }

    #[test]
    fn propriedades_em_varias_camadas_num_passo_so_e_congelar_a_atual_e_recusado() {
        let mut d = com_camadas(&["A", "B"]);
        d.definir_propriedade_de_camadas(&[1, 2], Propriedade::Bloqueada(true)).unwrap();
        d.definir_propriedade_de_camadas(&[1], Propriedade::TipoLinha(TipoLinha::Oculta)).unwrap();
        d.definir_propriedade_de_camadas(&[2], Propriedade::Transparencia(200)).unwrap();
        assert!(d.camadas()[1].bloqueada && d.camadas()[2].bloqueada && d.camadas()[1].tipo_linha == TipoLinha::Oculta && d.camadas()[2].transparencia == 90);
        assert_eq!(d.definir_propriedade_de_camadas(&[0], Propriedade::Congelada(true)), Err(ErroCamada::CamadaAtual));
        assert_eq!(d.definir_propriedade_de_camadas(&[9], Propriedade::Ligada(false)), Err(ErroCamada::SemCamada));
        let versao = d.versao();
        d.definir_propriedade_de_camadas(&[1], Propriedade::Bloqueada(true)).unwrap();
        assert_eq!(d.versao(), versao, "não mudou nada: não suja o desenho");
        assert!(d.desfazer() && d.camadas()[2].transparencia == 0);
        assert!(d.desfazer() && d.camadas()[1].tipo_linha == TipoLinha::Continua);
        assert!(d.desfazer() && !d.camadas()[1].bloqueada && !d.camadas()[2].bloqueada, "as duas camadas voltam juntas");
    }

    #[test]
    fn excluir_so_o_que_nao_tem_uso_remapeia_indices_e_desfaz() {
        let mut d = com_camadas(&["Vazia", "Usada", "Outra"]);
        d.tornar_camada_atual(2).unwrap();
        d.adicionar_linha(crate::geom::Ponto::novo(0.0, 0.0), crate::geom::Ponto::novo(1.0, 0.0));
        d.tornar_camada_atual(3).unwrap();
        d.adicionar_linha(crate::geom::Ponto::novo(0.0, 1.0), crate::geom::Ponto::novo(1.0, 1.0));
        d.tornar_camada_atual(0).unwrap();
        // Vazia (1) sai; Usada (2) tem objeto; Outra (3) tem objeto; 0 é fixa.
        let (n, motivo) = d.excluir_camadas(&[0, 1, 2, 3]);
        assert_eq!((n, motivo), (1, Some(ErroCamada::CamadaZero)));
        assert_eq!(d.camadas().iter().map(|c| c.nome.as_str()).collect::<Vec<_>>(), ["0", "Usada", "Outra"]);
        assert_eq!(d.entidades().iter().map(|e| e.camada).collect::<Vec<_>>(), [1, 2], "os índices dos objetos seguiram");
        assert!(d.desfazer());
        assert_eq!(d.camadas().len(), 4);
        assert_eq!(d.entidades().iter().map(|e| e.camada).collect::<Vec<_>>(), [2, 3]);
        assert_eq!(d.excluir_camadas(&[0]), (0, Some(ErroCamada::CamadaZero)));
        let mut d2 = com_camadas(&["Só"]);
        d2.tornar_camada_atual(1).unwrap();
        assert_eq!(d2.excluir_camadas(&[1]), (0, Some(ErroCamada::CamadaAtual)));
    }

    #[test]
    fn camada_congelada_some_e_bloqueada_nao_se_escolhe() {
        use crate::geom::Ponto;
        let mut d = com_camadas(&["Trava"]);
        d.tornar_camada_atual(1).unwrap();
        d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(10.0, 0.0));
        let e = d.entidades()[0].clone();
        assert!(d.visivel(&e) && !d.bloqueada(&e));
        d.definir_propriedade_de_camadas(&[1], Propriedade::Bloqueada(true)).unwrap();
        assert!(d.visivel(&e) && d.bloqueada(&d.entidades()[0].clone()));
        assert_eq!(crate::edicao::escolher(&d, Ponto::novo(5.0, 0.0), 1.0), None, "bloqueada não se escolhe");
        d.tornar_camada_atual(0).unwrap();
        d.definir_propriedade_de_camadas(&[1], Propriedade::Congelada(true)).unwrap();
        assert!(!d.visivel(&d.entidades()[0].clone()));
    }
}

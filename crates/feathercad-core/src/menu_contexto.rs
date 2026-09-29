//! O menu do botão direito. **Quem decide o que aparece é o motor**, conforme o momento (comando em
//! andamento, objetos selecionados, nada); a interface só desenha a lista e devolve o `id` escolhido.
//!
//! Os `id` são de três famílias, que a interface despacha sem interpretar o desenho:
//! - `enter` e `cancelar`: as duas teclas do comando em andamento;
//! - `interruptor:snap|orto|osnap|grade`: alterna (o item traz `marcado`);
//! - `janela:propriedades|zoom_ajustar|configuracoes`: ação de tela, feita pela interface;
//! - qualquer outro é o nome de um comando do motor (`mover`, `colar`…), para `executar`.

use serde::Serialize;

use crate::documento::Documento;
use crate::sessao::Fundo;
use crate::sessao::{Ferramenta, Sessao};

/// Uma linha do menu (ou um separador).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ItemDeMenu {
    pub id: String,
    /// A chave do rótulo na língua (`menu.item.mover`).
    pub chave: String,
    /// `Ctrl+X`, `F8`…: só informativo (o motor já trata a tecla).
    pub atalho: Option<&'static str>,
    /// Interruptor: `Some(ligado)` mostra o visto.
    pub marcado: Option<bool>,
    pub ativo: bool,
    pub separador: bool,
    /// "Repetir X": o comando repetido, para a interface pôr o nome no rótulo.
    pub repetido: Option<String>,
}

impl ItemDeMenu {
    fn novo(id: &str) -> Self {
        Self { id: id.to_string(), chave: format!("menu.item.{}", id.rsplit(':').next().unwrap_or(id)), atalho: None, marcado: None, ativo: true, separador: false, repetido: None }
    }

    fn atalho(mut self, a: &'static str) -> Self {
        self.atalho = Some(a);
        self
    }

    fn ativo(mut self, ativo: bool) -> Self {
        self.ativo = ativo;
        self
    }

    fn marcado(mut self, m: bool) -> Self {
        self.marcado = Some(m);
        self
    }

    fn sep() -> Self {
        Self { separador: true, ..Self::novo("-") }
    }
}

impl Sessao {
    /// Os itens do menu do botão direito para o estado de agora.
    pub fn menu_contexto(&self, doc: &Documento) -> Vec<ItemDeMenu> {
        let mut v: Vec<ItemDeMenu> = Vec::new();
        let separar = |v: &mut Vec<ItemDeMenu>| {
            if v.last().is_some_and(|i| !i.separador) {
                v.push(ItemDeMenu::sep());
            }
        };
        let interruptores = |v: &mut Vec<ItemDeMenu>| {
            v.push(ItemDeMenu::novo("interruptor:snap").atalho("F9").marcado(self.interruptores.snap));
            v.push(ItemDeMenu::novo("interruptor:orto").atalho("F8").marcado(self.interruptores.orto));
            v.push(ItemDeMenu::novo("interruptor:osnap").atalho("F3").marcado(self.interruptores.osnap));
            v.push(ItemDeMenu::novo("interruptor:grade").atalho("F7").marcado(self.fundo != Fundo::Nada));
        };

        if self.ferramenta != Ferramenta::Nenhuma {
            // Comando em andamento: as duas teclas dele, e o que a ferramenta oferece.
            v.push(ItemDeMenu::novo("enter").atalho("Enter"));
            v.push(ItemDeMenu::novo("cancelar").atalho("Esc"));
            if matches!(&self.ferramenta, Ferramenta::Polilinha { pontos } if pontos.len() >= 3) {
                separar(&mut v);
                v.push(ItemDeMenu::novo("fechar"));
            }
            separar(&mut v);
            interruptores(&mut v);
            return v;
        }

        if self.selecionados() > 0 {
            for c in ["mover", "copiar", "girar", "escala", "espelhar", "esticar", "matriz"] {
                v.push(ItemDeMenu::novo(c));
            }
            separar(&mut v);
            v.push(ItemDeMenu::novo("recortar").atalho("Ctrl+X"));
            v.push(ItemDeMenu::novo("copiar_area").atalho("Ctrl+C"));
            v.push(ItemDeMenu::novo("colar").atalho("Ctrl+V").ativo(self.prancheta.is_some()));
            separar(&mut v);
            v.push(ItemDeMenu::novo("criar_bloco"));
            v.push(ItemDeMenu::novo("agrupar"));
            separar(&mut v);
            v.push(ItemDeMenu::novo("apagar").atalho("Del"));
            separar(&mut v);
            v.push(ItemDeMenu::novo("janela:propriedades").atalho("Ctrl+1"));
            v.push(ItemDeMenu::novo("cancelar").atalho("Esc"));
            return v;
        }

        // Nada em andamento e nada selecionado.
        if let Some(c) = &self.ultimo_comando {
            v.push(ItemDeMenu { chave: "menu.repetir".into(), repetido: Some(c.clone()), ..ItemDeMenu::novo(c) }.atalho("Enter"));
            separar(&mut v);
        }
        v.push(ItemDeMenu::novo("colar").atalho("Ctrl+V").ativo(self.prancheta.is_some()));
        v.push(ItemDeMenu::novo("desfazer").atalho("Ctrl+Z").ativo(doc.pode_desfazer()));
        v.push(ItemDeMenu::novo("refazer").atalho("Ctrl+Y").ativo(doc.pode_refazer()));
        separar(&mut v);
        v.push(ItemDeMenu::novo("janela:zoom_ajustar"));
        v.push(ItemDeMenu::novo("janela:propriedades").atalho("Ctrl+1"));
        separar(&mut v);
        interruptores(&mut v);
        separar(&mut v);
        v.push(ItemDeMenu::novo("janela:configuracoes").atalho("Ctrl+,"));
        v
    }

    /// Botão direito sobre um objeto que não estava selecionado: ele vira a seleção (como no explorador
    /// de arquivos); sobre o vazio, nada muda.
    pub fn menu_pegar_objeto(&mut self, doc: &Documento, ponto: crate::geom::Ponto, tolerancia: f64) {
        if self.ferramenta != Ferramenta::Nenhuma {
            return;
        }
        self.podar_selecao(doc);
        if let Some(id) = crate::edicao::escolher(doc, ponto, tolerancia) {
            if !self.selecao.contains(&id) {
                self.selecao.clear();
                self.selecao.insert(id);
                self.rev_selecao += 1;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ids(v: &[ItemDeMenu]) -> Vec<&str> {
        v.iter().filter(|i| !i.separador).map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn sem_nada_oferece_desfazer_zoom_e_configuracoes_e_sem_separador_dobrado() {
        let s = Sessao::default();
        let v = s.menu_contexto(&Documento::default());
        assert!(ids(&v).contains(&"janela:configuracoes"));
        assert!(v.first().is_some_and(|i| !i.separador) && v.last().is_some_and(|i| !i.separador));
        assert!(v.windows(2).all(|w| !(w[0].separador && w[1].separador)));
        assert!(!v.iter().find(|i| i.id == "desfazer").unwrap().ativo, "nada para desfazer");
    }

    #[test]
    fn com_comando_em_andamento_enter_e_cancelar_vem_primeiro() {
        let mut s = Sessao::default();
        s.ferramenta = Ferramenta::Linha { primeiro: None };
        let v = s.menu_contexto(&Documento::default());
        assert_eq!(&ids(&v)[..2], ["enter", "cancelar"]);
    }

    #[test]
    fn com_objeto_selecionado_oferece_mover_e_apagar() {
        let mut d = Documento::default();
        d.adicionar_linha(crate::geom::Ponto::novo(0.0, 0.0), crate::geom::Ponto::novo(5.0, 0.0));
        let mut s = Sessao::default();
        s.menu_pegar_objeto(&d, crate::geom::Ponto::novo(2.0, 0.0), 0.5);
        assert_eq!(s.selecionados(), 1);
        let v = s.menu_contexto(&d);
        assert!(ids(&v).contains(&"mover") && ids(&v).contains(&"apagar"));
    }

    #[test]
    fn repetir_o_ultimo_comando_aparece_com_o_nome_dele() {
        let mut d = Documento::default();
        let mut s = Sessao::default();
        s.executar(&mut d, "circulo");
        s.ferramenta = Ferramenta::Nenhuma;
        let v = s.menu_contexto(&d);
        assert_eq!(v[0].repetido.as_deref(), Some("circulo"));
    }
}

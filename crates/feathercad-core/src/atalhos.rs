//! As teclas de atalho — as mesmas do AutoCAD (`docs/ATALHOS.md`). A interface
//! só traduz o evento do teclado em texto (`Ctrl+G`, `F8`, `Esc`); **o que a
//! tecla faz é decidido aqui**.

/// Os interruptores do rodapé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interruptor {
    /// Mostrar a grade (o fundo do desenho): F7.
    Grade,
    /// Snap à grade (o cursor grudar nos pontos): F9.
    Snap,
    Orto,
    Osnap,
}

/// O que uma tecla pede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Alternar(Interruptor),
    Comando(&'static str),
    Cancelar,
    /// Pede à interface uma ação de janela (abrir/salvar…): `novo`, `abrir`,
    /// `salvar`, `salvar_como`. O motor decide **qual**; a interface executa.
    Janela(&'static str),
}

/// Toda tecla já implementada. Cada uma tem que estar em `docs/ATALHOS.md`.
pub const TECLAS: [(&str, Acao); 20] = [
    ("F7", Acao::Alternar(Interruptor::Grade)),
    ("CTRL+G", Acao::Alternar(Interruptor::Grade)),
    ("F9", Acao::Alternar(Interruptor::Snap)),
    ("F8", Acao::Alternar(Interruptor::Orto)),
    ("F3", Acao::Alternar(Interruptor::Osnap)),
    ("CTRL+F", Acao::Alternar(Interruptor::Osnap)),
    ("CTRL+Z", Acao::Comando("desfazer")),
    ("CTRL+Y", Acao::Comando("refazer")),
    ("DELETE", Acao::Comando("apagar")),
    ("CTRL+C", Acao::Comando("copiar_area")),
    ("CTRL+X", Acao::Comando("recortar")),
    ("CTRL+V", Acao::Comando("colar")),
    ("ESC", Acao::Cancelar),
    ("ESCAPE", Acao::Cancelar),
    ("CTRL+[", Acao::Cancelar),
    ("CTRL+\\", Acao::Cancelar),
    ("CTRL+N", Acao::Janela("novo")),
    ("CTRL+O", Acao::Janela("abrir")),
    ("CTRL+S", Acao::Janela("salvar")),
    ("CTRL+SHIFT+S", Acao::Janela("salvar_como")),
];

/// A ação da tecla (sem diferenciar maiúscula de minúscula), se houver.
pub fn acao_da_tecla(tecla: &str) -> Option<Acao> {
    let t = tecla.trim().to_uppercase();
    TECLAS.iter().find(|(nome, _)| *nome == t).map(|(_, a)| *a)
}

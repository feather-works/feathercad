//! A ponte: um despacho único (`Ponte::chamar`) casa o nome do comando com a
//! função do motor. Comando que muda algo devolve o **estado novo** (pequeno);
//! o desenho em si só sai pelo `quadro`, em binário.
//!
//! Nada que roda a cada movimento do mouse pergunta item a item: o
//! `cursor_mover` devolve, de uma vez, o ponto resolvido, as guias, a prévia e
//! o texto da entrada dinâmica.

#[cfg(target_arch = "wasm32")]
mod wasm;

use std::collections::BTreeMap;

use feathercad_core::atalhos::Interruptor;
use feathercad_core::camadas;
use feathercad_core::comandos::CONHECIDOS;
use feathercad_core::cursor::resolver;
use feathercad_core::documento::{Documento, TipoLinha, Unidade};
use feathercad_core::formato::Regional;
use feathercad_core::geom::Ponto;
use feathercad_core::propriedades;
use feathercad_core::quadro;
use feathercad_core::sessao::{Entrada, Fundo, Mensagem, Sessao, PASSO_DA_GRADE};
use feathercad_core::vista::Vista;
use feathercad_io::Formato;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Quantas linhas do histórico da linha de comando vão no estado.
const HISTORICO_NO_ESTADO: usize = 50;
/// Alcance do OSNAP, em pixels.
const ALCANCE_OSNAP_PX: f64 = 12.0;
/// Quão perto do objeto, em pixels, o clique o escolhe.
const ALCANCE_DA_ESCOLHA_PX: f64 = 6.0;

/// Os nomes de comando que a ponte atende (o teste confere contra o TypeScript).
pub const COMANDOS_DA_PONTE: [&str; 42] = [
    "estado",
    "comandos",
    "comandos_da_ponte",
    "idioma_definir",
    "interruptor",
    "orto_angulo_definir",
    "osnap_modos_definir",
    "atalho",
    "digitar",
    "executar",
    "cancelar",
    "clique",
    "cursor_mover",
    "linha_criar",
    "desfazer",
    "refazer",
    "camadas",
    "camada_acao",
    "propriedades_selecao",
    "propriedades_definir",
    "vista_inicial",
    "vista_deslocar",
    "vista_zoom",
    "vista_zoom_centro",
    "vista_ajustar",
    "vista_texto",
    "fundo_definir",
    "documento_novo",
    "documento_unidade_definir",
    "cota_tamanho_definir",
    "menu_contexto",
    "arrasto_iniciar",
    "arrasto_fim",
    "arrasto_cancelar",
    "documento_abrir",
    "documento_gravar",
    "documento_exportar_pdf",
    "documento_folhas",
    "documento_arquivo_definir",
    "documento_arquivo",
    "quadro",
    "quadro_arrasto",
];

/// O documento vivo + a sessão de trabalho.
#[derive(Default)]
pub struct Ponte {
    /// O último ponto do cursor (já resolvido): a direção de uma distância digitada.
    ultimo_cursor: std::cell::Cell<Option<Ponto>>,
    doc: Documento,
    sessao: Sessao,
    /// O arquivo em disco de onde veio (ou onde foi salvo) o documento.
    arquivo: Option<Arquivo>,
    /// O que a última gravação simplificou (vira aviso quando o disco confirma).
    avisos_da_gravacao: Vec<Mensagem>,
    /// O que ficou de fora ou foi simplificado no último abrir/salvar (a interface
    /// registra no console, que vai para o Monitor e para o terminal).
    avisos_do_arquivo: Vec<Mensagem>,
}

/// Onde o documento mora no disco.
#[derive(Clone, Serialize)]
struct Arquivo {
    caminho: String,
    /// O nome do arquivo, sem a pasta.
    nome: String,
    formato: &'static str,
}

#[derive(Deserialize)]
struct Idioma {
    regional: Regional,
    comandos: BTreeMap<String, Vec<String>>,
    /// Opções de ferramenta (`Fechar`); um motor/arquivo antigo pode não mandar.
    #[serde(default)]
    opcoes: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct PedidoFormato {
    formato: String,
}

/// Exportar em PDF: o papel (`A4`, `A3`, `A2`, `A1`) e se as cores do desenho ficam como estão (`cores`).
#[derive(Deserialize)]
struct PedidoPdf {
    papel: String,
    #[serde(default)]
    cores: bool,
    /// `automatica` (padrão), `horizontal` ou `vertical`.
    #[serde(default)]
    orientacao: Option<String>,
    /// Posições (a partir de 0) das folhas a exportar, na ordem; ausente = todas.
    #[serde(default)]
    folhas: Option<Vec<usize>>,
}

#[derive(Deserialize)]
struct PedidoArquivo {
    caminho: String,
    formato: String,
    /// O disco acabou de gravar este arquivo (o documento passa a "salvo").
    salvo: bool,
}

#[derive(Deserialize)]
struct PedidoInterruptor {
    nome: String,
    ligado: bool,
}

#[derive(Deserialize)]
struct Tecla {
    tecla: String,
}

#[derive(Deserialize)]
struct Texto {
    texto: String,
}

#[derive(Deserialize)]
struct PedidoComando {
    comando: String,
}

#[derive(Deserialize)]
struct PedidoMouse {
    vista: Vista,
    x: f64,
    y: f64,
    /// Shift: acrescenta/alterna na seleção em vez de trocá-la.
    #[serde(default)]
    aditivo: bool,
}

#[derive(Deserialize)]
struct PedidoLinha {
    a: Ponto,
    b: Ponto,
}

#[derive(Deserialize)]
struct PedidoTamanho {
    largura: f64,
    altura: f64,
}

#[derive(Deserialize)]
struct PedidoPan {
    vista: Vista,
    dx: f64,
    dy: f64,
}

#[derive(Deserialize)]
struct PedidoZoom {
    vista: Vista,
    x: f64,
    y: f64,
    fator: f64,
}

#[derive(Deserialize)]
struct PedidoZoomCentro {
    vista: Vista,
    fator: f64,
}

#[derive(Deserialize)]
struct PedidoVista {
    vista: Vista,
}

#[derive(Deserialize)]
struct PedidoQuadroArrasto {
    vista: Vista,
    x: f64,
    y: f64,
    #[serde(default)]
    fundo_claro: bool,
}

/// Liga/desliga tipos de snap ao objeto: só os nomes citados mudam.
#[derive(Deserialize)]
struct PedidoModosSnap {
    modos: std::collections::BTreeMap<String, bool>,
}

#[derive(Deserialize)]
struct PedidoAngulo {
    graus: f64,
}

#[derive(Deserialize)]
struct PedidoCota {
    tamanho: Option<f64>,
}

#[derive(Deserialize)]
struct PedidoUnidade {
    unidade: Option<String>,
}

#[derive(Deserialize)]
struct PedidoFundo {
    modo: Fundo,
}

#[derive(Deserialize)]
struct PedidoQuadro {
    vista: Vista,
    versao_conhecida: Option<u64>,
    /// O tema é claro? (o branco do CAD vira preto.)
    #[serde(default)]
    fundo_claro: bool,
}

#[derive(Serialize)]
struct Estado {
    versao: u64,
    /// `mm`, `cm`, `m`, ou nada se o arquivo não diz.
    unidade: Option<&'static str>,
    /// O tamanho da cota escolhido, ou nada se é o automático.
    cota_tamanho: Option<f64>,
    /// O passo do Orto, em graus (90 = só horizontal e vertical).
    orto_angulo: f64,
    /// Um objeto está sendo arrastado com o mouse (o "grab").
    arrastando: bool,
    /// O primeiro canto de uma janela de seleção já foi marcado.
    janela_pendente: bool,
    sujo: bool,
    pode_desfazer: bool,
    pode_refazer: bool,
    camada_atual: String,
    ferramenta: &'static str,
    /// Quantos objetos estão selecionados.
    selecionados: usize,
    /// Muda a cada mudança da seleção (mesmo que a contagem fique igual): a interface refaz a paleta de propriedades.
    selecao_rev: u64,
    arquivo: Option<Arquivo>,
    avisos_do_arquivo: Vec<Mensagem>,
    fundo: Fundo,
    snap: bool,
    orto: bool,
    osnap: bool,
    /// Que tipos de snap ao objeto estão ligados (`extremidade`, `meio`, `centro`…).
    osnap_modos: std::collections::BTreeMap<&'static str, bool>,
    prompt: Mensagem,
    /// Só enquanto o Inserir espera o nome: os blocos para escolher (a interface digita o nome escolhido).
    blocos: Vec<String>,
    historico: Vec<Entrada>,
}

#[derive(Serialize)]
struct Segmento {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

#[derive(Serialize)]
struct RespostaCursor {
    x: f64,
    y: f64,
    texto_x: String,
    texto_y: String,
    texto_distancia: Option<String>,
    texto_angulo: Option<String>,
    guias: Vec<Segmento>,
    /// A prévia do que está sendo desenhado: os trechos já marcados e o último até o cursor.
    previa: Vec<Segmento>,
    marca: bool,
    /// O tipo da marca de snap (`extremidade`, `meio`, `interseccao`…), se há marca.
    snap: Option<&'static str>,
    /// Com a janela de seleção aberta: `dentro` (só o que cabe inteiro, azul) ou `toca` (basta tocar, verde).
    janela: Option<&'static str>,
}

/// O pedido de uma ação sobre camadas (o gerenciador).
#[derive(Deserialize)]
struct PedidoCamada {
    /// `nova`, `renomear`, `propriedade`, `atual`, `excluir`.
    acao: String,
    #[serde(default)]
    indices: Vec<usize>,
    #[serde(default)]
    nome: String,
    /// Uma propriedade: `ligada`, `congelada`, `bloqueada`, `plotar`, `cor`, `tipo_linha`, `espessura`, `transparencia`.
    #[serde(default)]
    propriedade: String,
    #[serde(default)]
    valor: serde_json::Value,
    #[serde(default)]
    consulta: camadas::Consulta,
}

#[derive(Deserialize)]
struct PedidoCampo {
    campo: propriedades::QualCampo,
    valor: String,
}

/// A resposta de uma ação de camada: o erro (chave de mensagem), a tabela nova e o estado (para o quadro refazer).
#[derive(Serialize)]
struct RespostaCamada {
    erro: Option<&'static str>,
    /// Quantas camadas saíram (excluir) — e, se sobrou alguma, `erro` diz por quê.
    afetadas: usize,
    tabela: camadas::Tabela,
    estado: serde_json::Value,
}

/// A propriedade de camada pelo nome e valor JSON que a interface mandou.
fn propriedade_de_camada(nome: &str, valor: &serde_json::Value) -> Option<camadas::Propriedade> {
    use camadas::Propriedade as P;
    let booleano = valor.as_bool();
    match nome {
        "ligada" => booleano.map(P::Ligada),
        "congelada" => booleano.map(P::Congelada),
        "bloqueada" => booleano.map(P::Bloqueada),
        "plotar" => booleano.map(P::Plotar),
        "cor" => valor.as_str().and_then(propriedades::ler_cor).map(P::Cor),
        "tipo_linha" => valor.as_str().and_then(|v| TipoLinha::TODOS.into_iter().find(|t| t.nome_dxf().eq_ignore_ascii_case(v))).map(P::TipoLinha),
        // `null` ou texto vazio = espessura padrão.
        "espessura" => match valor {
            serde_json::Value::Null => Some(P::Espessura(None)),
            v => v.as_f64().map(|e| P::Espessura(Some(e as f32))),
        },
        "transparencia" => valor.as_u64().map(|t| P::Transparencia(t.min(90) as u8)),
        _ => None,
    }
}

/// `dxf`/`fcdz` → o formato, com erro em português.
fn formato_de(nome: &str) -> Result<Formato, String> {
    Formato::da_extensao(nome).ok_or_else(|| format!("não conheço o formato de arquivo `{nome}`"))
}

/// Lê o corpo JSON do pedido; a mensagem de erro é em português.
fn ler<T: DeserializeOwned>(comando: &str, corpo: &[u8]) -> Result<T, String> {
    serde_json::from_slice(corpo).map_err(|e| format!("pedido inválido para `{comando}`: {e}"))
}

fn json<T: Serialize>(valor: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(valor).map_err(|e| format!("não consegui montar a resposta: {e}"))
}

fn segmento(v: &Vista, a: Ponto, b: Ponto) -> Segmento {
    let (x1, y1) = v.para_tela(a);
    let (x2, y2) = v.para_tela(b);
    Segmento { x1, y1, x2, y2 }
}

impl Ponte {
    /// Atende um comando. O `Err` é a mensagem para o usuário, em português.
    pub fn chamar(&mut self, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
        match comando {
            "estado" => self.estado(),
            "comandos" => json(&CONHECIDOS.as_slice()),
            "comandos_da_ponte" => json(&COMANDOS_DA_PONTE.as_slice()),
            "idioma_definir" => self.idioma_definir(corpo),
            "interruptor" => self.interruptor(corpo),
            "atalho" => self.atalho(corpo),
            "digitar" => self.digitar(corpo),
            "executar" => self.executar(corpo),
            "cancelar" => self.cancelar(),
            "clique" => self.clique(corpo),
            "cursor_mover" => self.cursor_mover(corpo),
            "linha_criar" => self.linha_criar(corpo),
            "desfazer" | "refazer" => self.desfazer_refazer(comando),
            "camadas" => self.camadas(corpo),
            "camada_acao" => self.camada_acao(corpo),
            "propriedades_selecao" => json(&propriedades::resumo(&self.doc, self.sessao.selecao())),
            "propriedades_definir" => self.propriedades_definir(corpo),
            "vista_inicial" | "vista_deslocar" | "vista_zoom" | "vista_zoom_centro" | "vista_ajustar" => self.vista(comando, corpo),
            "vista_texto" => self.vista_texto(corpo),
            "fundo_definir" => self.fundo_definir(corpo),
            "documento_novo" => self.documento_novo(corpo),
            "menu_contexto" => self.menu_contexto(corpo),
            "arrasto_iniciar" => self.arrasto_iniciar(corpo),
            "arrasto_fim" => self.arrasto_fim(corpo),
            "arrasto_cancelar" => {
                self.sessao.arrasto_cancelar();
                self.estado()
            }
            "orto_angulo_definir" => self.orto_angulo_definir(corpo),
            "osnap_modos_definir" => self.osnap_modos_definir(corpo),
            "cota_tamanho_definir" => self.cota_tamanho_definir(corpo),
            "documento_unidade_definir" => self.documento_unidade_definir(corpo),
            "documento_abrir" => self.documento_abrir(corpo),
            "documento_gravar" => self.documento_gravar(corpo),
            "documento_exportar_pdf" => self.documento_exportar_pdf(corpo),
            "documento_folhas" => json(&feathercad_io::pdf::descrever_folhas(&self.doc)),
            "documento_arquivo_definir" => self.documento_arquivo_definir(corpo),
            "documento_arquivo" => json(&self.arquivo),
            "quadro" => self.quadro(corpo),
            "quadro_arrasto" => self.quadro_arrasto(corpo),
            _ => Err(format!("comando desconhecido na ponte: `{comando}`")),
        }
    }

    fn estado(&self) -> Result<Vec<u8>, String> {
        let s = &self.sessao;
        let h = s.historico();
        let camada = self.doc.camadas().get(self.doc.camada_atual()).map(|c| c.nome.clone());
        json(&Estado {
            versao: self.doc.versao(),
            unidade: self.doc.unidade().map(Unidade::nome),
            cota_tamanho: self.sessao.altura_cota,
            orto_angulo: self.sessao.angulo_orto,
            arrastando: self.sessao.arrastando(),
            janela_pendente: self.sessao.janela_pendente(),
            sujo: self.doc.sujo(),
            pode_desfazer: self.doc.pode_desfazer(),
            pode_refazer: self.doc.pode_refazer(),
            camada_atual: camada.unwrap_or_default(),
            ferramenta: s.ferramenta.nome(),
            selecionados: s.selecionados(),
            selecao_rev: s.revisao_da_selecao(),
            arquivo: self.arquivo.clone(),
            avisos_do_arquivo: self.avisos_do_arquivo.clone(),
            fundo: s.fundo,
            snap: s.interruptores.snap,
            orto: s.interruptores.orto,
            osnap: s.interruptores.osnap,
            osnap_modos: feathercad_core::snap::TipoSnap::TODOS.into_iter().map(|t| (t.nome(), s.modos_snap.ligado(t))).collect(),
            prompt: s.prompt(),
            blocos: if matches!(s.ferramenta, feathercad_core::sessao::Ferramenta::Insercao { bloco: None }) { self.doc.nomes_de_blocos_inseriveis() } else { Vec::new() },
            historico: h[h.len().saturating_sub(HISTORICO_NO_ESTADO)..].to_vec(),
        })
    }

    fn idioma_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let i: Idioma = ler("idioma_definir", corpo)?;
        self.sessao.comandos.definir(&i.comandos, &i.opcoes)?;
        self.sessao.regional = i.regional;
        self.estado()
    }

    fn interruptor(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoInterruptor = ler("interruptor", corpo)?;
        let i = match p.nome.as_str() {
            "grade" => Interruptor::Grade,
            "snap" => Interruptor::Snap,
            "orto" => Interruptor::Orto,
            "osnap" => Interruptor::Osnap,
            outro => return Err(format!("não existe o interruptor `{outro}`")),
        };
        self.sessao.definir(i, p.ligado);
        self.estado()
    }

    /// Responde `{ tratado, janela, estado }`: `tratado` diz se a tecla é de algum
    /// comando (a interface só usa isso para segurar a tecla do navegador) e
    /// `janela` é a ação de janela pedida (`abrir`, `salvar`…), que a interface executa.
    fn atalho(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let t: Tecla = ler("atalho", corpo)?;
        let r = self.sessao.tecla(&mut self.doc, &t.tecla);
        let estado: serde_json::Value = serde_json::from_slice(&self.estado()?).map_err(|e| format!("estado ilegível: {e}"))?;
        json(&serde_json::json!({ "tratado": r.tratado, "janela": r.janela, "estado": estado }))
    }

    fn digitar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let t: Texto = ler("digitar", corpo)?;
        self.sessao.cursor = self.ultimo_cursor.get();
        self.sessao.unidade = self.doc.unidade();
        self.sessao.digitar(&mut self.doc, &t.texto);
        self.estado()
    }

    /// Executa um comando pelo nome interno (botão da Ribbon): não passa pela
    /// tabela de apelidos da língua.
    fn executar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoComando = ler("executar", corpo)?;
        if !CONHECIDOS.contains(&p.comando.as_str()) {
            return Err(format!("o motor não tem o comando `{}`", p.comando));
        }
        self.sessao.executar(&mut self.doc, &p.comando);
        self.estado()
    }

    fn cancelar(&mut self) -> Result<Vec<u8>, String> {
        self.sessao.cancelar(&mut self.doc);
        self.estado()
    }

    fn desfazer_refazer(&mut self, comando: &str) -> Result<Vec<u8>, String> {
        self.sessao.executar(&mut self.doc, comando);
        self.estado()
    }

    fn linha_criar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let l: PedidoLinha = ler("linha_criar", corpo)?;
        self.doc.adicionar_linha(l.a, l.b);
        self.estado()
    }

    /// A tabela do gerenciador de camadas: filtrada, buscada e ordenada (tudo no Rust).
    fn camadas(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let consulta: camadas::Consulta = if corpo.is_empty() { camadas::Consulta::default() } else { ler("camadas", corpo)? };
        json(&self.doc.tabela_de_camadas(&consulta))
    }

    /// Uma ação sobre camadas; responde com o erro (se houve), a tabela nova e o estado.
    fn camada_acao(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoCamada = ler("camada_acao", corpo)?;
        let (mut erro, mut afetadas) = (None, 0);
        let resultado: Result<(), camadas::ErroCamada> = match p.acao.as_str() {
            "nova" => {
                let nome = if p.nome.trim().is_empty() { self.doc.nome_livre_de_camada("Camada") } else { p.nome.clone() };
                self.doc.nova_camada(&nome, p.indices.first().copied()).map(|_| ())
            }
            "renomear" => self.doc.renomear_camada(p.indices.first().copied().unwrap_or(usize::MAX), &p.nome),
            "atual" => self.doc.tornar_camada_atual(p.indices.first().copied().unwrap_or(usize::MAX)),
            "excluir" => {
                let (n, motivo) = self.doc.excluir_camadas(&p.indices);
                afetadas = n;
                motivo.map_or(Ok(()), Err)
            }
            "propriedade" => match propriedade_de_camada(&p.propriedade, &p.valor) {
                Some(prop) => self.doc.definir_propriedade_de_camadas(&p.indices, prop),
                None => return Err(format!("propriedade de camada desconhecida ou sem valor válido: `{}`", p.propriedade)),
            },
            outra => return Err(format!("ação de camada desconhecida: `{outra}`")),
        };
        if let Err(e) = resultado {
            erro = Some(e.chave());
        }
        let estado: serde_json::Value = serde_json::from_slice(&self.estado()?).map_err(|e| format!("estado ilegível: {e}"))?;
        json(&RespostaCamada { erro, afetadas, tabela: self.doc.tabela_de_camadas(&p.consulta), estado })
    }

    /// Muda um campo (camada, cor, tipo de linha, espessura) de todos os objetos selecionados.
    fn propriedades_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoCampo = ler("propriedades_definir", corpo)?;
        match propriedades::definir(&self.doc, self.sessao.selecao(), p.campo, &p.valor) {
            Ok(ms) => {
                self.doc.aplicar_mudancas(ms);
            }
            Err(chave) => self.sessao.dizer(Mensagem::nova(chave)),
        }
        self.estado()
    }

    /// O ponto do mouse já resolvido (snap, ortogonal, grade).
    fn resolver_mouse(&self, vista: &Vista, x: f64, y: f64) -> feathercad_core::cursor::Resolvido {
        let bruto = vista.para_mundo(x, y);
        let tolerancia = vista.pixels_em_unidades(ALCANCE_OSNAP_PX);
        resolver(&self.doc, self.sessao.base(), bruto, tolerancia, &self.sessao.ajudas())
    }

    fn clique(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoMouse = ler("clique", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        let tolerancia = m.vista.pixels_em_unidades(ALCANCE_DA_ESCOLHA_PX);
        self.sessao.clique_com(&mut self.doc, r.ponto, tolerancia, m.aditivo);
        self.estado()
    }

    /// O botão direito: o objeto sob o mouse (se não estava) vira a seleção, e volta o menu que cabe ao
    /// momento junto com o estado (a seleção pode ter mudado).
    fn menu_contexto(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoMouse = ler("menu_contexto", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        let tolerancia = m.vista.pixels_em_unidades(ALCANCE_DA_ESCOLHA_PX);
        self.sessao.menu_pegar_objeto(&self.doc, r.ponto, tolerancia);
        let itens = self.sessao.menu_contexto(&self.doc);
        let estado: serde_json::Value = serde_json::from_slice(&self.estado()?).map_err(|e| e.to_string())?;
        json(&serde_json::json!({ "estado": estado, "itens": itens }))
    }

    /// Mouse apertado sobre um objeto: começa o "grab" (arrastar para mover). `arrastando` no estado diz se pegou.
    fn arrasto_iniciar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoMouse = ler("arrasto_iniciar", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        let tolerancia = m.vista.pixels_em_unidades(ALCANCE_DA_ESCOLHA_PX);
        self.sessao.arrasto_iniciar(&self.doc, r.ponto, tolerancia);
        self.estado()
    }

    /// Mouse solto: os objetos arrastados vão para onde o cursor (já resolvido: snap, orto) está.
    fn arrasto_fim(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoMouse = ler("arrasto_fim", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        self.sessao.arrasto_fim(&mut self.doc, r.ponto);
        self.estado()
    }

    fn cursor_mover(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoMouse = ler("cursor_mover", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        self.ultimo_cursor.set(Some(r.ponto));
        let reg = &self.sessao.regional;
        let base = self.sessao.base();
        let (x, y) = m.vista.para_tela(r.ponto);
        json(&RespostaCursor {
            x,
            y,
            texto_x: reg.numero(r.ponto.x),
            texto_y: reg.numero(r.ponto.y),
            texto_distancia: base.map(|b| reg.numero(b.distancia(r.ponto))),
            texto_angulo: base.map(|b| reg.numero(b.angulo_ate(r.ponto))),
            guias: r.guias.iter().map(|g| segmento(&m.vista, g.a, g.b)).collect(),
            previa: self.previa(&m.vista, r.ponto),
            marca: r.marca,
            snap: r.tipo.map(feathercad_core::snap::TipoSnap::nome),
            janela: self.sessao.tipo_da_janela(r.ponto).map(|cruzando| if cruzando { "toca" } else { "dentro" }),
        })
    }

    /// Os segmentos da prévia, em pixels: o que a ferramenta desenharia se o usuário clicasse agora.
    fn previa(&self, vista: &Vista, cursor: Ponto) -> Vec<Segmento> {
        self.sessao
            .previa_com(&self.doc, cursor)
            .iter()
            .flat_map(|g| quadro::tracos(g, vista))
            .map(|((x1, y1), (x2, y2))| Segmento { x1, y1, x2, y2 })
            .collect()
    }

    fn vista(&self, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let nova = match comando {
            "vista_inicial" => {
                let t: PedidoTamanho = ler(comando, corpo)?;
                Vista::inicial(t.largura, t.altura)
            }
            "vista_deslocar" => {
                let p: PedidoPan = ler(comando, corpo)?;
                p.vista.deslocar(p.dx, p.dy)
            }
            "vista_zoom_centro" => {
                let z: PedidoZoomCentro = ler(comando, corpo)?;
                z.vista.zoom_centro(z.fator)
            }
            "vista_ajustar" => {
                let v: PedidoVista = ler(comando, corpo)?;
                v.vista.ajustar(self.doc.caixa())
            }
            _ => {
                let z: PedidoZoom = ler(comando, corpo)?;
                z.vista.zoom(z.x, z.y, z.fator)
            }
        };
        json(&nova)
    }

    /// O zoom como texto pronto (`100%`), na língua.
    fn vista_texto(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let v: PedidoVista = ler("vista_texto", corpo)?;
        let reg = &self.sessao.regional;
        json(&serde_json::json!({ "zoom": format!("{}%", reg.numero_com(v.vista.escala * 100.0, 0)) }))
    }

    /// Desenho em branco.
    fn documento_novo(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        // O pedido pode vir sem corpo (testes antigos); com corpo, traz a unidade escolhida.
        let unidade = if corpo.is_empty() { None } else { ler::<PedidoUnidade>("documento_novo", corpo)?.unidade.as_deref().and_then(Unidade::de_nome) };
        self.doc = Documento::default().com_unidade(unidade);
        self.arquivo = None;
        self.avisos_do_arquivo.clear();
        self.sessao.recomecar();
        self.estado()
    }

    /// De quantos em quantos graus o Orto trava a direção (90 = só horizontal e vertical).
    fn orto_angulo_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoAngulo = ler("orto_angulo_definir", corpo)?;
        if !p.graus.is_finite() || p.graus <= 0.0 || p.graus > 180.0 {
            return Err("o ângulo do Orto tem que estar entre 0 e 180 graus".into());
        }
        self.sessao.angulo_orto = p.graus;
        self.estado()
    }

    /// Liga ou desliga tipos de snap ao objeto (`extremidade`, `intersecao`, `meio`, `centro`, `quadrante`, `no`,
    /// `perpendicular`, `tangente`, `proximo`). Nome que não existe é erro (nada muda).
    fn osnap_modos_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoModosSnap = ler("osnap_modos_definir", corpo)?;
        let mut novos = self.sessao.modos_snap;
        for (nome, ligado) in &p.modos {
            let tipo = feathercad_core::snap::TipoSnap::de_nome(nome).ok_or_else(|| format!("tipo de snap `{nome}` não existe"))?;
            novos.definir(tipo, *ligado);
        }
        self.sessao.modos_snap = novos;
        self.estado()
    }

    /// O tamanho da cota (altura do texto, na unidade do desenho); `null` volta ao automático.
    fn cota_tamanho_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoCota = ler("cota_tamanho_definir", corpo)?;
        if p.tamanho.is_some_and(|t| !t.is_finite() || t <= 0.0) {
            return Err("o tamanho da cota tem que ser maior que zero".into());
        }
        self.sessao.altura_cota = p.tamanho;
        self.estado()
    }

    /// Troca a unidade do desenho já criado (conta como alteração).
    fn documento_unidade_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoUnidade = ler("documento_unidade_definir", corpo)?;
        let unidade = match p.unidade.as_deref() {
            None => None,
            Some(nome) => Some(Unidade::de_nome(nome).ok_or_else(|| format!("unidade desconhecida: `{nome}`"))?),
        };
        self.doc.definir_unidade(unidade);
        self.estado()
    }

    /// Abre os bytes de um arquivo (DXF ou `.fcdz`, pelo conteúdo) no lugar do
    /// documento atual. O que ficou de fora vai para a linha de comando.
    fn documento_abrir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let aberto = feathercad_io::abrir(corpo)?;
        self.doc = aberto.doc;
        self.arquivo = None;
        self.sessao.recomecar();
        self.avisos_do_arquivo = aberto.avisos.clone();
        for aviso in aberto.avisos {
            self.sessao.dizer(aviso);
        }
        self.estado()
    }

    /// O PDF do documento, uma página por folha (camada `FOLHA`; sem folhas, o desenho inteiro).
    fn documento_exportar_pdf(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoPdf = ler("documento_exportar_pdf", corpo)?;
        let papel = feathercad_io::pdf::Papel::de_nome(&p.papel).ok_or_else(|| format!("papel `{}` não existe (A4, A3, A2, A1, A0 ou FOLHA)", p.papel))?;
        let orientacao = match &p.orientacao {
            Some(o) => feathercad_io::pdf::Orientacao::de_nome(o).ok_or_else(|| format!("orientação `{o}` não existe (automatica, horizontal ou vertical)"))?,
            None => feathercad_io::pdf::Orientacao::Automatica,
        };
        feathercad_io::pdf::exportar_folhas_escolhidas(&self.doc, p.folhas.as_deref(), &feathercad_io::pdf::Opcoes { papel, orientacao, modo_papel: !p.cores })
    }

    /// Os bytes do documento no formato pedido (`dxf` ou `fcdz`).
    fn documento_gravar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFormato = ler("documento_gravar", corpo)?;
        let formato = formato_de(&p.formato)?;
        let g = feathercad_io::gravar(&self.doc, formato)?;
        self.avisos_da_gravacao = g.avisos;
        Ok(g.bytes)
    }

    /// O disco diz onde o documento está; se acabou de gravar, ele passa a "salvo".
    fn documento_arquivo_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoArquivo = ler("documento_arquivo_definir", corpo)?;
        let formato = formato_de(&p.formato)?;
        let nome = std::path::Path::new(&p.caminho).file_name().map_or_else(|| p.caminho.clone(), |n| n.to_string_lossy().into_owned());
        self.arquivo = Some(Arquivo { caminho: p.caminho, nome, formato: formato.extensao() });
        if p.salvo {
            self.doc.marcar_salvo();
            self.avisos_do_arquivo = std::mem::take(&mut self.avisos_da_gravacao);
            for aviso in self.avisos_do_arquivo.clone() {
                self.sessao.dizer(aviso);
            }
        }
        self.estado()
    }

    fn fundo_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFundo = ler("fundo_definir", corpo)?;
        self.sessao.definir_fundo(p.modo);
        self.estado()
    }

    fn quadro(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let q: PedidoQuadro = ler("quadro", corpo)?;
        let fundo = (self.sessao.fundo != Fundo::Nada).then_some(PASSO_DA_GRADE);
        Ok(quadro::montar_arrastando(&self.doc, &q.vista, q.versao_conhecida, fundo, q.fundo_claro, self.sessao.selecao(), self.sessao.revisao_da_selecao(), self.sessao.arrastando()))
    }

    /// Durante o "grab": os objetos arrastados, desenhados onde o cursor (já resolvido: snap, orto) os põe.
    fn quadro_arrasto(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let m: PedidoQuadroArrasto = ler("quadro_arrasto", corpo)?;
        let r = self.resolver_mouse(&m.vista, m.x, m.y);
        let (dx, dy) = self.sessao.deslocamento_do_arrasto(r.ponto).unwrap_or((0.0, 0.0));
        Ok(quadro::montar_deslocado(&self.doc, &m.vista, m.fundo_claro, self.sessao.selecao(), dx, dy))
    }
}

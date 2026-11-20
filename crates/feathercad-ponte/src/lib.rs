//! A ponte: um despacho único (`Ponte::chamar`) casa o nome do comando com a
//! função do motor. Comando que muda algo devolve o **estado novo** (pequeno);
//! o desenho em si só sai pelo `quadro`, em binário.
//!
//! Nada que roda a cada movimento do mouse pergunta item a item: o
//! `cursor_mover` devolve, de uma vez, o ponto resolvido, as guias, a prévia e
//! o texto da entrada dinâmica.

pub mod drawing_export;

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
pub const COMANDOS_DA_PONTE: [&str; 53] = [
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
    "folha_listar",
    "folha_inserir",
    "folha_escala",
    "folha_ajustar",
    "folha_campos",
    "folha_campos_definir",
    "hachura_opcoes",
    "hachura_definir",
    "hachura_previa",
    "documento_arquivo_definir",
    "documento_arquivo",
    "quadro",
    "quadro_arrasto",
    "layout_criar",
    "layout_alternar",
];

/// O documento vivo + a sessão de trabalho.
#[derive(Default)]
pub struct Ponte {
    /// O último ponto do cursor (já resolvido): a direção de uma distância digitada.
    /// (`Mutex`, não `Cell`: os comandos de leitura rodam ao mesmo tempo em várias threads.)
    ultimo_cursor: std::sync::Mutex<Option<Ponto>>,
    /// O tema da interface (0 = ainda não se sabe, 1 = fundo claro, 2 = escuro): visto nos pedidos de quadro, que levam `fundo_claro`.
    tema: std::sync::atomic::AtomicU8,
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

#[derive(Deserialize)]
struct PedidoFolhaNova {
    papel: String,
    #[serde(default)]
    escala: Option<f64>,
    #[serde(default)]
    campos: feathercad_core::folha::Campos,
    #[serde(default)]
    cobrir_selecao: Option<bool>,
    #[serde(default)]
    ids: Option<Vec<feathercad_core::documento::Id>>,
}

#[derive(Deserialize)]
struct PedidoFolhaEscala {
    indice: usize,
    escala: f64,
}

#[derive(Deserialize)]
struct PedidoFolhaIndice {
    indice: usize,
}

#[derive(Deserialize)]
struct PedidoFolhaCampos {
    indice: usize,
    campos: feathercad_core::folha::Campos,
}

#[derive(Deserialize)]
struct PedidoHachura {
    padrao: String,
    #[serde(default)]
    escala: Option<f64>,
}

#[derive(Deserialize)]
struct PedidoLayoutCriar {
    nome: String,
    papel: String,
    largura_mm: f64,
    altura_mm: f64,
}

#[derive(Deserialize)]
struct PedidoLayoutAlternar {
    nome: String,
}

#[derive(Serialize)]
struct OpcoesDeHachura {
    padroes: Vec<String>,
    atual: String,
    escala: f64,
    escalas: Vec<(f64, String)>,
}

#[derive(Serialize)]
struct FolhasDaPonte {
    folhas: Vec<feathercad_core::folha::InfoFolha>,
    papeis: Vec<String>,
    /// (denominador, texto) — `[100, "1:100"]`.
    escalas: Vec<(f64, String)>,
}

#[derive(Serialize)]
struct RespostaFolha {
    folha: feathercad_core::folha::InfoFolha,
    estado: serde_json::Value,
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
    layouts: Vec<feathercad_core::documento::InfoLayout>,
    espaco_ativo: feathercad_core::documento::Espaco,
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
    z: f64,
    texto_x: String,
    texto_y: String,
    texto_z: String,
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
    /// O documento vivo, para quem trabalha **nele** por fora da interface (o MCP ao vivo). Quem muda o documento deve chamar
    /// [`Ponte::depois_de_mudanca_externa`] se a versão mudou.
    pub fn documento_mut(&mut self) -> &mut Documento {
        &mut self.doc
    }

    /// O tema da interface: `Some(true)` = fundo claro; `None` = nenhum quadro foi pedido ainda.
    pub fn tema_claro(&self) -> Option<bool> {
        match self.tema.load(std::sync::atomic::Ordering::Relaxed) {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }

    /// O caminho do arquivo aberto, se há.
    pub fn caminho_do_arquivo(&self) -> Option<&str> {
        self.arquivo.as_ref().map(|a| a.caminho.as_str())
    }

    /// O desenho passou a não ter arquivo (ex.: a IA abriu uma nuvem de pontos, que ainda não é um desenho gravado).
    pub fn esquecer_arquivo(&mut self) {
        self.arquivo = None;
    }

    /// O documento mudou por fora (a IA desenhou): cancela a ferramenta em andamento e a seleção, que podem apontar para
    /// objetos que não existem mais. Mantém o histórico do desenho (Ctrl+Z desfaz o passo da IA) e os interruptores.
    pub fn depois_de_mudanca_externa(&mut self) {
        self.sessao.recomecar();
    }

    /// Atende um comando. O `Err` é a mensagem para o usuário, em português.
    /// Os comandos que **só leem** o estado (o quadro, o cursor, a vista): podem rodar ao mesmo tempo, sem esperar
    /// uns pelos outros. A casca os chama com uma trava de leitura, para o mouse continuar andando enquanto o
    /// quadro é calculado. `None` = não é comando de leitura (use `chamar`).
    pub fn chamar_leitura(&self, comando: &str, corpo: &[u8]) -> Option<Result<Vec<u8>, String>> {
        Some(match comando {
            "quadro" => self.quadro(corpo),
            "quadro_arrasto" => self.quadro_arrasto(corpo),
            "cursor_mover" => self.cursor_mover(corpo),
            "vista_inicial" | "vista_deslocar" | "vista_zoom" | "vista_zoom_centro" | "vista_ajustar" => self.vista(comando, corpo),
            "vista_texto" => self.vista_texto(corpo),
            "estado" => self.estado(),
            _ => return None,
        })
    }

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
            "folha_listar" => json(&self.folhas_da_ponte()),
            "folha_inserir" => self.folha_inserir(corpo),
            "folha_escala" => self.folha_escala(corpo),
            "folha_ajustar" => self.folha_ajustar(corpo),
            "folha_campos" => {
                let p: PedidoFolhaIndice = ler("folha_campos", corpo)?;
                json(&self.doc.folha_campos(p.indice)?)
            }
            "folha_campos_definir" => self.folha_campos_definir(corpo),
            "hachura_opcoes" => json(&self.hachura_opcoes()),
            "hachura_definir" => {
                let p: PedidoHachura = ler("hachura_definir", corpo)?;
                self.sessao.definir_hachura(&p.padrao, p.escala.unwrap_or_else(|| self.sessao.hachura_escolhida().1))?;
                json(&self.hachura_opcoes())
            }
            "hachura_previa" => {
                let p: PedidoHachura = ler("hachura_previa", corpo)?;
                json(&feathercad_core::padroes::previa_svg(&p.padrao, p.escala.unwrap_or(1.0), 220.0, 120.0))
            }
            "documento_arquivo_definir" => self.documento_arquivo_definir(corpo),
            "documento_arquivo" => json(&self.arquivo),
            "quadro" => self.quadro(corpo),
            "quadro_arrasto" => self.quadro_arrasto(corpo),
            "layout_criar" => self.layout_criar(corpo),
            "layout_alternar" => self.layout_alternar(corpo),
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
            layouts: self.doc.layouts().to_vec(),
            espaco_ativo: self.doc.espaco_ativo().clone(),
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

    fn ultimo_cursor(&self) -> Option<Ponto> {
        *self.ultimo_cursor.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn guardar_ultimo_cursor(&self, p: Ponto) {
        *self.ultimo_cursor.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
    }

    fn digitar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let t: Texto = ler("digitar", corpo)?;
        self.sessao.cursor = self.ultimo_cursor();
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
        let z_plano = self.sessao.base().map(|b| b.z).unwrap_or(self.sessao.elevacao);
        let bruto = vista.para_mundo_em_z_com_ref(x, y, z_plano, self.sessao.base());
        let tolerancia = vista.pixels_em_unidades(ALCANCE_OSNAP_PX);

        if self.sessao.ajudas().osnap {
            if let Some(a) = feathercad_core::snap::achar(&self.doc, self.sessao.base(), bruto, tolerancia, &self.sessao.ajudas().modos) {
                return feathercad_core::cursor::Resolvido { ponto: a.ponto, guias: Vec::new(), marca: true, tipo: Some(a.tipo) };
            }
        }

        let r_xy = resolver(&self.doc, self.sessao.base(), bruto, tolerancia, &self.sessao.ajudas());

        if let (Some(b), true) = (self.sessao.base(), vista.rotacao_x.abs() > 1e-4 || vista.rotacao_z.abs() > 1e-4) {
            let (s0_x, s0_y) = vista.para_tela(b);

            // 1. Z-axis projection (vertical ray in 3D)
            let s1_z = vista.para_tela(Ponto::novo3d(b.x, b.y, b.z + 1.0));
            let uz_x = s1_z.0 - s0_x;
            let uz_y = s1_z.1 - s0_y;
            let len_z_sq = uz_x * uz_x + uz_y * uz_y;

            let (p_z, dist_z_perp) = if len_z_sq > 1e-6 {
                let mx = x - s0_x;
                let my = y - s0_y;
                let dz = (mx * uz_x + my * uz_y) / len_z_sq;
                let p = Ponto::novo3d(b.x, b.y, b.z + dz);
                let (proj_x, proj_y) = (s0_x + dz * uz_x, s0_y + dz * uz_y);
                let d_perp = ((x - proj_x).powi(2) + (y - proj_y).powi(2)).sqrt();
                (p, d_perp)
            } else {
                (b, f64::INFINITY)
            };

            // 2. X-axis projection
            let s1_x = vista.para_tela(Ponto::novo3d(b.x + 1.0, b.y, b.z));
            let ux_x = s1_x.0 - s0_x;
            let ux_y = s1_x.1 - s0_y;
            let len_x_sq = ux_x * ux_x + ux_y * ux_y;

            let (p_x, dist_x_perp) = if len_x_sq > 1e-6 {
                let mx = x - s0_x;
                let my = y - s0_y;
                let dx = (mx * ux_x + my * ux_y) / len_x_sq;
                let p = Ponto::novo3d(b.x + dx, b.y, b.z);
                let (proj_x, proj_y) = (s0_x + dx * ux_x, s0_y + dx * ux_y);
                let d_perp = ((x - proj_x).powi(2) + (y - proj_y).powi(2)).sqrt();
                (p, d_perp)
            } else {
                (b, f64::INFINITY)
            };

            // 3. Y-axis projection
            let s1_y = vista.para_tela(Ponto::novo3d(b.x, b.y + 1.0, b.z));
            let uy_x = s1_y.0 - s0_x;
            let uy_y = s1_y.1 - s0_y;
            let len_y_sq = uy_x * uy_x + uy_y * uy_y;

            let (p_y, dist_y_perp) = if len_y_sq > 1e-6 {
                let mx = x - s0_x;
                let my = y - s0_y;
                let dy = (mx * uy_x + my * uy_y) / len_y_sq;
                let p = Ponto::novo3d(b.x, b.y + dy, b.z);
                let (proj_x, proj_y) = (s0_x + dy * uy_x, s0_y + dy * uy_y);
                let d_perp = ((x - proj_x).powi(2) + (y - proj_y).powi(2)).sqrt();
                (p, d_perp)
            } else {
                (b, f64::INFINITY)
            };

            let limit_px = if self.sessao.ajudas().orto { 120.0 } else { 35.0 };
            if dist_z_perp < limit_px && dist_z_perp <= dist_x_perp && dist_z_perp <= dist_y_perp {
                return feathercad_core::cursor::Resolvido {
                    ponto: p_z,
                    guias: vec![feathercad_core::cursor::Guia { a: b, b: p_z }],
                    marca: false,
                    tipo: None,
                };
            } else if self.sessao.ajudas().orto {
                if dist_x_perp < dist_y_perp && dist_x_perp < limit_px {
                    return feathercad_core::cursor::Resolvido {
                        ponto: p_x,
                        guias: vec![feathercad_core::cursor::Guia { a: b, b: p_x }],
                        marca: false,
                        tipo: None,
                    };
                } else if dist_y_perp < limit_px {
                    return feathercad_core::cursor::Resolvido {
                        ponto: p_y,
                        guias: vec![feathercad_core::cursor::Guia { a: b, b: p_y }],
                        marca: false,
                        tipo: None,
                    };
                }
            }
        }

        r_xy
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
        self.guardar_ultimo_cursor(r.ponto);
        let reg = &self.sessao.regional;
        let base = self.sessao.base();
        let (x, y) = m.vista.para_tela(r.ponto);
        json(&RespostaCursor {
            x,
            y,
            z: r.ponto.z,
            texto_x: reg.numero(r.ponto.x),
            texto_y: reg.numero(r.ponto.y),
            texto_z: reg.numero(r.ponto.z),
            texto_distancia: base.map(|b| reg.numero(b.distancia_3d(r.ponto))),
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
    /// As folhas inseridas e as escalas oferecidas, para o diálogo (o texto de cada escala vem daqui, não da interface).
    fn folhas_da_ponte(&self) -> FolhasDaPonte {
        FolhasDaPonte {
            folhas: self.doc.folhas_inseridas(),
            papeis: feathercad_core::folha::PAPEIS.iter().map(|(n, _, _)| (*n).to_string()).collect(),
            escalas: feathercad_core::folha::ESCALAS.iter().map(|e| (*e, feathercad_core::folha::texto_da_escala(*e))).collect(),
        }
    }

    /// Insere uma folha (`papel`, `escala` ou nada = a que cabe, campos do carimbo) e devolve o que mudou.
    fn folha_inserir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFolhaNova = ler("folha_inserir", corpo)?;
        let ids_alvo = match (p.ids.as_deref(), p.cobrir_selecao) {
            (Some(ids), _) if !ids.is_empty() => Some(ids.to_vec()),
            (_, Some(true)) | (_, None) if !self.sessao.selecao().is_empty() => {
                Some(self.sessao.selecao().iter().copied().collect())
            }
            _ => None,
        };
        let folha = self.doc.folha_inserir_ids(&p.papel, p.escala, &p.campos, ids_alvo.as_deref())?;
        json(&RespostaFolha { folha, estado: serde_json::from_slice(&self.estado()?).map_err(|e| e.to_string())? })
    }

    fn folha_escala(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFolhaEscala = ler("folha_escala", corpo)?;
        let folha = self.doc.folha_escala(p.indice, p.escala)?;
        json(&RespostaFolha { folha, estado: serde_json::from_slice(&self.estado()?).map_err(|e| e.to_string())? })
    }

    /// A lista de padrões de hachura, o escolhido e as escalas oferecidas (o texto de cada um vem dos arquivos de língua).
    fn hachura_opcoes(&self) -> OpcoesDeHachura {
        let (atual, escala) = self.sessao.hachura_escolhida();
        OpcoesDeHachura {
            padroes: std::iter::once(feathercad_core::padroes::SOLIDA).chain(feathercad_core::padroes::NOMES).map(str::to_string).collect(),
            atual: atual.to_string(),
            escala,
            escalas: feathercad_core::padroes::ESCALAS.iter().map(|(v, t)| (*v, (*t).to_string())).collect(),
        }
    }

    fn folha_campos_definir(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFolhaCampos = ler("folha_campos_definir", corpo)?;
        let folha = self.doc.folha_campos_definir(p.indice, &p.campos)?;
        json(&RespostaFolha { folha, estado: serde_json::from_slice(&self.estado()?).map_err(|e| e.to_string())? })
    }

    fn folha_ajustar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoFolhaIndice = ler("folha_ajustar", corpo)?;
        let folha = self.doc.folha_ajustar(p.indice)?;
        json(&RespostaFolha { folha, estado: serde_json::from_slice(&self.estado()?).map_err(|e| e.to_string())? })
    }

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
        let papel = feathercad_io::pdf::Papel::de_nome(&p.papel).ok_or_else(|| format!("papel `{}` não existe (A4, A3, A2, A1, A0, 2A0, 4A0, 8A0, 16A0 ou FOLHA)", p.papel))?;
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

    fn layout_criar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoLayoutCriar = ler("layout_criar", corpo)?;
        self.doc.adicionar_layout(p.nome, p.papel, p.largura_mm, p.altura_mm);
        self.estado()
    }

    fn layout_alternar(&mut self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let p: PedidoLayoutAlternar = ler("layout_alternar", corpo)?;
        let espaco = if p.nome.eq_ignore_ascii_case("modelo") || p.nome.eq_ignore_ascii_case("model") {
            feathercad_core::documento::Espaco::Modelo
        } else {
            feathercad_core::documento::Espaco::Layout(p.nome)
        };
        self.doc.definir_espaco_ativo(espaco);
        self.estado()
    }

    fn quadro(&self, corpo: &[u8]) -> Result<Vec<u8>, String> {
        let q: PedidoQuadro = ler("quadro", corpo)?;
        let fundo = (self.sessao.fundo != Fundo::Nada).then_some(PASSO_DA_GRADE);
        self.tema.store(if q.fundo_claro { 1 } else { 2 }, std::sync::atomic::Ordering::Relaxed);
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

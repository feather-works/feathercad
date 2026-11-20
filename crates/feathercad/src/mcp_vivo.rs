//! MCP **ao vivo**: o `feathercad-mcp` que a IA abre (stdio) repassa cada chamada para cá pelo canal local de
//! `feathercad_mcp::ao_vivo` (socket do usuário ou TCP com token no Windows; só nesta máquina). A thread do canal entrega o
//! pedido ao `update` por uma subscription e espera a resposta. O documento **não é copiado**: durante a ferramenta ele
//! troca de lugar com o do `Agente` (que guarda o resto da sessão: alinhamentos, nuvens, greides) e volta no fim.

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use feathercad_mcp::agente::Agente;
use feathercad_mcp::fonte::{self, Fonte};
use feathercad_mcp::{ao_vivo, rpc};
use pelicano::futures::{channel::mpsc as canal, StreamExt};
use pelicano::Subscription;
use serde_json::{json, Value};

use crate::motor::Motor;

/// Um pedido vindo do canal, com o caminho de volta da resposta.
#[derive(Clone)]
pub struct PedidoMcp(Arc<(Value, Mutex<Option<mpsc::Sender<Value>>>)>);

impl std::fmt::Debug for PedidoMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PedidoMcp({})", self.0 .0.get("method").and_then(Value::as_str).unwrap_or("?"))
    }
}

impl PedidoMcp {
    /// A thread do canal desistiu de esperar (e já disse à IA que falhou): rodar agora faria a ferramenta duas
    /// vezes quando a IA tentasse de novo.
    fn vencido(&self) -> bool {
        self.0 .0.get("id").is_some() && self.0 .1.lock().map_or(true, |g| g.is_none())
    }

    fn responder(&self, r: Value) {
        if let Some(tx) = self.0 .1.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(r);
        }
    }
}

static RECEPTOR: OnceLock<Mutex<Option<canal::UnboundedReceiver<PedidoMcp>>>> = OnceLock::new();
static ENVIO: OnceLock<Mutex<canal::UnboundedSender<PedidoMcp>>> = OnceLock::new();

/// Quanto a thread do canal espera o aplicativo (abrir uma nuvem grande demora).
const ESPERA: Duration = Duration::from_secs(300);

fn recusa(id: Value, texto: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": { "isError": true, "content": [{ "type": "text", "text": texto }] } })
}

/// Abre o canal na largada. Fora nos testes e com `FEATHERCAD_VIVO=0`; um segundo FeatherCAD não toma o canal do primeiro
/// (ele entrega os arquivos ao primeiro e sai: ver `main.rs`).
pub fn ligar() -> String {
    if std::env::var("FEATHERCAD_VIVO").is_ok_and(|v| v == "0") || cfg!(test) {
        return "MCP ao vivo desligado".into();
    }
    let (tx, rx) = canal::unbounded::<PedidoMcp>();
    let _ = RECEPTOR.set(Mutex::new(Some(rx)));
    let _ = ENVIO.set(Mutex::new(tx));
    let tratar = |msg: Value| -> Option<Value> {
        let id = msg.get("id").cloned();
        let (rtx, rrx) = mpsc::channel();
        let par = Arc::new((msg, Mutex::new(Some(rtx))));
        ENVIO.get()?.lock().ok()?.unbounded_send(PedidoMcp(Arc::clone(&par))).ok()?;
        // Notificação (sem `id`): ninguém espera resposta.
        let id = id?;
        Some(rrx.recv_timeout(ESPERA).unwrap_or_else(|_| {
            // Tira o canal de resposta: o pedido que ficou na fila vira "vencido" e não roda mais.
            if let Ok(mut g) = par.1.lock() {
                g.take();
            }
            recusa(id, "O FeatherCAD não respondeu a tempo (ocupado ou com uma caixa de diálogo aberta).")
        }))
    };
    match ao_vivo::servir(tratar) {
        Ok(true) => format!("MCP ao vivo escutando em {}", ao_vivo::onde_escuta().display()),
        Ok(false) => "MCP ao vivo: outro FeatherCAD já atende o canal".into(),
        Err(e) => format!("MCP ao vivo indisponível: {e}"),
    }
}

pub fn encerrar() {
    if RECEPTOR.get().is_some() {
        ao_vivo::encerrar();
    }
}

pub fn assinatura<M: Send + 'static>(f: fn(PedidoMcp) -> M) -> Subscription<M> {
    let Some(rx) = RECEPTOR.get().and_then(|m| m.lock().ok()?.take()) else {
        return if RECEPTOR.get().is_some() { Subscription::run_with_id("feathercad.mcp", pelicano::futures::stream::pending()) } else { Subscription::none() };
    };
    Subscription::run_with_id("feathercad.mcp", rx.map(f))
}

/// A sessão da IA (o que não é o documento) e a fonte das legendas da imagem.
pub struct Vivo {
    agente: Agente,
    fonte: Fonte,
}

impl Default for Vivo {
    fn default() -> Self {
        Vivo { agente: Agente::default(), fonte: fonte::carregar() }
    }
}

/// O que a ferramenta fez, para o app se pôr em dia.
pub struct Resultado {
    pub mudou_o_desenho: bool,
    /// O arquivo do desenho mudou (a IA abriu, criou ou salvou como): o novo caminho.
    pub arquivo: Option<Option<PathBuf>>,
    /// Pedido de outro FeatherCAD aberto: os arquivos para abrir aqui.
    pub abrir: Vec<String>,
}

impl Vivo {
    /// Atende o pedido sobre o desenho da tela e responde à thread do canal.
    pub fn atender(&mut self, motor: &mut Motor, pedido: &PedidoMcp, tema_claro: bool) -> Resultado {
        let msg = &pedido.0 .0;
        let mut r = Resultado { mudou_o_desenho: false, arquivo: None, abrir: Vec::new() };
        if pedido.vencido() {
            return r;
        }
        let resposta = match msg.get("method").and_then(Value::as_str) {
            Some(ao_vivo::METODO_INSTANCIA) => {
                r.abrir = msg.pointer("/params/arquivos").and_then(Value::as_array).into_iter().flatten().filter_map(|a| a.as_str().map(str::to_string)).collect();
                Some(json!({ "jsonrpc": "2.0", "id": msg.get("id").cloned().unwrap_or(Value::Null), "result": { "ok": true } }))
            }
            Some("tools/call") => {
                let ag = &mut self.agente;
                std::mem::swap(&mut ag.doc, motor.ponte.documento_mut());
                let versao = ag.doc.versao();
                ag.tema_claro = Some(tema_claro);
                ag.arquivo = motor.ponte.caminho_do_arquivo().map(PathBuf::from);
                let antes = ag.arquivo.clone();
                let resposta = rpc::atender(ag, &self.fonte, msg);
                r.mudou_o_desenho = ag.doc.versao() != versao;
                std::mem::swap(&mut ag.doc, motor.ponte.documento_mut());
                if r.mudou_o_desenho {
                    motor.ponte.depois_de_mudanca_externa();
                }
                if ag.arquivo != antes {
                    r.arquivo = Some(ag.arquivo.clone());
                }
                resposta
            }
            _ => rpc::atender(&mut self.agente, &self.fonte, msg),
        };
        if let Some(v) = resposta {
            pedido.responder(v);
        }
        r
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pedido(v: Value) -> (PedidoMcp, mpsc::Receiver<Value>) {
        let (tx, rx) = mpsc::channel();
        (PedidoMcp(Arc::new((v, Mutex::new(Some(tx))))), rx)
    }

    #[test]
    fn pedido_que_venceu_nao_roda_mais() {
        let mut motor = Motor::default();
        motor.estado_de("documento_novo", json!({ "unidade": "mm" })).unwrap();
        let antes = motor.estado().unwrap().versao;
        let (p, _rx) = pedido(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "cad_criar", "arguments": { "formas": [{ "tipo": "linha", "a": [0, 0], "b": [10, 0] }] } } }));
        p.0 .1.lock().unwrap().take();
        assert!(!Vivo::default().atender(&mut motor, &p, true).mudou_o_desenho);
        assert_eq!(motor.estado().unwrap().versao, antes);
    }

    #[test]
    fn a_ferramenta_desenha_no_documento_da_tela() {
        let mut motor = Motor::default();
        motor.estado_de("documento_novo", json!({ "unidade": "mm" })).unwrap();
        let antes = motor.estado().unwrap().versao;
        let mut vivo = Vivo::default();
        let (p, rx) = pedido(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "cad_criar", "arguments": { "formas": [{ "tipo": "linha", "a": [0, 0], "b": [10, 0] }] } } }));
        let r = vivo.atender(&mut motor, &p, true);
        let resposta = rx.recv().unwrap();
        assert!(r.mudou_o_desenho, "resposta: {resposta}");
        assert_ne!(motor.estado().unwrap().versao, antes);
        assert!(motor.estado().unwrap().pode_desfazer, "a ferramenta é um passo do Ctrl+Z");
    }
}

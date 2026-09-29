//! Métricas de cada comando do `ponte://`, no terminal do aplicativo.
//!
//! Pedido do usuário (24/09/2026): todo comando com número, para comparar.
//! Copiado do `feathersheet/src-tauri/src/metricas.rs` (29/09/2026, pedido do
//! usuário: manter o F12). É o lado de DENTRO do motor; o lado da interface (`src/ponte/metricas.ts`)
//! mede a ida e volta e o gesto inteiro. A diferença entre os dois é a
//! travessia (o XHR síncrono e o JSON).
//!
//! - comando lento (≥ 5 ms) ou que falhou: uma linha na hora,
//!   `[motor] borda — 31 ms (espera 0 ms) · 120 B -> 12 B`;
//! - o resto é agrupado e sai no máximo uma vez por segundo, na chamada
//!   seguinte: `[motor] 1,0 s: cursor_mover ×37 12 ms (máx 1 ms) · quadro ×4 0 ms`.
//!   Sem isso o movimento do mouse imprimiria dezenas de linhas por segundo.
//!
//! A espera é o tempo parado na trava da ponte: um comando rápido que esperou
//! 2 s atrás de uma gravação aparece aqui, e não seria visto de outro jeito.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LENTO: Duration = Duration::from_millis(5);
const JANELA: Duration = Duration::from_secs(1);

#[derive(Default)]
struct Grupo {
    vezes: u32,
    total: Duration,
    maximo: Duration,
}

struct Agregado {
    desde: Instant,
    grupos: BTreeMap<String, Grupo>,
}

static AGREGADO: Mutex<Option<Agregado>> = Mutex::new(None);

/// As últimas linhas impressas, para a página Monitor ler sem o terminal
/// (`metricas_do_motor`). Cada uma com um número crescente: quem lê pede
/// "o que veio depois do número tal".
static HISTORICO: Mutex<std::collections::VecDeque<(u64, String)>> = Mutex::new(std::collections::VecDeque::new());
static PROXIMO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
const TETO_DO_HISTORICO: usize = 2000;

/// Imprime no terminal e guarda no histórico.
fn publicar(linha: String) {
    eprintln!("{linha}");
    let id = PROXIMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut h = HISTORICO.lock().unwrap_or_else(|p| p.into_inner());
    if h.len() >= TETO_DO_HISTORICO {
        h.pop_front();
    }
    h.push_back((id, linha));
}

/// As linhas do motor depois de `desde` (0 = todas as guardadas).
#[tauri::command]
pub fn metricas_do_motor(desde: u64) -> Vec<(u64, String)> {
    let h = HISTORICO.lock().unwrap_or_else(|p| p.into_inner());
    h.iter().filter(|(id, _)| *id > desde).cloned().collect()
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}

/// Anota um comando atendido.
pub fn registrar(comando: &str, espera: Duration, motor: Duration, entrada: usize, saida: Result<usize, &str>) {
    if motor >= LENTO || espera >= LENTO || saida.is_err() {
        let resultado = match saida {
            Ok(n) => format!("{entrada} B -> {n} B"),
            Err(e) => format!("{entrada} B -> ERRO: {e}"),
        };
        publicar(format!("[motor] {comando} — {} (espera {}) · {resultado}", ms(motor), ms(espera)));
    } else {
        let mut guarda = AGREGADO.lock().unwrap_or_else(|p| p.into_inner());
        let agregado = guarda.get_or_insert_with(|| Agregado { desde: Instant::now(), grupos: BTreeMap::new() });
        let g = agregado.grupos.entry(comando.to_string()).or_default();
        g.vezes += 1;
        g.total += motor;
        g.maximo = g.maximo.max(motor);
    }
    despejar_se_passou();
}

/// Imprime o agrupado quando a janela de um segundo passou.
fn despejar_se_passou() {
    let mut guarda = AGREGADO.lock().unwrap_or_else(|p| p.into_inner());
    let Some(agregado) = guarda.as_ref() else { return };
    let passou = agregado.desde.elapsed();
    if passou < JANELA {
        return;
    }
    let agregado = guarda.take().expect("conferido acima");
    let partes: Vec<String> = agregado
        .grupos
        .iter()
        .map(|(nome, g)| format!("{nome} ×{} {} (máx {})", g.vezes, ms(g.total), ms(g.maximo)))
        .collect();
    if !partes.is_empty() {
        let segundos = format!("{:.1}", passou.as_secs_f64()).replace('.', ",");
        publicar(format!("[motor] {segundos} s: {}", partes.join(" · ")));
    }
}

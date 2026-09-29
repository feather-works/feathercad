//! Memória residente da **árvore inteira** de processos do aplicativo.
//! (Copiado do FeatherSheet em 29/09/2026: o mesmo medidor, pelo mesmo motivo.)
//!
//! Existe porque `uso_memoria_processo()` (`/proc/self/status`) mede só o
//! processo principal do Tauri, e no Linux quem executa o JavaScript e guarda
//! o desenho e o JavaScript **não é ele**: o WebKitGTK põe o conteúdo web num
//! processo filho (`WebKitWebProcess`). Medir só o pai é medir o lado que não
//! estoura.
//!
//! Isso não é detalhe: os três commits de 21/09 (PIO2, fantasma, PIW1) foram
//! revertidos por terem PIORADO o estouro, e ninguém viu na hora porque os
//! números vinham deste medidor cego e de exemplos rodando fora do aplicativo
//! (`examples/medir_leitura.rs`, Node). É a quarta vez que o projeto persegue
//! um pico de RAM às cegas — ver "Cuidados ao mexer" no `CLAUDE.md`.
//!
//! Aqui se percorre `/proc` uma vez, monta-se pai -> filhos e soma-se o RSS de
//! todo descendente do processo atual, devolvendo também a lista por processo:
//! sem a quebra por processo não dá para saber se quem cresceu foi o Rust ou a
//! WebView, que é exatamente a pergunta que não se conseguiu responder ontem.

#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::fs;

/// Um processo da árvore: nome curto, pid e RSS em bytes.
#[derive(serde::Serialize, Clone)]
pub struct ProcessoMedido {
    pub pid: u32,
    pub nome: String,
    pub rss: u64,
    /// Pico histórico deste processo (`VmHWM`), em bytes.
    pub pico: u64,
}

/// O retrato inteiro: o total e a quebra por processo, do maior para o menor.
#[derive(serde::Serialize)]
pub struct MemoriaArvore {
    /// Soma do RSS de todos os processos da árvore, em bytes.
    pub total: u64,
    /// Só o processo principal do Tauri (o que o medidor antigo via).
    pub principal: u64,
    /// Só os processos de WebView (é aqui que a planilha realmente mora).
    pub webview: u64,
    /// Soma dos **picos** (`VmHWM`) da árvore.
    ///
    /// Sem isto o medidor mente por omissão: ele só vê o instante em que foi
    /// chamado, e o pico costuma acontecer *dentro* de uma etapa, entre duas
    /// chamadas. Foi exatamente assim que um pico real de 12 GB não apareceu
    /// numa medição que dizia 681 MB.
    pub pico: u64,
    /// O pico só do processo principal do Tauri.
    pub pico_principal: u64,
    pub processos: Vec<ProcessoMedido>,
}

/// Lê `VmRSS` (agora) e `VmHWM` (o pico) de `/proc/<pid>/status`, em bytes.
#[cfg(target_os = "linux")]
fn rss_de(pid: u32) -> Option<(u64, u64)> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let campo = |nome: &str| -> Option<u64> {
        status
            .lines()
            .find_map(|l| l.strip_prefix(nome))
            .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<u64>().ok())
            .map(|kb| kb * 1024)
    };
    Some((campo("VmRSS:")?, campo("VmHWM:").unwrap_or(0)))
}

/// Nome curto do processo (`/proc/<pid>/comm`).
#[cfg(target_os = "linux")]
fn nome_de(pid: u32) -> String {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| String::from("?"))
}

/// O pai de um processo — campo 4 de `/proc/<pid>/stat`.
///
/// O nome do executável é o campo 2 e vem entre parênteses, podendo conter
/// espaço; por isso se corta a partir do `)` final em vez de dividir a linha
/// inteira por espaço.
#[cfg(target_os = "linux")]
fn pai_de(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let depois = &stat[stat.rfind(')')? + 1..];
    depois.split_whitespace().nth(1)?.parse().ok()
}

/// Soma o RSS do processo atual e de todos os seus descendentes.
#[cfg(target_os = "linux")]
pub fn medir() -> Option<MemoriaArvore> {
    let eu = std::process::id();

    // Uma varredura só de /proc, montando pai -> filhos.
    let mut filhos: HashMap<u32, Vec<u32>> = HashMap::new();
    for entrada in fs::read_dir("/proc").ok()?.flatten() {
        let Ok(pid) = entrada.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if let Some(pai) = pai_de(pid) {
            filhos.entry(pai).or_default().push(pid);
        }
    }

    // Desce a árvore a partir de mim.
    let mut processos = Vec::new();
    let mut fila = vec![eu];
    while let Some(pid) = fila.pop() {
        if let Some((rss, pico)) = rss_de(pid) {
            processos.push(ProcessoMedido { pid, nome: nome_de(pid), rss, pico });
        }
        if let Some(meus) = filhos.get(&pid) {
            fila.extend(meus.iter().copied());
        }
    }

    let total = processos.iter().map(|p| p.rss).sum();
    let pico = processos.iter().map(|p| p.pico).sum();
    let principal = processos.iter().find(|p| p.pid == eu).map_or(0, |p| p.rss);
    let pico_principal = processos.iter().find(|p| p.pid == eu).map_or(0, |p| p.pico);
    // O WebKitGTK nomeia os filhos "WebKitWebProcess"/"WebKitNetworkProcess";
    // a comparação é sem caixa e por trecho para não depender da versão.
    let webview = processos
        .iter()
        .filter(|p| p.nome.to_lowercase().contains("webkit"))
        .map(|p| p.rss)
        .sum();

    processos.sort_by_key(|p| std::cmp::Reverse(p.rss));
    Some(MemoriaArvore { total, principal, webview, pico, pico_principal, processos })
}

#[cfg(not(target_os = "linux"))]
pub fn medir() -> Option<MemoriaArvore> {
    None
}

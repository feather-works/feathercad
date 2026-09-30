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
#[cfg(target_os = "linux")]
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::time::Instant;

/// Um processo da árvore: nome curto, pid e RSS em bytes.
#[derive(serde::Serialize, Clone)]
pub struct ProcessoMedido {
    pub pid: u32,
    pub nome: String,
    pub rss: u64,
    /// Pico histórico deste processo (`VmHWM`), em bytes.
    pub pico: u64,
    /// Uso de CPU desde a medição anterior, em % de **um núcleo** (200 = dois núcleos cheios).
    pub cpu: f64,
    /// Uso da GPU desde a medição anterior, em % do tempo das engines do processo (`fdinfo` do DRM); 0 se ele não usa GPU.
    pub gpu: f64,
    /// VRAM residente que o processo ocupa, em bytes (`fdinfo` do DRM).
    pub vram: u64,
}

/// A placa de vídeo do sistema, como o kernel a mostra.
#[derive(serde::Serialize)]
pub struct Gpu {
    /// Fabricante e driver (`AMD (amdgpu)`), ou o nome que o `nvidia-smi` dá.
    pub nome: String,
    /// Quanto do tempo a GPU esteve ocupada, 0–100; `None` se o driver não informa.
    pub uso: Option<f64>,
    pub vram_usada: u64,
    pub vram_total: u64,
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
    /// Soma da CPU da árvore, em % de um núcleo.
    pub cpu_total: f64,
    /// Quantos núcleos a máquina tem (100 % × núcleos é o teto de `cpu_total`).
    pub nucleos: u32,
    pub gpu: Option<Gpu>,
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

/// Tique do relógio do kernel (`USER_HZ`): 100 por segundo em todo Linux comum (`getconf CLK_TCK`).
#[cfg(target_os = "linux")]
const TIQUES_POR_SEGUNDO: f64 = 100.0;

/// Tempo de CPU acumulado (usuário + sistema) de um `/proc/<pid>/stat`, em milissegundos.
/// Campos 14 (`utime`) e 15 (`stime`); o nome vem entre parênteses e pode ter espaço, então se corta no `)` final.
#[cfg(target_os = "linux")]
fn cpu_ms_de_stat(stat: &str) -> Option<u64> {
    let depois = &stat[stat.rfind(')')? + 1..];
    let mut campos = depois.split_whitespace();
    let utime: u64 = campos.nth(11)?.parse().ok()?;
    let stime: u64 = campos.next()?.parse().ok()?;
    Some(((utime + stime) as f64 * 1000.0 / TIQUES_POR_SEGUNDO) as u64)
}

/// O que um `/proc/<pid>/fdinfo/<fd>` de dispositivo DRM diz: quem é o cliente e quanto ele usou.
#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq)]
struct ClienteDrm {
    /// `pdev` + `client-id`: o mesmo cliente aparece em vários descritores (duplicados) e só conta uma vez.
    chave: String,
    /// Tempo somado de todas as engines (`drm-engine-*`, em nanossegundos).
    ns: u64,
    /// VRAM residente em bytes (`drm-resident-vram`, senão `drm-total-vram`).
    vram: u64,
}

#[cfg(target_os = "linux")]
fn cliente_drm(fdinfo: &str) -> Option<ClienteDrm> {
    if !fdinfo.lines().any(|l| l.starts_with("drm-driver:")) {
        return None;
    }
    let (mut pdev, mut cliente, mut ns, mut residente, mut total) = (String::new(), String::new(), 0u64, None, None);
    for linha in fdinfo.lines() {
        let Some((chave, valor)) = linha.split_once(':') else { continue };
        let valor = valor.trim();
        let numero = |v: &str| v.split_whitespace().next().and_then(|n| n.parse::<u64>().ok());
        match chave {
            "drm-pdev" => pdev = valor.to_string(),
            "drm-client-id" => cliente = valor.to_string(),
            "drm-resident-vram" => residente = numero(valor).map(|kib| kib * 1024),
            "drm-total-vram" => total = numero(valor).map(|kib| kib * 1024),
            k if k.starts_with("drm-engine-") && !k.starts_with("drm-engine-capacity-") => ns += numero(valor).unwrap_or(0),
            _ => {}
        }
    }
    Some(ClienteDrm { chave: format!("{pdev}/{cliente}"), ns, vram: residente.or(total).unwrap_or(0) })
}

/// GPU (ns de engine acumulados) e VRAM de um processo: soma dos clientes DRM distintos dos seus descritores.
#[cfg(target_os = "linux")]
fn gpu_de(pid: u32) -> (u64, u64) {
    let Ok(dir) = fs::read_dir(format!("/proc/{pid}/fdinfo")) else { return (0, 0) };
    let mut vistos: HashMap<String, (u64, u64)> = HashMap::new();
    for e in dir.flatten() {
        if let Some(c) = fs::read_to_string(e.path()).ok().and_then(|t| cliente_drm(&t)) {
            vistos.insert(c.chave, (c.ns, c.vram));
        }
    }
    vistos.values().fold((0, 0), |(a, b), (ns, vram)| (a + ns, b + vram))
}

/// A medição anterior: quando foi e, por processo, a CPU (ms) e a GPU (ns) acumuladas. As porcentagens saem da diferença.
#[cfg(target_os = "linux")]
struct Amostra {
    instante: Instant,
    por_pid: HashMap<u32, (u64, u64)>,
}

#[cfg(target_os = "linux")]
static ANTERIOR: Mutex<Option<Amostra>> = Mutex::new(None);

#[cfg(target_os = "linux")]
fn ler_u64(caminho: &str) -> Option<u64> {
    fs::read_to_string(caminho).ok()?.trim().parse().ok()
}

/// A GPU do sistema: o sysfs do DRM (AMD dá uso e VRAM; Intel só o nome) ou, na NVIDIA, o `nvidia-smi`.
#[cfg(target_os = "linux")]
fn gpu_do_sistema() -> Option<Gpu> {
    let mut placas: Vec<_> = fs::read_dir("/sys/class/drm").ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit())).collect();
    placas.sort();
    for placa in placas {
        let dev = format!("/sys/class/drm/{placa}/device");
        let Ok(vendor) = fs::read_to_string(format!("{dev}/vendor")) else { continue };
        let driver = fs::read_link(format!("{dev}/driver")).ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
        match vendor.trim() {
            "0x1002" => {
                return Some(Gpu {
                    nome: format!("AMD ({driver})"),
                    uso: ler_u64(&format!("{dev}/gpu_busy_percent")).map(|v| v as f64),
                    vram_usada: ler_u64(&format!("{dev}/mem_info_vram_used")).unwrap_or(0),
                    vram_total: ler_u64(&format!("{dev}/mem_info_vram_total")).unwrap_or(0),
                });
            }
            "0x10de" => {
                if let Some(g) = gpu_nvidia() {
                    return Some(g);
                }
            }
            "0x8086" => return Some(Gpu { nome: format!("Intel ({driver})"), uso: None, vram_usada: 0, vram_total: 0 }),
            _ => {}
        }
    }
    None
}

/// `nvidia-smi` (se existir): nome, uso e memória. Não testado nesta máquina (a placa é AMD).
#[cfg(target_os = "linux")]
fn gpu_nvidia() -> Option<Gpu> {
    let saida = std::process::Command::new("nvidia-smi").args(["--query-gpu=name,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"]).output().ok()?;
    let texto = String::from_utf8(saida.stdout).ok()?;
    let mut c = texto.lines().next()?.split(',').map(str::trim);
    let nome = c.next()?.to_string();
    let uso = c.next()?.parse::<f64>().ok();
    let mib = |v: Option<&str>| v.and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) * 1_048_576;
    Some(Gpu { nome, uso, vram_usada: mib(c.next()), vram_total: mib(c.next()) })
}

/// Soma o RSS (e mede CPU e GPU) do processo atual e de todos os seus descendentes.
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

    // Desce a árvore a partir de mim. CPU e GPU vêm acumuladas; a porcentagem é a diferença desde a medição anterior.
    let agora = Instant::now();
    let anterior = ANTERIOR.lock().unwrap_or_else(|e| e.into_inner()).take();
    let janela_ms = anterior.as_ref().map_or(0.0, |a| agora.duration_since(a.instante).as_secs_f64() * 1000.0);
    let mut acumulado: HashMap<u32, (u64, u64)> = HashMap::new();
    let mut processos = Vec::new();
    let mut fila = vec![eu];
    while let Some(pid) = fila.pop() {
        if let Some((rss, pico)) = rss_de(pid) {
            let cpu_ms = fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| cpu_ms_de_stat(&s)).unwrap_or(0);
            let (gpu_ns, vram) = gpu_de(pid);
            acumulado.insert(pid, (cpu_ms, gpu_ns));
            let (cpu, gpu) = match anterior.as_ref().and_then(|a| a.por_pid.get(&pid)) {
                Some((cpu0, gpu0)) if janela_ms > 0.0 => (
                    cpu_ms.saturating_sub(*cpu0) as f64 / janela_ms * 100.0,
                    (gpu_ns.saturating_sub(*gpu0) as f64 / 1e6 / janela_ms * 100.0).min(100.0),
                ),
                _ => (0.0, 0.0),
            };
            processos.push(ProcessoMedido { pid, nome: nome_de(pid), rss, pico, cpu, gpu, vram });
        }
        if let Some(meus) = filhos.get(&pid) {
            fila.extend(meus.iter().copied());
        }
    }
    *ANTERIOR.lock().unwrap_or_else(|e| e.into_inner()) = Some(Amostra { instante: agora, por_pid: acumulado });

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
    let cpu_total = processos.iter().map(|p| p.cpu).sum();
    let nucleos = std::thread::available_parallelism().map_or(1, |n| n.get() as u32);

    processos.sort_by_key(|p| std::cmp::Reverse(p.rss));
    Some(MemoriaArvore { total, principal, webview, pico, pico_principal, cpu_total, nucleos, gpu: gpu_do_sistema(), processos })
}

#[cfg(all(test, target_os = "linux"))]
mod testes {
    use super::*;

    #[test]
    fn cpu_do_stat_soma_usuario_e_sistema_mesmo_com_espaco_no_nome() {
        // utime = campo 14, stime = campo 15; o nome (campo 2) tem espaço e parêntese
        let stat = "123 (Web Content (x)) S 1 2 3 4 5 6 7 8 9 10 250 50 0 0 20 0 1 0 100 0 0";
        assert_eq!(cpu_ms_de_stat(stat), Some(3000), "(250 + 50) tiques de 10 ms");
        assert_eq!(cpu_ms_de_stat("sem parenteses"), None);
    }

    #[test]
    fn fdinfo_do_drm_soma_as_engines_e_pega_a_vram_residente() {
        let t = "pos:\t0\nflags:\t02\ndrm-driver:\tamdgpu\ndrm-client-id:\t181\ndrm-pdev:\t0000:03:00.0\ndrm-total-vram:\t418192 KiB\ndrm-resident-vram:\t400000 KiB\ndrm-engine-gfx:\t1500 ns\ndrm-engine-compute:\t500 ns\ndrm-engine-capacity-gfx:\t1\n";
        let c = cliente_drm(t).unwrap();
        assert_eq!(c, ClienteDrm { chave: "0000:03:00.0/181".into(), ns: 2000, vram: 400000 * 1024 });
        assert!(cliente_drm("pos:\t0\nflags:\t02\n").is_none(), "descritor que não é de GPU");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn medir() -> Option<MemoriaArvore> {
    None
}

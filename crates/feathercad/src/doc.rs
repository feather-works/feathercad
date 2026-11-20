//! Um desenho aberto (uma aba): o motor dele (com o próprio Ctrl+Z), o último estado, a vista e a cena pintada, e os
//! gestos do mouse sobre a área (o antigo `area-desenho.ts`). Toda conta é do motor.

use std::time::{Duration, Instant};

use feathercad_core::vista::Vista;
use pelicano::{MouseButton, Point, Size};
use serde_json::{json, Value};

use crate::area::{Cena, EventoArea};
use crate::motor::{Cursor, Estado, ItemDeMenu, Motor};
use crate::quadro;

pub const FATOR_DO_ZOOM: f32 = 1.25;
/// Dois cliques da roda em menos que isto enquadram o desenho (como no AutoCAD).
const DUPLO_CLIQUE: Duration = Duration::from_millis(350);
/// Um quadro de tela (60 Hz): um mouse de 1000 Hz não refaz o desenho inteiro mil vezes por segundo.
pub const QUADRO_DO_ARRASTO: Duration = Duration::from_millis(16);
/// Quanto o mouse precisa andar para o "grab" (ou a janela) valer como arrasto e não como clique.
const LIMIAR_DO_ARRASTO_PX: f32 = 4.0;

pub struct Doc {
    pub motor: Motor,
    pub estado: Estado,
    pub vista: Vista,
    pub cena: Cena,
    /// A vista, o fundo e o tema com que a cena foi feita: iguais = o motor pode responder "nada mudou".
    feito_com: Option<(Vista, String, bool)>,
    pub arrastados: Option<Cena>,
    pub cursor: Option<Cursor>,
    pub mouse: Option<Point>,
    pub zoom: String,
    pan: Option<Point>,
    orbita: Option<Point>,
    ultimo_clique_da_roda: Option<Instant>,
    /// Sem comando ativo, apertar sobre um objeto o "pega" (grab): arrastar move, soltar sem andar é um clique.
    pegou: Option<Point>,
    /// O botão desceu no vazio e abriu a janela de seleção: se andar até soltar, a janela fecha ali.
    janela_por_arrasto: Option<(Point, bool)>,
    /// Nome de "Sem título N" (o arquivo, quando houver, vem do estado).
    pub sem_titulo: String,
    pub claro: bool,
    pub erro: Option<String>,
    /// O que o motor demorou no último quadro (o medidor de FPS).
    pub ms_quadro: f32,
    /// A data do arquivo no disco quando foi lido ou gravado aqui: se mudar por fora, o desenho é recarregado.
    pub visto: Option<std::time::SystemTime>,
    /// Já avisou que o arquivo mudou no disco com alterações pendentes aqui (avisa uma vez por mudança de fora).
    pub avisou_conflito: bool,
    /// Identifica a aba enquanto uma caixa está aberta (o índice muda se outra aba fecha nesse meio-tempo).
    pub id: u64,
    /// Aberto de um DXF que não é R12 ou que perdeu algo na leitura: o Ctrl+S pergunta onde gravar em vez de
    /// regravar por cima em R12 (o que o app não lê sumiria do arquivo original).
    pub dxf_perderia: bool,
    /// Arrastando a vista, o quadro sai no máximo a cada `QUADRO_DO_ARRASTO`; o que ficou para trás sai em `em_dia`.
    pub pan_pendente: bool,
    ultimo_quadro_do_pan: Option<Instant>,
}

/// O que o botão direito pediu: abrir o menu em `canto` (posição na janela).
pub struct PedidoDeMenu {
    pub canto: Point,
    pub itens: Vec<ItemDeMenu>,
}

impl Doc {
    /// Um motor novo com a língua e as preferências do usuário já entregues.
    pub fn novo(motor_inicial: Value, config: &crate::config::Config, sem_titulo: String, claro: bool) -> Doc {
        let mut motor = Motor::default();
        let _ = motor.estado_de("idioma_definir", motor_inicial);
        let _ = motor.estado_de("cota_tamanho_definir", json!({ "tamanho": config.cota }));
        let _ = motor.estado_de("orto_angulo_definir", json!({ "graus": config.orto_angulo }));
        if !config.snap_modos.is_empty() {
            let _ = motor.estado_de("osnap_modos_definir", json!({ "modos": config.snap_modos }));
        }
        let _ = motor.estado_de("fundo_definir", json!({ "modo": config.fundo }));
        let vista = motor.vista("vista_inicial", json!({ "largura": 800.0, "altura": 600.0 })).unwrap_or(Vista {
            centro: feathercad_core::geom::Ponto::novo(0.0, 0.0),
            escala: 1.0,
            largura: 800.0,
            altura: 600.0,
            rotacao_x: 0.0,
            rotacao_z: 0.0,
        });
        let estado = motor.estado().unwrap_or_default();
        Doc {
            motor,
            estado,
            vista,
            cena: Cena::default(),
            feito_com: None,
            arrastados: None,
            cursor: None,
            mouse: None,
            zoom: String::new(),
            pan: None,
            orbita: None,
            ultimo_clique_da_roda: None,
            pegou: None,
            janela_por_arrasto: None,
            sem_titulo,
            claro,
            erro: None,
            ms_quadro: 0.0,
            visto: None,
            avisou_conflito: false,
            dxf_perderia: false,
            pan_pendente: false,
            ultimo_quadro_do_pan: None,
            id: {
                static PROXIMO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                PROXIMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            },
        }
    }

    pub fn nome(&self) -> String {
        self.estado.arquivo.as_ref().map_or_else(|| self.sem_titulo.clone(), |a| a.nome.clone())
    }

    // ---------------------------------------------------------------- estado e quadro

    pub fn aplicar(&mut self, e: Estado) {
        self.estado = e;
        self.redesenhar();
        self.mover_cursor();
    }

    /// Um pedido que devolve o estado: aplica ou guarda o erro para a interface mostrar.
    pub fn pedir(&mut self, comando: &str, corpo: Value) {
        match self.motor.estado_de(comando, corpo) {
            Ok(e) => self.aplicar(e),
            Err(e) => self.erro = Some(e),
        }
    }

    /// O desenho mudou por fora (MCP, arquivo recarregado, tema): a cena guardada não vale mais.
    pub fn invalidar(&mut self) {
        self.feito_com = None;
        if let Ok(e) = self.motor.estado() {
            self.aplicar(e);
        }
    }

    /// Pede o quadro ao motor; se nada mudou (mesma versão, vista, fundo e tema), ele responde `FCQ0` e a cena fica.
    pub fn redesenhar(&mut self) {
        let inicio = Instant::now();
        let chave = (self.vista, self.estado.fundo.clone(), self.claro);
        let conhecida = if self.feito_com.as_ref() == Some(&chave) { self.cena.versao } else { None };
        match self.motor.quadro(&self.vista, conhecida, self.claro).and_then(|b| quadro::ler(&b)) {
            Ok(Some(q)) => {
                self.cena = Cena::nova(q, &self.estado.fundo, self.tamanho(), &self.vista);
                self.feito_com = Some(chave);
            }
            Ok(None) => {}
            Err(e) => self.erro = Some(e),
        }
        self.ms_quadro = inicio.elapsed().as_secs_f32() * 1000.0;
    }

    /// Desenha o quadro que o arrasto da vista deixou para trás.
    pub fn em_dia(&mut self) {
        if self.pan_pendente {
            self.pan_pendente = false;
            self.redesenhar();
        }
    }

    pub fn tamanho(&self) -> Size {
        Size::new(self.vista.largura as f32, self.vista.altura as f32)
    }

    pub fn mover_cursor(&mut self) {
        let Some(m) = self.mouse else { return };
        self.cursor = self.motor.cursor(&self.vista, m.x, m.y);
        self.arrastados = None;
        if self.pegou.is_some() {
            if let Ok(Some(q)) = self.motor.quadro_arrasto(&self.vista, m.x, m.y, self.claro).and_then(|b| quadro::ler(&b)) {
                self.arrastados = Some(Cena::nova(q, "nada", self.tamanho(), &self.vista));
            }
        }
    }

    pub fn vista_mudou(&mut self, v: Result<Vista, String>) {
        match v {
            Ok(v) => self.vista = v,
            Err(e) => self.erro = Some(e),
        }
        self.zoom = self.motor.vista_texto(&self.vista);
        self.redesenhar();
        self.mover_cursor();
    }

    pub fn zoom_em(&mut self, x: f32, y: f32, fator: f32) {
        let v = self.motor.vista("vista_zoom", json!({ "vista": self.vista, "x": x, "y": y, "fator": fator }));
        self.vista_mudou(v);
    }

    pub fn zoom_no_centro(&mut self, fator: f32) {
        self.zoom_em(self.vista.largura as f32 / 2.0, self.vista.altura as f32 / 2.0, fator);
    }

    pub fn ajustar(&mut self) {
        let v = self.motor.vista("vista_ajustar", json!({ "vista": self.vista }));
        self.vista_mudou(v);
    }

    /// Gira a vista 3D (graus); `(0, 0)` = vista de cima.
    pub fn girar_para(&mut self, rx: f64, rz: f64) {
        self.vista.rotacao_x = rx.clamp(-180.0, 180.0);
        self.vista.rotacao_z = rz.rem_euclid(360.0);
        self.vista_mudou(Ok(self.vista));
    }

    // ---------------------------------------------------------------- mouse

    /// Um evento da área. Devolve o menu do botão direito, quando é o caso.
    pub fn area(&mut self, ev: EventoArea) -> Option<PedidoDeMenu> {
        match ev {
            EventoArea::Tamanho(s) => {
                self.vista.largura = f64::from(s.width.max(1.0));
                self.vista.altura = f64::from(s.height.max(1.0));
                self.vista_mudou(Ok(self.vista));
            }
            EventoArea::Mover(p) => {
                if let Some(antes) = self.orbita {
                    self.orbita = Some(p);
                    self.girar_para(self.vista.rotacao_x + f64::from(p.y - antes.y) * 0.5, self.vista.rotacao_z + f64::from(p.x - antes.x) * 0.5);
                } else if let Some(antes) = self.pan {
                    self.pan = Some(p);
                    let v = self.motor.vista("vista_deslocar", json!({ "vista": self.vista, "dx": p.x - antes.x, "dy": p.y - antes.y }));
                    self.vista = v.unwrap_or(self.vista);
                    if self.ultimo_quadro_do_pan.map_or(true, |t| t.elapsed() >= QUADRO_DO_ARRASTO) {
                        self.ultimo_quadro_do_pan = Some(Instant::now());
                        self.pan_pendente = false;
                        self.redesenhar();
                    } else {
                        self.pan_pendente = true;
                    }
                }
                self.mouse = Some(p);
                // Arrastando a vista o snap não serve: não consulta o motor a cada evento.
                if self.pan.is_none() {
                    self.mover_cursor();
                }
            }
            EventoArea::Saiu => {
                self.mouse = None;
                self.cursor = None;
            }
            EventoArea::Roda(dy, p) => {
                // Positivo = roda para cima = aproximar; um "clique" de roda (~100 px) dá 1,25×.
                let fator = FATOR_DO_ZOOM.powf(dy.clamp(-120.0, 120.0) / 100.0);
                self.mouse = Some(p);
                self.zoom_em(p.x, p.y, fator);
            }
            EventoArea::Apertar(b, p, global, shift) => return self.apertar(b, p, global, shift),
            EventoArea::Soltar(b, p) => self.soltar(b, p),
        }
        None
    }

    fn apertar(&mut self, botao: MouseButton, p: Point, global: Point, shift: bool) -> Option<PedidoDeMenu> {
        self.mouse = Some(p);
        match botao {
            // Shift + roda apertada = órbita 3D (como no AutoCAD).
            MouseButton::Middle if shift => self.orbita = Some(p),
            MouseButton::Middle => {
                if self.ultimo_clique_da_roda.is_some_and(|t| t.elapsed() < DUPLO_CLIQUE) {
                    self.ajustar();
                }
                self.ultimo_clique_da_roda = Some(Instant::now());
                self.pan = Some(p);
            }
            MouseButton::Left => {
                if !shift && self.estado.ferramenta == "nenhuma" {
                    if let Ok(e) = self.motor.mouse("arrasto_iniciar", &self.vista, p.x, p.y, false) {
                        if e.arrastando {
                            self.pegou = Some(p);
                            self.aplicar(e);
                            return None;
                        }
                    }
                }
                let antes = self.estado.janela_pendente;
                match self.motor.mouse("clique", &self.vista, p.x, p.y, shift) {
                    Ok(e) => self.aplicar(e),
                    Err(e) => self.erro = Some(e),
                }
                // Abriu a janela agora (não era o 2º clique de uma já aberta): arrastar até soltar a fecha.
                self.janela_por_arrasto = (!antes && self.estado.janela_pendente).then_some((p, shift));
            }
            MouseButton::Right => match self.motor.menu(&self.vista, p.x, p.y) {
                Ok((e, itens)) => {
                    self.aplicar(e);
                    return Some(PedidoDeMenu { canto: global, itens });
                }
                Err(e) => self.erro = Some(e),
            },
            _ => {}
        }
        None
    }

    fn soltar(&mut self, botao: MouseButton, p: Point) {
        if botao == MouseButton::Middle {
            self.pan = None;
            self.orbita = None;
            self.em_dia();
            self.mover_cursor();
            return;
        }
        if botao != MouseButton::Left {
            return;
        }
        let andou = |a: Point| (p.x - a.x).hypot(p.y - a.y) >= LIMIAR_DO_ARRASTO_PX;
        if let Some((inicio, aditivo)) = self.janela_por_arrasto.take() {
            // Andou: soltar fecha a janela. Não andou: segue esperando o 2º clique.
            if andou(inicio) {
                let r = self.motor.mouse("clique", &self.vista, p.x, p.y, aditivo);
                self.resultado(r);
            }
        }
        if let Some(inicio) = self.pegou.take() {
            let r = if andou(inicio) {
                self.motor.mouse("arrasto_fim", &self.vista, p.x, p.y, false)
            } else {
                let _ = self.motor.estado_de("arrasto_cancelar", Value::Null);
                self.motor.mouse("clique", &self.vista, inicio.x, inicio.y, false)
            };
            self.arrastados = None;
            self.resultado(r);
        }
    }

    fn resultado(&mut self, r: Result<Estado, String>) {
        match r {
            Ok(e) => self.aplicar(e),
            Err(e) => self.erro = Some(e),
        }
    }
}

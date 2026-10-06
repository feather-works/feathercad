//! `cad_projeto`: o **funil** que conduz a IA do pedido de projeto até a receita. Disciplina → tipo de projeto → **receita**
//! (as fases e os itens que um projeto profissional daquele tipo sempre tem, extraídos dos projetos de referência) e,
//! depois, o **status**: cada item conferido contra o desenho aberto, com o próximo passo.
//!
//! Só vale para pedido de PROJETO (várias folhas, quadros, legendas…). Pedido simples ("faça um círculo") vai direto às
//! ferramentas de desenho: o funil existe para o projeto não sair com ingrediente demais num canto e faltando noutro.

use feathercad_core::documento::{Documento, Geometria};
use feathercad_core::folha::{AREA, BASE_W, X_LEG, X_NOTAS, Y_FAIXA};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{texto_opcional, Res};

/// Como um item da receita é conferido no desenho.
#[derive(Clone, Copy)]
enum Detecta {
    /// Alguma camada começa com um destes prefixos e tem entidades.
    Camada(&'static [&'static str]),
    /// Algum texto do desenho contém um destes trechos (sem diferenciar maiúscula).
    Texto(&'static [&'static str]),
    Unidade,
    Folha,
    Carimbo,
    /// Não dá para conferir sozinho: a IA confirma.
    Manual,
}

#[derive(Clone, Copy)]
struct Item {
    id: &'static str,
    fase: &'static str,
    o_que: &'static str,
    ferramenta: &'static str,
    obrigatorio: bool,
    detecta: Detecta,
}

const fn it(id: &'static str, fase: &'static str, o_que: &'static str, ferramenta: &'static str, obrigatorio: bool, detecta: Detecta) -> Item {
    Item { id, fase, o_que, ferramenta, obrigatorio, detecta }
}

struct Receita {
    nome: &'static str,
    maturidade: &'static str,
    perguntar: &'static [&'static str],
    regras: &'static [&'static str],
    itens: &'static [Item],
}

struct No {
    id: &'static str,
    titulo: &'static str,
    /// Texto para a IA perguntar ao usuário neste nível.
    pergunta: &'static str,
    filhos: &'static [No],
    receita: Option<&'static Receita>,
}

const F1: &str = "1. Dados e base";
const F2: &str = "2. Cálculo";
const F3: &str = "3. Planta";
const F4: &str = "4. Documentação da folha";
const F5: &str = "5. Fechamento";

const BASE: [Item; 5] = [
    it("dados", F1, "Perguntar ao usuário o que falta (veja `perguntar_antes`) e confirmar as premissas", "(perguntar)", true, Detecta::Manual),
    it("unidade", F1, "Definir a unidade do desenho (m para ambientes grandes, mm para peças) ", "cad_novo", true, Detecta::Unidade),
    it("camadas", F1, "Criar as camadas com prefixo por assunto (ARQ-, EL-, DI-, F-) e cor/espessura", "cad_camadas", true, Detecta::Camada(&["EL-", "ARQ-", "DI-"])),
    it("folha", F1, "Inserir a(s) folha(s) com moldura e carimbo, na escala do projeto", "cad_folha", true, Detecta::Folha),
    it("carimbo", F1, "Preencher o carimbo (cliente, fase, disciplina, título, nº, folha, revisão, responsável) e as linhas CREA/ART/proprietário/área/data", "cad_folha campos, cad_carimbo", true, Detecta::Carimbo),
];

const ELETRICA_LUMINOTECNICO: Receita = Receita {
    nome: "Elétrica — projeto luminotécnico",
    maturidade: "completa (usada no v3 do galpão)",
    perguntar: &[
        "Dimensões do ambiente (comprimento, largura, pé-direito) e a atividade (define a iluminância e a norma)",
        "Luminária: fluxo (lm), potência (W), altura de montagem, se o fabricante deu o fator de utilização",
        "Funcionamento (horas), luz natural (claraboias), emergência, dimerização",
        "Tensão do quadro (ex. 380/220 V), distância do quadro ao ambiente, papel e escala",
        "Dados do carimbo: cliente, projetista, CREA/ART, proprietário, data",
    ],
    regras: &[
        "O cálculo é do motor (cad_luminotecnico, cad_circuitos): nunca faça conta à mão no texto da folha",
        "Quadro de cargas, demanda, unifilar e lista de materiais saem da MESMA lista de circuitos",
        "Legenda e notas vão DENTRO das caixas da folha (cad_carimbo devolve onde); sem caixa vazia nem texto solto",
        "Sem informação, escreva A PREENCHER; não invente CREA, ART ou dados do cliente",
        "Mostre as premissas e os avisos que as ferramentas devolveram (uniformidade, fatores, demanda)",
    ],
    itens: &[
        BASE[0], BASE[1], BASE[2], BASE[3], BASE[4],
        it("calculo_lux", F2, "Método dos lumens + grade de lux: nº de luminárias, arranjo, posições, uniformidade", "cad_luminotecnico", true, Detecta::Texto(&["MÉTODO DOS LUMENS"])),
        it("circuitos", F2, "Lista de circuitos: corrente, disjuntor, seção, queda de tensão, fases, alimentador", "cad_circuitos", true, Detecta::Texto(&["QUADRO DE CARGAS"])),
        it("arquitetura", F3, "Paredes, pilares/pórticos, esquadrias e eixos", "cad_criar / cad_lote", true, Detecta::Camada(&["ARQ-Paredes", "PAREDE"])),
        it("cotas", F3, "Cotas gerais, vãos e posição das luminárias", "cad_criar / cad_lote", true, Detecta::Camada(&["ARQ-Cotas", "COTA"])),
        it("luminarias", F3, "Luminárias com tag (L01…) e circuito, nas posições calculadas", "cad_simbolo luminaria_led", true, Detecta::Camada(&["EL-Luminarias"])),
        it("eletrodutos", F3, "Eletrodutos com as marcas de condutor, circuito e seção", "cad_eletroduto", true, Detecta::Camada(&["EL-Condutos"])),
        it("quadro_luz", F3, "Quadro de distribuição de luz (QDL) na planta", "cad_criar / cad_simbolo", true, Detecta::Texto(&["QDL"])),
        it("emergencia", F3, "Iluminação de emergência e sinalização de saída em circuito próprio (uso público/industrial)", "cad_simbolo bloco_emergencia, saida_emergencia", true, Detecta::Camada(&["EL-Emergencia"])),
        it("claraboias", F3, "Claraboias e fotocélulas, se há luz natural/dimerização", "cad_criar, cad_simbolo fotocelula", false, Detecta::Camada(&["EL-Claraboias"])),
        it("dados_projeto", F4, "Quadro DADOS DO PROJETO (área, iluminância, potência, uniformidade)", "cad_tabela", true, Detecta::Texto(&["DADOS DO PROJETO"])),
        it("quadro_luminarias", F4, "Quadro de luminárias e comando (tag, qtd, descrição, circuito, comando)", "cad_tabela", true, Detecta::Texto(&["QUADRO DE LUMINÁRIAS"])),
        it("mapa_lux", F4, "Mapa de iluminância com legenda das faixas", "cad_luminotecnico desenhar_mapa", true, Detecta::Camada(&["LUX"])),
        it("corte", F4, "Corte transversal com luminárias, claraboias e plano de trabalho", "cad_criar / cad_lote", true, Detecta::Texto(&["CORTE TRANSVERSAL"])),
        it("quadro_demanda", F4, "Quadro de demanda", "cad_circuitos desenhar.demanda", true, Detecta::Texto(&["QUADRO DE DEMANDA"])),
        it("unifilar", F4, "Diagrama unifilar do quadro", "cad_circuitos desenhar.unifilar", true, Detecta::Camada(&["DI-Linhas do diagrama"])),
        it("lista_materiais", F4, "Lista de materiais (item, descrição, un, qtd, referência)", "cad_tabela + cad_contar", true, Detecta::Texto(&["LISTA DE MATERIAIS"])),
        it("legenda", F4, "Legenda dos símbolos na caixa LEGENDA da folha", "cad_carimbo + cad_simbolo", true, Detecta::Camada(&["F-Legenda"])),
        it("notas", F4, "Notas normativas na caixa NOTAS da folha", "cad_notas_padrao", true, Detecta::Camada(&["F-Notas"])),
        it("advertencia", F4, "Advertência sobre disjuntor/DR fixada no quadro", "cad_texto_bloco", false, Detecta::Texto(&["ADVERTÊNCIA"])),
        it("verificar", F5, "Conferir leiaute e duplicadas e olhar cada folha", "cad_verificar {leiaute:true}, cad_ver", true, Detecta::Manual),
        it("exportar", F5, "Salvar (.fcdz/.dxf) e exportar o PDF das folhas", "cad_salvar, cad_pdf", true, Detecta::Manual),
    ],
};

const ELETRICA_GERAL: Receita = Receita {
    nome: "Elétrica — instalações (iluminação, tomadas, quadro)",
    maturidade: "básica (receita pelos 3 projetos de referência; falta exemplo gerado)",
    perguntar: &[
        "Tipo de edificação, área e a planta de arquitetura (arquivo ou desenhar)",
        "Tensão e tipo de ligação (monofásico/bifásico/trifásico) e a concessionária",
        "Pontos de luz, tomadas (TUG/TUE), interruptores, ar-condicionado, chuveiro e outras cargas específicas",
        "Papel e escala (planta em 1:50 ou 1:75) e dados do carimbo",
    ],
    regras: &[
        "Planta sozinha numa folha e documentação (quadros, unifilar, lista) na seguinte, quando o projeto é pequeno",
        "Pontos com circuito e potência; interruptor com a letra do ponto; tomada com a altura na legenda",
        "Eletroduto não cotado e fiação não cotada vão em NOTA (PVC flexível Ø3/4\", #2,5 mm²)",
        "Quadro de cargas, demanda e unifilar da MESMA lista de circuitos (cad_circuitos)",
    ],
    itens: &[
        BASE[0], BASE[1], BASE[2], BASE[3], BASE[4],
        it("circuitos", F2, "Lista de circuitos: corrente, disjuntor, DR, seção, queda de tensão, fases", "cad_circuitos", true, Detecta::Texto(&["QUADRO DE CARGAS"])),
        it("arquitetura", F3, "Arquitetura em linha fina e cinza atrás da elétrica (paredes, esquadrias, cotas)", "cad_abrir / cad_criar", true, Detecta::Camada(&["ARQ-Paredes", "PAREDE"])),
        it("luminarias", F3, "Pontos de luz com circuito, potência e comando", "cad_simbolo ponto_luz_teto", true, Detecta::Camada(&["EL-Luminarias", "EL-Lampada"])),
        it("tomadas", F3, "Tomadas baixa/média/alta com circuito e potência", "cad_simbolo tomada_*", true, Detecta::Camada(&["EL-Tomadas"])),
        it("interruptores", F3, "Interruptores com a letra do ponto que comandam", "cad_simbolo interruptor_*", true, Detecta::Camada(&["EL-Interruptor"])),
        it("eletrodutos", F3, "Eletrodutos com marcas de condutor, circuito e seção", "cad_eletroduto", true, Detecta::Camada(&["EL-Condutos"])),
        it("quadro", F3, "Quadro de distribuição e caixas de passagem", "cad_simbolo quadro_*, caixa_passagem_piso", true, Detecta::Texto(&["QD"])),
        it("quadro_demanda", F4, "Quadro de demanda", "cad_circuitos desenhar.demanda", true, Detecta::Texto(&["QUADRO DE DEMANDA"])),
        it("unifilar", F4, "Diagrama unifilar com disjuntores, DR, DPS e seções", "cad_circuitos desenhar.unifilar", true, Detecta::Camada(&["DI-Linhas do diagrama"])),
        it("legenda", F4, "Legenda dos símbolos com a altura de instalação", "cad_carimbo + cad_simbolo", true, Detecta::Camada(&["F-Legenda"])),
        it("notas", F4, "Notas normativas", "cad_notas_padrao", true, Detecta::Camada(&["F-Notas"])),
        it("detalhes", F4, "Entrada de energia, caixa de medição e aterramento", "cad_componente / cad_criar", true, Detecta::Texto(&["ATERRAMENTO", "ENTRADA"])),
        it("lista_materiais", F4, "Lista de materiais", "cad_tabela + cad_contar", true, Detecta::Texto(&["LISTA DE MATERIAIS"])),
        it("verificar", F5, "Conferir leiaute e duplicadas e olhar cada folha", "cad_verificar {leiaute:true}, cad_ver", true, Detecta::Manual),
        it("exportar", F5, "Salvar e exportar o PDF", "cad_salvar, cad_pdf", true, Detecta::Manual),
    ],
};

const CIVIL_PLANTA: Receita = Receita {
    nome: "Civil — planta baixa de arquitetura",
    maturidade: "esboço (sem projeto de referência estudado ainda: peça exemplos ao usuário)",
    perguntar: &["Programa de necessidades e área do terreno", "Pé-direito, espessura das paredes, esquadrias", "Escala, papel e dados do carimbo"],
    regras: &["Receita ainda enxuta: confirme com o usuário o que mais o projeto exige antes de entregar"],
    itens: &[
        BASE[0], BASE[1], BASE[2], BASE[3], BASE[4],
        it("paredes", F3, "Paredes e pilares", "cad_criar / cad_lote", true, Detecta::Camada(&["ARQ-Paredes", "PAREDE"])),
        it("esquadrias", F3, "Portas e janelas com giro e dimensões", "cad_componente porta_80, cad_criar", true, Detecta::Camada(&["ARQ-Esquadrias", "ESQUADRIAS"])),
        it("cotas", F3, "Cotas externas e internas", "cad_criar / cad_lote", true, Detecta::Camada(&["ARQ-Cotas", "COTA"])),
        it("ambientes", F3, "Nome e área de cada ambiente", "cad_regioes + cad_criar", true, Detecta::Texto(&["M²"])),
        it("quadro_areas", F4, "Quadro de áreas", "cad_tabela", true, Detecta::Texto(&["QUADRO DE ÁREAS"])),
        it("cortes", F4, "Cortes e fachadas", "cad_criar / cad_lote", true, Detecta::Texto(&["CORTE"])),
        it("notas", F4, "Notas e legenda", "cad_texto_bloco", true, Detecta::Camada(&["F-Notas", "F-Legenda"])),
        it("verificar", F5, "Conferir leiaute e olhar cada folha", "cad_verificar {leiaute:true}, cad_ver", true, Detecta::Manual),
        it("exportar", F5, "Salvar e exportar o PDF", "cad_salvar, cad_pdf", true, Detecta::Manual),
    ],
};

const FERROVIARIO_NUVEM: Receita = Receita {
    nome: "Ferroviário — traçado a partir de nuvem de pontos",
    maturidade: "completa (cad_modelo ferrovia e a cadeia de ferramentas viárias)",
    perguntar: &["Arquivo da nuvem (.csv/.xyz/.pts) e a bitola", "Velocidade de projeto, ripagem permitida e o que se quer entregar (traçado, superelevação, greide, folhas)", "Zona UTM e papel/escala"],
    regras: &[
        "Ligue o modo projeto (cad_modo) e passe de uma ferramenta à outra pelo nome guardado na sessão: não copie números",
        "Confira em texto (cad_verificar leiaute) antes de pedir imagem; use o prompt `projeto_grande`",
        "Limites normativos vêm da tabela `regras`; diga ao usuário quais usou",
    ],
    itens: &[
        BASE[0], BASE[1],
        it("nuvem", F2, "Abrir a nuvem e extrair seções e feições dos trilhos", "cad_nuvem_secoes, cad_nuvem_feicoes, cad_nuvem_qualidade", true, Detecta::Manual),
        it("alinhamento", F2, "Ajustar o traçado ao eixo existente", "cad_alinhamento_ajustar", true, Detecta::Manual),
        it("superelevacao", F2, "Superelevação por curva", "cad_superelevacao", true, Detecta::Manual),
        it("greide", F2, "Greide de projeto sobre o perfil existente", "cad_greide", true, Detecta::Manual),
        it("analise", F2, "Análise de criticidade de curvas e tangentes", "cad_analise", false, Detecta::Manual),
        it("folhas", F4, "Gerar as folhas do projeto de uma vez", "cad_modelo ferrovia", true, Detecta::Folha),
        it("tabela_estacas", F4, "Tabela por estaca", "cad_tabela_estacas", true, Detecta::Texto(&["ESTACA"])),
        it("verificar", F5, "Conferir leiaute e olhar as folhas", "cad_verificar {leiaute:true}, cad_ver", true, Detecta::Manual),
        it("exportar", F5, "Salvar e exportar o PDF", "cad_salvar, cad_pdf", true, Detecta::Manual),
    ],
};

const ARVORE: No = No {
    id: "raiz",
    titulo: "Projeto",
    pergunta: "Que tipo de projeto você quer gerar?",
    receita: None,
    filhos: &[
        No { id: "eletrica", titulo: "Elétrica", pergunta: "O que vamos gerar na elétrica?", receita: None, filhos: &[
            No { id: "luminotecnico", titulo: "Projeto luminotécnico (iluminação de um ambiente, cálculo de lux)", pergunta: "", filhos: &[], receita: Some(&ELETRICA_LUMINOTECNICO) },
            No { id: "eletrica_geral", titulo: "Elétrica geral (iluminação, tomadas, quadro de distribuição)", pergunta: "", filhos: &[], receita: Some(&ELETRICA_GERAL) },
        ] },
        No { id: "civil", titulo: "Civil / arquitetura", pergunta: "O que vamos gerar na parte civil?", receita: None, filhos: &[
            No { id: "planta_baixa", titulo: "Planta baixa de arquitetura", pergunta: "", filhos: &[], receita: Some(&CIVIL_PLANTA) },
        ] },
        No { id: "ferroviario", titulo: "Ferroviário", pergunta: "O que vamos gerar no ferroviário?", receita: None, filhos: &[
            No { id: "tracado_nuvem", titulo: "Traçado a partir de nuvem de pontos", pergunta: "", filhos: &[], receita: Some(&FERROVIARIO_NUVEM) },
        ] },
    ],
};

fn opcoes(no: &No) -> Value {
    let mut v: Vec<Value> = no.filhos.iter().map(|f| json!({ "id": f.id, "titulo": f.titulo })).collect();
    v.push(json!({ "id": "outra", "titulo": "Outra coisa (o usuário descreve; sem receita pronta, siga as regras gerais)" }));
    json!(v)
}

fn descer<'a>(caminho: &[&str]) -> Res<&'a No> {
    let mut no: &No = &ARVORE;
    for id in caminho {
        no = no.filhos.iter().find(|f| f.id == *id).ok_or_else(|| format!("`{id}` não é uma opção aqui; opções: {}", no.filhos.iter().map(|f| f.id).collect::<Vec<_>>().join(", ")))?;
    }
    Ok(no)
}

fn caminho_de(args: &Value) -> Vec<String> {
    args.get("caminho").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

/// A receita montada para a IA: perguntas, regras e os itens por fase.
fn apresentar(r: &Receita, caminho: &[String]) -> Value {
    let mut fases: Vec<(&str, Vec<Value>)> = Vec::new();
    for i in r.itens {
        let item = json!({ "id": i.id, "o_que": i.o_que, "ferramenta": i.ferramenta, "obrigatorio": i.obrigatorio });
        match fases.iter_mut().find(|(f, _)| *f == i.fase) {
            Some((_, l)) => l.push(item),
            None => fases.push((i.fase, vec![item])),
        }
    }
    json!({
        "nivel": "receita", "receita": r.nome, "maturidade": r.maturidade, "caminho": caminho,
        "perguntar_antes": r.perguntar, "regras": r.regras,
        "fases": fases.into_iter().map(|(f, itens)| json!({ "fase": f, "itens": itens })).collect::<Vec<_>>(),
        "como_seguir": "Pergunte o que falta, depois siga as fases em ordem. A cada fase chame `cad_projeto {acao:\"status\", caminho}`: ele confere o desenho e diz o próximo passo. Só entregue com todos os itens obrigatórios feitos (ou justificados ao usuário).",
    })
}

/// Os textos e as camadas do desenho, para os detectores.
struct Cena {
    textos: Vec<String>,
    camadas: Vec<(String, usize)>,
}

fn cena(d: &Documento) -> Cena {
    let textos = d.entidades().iter().filter_map(|e| if let Geometria::Texto { conteudo, .. } = &e.geometria { Some(conteudo.to_uppercase()) } else { None }).collect();
    let mut camadas: Vec<(String, usize)> = d.camadas().iter().map(|c| (c.nome.to_uppercase(), 0)).collect();
    for e in d.entidades() {
        if let Some(c) = camadas.get_mut(e.camada) {
            c.1 += 1;
        }
    }
    Cena { textos, camadas }
}

fn confere(d: &Documento, c: &Cena, det: Detecta) -> Option<bool> {
    match det {
        Detecta::Camada(ps) => Some(c.camadas.iter().any(|(n, q)| *q > 0 && ps.iter().any(|p| n.starts_with(&p.to_uppercase())))),
        Detecta::Texto(ts) => Some(c.textos.iter().any(|t| ts.iter().any(|x| t.contains(&x.to_uppercase())))),
        Detecta::Unidade => Some(d.unidade().is_some()),
        Detecta::Folha => Some(!d.folhas_inseridas().is_empty()),
        Detecta::Carimbo => Some(d.folha_campos(0).is_ok_and(|k| ![&k.cliente, &k.disciplina, &k.titulo, &k.responsavel].iter().any(|v| v.trim().is_empty()))),
        Detecta::Manual => None,
    }
}


/// Pontos que "pintam" a folha: ao longo de linhas e polilinhas, e os pontos de caixa das demais formas.
fn amostras(g: &Geometria) -> Vec<Ponto> {
    let mut v: Vec<Ponto> = Vec::new();
    let mut ao_longo = |a: Ponto, b: Ponto| (0..=12).for_each(|i| v.push(Ponto::novo(a.x + (b.x - a.x) * f64::from(i) / 12.0, a.y + (b.y - a.y) * f64::from(i) / 12.0)));
    match g {
        Geometria::Linha { a, b } => ao_longo(*a, *b),
        Geometria::Polilinha { vertices, fechada } => {
            vertices.windows(2).for_each(|w| ao_longo(w[0].p, w[1].p));
            if *fechada && vertices.len() > 2 {
                ao_longo(vertices[vertices.len() - 1].p, vertices[0].p);
            }
        }
        Geometria::Insercao { .. } => {}
        outra => outra.para_cada_ponto_de_caixa(&mut |p| v.push(p)),
    }
    v
}

const COLUNAS: usize = 24;
const LINHAS_GRADE: usize = 12;
/// Abaixo disto a folha é considerada esparsa. Calibrado em 04/10/2026 medindo a mesma grade nos A1 de referência
/// (Bombeiros-GO 89 e 95 %, IPESAÚDE-SE 98 % da área útil); o limiar deixa folha simples passar, mas não folha meio vazia.
const OCUPACAO_MINIMA: f64 = 0.60;

/// Por folha: quanto da área útil tem desenho, e se as caixas NOTAS e LEGENDA têm conteúdo.
fn equilibrio(d: &Documento) -> Vec<Value> {
    let em_mm = d.unidade().map_or(1.0, feathercad_core::documento::Unidade::em_milimetros);
    d.folhas_inseridas()
        .iter()
        .map(|f| {
            let k = f.largura_mm / BASE_W * f.escala / em_mm;
            let pt = |x: f64, y: f64| Ponto::novo(f.min.x + x * k, f.min.y + y * k);
            let (a, b) = (pt(AREA.0, AREA.1), pt(AREA.2, AREA.3));
            let (na0, na1) = (pt(25.0, 10.0), pt(X_NOTAS, Y_FAIXA - 6.0));
            let (le0, le1) = (pt(X_NOTAS, 10.0), pt(X_LEG, Y_FAIXA - 6.0));
            let mut celulas = vec![false; COLUNAS * LINHAS_GRADE];
            let (mut notas, mut legenda) = (0usize, 0usize);
            for e in d.entidades().iter().filter(|e| d.visivel(e)) {
                for p in amostras(&e.geometria) {
                    let dentro = |p0: Ponto, p1: Ponto| p.x >= p0.x && p.x <= p1.x && p.y >= p0.y && p.y <= p1.y;
                    if dentro(a, b) {
                        let (cx, cy) = (((p.x - a.x) / (b.x - a.x) * COLUNAS as f64) as usize, ((p.y - a.y) / (b.y - a.y) * LINHAS_GRADE as f64) as usize);
                        celulas[cy.min(LINHAS_GRADE - 1) * COLUNAS + cx.min(COLUNAS - 1)] = true;
                    }
                    notas += usize::from(dentro(na0, na1));
                    legenda += usize::from(dentro(le0, le1));
                }
            }
            let ocupacao = celulas.iter().filter(|c| **c).count() as f64 / celulas.len() as f64;
            let mut avisos: Vec<&str> = Vec::new();
            if ocupacao < OCUPACAO_MINIMA {
                avisos.push("folha esparsa: o desenho ocupa pouco da área útil — aumente a escala útil (texto/símbolos), junte conteúdo ou redistribua");
            }
            if notas == 0 {
                avisos.push("caixa NOTAS vazia");
            }
            if legenda == 0 {
                avisos.push("caixa LEGENDA vazia");
            }
            json!({ "folha": f.indice + 1, "ocupacao_pct": (ocupacao * 100.0).round(), "avisos": avisos })
        })
        .collect()
}

fn status(d: &Documento, r: &Receita, caminho: &[String]) -> Value {
    let c = cena(d);
    let itens: Vec<Value> = r
        .itens
        .iter()
        .map(|i| {
            let estado = match confere(d, &c, i.detecta) {
                Some(true) => "feito",
                Some(false) => "pendente",
                None => "manual",
            };
            json!({ "id": i.id, "fase": i.fase, "estado": estado, "obrigatorio": i.obrigatorio, "o_que": i.o_que, "ferramenta": i.ferramenta })
        })
        .collect();
    let pend: Vec<&Value> = itens.iter().filter(|i| i["estado"] == "pendente" && i["obrigatorio"] == true).collect();
    let manuais: Vec<&Value> = itens.iter().filter(|i| i["estado"] == "manual").collect();
    let feitos = itens.iter().filter(|i| i["estado"] == "feito").count();
    let equilibrio_json = equilibrio(d);
    let desequilibrada = equilibrio_json.iter().any(|f| f["avisos"].as_array().is_some_and(|a| !a.is_empty()));
    json!({
        "nivel": "status", "receita": r.nome, "caminho": caminho, "feitos": feitos, "total": itens.len(),
        "pendentes_obrigatorios": pend.len(), "proximo": pend.first().map(|p| json!({ "id": p["id"], "fase": p["fase"], "o_que": p["o_que"], "ferramenta": p["ferramenta"] })),
        "a_confirmar": manuais.iter().map(|m| m["id"].clone()).collect::<Vec<_>>(),
        "equilibrio": equilibrio_json,
        "pronto_para_entregar": pend.is_empty() && !desequilibrada,
        "itens": itens,
        "nota": "feito/pendente vêm do desenho (camadas e textos esperados); `manual` é por sua conta (dados, verificar, exportar). `equilibrio` mostra por folha quanto da área útil tem desenho e se NOTAS/LEGENDA têm conteúdo: folha esparsa ou caixa vazia impede `pronto_para_entregar`. Confira com cad_ver: o detector só vê que o item existe, não que está bom.",
    })
}

/// `cad_projeto` {acao: iniciar|escolher|status, caminho: [ids]}.
pub fn projeto(d: &Documento, args: &Value) -> Res<Value> {
    let acao = texto_opcional(args, "acao").unwrap_or("iniciar");
    let caminho = caminho_de(args);
    let ids: Vec<&str> = caminho.iter().map(String::as_str).collect();
    match acao {
        "iniciar" => Ok(json!({ "nivel": "raiz", "perguntar_ao_usuario": ARVORE.pergunta, "opcoes": opcoes(&ARVORE),
            "como_seguir": "Pergunte ao usuário e chame `cad_projeto {acao:\"escolher\", caminho:[…]}` com a resposta. Repita até receber a receita. Pedido simples (desenhar uma forma, editar) NÃO precisa do funil." })),
        "escolher" | "status" => {
            if ids.last() == Some(&"outra") {
                return Ok(json!({ "nivel": "livre", "caminho": caminho, "como_seguir": "Sem receita pronta: pergunte ao usuário o que ele quer, proponha as folhas e os itens, confirme com ele e siga as regras gerais (camadas com prefixo, folha com carimbo, cálculo no Rust, verificar antes de entregar)." }));
            }
            let no = descer(&ids)?;
            match (no.receita, acao) {
                (Some(r), "status") => Ok(status(d, r, &caminho)),
                (Some(r), _) => Ok(apresentar(r, &caminho)),
                (None, "status") => Err("o `caminho` ainda não chegou a uma receita: continue com `escolher`".into()),
                (None, _) => Ok(json!({ "nivel": no.id, "perguntar_ao_usuario": no.pergunta, "opcoes": opcoes(no), "caminho": caminho })),
            }
        }
        outra => Err(format!("ação `{outra}` não existe (iniciar, escolher, status)")),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::documento::Unidade;

    fn doc() -> Documento {
        Documento::default().com_unidade(Some(Unidade::Metro))
    }

    #[test]
    fn funil_vai_da_raiz_a_receita_e_recusa_caminho_errado() {
        let d = doc();
        let r = projeto(&d, &json!({})).unwrap();
        assert_eq!(r["nivel"], "raiz");
        assert!(r["opcoes"].as_array().unwrap().iter().any(|o| o["id"] == "eletrica"));
        let n2 = projeto(&d, &json!({ "acao": "escolher", "caminho": ["eletrica"] })).unwrap();
        assert!(n2["opcoes"].as_array().unwrap().iter().any(|o| o["id"] == "luminotecnico"));
        let rec = projeto(&d, &json!({ "acao": "escolher", "caminho": ["eletrica", "luminotecnico"] })).unwrap();
        assert_eq!(rec["nivel"], "receita");
        assert!(rec["fases"].as_array().unwrap().len() >= 5);
        assert!(projeto(&d, &json!({ "acao": "escolher", "caminho": ["nada"] })).is_err());
        assert_eq!(projeto(&d, &json!({ "acao": "escolher", "caminho": ["eletrica", "outra"] })).unwrap()["nivel"], "livre");
    }

    #[test]
    fn status_marca_o_que_o_desenho_ja_tem_e_aponta_o_proximo() {
        let mut ag = crate::agente::Agente { doc: doc(), ..crate::agente::Agente::default() };
        let c = json!({ "acao": "status", "caminho": ["eletrica", "luminotecnico"] });
        let antes = projeto(&ag.doc, &c).unwrap();
        assert_eq!(antes["feitos"], 1, "só a unidade existe: {antes}");
        assert_eq!(antes["pronto_para_entregar"], false);
        crate::folhas::folha(&mut ag.doc, &json!({ "acao": "inserir", "papel": "A1", "escala": 100, "origem": true, "campos": { "cliente": "X", "disciplina": "ELÉTRICA", "titulo": "PLANTA", "responsavel": "Fulano" } })).unwrap();
        crate::mutacoes::camadas(&mut ag.doc, &json!({ "acao": "criar", "nome": "EL-Luminarias" })).unwrap();
        crate::mutacoes::criar(&mut ag.doc, &json!({ "camada": "EL-Luminarias", "formas": [{ "tipo": "circulo", "centro": [1, 1], "raio": 0.2 }, { "tipo": "texto", "ponto": [0, 0], "altura": 0.2, "conteudo": "Quadro de cargas" }] })).unwrap();
        let depois = projeto(&ag.doc, &c).unwrap();
        let estado = |id: &str| depois["itens"].as_array().unwrap().iter().find(|i| i["id"] == id).unwrap()["estado"].clone();
        assert_eq!(estado("folha"), "feito");
        assert_eq!(estado("carimbo"), "feito");
        assert_eq!(estado("luminarias"), "feito");
        assert_eq!(estado("circuitos"), "feito");
        assert_eq!(estado("unifilar"), "pendente");
        assert!(depois["feitos"].as_u64().unwrap() > antes["feitos"].as_u64().unwrap());
        assert!(depois["proximo"]["id"].is_string());
    }
}

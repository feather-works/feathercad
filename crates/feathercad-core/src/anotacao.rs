//! Formas de anotação prontas: cotas (linear, alinhada, raio e diâmetro), chamada (leader) e tabela. Só geometria: a sessão
//! escolhe os pontos e grava o resultado como entidades comuns (linhas, setas cheias e texto).

use crate::documento::{AlinhaH, AlinhaV, Documento, Geometria};
use crate::formato::Regional;
use crate::geom::{normalizar_graus, Ponto, Vertice};

/// A altura de texto quando o usuário não escolheu: 1/150 da diagonal do desenho (2,5 se ele está vazio).
pub fn altura_padrao(doc: &Documento) -> f64 {
    match doc.caixa() {
        Some((min, max)) => (min.distancia(max) / 150.0).max(1e-6),
        None => 2.5,
    }
}

fn soma(p: Ponto, dx: f64, dy: f64) -> Ponto {
    Ponto::novo(p.x + dx, p.y + dy)
}

/// Uma ponta de seta cheia: a ponta em `ponta`, apontando de `de` para `ponta`, com `tamanho` de comprimento.
pub fn seta(ponta: Ponto, de: Ponto, tamanho: f64) -> Option<Geometria> {
    let d = ponta.distancia(de);
    if d < 1e-12 || tamanho <= 0.0 {
        return None;
    }
    let (ux, uy) = ((de.x - ponta.x) / d, (de.y - ponta.y) / d);
    let base = soma(ponta, ux * tamanho, uy * tamanho);
    let meia = tamanho / 6.0;
    let laco = [ponta, soma(base, -uy * meia, ux * meia), soma(base, uy * meia, -ux * meia)].into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect();
    Some(Geometria::Hachura { contornos: vec![laco], padrao: Vec::new(), nome: "SOLID".into() })
}

fn texto(ponto: Ponto, altura: f64, rotacao: f64, conteudo: String, h: AlinhaH, v: AlinhaV) -> Geometria {
    Geometria::Texto { ponto, altura, rotacao, conteudo, h, v }
}

/// Uma cota linear (não associativa) entre `p1` e `p2`, com a linha de cota passando por `posicao`. Horizontal se
/// `posicao` está mais acima/abaixo que ao lado do meio do par; senão vertical. Devolve as linhas de chamada, a
/// linha de cota, as duas setas e o texto com a medida (na formatação da língua). `None` se a medida é nula.
pub fn cota_linear(p1: Ponto, p2: Ponto, posicao: Ponto, altura: f64, reg: &Regional) -> Option<Vec<Geometria>> {
    let meio = Ponto::novo((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
    let horizontal = (posicao.y - meio.y).abs() >= (posicao.x - meio.x).abs();
    let u = if horizontal { (1.0, 0.0) } else { (0.0, 1.0) };
    cota_ao_longo(p1, p2, posicao, u, altura, reg)
}

/// Cota alinhada: mede a distância real entre `p1` e `p2`, com a linha de cota paralela ao par, passando por
/// `posicao`. `None` se os pontos coincidem.
pub fn cota_alinhada(p1: Ponto, p2: Ponto, posicao: Ponto, altura: f64, reg: &Regional) -> Option<Vec<Geometria>> {
    let c = p1.distancia(p2);
    if c < 1e-12 {
        return None;
    }
    let mut u = ((p2.x - p1.x) / c, (p2.y - p1.y) / c);
    // o texto nunca fica de cabeça para baixo: a direção aponta para a direita (ou para cima, se vertical)
    if u.0 < -1e-9 || (u.0.abs() <= 1e-9 && u.1 < 0.0) {
        u = (-u.0, -u.1);
    }
    cota_ao_longo(p1, p2, posicao, u, altura, reg)
}

/// O desenho de uma cota cuja linha corre na direção `u` (unitária): a medida é a projeção de `p1→p2` em `u`.
/// A normal é a perpendicular à esquerda de `u`, e é para onde o texto sobe.
fn cota_ao_longo(p1: Ponto, p2: Ponto, posicao: Ponto, u: (f64, f64), altura: f64, reg: &Regional) -> Option<Vec<Geometria>> {
    let n = (-u.1, u.0);
    let medida = ((p2.x - p1.x) * u.0 + (p2.y - p1.y) * u.1).abs();
    if medida < 1e-12 {
        return None;
    }
    let ao_longo_de_n = |a: Ponto, b: Ponto| (b.x - a.x) * n.0 + (b.y - a.y) * n.1;
    // Os pés da linha de cota sobre cada ponto.
    let pe = |p: Ponto| {
        let k = ao_longo_de_n(p, posicao);
        soma(p, n.0 * k, n.1 * k)
    };
    let (d1, d2) = (pe(p1), pe(p2));
    let (folga, alem) = (altura * 0.4, altura * 0.5);
    let mut v = Vec::new();
    for (p, d) in [(p1, d1), (p2, d2)] {
        let sinal = if ao_longo_de_n(p, d) >= 0.0 { 1.0 } else { -1.0 };
        // A linha de chamada só existe se há distância para ela.
        if p.distancia(d) > folga {
            v.push(Geometria::Linha { a: soma(p, n.0 * sinal * folga, n.1 * sinal * folga), b: soma(d, n.0 * sinal * alem, n.1 * sinal * alem) });
        }
    }
    v.push(Geometria::Linha { a: d1, b: d2 });
    let tamanho = altura.min(d1.distancia(d2) / 3.0);
    v.extend(seta(d1, d2, tamanho));
    v.extend(seta(d2, d1, tamanho));
    let rotacao = normalizar_graus(u.1.atan2(u.0).to_degrees());
    let centro = Ponto::novo((d1.x + d2.x) / 2.0 + n.0 * altura * 0.6, (d1.y + d2.y) / 2.0 + n.1 * altura * 0.6);
    v.push(texto(centro, altura, rotacao, reg.numero(medida), AlinhaH::Centro, AlinhaV::Base));
    Some(v)
}

/// Cota de raio (`diametro` falso, texto `R5`) ou de diâmetro (texto `Ø10`) de um círculo ou arco. A reta vai do
/// centro (raio) ou do lado oposto (diâmetro) até a circunferência, na direção de `posicao`, e segue até ela se
/// estiver fora; o texto fica na ponta. `None` se `posicao` coincide com o centro ou o raio não é positivo.
pub fn cota_de_raio(centro: Ponto, raio: f64, posicao: Ponto, diametro: bool, altura: f64, reg: &Regional) -> Option<Vec<Geometria>> {
    let d = centro.distancia(posicao);
    if d < 1e-12 || raio <= 0.0 {
        return None;
    }
    let dir = ((posicao.x - centro.x) / d, (posicao.y - centro.y) / d);
    let na_borda = soma(centro, dir.0 * raio, dir.1 * raio);
    // a reta termina na borda ou, se o texto foi posto fora do círculo, no ponto escolhido
    let fim = if d > raio { posicao } else { na_borda };
    let inicio = if diametro { soma(centro, -dir.0 * raio, -dir.1 * raio) } else { centro };
    let tamanho = altura.min(inicio.distancia(fim) / 3.0);
    let mut v = vec![Geometria::Linha { a: inicio, b: fim }];
    v.extend(seta(na_borda, inicio, tamanho));
    if diametro {
        v.extend(seta(inicio, na_borda, tamanho));
    }
    let (dx, h) = if dir.0 >= 0.0 { (altura * 0.4, AlinhaH::Esquerda) } else { (-altura * 0.4, AlinhaH::Direita) };
    let rotulo = if diametro { format!("Ø{}", reg.numero(raio * 2.0)) } else { format!("R{}", reg.numero(raio)) };
    v.push(texto(soma(fim, dx, 0.0), altura, 0.0, rotulo, h, AlinhaV::Meio));
    Some(v)
}

/// Uma chamada: linha da `ponta` (com seta) até o `pouso`, e o texto ao lado do pouso (à direita se o pouso
/// está à direita da ponta, senão à esquerda). Texto vazio: só a linha e a seta.
pub fn chamada(ponta: Ponto, pouso: Ponto, altura: f64, conteudo: &str) -> Option<Vec<Geometria>> {
    if ponta.distancia(pouso) < 1e-12 {
        return None;
    }
    let mut v = vec![Geometria::Linha { a: ponta, b: pouso }];
    v.extend(seta(ponta, pouso, altura.min(ponta.distancia(pouso) / 2.0)));
    if !conteudo.trim().is_empty() {
        let direita = pouso.x >= ponta.x;
        let (dx, h) = if direita { (altura * 0.4, AlinhaH::Esquerda) } else { (-altura * 0.4, AlinhaH::Direita) };
        v.push(texto(soma(pouso, dx, 0.0), altura, 0.0, conteudo.to_string(), h, AlinhaV::Meio));
    }
    Some(v)
}

/// Uma tabela vazia (só a grade): o canto superior esquerdo em `origem`, crescendo para a direita e para baixo.
/// `None` se falta linha, coluna ou tamanho.
pub fn tabela(origem: Ponto, linhas: usize, colunas: usize, altura_linha: f64, largura_coluna: f64) -> Option<Vec<Geometria>> {
    if linhas == 0 || colunas == 0 || altura_linha <= 0.0 || largura_coluna <= 0.0 || linhas > 500 || colunas > 500 {
        return None;
    }
    let (largura, altura) = (colunas as f64 * largura_coluna, linhas as f64 * altura_linha);
    let mut v = Vec::new();
    for i in 0..=linhas {
        let y = origem.y - i as f64 * altura_linha;
        v.push(Geometria::Linha { a: Ponto::novo(origem.x, y), b: Ponto::novo(origem.x + largura, y) });
    }
    for j in 0..=colunas {
        let x = origem.x + j as f64 * largura_coluna;
        v.push(Geometria::Linha { a: Ponto::novo(x, origem.y), b: Ponto::novo(x, origem.y - altura) });
    }
    Some(v)
}

/// Área (fórmula do laço) e perímetro de um polígono fechado.
pub fn area_e_perimetro(pontos: &[Ponto]) -> (f64, f64) {
    let n = pontos.len();
    let mut area = 0.0;
    let mut perimetro = 0.0;
    for i in 0..n {
        let (a, b) = (pontos[i], pontos[(i + 1) % n]);
        area += a.x * b.y - b.x * a.y;
        perimetro += a.distancia(b);
    }
    (area.abs() / 2.0, perimetro)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    #[test]
    fn cota_horizontal_mede_x_com_linha_de_cota_na_altura_pedida() {
        let reg = Regional { decimal: ',', separador_de_coordenadas: ';', casas: 2 };
        let v = cota_linear(p(0.0, 0.0), p(10.0, 3.0), p(5.0, 8.0), 1.0, &reg).unwrap();
        // Linha de cota em y = 8 entre x = 0 e x = 10.
        assert!(v.contains(&Geometria::Linha { a: p(0.0, 8.0), b: p(10.0, 8.0) }), "{v:?}");
        let Some(Geometria::Texto { conteudo, ponto, rotacao, .. }) = v.last() else { panic!() };
        assert_eq!(conteudo, "10,00");
        assert!((ponto.x - 5.0).abs() < 1e-9 && ponto.y > 8.0 && *rotacao == 0.0);
        // Duas setas cheias, duas linhas de chamada, a de cota e o texto.
        assert_eq!(v.iter().filter(|g| matches!(g, Geometria::Hachura { .. })).count(), 2);
        assert_eq!(v.iter().filter(|g| matches!(g, Geometria::Linha { .. })).count(), 3);
    }

    #[test]
    fn cota_vertical_gira_o_texto_e_mede_y() {
        let reg = Regional { decimal: '.', separador_de_coordenadas: ',', casas: 1 };
        let v = cota_linear(p(0.0, 0.0), p(2.0, 6.0), p(-5.0, 3.0), 1.0, &reg).unwrap();
        assert!(v.contains(&Geometria::Linha { a: p(-5.0, 0.0), b: p(-5.0, 6.0) }), "{v:?}");
        assert!(matches!(v.last(), Some(Geometria::Texto { conteudo, rotacao, .. }) if conteudo == "6.0" && *rotacao == 90.0));
        assert!(cota_linear(p(0.0, 2.0), p(9.0, 2.0), p(4.0, 5.0), 1.0, &reg).is_some());
    }

    #[test]
    fn cota_alinhada_mede_a_distancia_real_e_o_texto_acompanha_a_inclinacao() {
        let reg = Regional { decimal: '.', separador_de_coordenadas: ',', casas: 1 };
        // 3-4-5: a medida é 5, a linha de cota é paralela ao par e passa por (0, 5) → deslocada 3 para o lado
        let v = cota_alinhada(p(0.0, 0.0), p(4.0, 3.0), p(0.0, 5.0), 1.0, &reg).unwrap();
        let Some(Geometria::Texto { conteudo, rotacao, .. }) = v.last() else { panic!() };
        assert_eq!(conteudo, "5.0");
        assert!((rotacao - 36.869897645844).abs() < 1e-6, "{rotacao}");
        // a linha de cota é a que liga os dois pés e tem o mesmo comprimento do par
        let cota = v.iter().filter_map(|g| if let Geometria::Linha { a, b } = g { Some(a.distancia(*b)) } else { None }).fold(0.0_f64, f64::max);
        assert!((cota - 5.0).abs() < 1e-9);
        // do outro lado do par, ou com os pontos invertidos, a medida e a legibilidade são as mesmas
        let inv = cota_alinhada(p(4.0, 3.0), p(0.0, 0.0), p(0.0, 5.0), 1.0, &reg).unwrap();
        assert!(matches!(inv.last(), Some(Geometria::Texto { rotacao, .. }) if *rotacao < 90.0 || *rotacao > 270.0));
        assert!(cota_alinhada(p(1.0, 1.0), p(1.0, 1.0), p(0.0, 5.0), 1.0, &reg).is_none());
    }

    #[test]
    fn cota_de_raio_e_de_diametro() {
        let reg = Regional { decimal: '.', separador_de_coordenadas: ',', casas: 1 };
        // raio: do centro até a borda (posição dentro do círculo); texto R5.0 do lado da direção
        let v = cota_de_raio(p(0.0, 0.0), 5.0, p(1.0, 0.0), false, 1.0, &reg).unwrap();
        assert!(v.contains(&Geometria::Linha { a: p(0.0, 0.0), b: p(5.0, 0.0) }), "{v:?}");
        assert!(matches!(v.last(), Some(Geometria::Texto { conteudo, h: AlinhaH::Esquerda, .. }) if conteudo == "R5.0"));
        assert_eq!(v.iter().filter(|g| matches!(g, Geometria::Hachura { .. })).count(), 1);
        // posição fora do círculo: a reta chega nela e o texto vai para lá
        let v = cota_de_raio(p(0.0, 0.0), 5.0, p(-9.0, 0.0), false, 1.0, &reg).unwrap();
        assert!(v.contains(&Geometria::Linha { a: p(0.0, 0.0), b: p(-9.0, 0.0) }));
        assert!(matches!(v.last(), Some(Geometria::Texto { h: AlinhaH::Direita, .. })));
        // diâmetro: de um lado ao outro, duas setas, texto Ø10.0
        let v = cota_de_raio(p(0.0, 0.0), 5.0, p(0.0, 2.0), true, 1.0, &reg).unwrap();
        assert!(v.contains(&Geometria::Linha { a: p(0.0, -5.0), b: p(0.0, 5.0) }), "{v:?}");
        assert!(matches!(v.last(), Some(Geometria::Texto { conteudo, .. }) if conteudo == "Ø10.0"));
        assert_eq!(v.iter().filter(|g| matches!(g, Geometria::Hachura { .. })).count(), 2);
        assert!(cota_de_raio(p(0.0, 0.0), 5.0, p(0.0, 0.0), false, 1.0, &reg).is_none());
    }

    #[test]
    fn chamada_tabela_e_area() {
        let v = chamada(p(0.0, 0.0), p(-5.0, 5.0), 1.0, "Aço").unwrap();
        assert!(matches!(v.last(), Some(Geometria::Texto { h: AlinhaH::Direita, conteudo, .. }) if conteudo == "Aço"));
        assert_eq!(chamada(p(0.0, 0.0), p(1.0, 1.0), 1.0, "  ").unwrap().len(), 2, "linha e seta");
        assert!(chamada(p(1.0, 1.0), p(1.0, 1.0), 1.0, "x").is_none());
        let t = tabela(p(0.0, 10.0), 2, 3, 4.0, 5.0).unwrap();
        assert_eq!(t.len(), 3 + 4);
        assert!(t.contains(&Geometria::Linha { a: p(0.0, 2.0), b: p(15.0, 2.0) }));
        assert!(tabela(p(0.0, 0.0), 0, 3, 1.0, 1.0).is_none());
        let (a, per) = area_e_perimetro(&[p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)]);
        assert_eq!((a, per), (12.0, 14.0));
    }
}

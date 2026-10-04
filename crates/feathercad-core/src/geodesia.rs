//! Geodésia mínima para o detalhamento geográfico das folhas: de coordenadas **UTM** (SIRGAS 2000/WGS 84, elipsoide GRS 80) para
//! latitude e longitude, com a convergência de meridianos e o fator de escala do ponto. Não adivinha a zona: quem chama diz
//! qual é (o projeto de referência cita "UTM SIRGAS 2000 zona 23 S, a confirmar").

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_222_101;
const K0: f64 = 0.9996;

/// Um ponto geográfico (graus) e as grandezas da projeção ali.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geografico {
    pub latitude: f64,
    pub longitude: f64,
    /// Convergência de meridianos (graus; positiva a leste do meridiano central no hemisfério norte, negativa no sul a leste).
    pub convergencia: f64,
    /// Fator de escala da projeção no ponto.
    pub escala: f64,
}

/// Meridiano central da zona UTM, em graus.
pub fn meridiano_central(zona: u32) -> f64 {
    f64::from(zona) * 6.0 - 183.0
}

/// UTM (E, N em metros) → graus. `sul` = hemisfério sul (N falso de 10 000 000 m).
pub fn utm_para_geografico(e: f64, n: f64, zona: u32, sul: bool) -> Geografico {
    let e2 = F * (2.0 - F);
    let ep2 = e2 / (1.0 - e2);
    let (x, y) = (e - 500_000.0, if sul { n - 10_000_000.0 } else { n });
    let m = y / K0;
    let mu = m / (A * (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2.powi(3) / 256.0));
    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
    let phi1 = mu + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin() + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin() + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin() + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();
    let (s1, c1) = phi1.sin_cos();
    let t1 = (s1 / c1).powi(2);
    let cc = ep2 * c1 * c1;
    let n1 = A / (1.0 - e2 * s1 * s1).sqrt();
    let r1 = A * (1.0 - e2) / (1.0 - e2 * s1 * s1).powf(1.5);
    let d = x / (n1 * K0);
    let phi = phi1 - (n1 * s1 / c1 / r1) * (d * d / 2.0 - (5.0 + 3.0 * t1 + 10.0 * cc - 4.0 * cc * cc - 9.0 * ep2) * d.powi(4) / 24.0 + (61.0 + 90.0 * t1 + 298.0 * cc + 45.0 * t1 * t1 - 252.0 * ep2 - 3.0 * cc * cc) * d.powi(6) / 720.0);
    let dlam = (d - (1.0 + 2.0 * t1 + cc) * d.powi(3) / 6.0 + (5.0 - 2.0 * cc + 28.0 * t1 - 3.0 * cc * cc + 8.0 * ep2 + 24.0 * t1 * t1) * d.powi(5) / 120.0) / c1;
    let gama = (dlam.tan() * phi.sin()).atan();
    let escala = K0 * (1.0 + (1.0 + ep2 * phi.cos().powi(2)) * (dlam * phi.cos()).powi(2) / 2.0);
    Geografico { latitude: phi.to_degrees(), longitude: meridiano_central(zona) + dlam.to_degrees(), convergencia: gama.to_degrees(), escala }
}

/// `-19.9167` → `19°55'00,0" S` (ou `N`/`L`/`O` conforme o eixo).
pub fn graus_minutos_segundos(valor: f64, positivo: char, negativo: char) -> String {
    let (hemi, v) = if valor >= 0.0 { (positivo, valor) } else { (negativo, -valor) };
    let (g, resto) = (v.trunc(), (v - v.trunc()) * 60.0);
    let (m, s) = (resto.trunc(), (resto - resto.trunc()) * 60.0);
    format!("{:.0}°{:02.0}'{:04.1}\" {hemi}", g, m, s).replace('.', ",")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn no_meridiano_central_e_no_equador() {
        let g = utm_para_geografico(500_000.0, 10_000_000.0, 23, true);
        assert!(g.latitude.abs() < 1e-9 && (g.longitude + 45.0).abs() < 1e-9 && g.convergencia.abs() < 1e-9);
        assert!((g.escala - 0.9996).abs() < 1e-9);
    }

    #[test]
    fn marco_zero_de_sao_paulo_cai_no_lugar() {
        // Praça da Sé: ≈ 23°33'03" S, 46°38'01" O  →  UTM 23 S ≈ 333 300 E, 7 394 300 N
        let g = utm_para_geografico(333_300.0, 7_394_300.0, 23, true);
        assert!((g.latitude + 23.551).abs() < 0.01 && (g.longitude + 46.633).abs() < 0.01, "{g:?}");
        assert!(g.convergencia > 0.0 && g.convergencia < 1.5, "a oeste do meridiano central no sul a convergência é positiva: {}", g.convergencia);
    }

    #[test]
    fn formata_graus_minutos_segundos() {
        assert_eq!(graus_minutos_segundos(-19.5, 'N', 'S'), "19°30'00,0\" S");
        assert_eq!(graus_minutos_segundos(-46.25, 'L', 'O'), "46°15'00,0\" O");
    }
}

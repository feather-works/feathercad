"""Uma folha A3 para cada CURVA e cada TANGENTE do traçado: planta na escala que o elemento pede (com dormentes, trilhos, estacas,
pontos notáveis e as zonas de aterro e escavação), perfil local (existente × greide hachurado), tabela de dados com a folga de cada
limite e a lista de pontos críticos e de atenção — tudo vindo do `cad_analise` do MCP.
"""
import math

from ferrovia_lib import Fct, Folha, escala_que_cabe, interpolar, paralela, recortar_poli, v

COR_NIVEL = {"CRÍTICO": "#E03030", "ATENÇÃO": "#E08000", "NORMAL": "#20A050"}
COR_ATERRO = "#D04040"
COR_ESCAVACAO = "#3080D0"
COR_EXISTENTE = "#FF7F00"
COR_PROJETO = "#FF00FF"
COR_TANGENTE = "#2080E0"
COR_ESPIRAL = "#B000B0"
COR_CIRCULAR = "#D02020"
COR_GRADE = "#909090"

# painéis da A3 (mm de papel): planta, perfil, dados e pontos de atenção
PLANTA = (20.0, 150.0, 205.0, 264.0)
PERFIL = (215.0, 150.0, 405.0, 264.0)
DADOS = (20.0, 60.0, 205.0, 142.0)
ATENCAO = (215.0, 60.0, 405.0, 142.0)

CONSELHOS = {
    "aterro": "Aterro: conferir compactação e capacidade de suporte da plataforma, drenagem e recalque; prever lastro extra e socaria de acomodação.",
    "escavação": "Escavação: conferir estabilidade do talude, drenagem lateral e tipo de solo/rocha; proteger a via durante o corte e refazer o sublastro.",
    "ripagem": "Ripagem grande: planejar a ripagem com socadora/ripadora em etapas e conferir fixação, dormentes e lastro no deslocamento.",
    "superelevação": "Superelevação no limite: não há reserva para aumentar h; qualquer ganho de velocidade pede revisão do raio ou da transição.",
    "transição": "Transição no mínimo: controlar a variação de h (dh/dt) e a rampa de superelevação na execução.",
    "raio": "Raio perto do mínimo: sem folga de velocidade; avaliar suavizar a curva.",
    "tangente curta": "Tangente curta entre curvas: as transições quase se tocam; atenção à geometria e à variação de superelevação.",
    "levantamento": "Poucos pontos do levantamento: complementar a topografia antes de executar.",
}


def fmt_est(prog):
    """117+12,66 (estaca de 20 m)."""
    n = math.floor(prog / 20.0 + 1e-9)
    r = prog - n * 20.0
    return f"{n}+{r:05.2f}".replace(".", ",")


def painel(c, p):
    c.camada("ELEM_MOLDURA", "#FFFFFF")
    c.ret((p[0], p[1]), (p[2], p[3]))


class Elemento:
    """Tudo o que uma folha de elemento precisa."""

    def __init__(self, el, dados):
        self.el = el
        self.d = dados
        self.tipo = el["tipo"]
        self.n = el["n"]
        self.de, self.ate = el["de"], el["ate"]
        self.nome = ("CURVA" if self.tipo == "curva" else "TANGENTE") + f" {self.n}"
        self.curto = ("C" if self.tipo == "curva" else "T") + str(self.n)

    def amostra(self, de=None, ate=None):
        a = self.d["al"]
        de = self.de if de is None else de
        ate = self.ate if ate is None else ate
        return [r for r in a if de - 1e-6 <= r[0] <= ate + 1e-6]


class Planta:
    """A planta do elemento: transformação mundo → mm da folha, girada para o elemento ficar na horizontal."""

    def __init__(self, e, painel_mm):
        self.p = painel_mm
        pad = max(0.08 * (e.ate - e.de), 25.0)
        s0 = e.d["al"][0][0]
        amostra = e.amostra(max(e.de - pad, s0), min(e.ate + pad, e.d["al"][-1][0]))
        if len(amostra) < 2:
            amostra = e.amostra(e.de - 10, e.ate + 10)
        a, b = amostra[0], amostra[-1]
        self.theta = math.atan2(b[2] - a[2], b[1] - a[1])
        c, s = math.cos(self.theta), math.sin(self.theta)
        uv = [((r[1] - a[1]) * c + (r[2] - a[2]) * s, -(r[1] - a[1]) * s + (r[2] - a[2]) * c) for r in amostra]
        umin, umax = min(u for u, _ in uv), max(u for u, _ in uv)
        vmin, vmax = min(w for _, w in uv), max(w for _, w in uv)
        self.origem = a
        folga = 14.0
        largura_p, altura_p = self.p[2] - self.p[0] - 12.0, self.p[3] - self.p[1] - 22.0
        self.esc = escala_que_cabe(max(umax - umin, 1.0) + 2 * folga, max(vmax - vmin, 1.0) + 2 * folga, (largura_p, altura_p))
        self.uc, self.vc = (umin + umax) / 2.0, (vmin + vmax) / 2.0
        self.cx, self.cy = (self.p[0] + self.p[2]) / 2.0, (self.p[1] + self.p[3]) / 2.0 + 3.0

    def uv(self, E, N):
        a = self.origem
        c, s = math.cos(self.theta), math.sin(self.theta)
        return ((E - a[1]) * c + (N - a[2]) * s, -(E - a[1]) * s + (N - a[2]) * c)

    def mm(self, E, N):
        u, w = self.uv(E, N)
        k = 1000.0 / self.esc
        return (self.cx + (u - self.uc) * k, self.cy + (w - self.vc) * k)

    def mundo(self, x, y):
        """Do mm da folha para E, N (para os cantos do painel)."""
        k = 1000.0 / self.esc
        u, w = (x - self.cx) / k + self.uc, (y - self.cy) / k + self.vc
        c, s = math.cos(self.theta), math.sin(self.theta)
        return (self.origem[1] + u * c - w * s, self.origem[2] + u * s + w * c)

    def area(self):
        return (self.p[0], self.p[1], self.p[2], self.p[3])


def desenhar_planta(c, e):
    pl = Planta(e, PLANTA)
    d = e.d
    area = pl.area()
    painel(c, PLANTA)
    c.camada("ELEM_TITULO_PAINEL", "#FFFFFF")
    c.texto((PLANTA[0] + 2, PLANTA[3] + 2.0), 3.0, f"PLANTA — ESCALA 1:{pl.esc} (girada: o elemento fica na horizontal)")
    # malha E/N
    passo = next(m for m in (50, 100, 200, 250, 500, 1000, 2000) if m * 1000.0 / pl.esc >= 22.0)
    c.camada("ELEM_MALHA", "#909090")
    cantos = [pl.mundo(PLANTA[0], PLANTA[1]), pl.mundo(PLANTA[2], PLANTA[3]), pl.mundo(PLANTA[0], PLANTA[3]), pl.mundo(PLANTA[2], PLANTA[1])]
    e0, e1 = min(q[0] for q in cantos), max(q[0] for q in cantos)
    n0, n1 = min(q[1] for q in cantos), max(q[1] for q in cantos)
    braco = 2.2 * pl.esc / 1000.0
    ge = math.ceil(e0 / passo) * passo
    while ge <= e1:
        gn = math.ceil(n0 / passo) * passo
        while gn <= n1:
            x, y = pl.mm(ge, gn)
            if area[0] + 3 < x < area[2] - 3 and area[1] + 3 < y < area[3] - 3:
                for dE, dN in ((braco, 0), (0, braco)):
                    a, b = pl.mm(ge - dE, gn - dN), pl.mm(ge + dE, gn + dN)
                    c.linha(a, b)
            gn += passo
        ge += passo
    c.texto((PLANTA[0] + 2, PLANTA[1] + 2.0), 2.0, f"Malha E/N a cada {passo} m. Cantos: E {cantos[0][0]:.0f} N {cantos[0][1]:.0f} (inf. esq.) — E {cantos[1][0]:.0f} N {cantos[1][1]:.0f} (sup. dir.)", cor="#909090")
    # eixo existente
    c.camada("ELEM_EXISTENTE", COR_EXISTENTE)
    for parte in recortar_poli([pl.mm(E, N) for E, N in d["existente"]], area):
        c.poli(parte)
    # traçado de projeto por tipo de elemento
    al = e.amostra(max(e.de - 60, d["al"][0][0]), min(e.ate + 60, d["al"][-1][0]))
    for tipo, cor, nome in (("T", COR_TANGENTE, "ELEM_TANGENTE"), ("E", COR_ESPIRAL, "ELEM_ESPIRAL"), ("C", COR_CIRCULAR, "ELEM_CIRCULAR")):
        c.camada(nome, cor, espessura=0.5)
        corrida = []
        for r in al:
            if r[5] == tipo:
                corrida.append(pl.mm(r[1], r[2]))
            elif corrida:
                for parte in recortar_poli(corrida, area):
                    c.poli(parte)
                corrida = []
        for parte in recortar_poli(corrida, area):
            c.poli(parte)
    centro = [(r[1], r[2]) for r in al]
    if pl.esc <= 2500:
        c.camada("ELEM_TRILHOS", "#7F9FFF")
        for off in (0.5375, -0.5375):
            for parte in recortar_poli([pl.mm(E, N) for E, N in paralela(centro, off)], area):
                c.poli(parte)
    if pl.esc <= 500:
        dormentes(c, pl, d, e)
    estacas(c, pl, e, area)
    marcos(c, pl, e, area)
    zonas(c, pl, e, area, d)
    rosa_e_barra(c, pl)
    detalhe_dormentes(c, e)
    return pl


def dormentes(c, pl, d, e):
    """Um retângulo de 2,00 × 0,22 m a cada 0,60 m ao longo do trecho visível (só em escalas ≤ 1:500)."""
    c.camada("ELEM_DORMENTES", "#A0A0A0")
    lo, hi = e.de - 30.0, e.ate + 30.0
    s = math.ceil(lo / 0.6) * 0.6
    amostra = d["al"]
    ss = [r[0] for r in amostra]
    while s <= hi:
        E = interpolar(ss, [r[1] for r in amostra], s)
        N = interpolar(ss, [r[2] for r in amostra], s)
        dirg = interpolar(ss, [r[3] for r in amostra], s)
        if E is not None and N is not None and dirg is not None:
            t = math.radians(dirg)
            cs, sn = math.cos(t), math.sin(t)
            cantos = [(-0.11, -1.0), (0.11, -1.0), (0.11, 1.0), (-0.11, 1.0)]
            pts = [pl.mm(E + a * cs - b * sn, N + a * sn + b * cs) for a, b in cantos]
            if all(PLANTA[0] < x < PLANTA[2] and PLANTA[1] < y < PLANTA[3] for x, y in pts):
                c.poli(pts, True)
        s += 0.6


def estacas(c, pl, e, area):
    """Marca a cada 20 m, rótulo a cada `passo` (que dá ao menos 16 mm de papel entre rótulos)."""
    d = e.d
    passo = next(p for p in (20, 40, 100, 200, 500, 1000, 2000) if p * 1000.0 / pl.esc >= 16.0)
    c.camada("ELEM_ESTACAS", "#808080")
    ss = [r[0] for r in d["al"]]
    s = math.ceil((e.de - 60) / 20.0) * 20.0
    while s <= e.ate + 60:
        E, N, dirg = (interpolar(ss, [r[i] for r in d["al"]], s) for i in (1, 2, 3))
        if E is not None:
            x, y = pl.mm(E, N)
            if area[0] + 4 < x < area[2] - 4 and area[1] + 4 < y < area[3] - 4:
                t = math.radians(dirg) - pl.theta
                nx, ny = -math.sin(t), math.cos(t)
                rotulada = abs(s / passo - round(s / passo)) < 1e-6
                m = 1.6 if rotulada else 0.8
                c.linha((x - nx * m, y - ny * m), (x + nx * m, y + ny * m))
                if rotulada:
                    c.texto((x - nx * 3.4, y - ny * 3.4), 2.2, fmt_est(s), "cm")
        s += 20.0


def marcos(c, pl, e, area):
    """TS, SC, CS, ST (e o início/fim da tangente) com a estaca."""
    c.camada("ELEM_NOTAVEIS", "#FF7F9F")
    if e.tipo == "curva":
        cv = e.el["curva"]
        pontos = [(n, cv[n]) for n in ("TS", "SC", "CS", "ST")]
    else:
        pontos = []
        for rotulo, s in (("INÍCIO", e.de), ("FIM", e.ate)):
            r = min(e.d["al"], key=lambda q: abs(q[0] - s))
            pontos.append((rotulo, {"progressiva": s, "E": r[1], "N": r[2]}))
    for k, (nome, m) in enumerate(pontos):
        x, y = pl.mm(m["E"], m["N"])
        if area[0] + 3 < x < area[2] - 3 and area[1] + 3 < y < area[3] - 3:
            c.circulo((x, y), 1.1)
            lado = -1 if k % 2 == 0 else 1
            c.texto((x, y + lado * 9.5), 2.4, f"{nome} {fmt_est(m['progressiva'])}", "cm")
            c.linha((x, y + lado * 1.2), (x, y + lado * 8.0))


def zonas(c, pl, e, area, d):
    """Faixa colorida ao lado do eixo onde há aterro (vermelho) ou escavação (azul), com o valor máximo."""
    al = d["al"]
    ss = [r[0] for r in al]
    for z in e.el["zonas"]:
        cor = COR_ATERRO if z["natureza"] == "aterro" else COR_ESCAVACAO
        s = z["de"]
        pts = []
        while s <= z["ate"] + 1e-9:
            E, N, dirg = (interpolar(ss, [r[i] for r in al], s) for i in (1, 2, 3))
            if E is not None:
                x, y = pl.mm(E, N)
                t = math.radians(dirg) - pl.theta
                pts.append((x - math.sin(t) * 3.6, y + math.cos(t) * 3.6))
            s += 5.0
        c.camada("ELEM_ZONA_ATERRO" if z["natureza"] == "aterro" else "ELEM_ZONA_ESCAVACAO", cor, espessura=0.9)
        for parte in recortar_poli(pts, area):
            c.poli(parte)
        if pts:
            mx, my = pts[len(pts) // 2]
            if area[0] + 12 < mx < area[2] - 12 and area[1] + 6 < my < area[3] - 6:
                c.texto((mx, my + 3.2), 2.2, f"{z['natureza'].upper()} {v(z['maximo_m'], 2)} m", "cm", cor=cor)


def rosa_e_barra(c, pl):
    """Seta do norte (girada com a planta) e a barra de escala."""
    x, y = PLANTA[2] - 12.0, PLANTA[3] - 14.0
    ang = math.pi / 2 - pl.theta  # direção do norte na folha
    nx, ny = math.cos(ang), math.sin(ang)
    c.camada("ELEM_NORTE", "#FFFFFF")
    c.linha((x - nx * 7, y - ny * 7), (x + nx * 7, y + ny * 7))
    c.poli([(x + nx * 7, y + ny * 7), (x + nx * 3 - ny * 2, y + ny * 3 + nx * 2), (x + nx * 3 + ny * 2, y + ny * 3 - nx * 2)], True)
    c.texto((x + nx * 10, y + ny * 10), 3.0, "N", "cm")
    barra_m = next(m for m in (5, 10, 20, 50, 100, 200, 500, 1000) if m * 1000.0 / pl.esc >= 20.0)
    bx, by, w = PLANTA[0] + 6.0, PLANTA[1] + 9.0, barra_m * 1000.0 / pl.esc
    c.poli([(bx, by), (bx + w, by)])
    for t in (0, 0.5, 1.0):
        c.linha((bx + w * t, by - 1.2), (bx + w * t, by + 1.2))
    c.texto((bx + w / 2, by + 3.0), 2.2, f"{barra_m} m", "cm")


def detalhe_dormentes(c, e):
    """Detalhe 1:100: 7 m de via (4 filetes de trilho, dormentes de 2,00 × 0,22 m a cada 0,60 m) no ponto mais exigido do elemento."""
    cx = (PLANTA[0] + PLANTA[2]) / 2.0
    x0, y0, x1, y1 = cx - 42.0, PLANTA[1] + 7.0, cx + 42.0, PLANTA[1] + 46.0
    c.camada("ELEM_DETALHE", "#FFFFFF")
    c.ret((x0, y0), (x1, y1))
    ponto = "SC" if e.tipo == "curva" else "meio"
    c.texto((x0 + 1.5, y1 - 2.6), 2.4, f"DETALHE 1:100 — DORMENTES E TRILHOS ({ponto})")
    k = 10.0  # mm por metro a 1:100
    cy = (y0 + y1) / 2.0
    c.camada("ELEM_DETALHE_DORMENTE", "#A0A0A0")
    xi = x0 + 4.0
    n = int((x1 - x0 - 8.0) / (0.6 * k))
    for i in range(n + 1):
        xc = xi + i * 0.6 * k
        c.ret((xc - 0.11 * k, cy - 1.0 * k), (xc + 0.11 * k, cy + 1.0 * k))
    c.camada("ELEM_DETALHE_TRILHO", "#7F9FFF")
    for off in (0.5375 - 0.0345, 0.5375 + 0.0345, -0.5375 + 0.0345, -0.5375 - 0.0345):
        c.linha((xi - 2, cy + off * k), (x1 - 2.0, cy + off * k))
    c.camada("ELEM_DETALHE", "#FFFFFF")
    c.texto((x0 + 1.5, y0 + 1.6), 2.0, "bitola 1,000 m (eixo a eixo 1,075 m) — dormente 2,00 × 0,22 m, espaçamento 0,60 m", cor="#909090")


def desenhar_perfil(c, e):
    """Existente × greide no trecho do elemento (± folga), hachurado: aterro em vermelho, escavação em azul."""
    d = e.d
    p = PERFIL
    painel(c, PERFIL)
    c.camada("ELEM_TITULO_PAINEL", "#FFFFFF")
    pad = max(0.08 * (e.ate - e.de), 15.0)
    s0, s1 = max(e.de - pad, d["tab"][0][0]), min(e.ate + pad, d["tab"][-1][0])
    linhas = [r for r in d["tab"] if s0 <= r[0] <= s1]
    zs = [r[1] for r in linhas if r[1] is not None] + [r[2] for r in linhas]
    zmin, zmax = min(zs), max(zs)
    cx0, cx1, cy0, cy1 = p[0] + 14.0, p[2] - 4.0, p[1] + 22.0, p[3] - 12.0
    esc_h = escala_que_cabe(s1 - s0, 0.0, (cx1 - cx0, 1.0))
    esc_v = next((ev for ev in (20, 50, 100, 200, 250, 500, 1000, 2000) if (zmax - zmin + 0.6) * 1000.0 / ev <= (cy1 - cy0)), 5000)
    z0 = math.floor((zmin - 0.3) * 10) / 10.0
    c.texto((p[0] + 2, p[3] + 2.0), 3.0, f"PERFIL LONGITUDINAL LOCAL — H 1:{esc_h}  V 1:{esc_v} (exagero {esc_h // esc_v if esc_h >= esc_v else 1}×)")
    X = lambda s: cx0 + (s - s0) * 1000.0 / esc_h
    Y = lambda z: cy0 + (z - z0) * 1000.0 / esc_v
    c.camada("ELEM_PERFIL_GRADE", COR_GRADE)
    z = math.ceil(z0 * 10) / 10.0
    passo_z = next(pz for pz in (0.05, 0.1, 0.2, 0.5, 1.0) if pz * 1000.0 / esc_v >= 6.0)
    while Y(z) <= cy1:
        c.linha((cx0, Y(z)), (cx1, Y(z)))
        c.texto((cx0 - 1.2, Y(z)), 2.0, v(z, 2), "dm")
        z += passo_z
    s = math.ceil(s0 / 100.0) * 100.0
    step = 100.0 if (s1 - s0) > 300 else 50.0 if (s1 - s0) > 120 else 20.0
    s = math.ceil(s0 / step) * step
    while s <= s1:
        c.linha((X(s), cy0), (X(s), cy1))
        c.texto((X(s), cy0 - 1.5), 2.0, fmt_est(s), "dm", 90.0)
        s += step
    # hachuras entre as linhas
    run, sinal = [], 0
    def fechar():
        nonlocal run, sinal
        if len(run) >= 2:
            topo = [(X(s), Y(zg)) for s, zg, ze in run]
            base = [(X(s), Y(ze)) for s, zg, ze in reversed(run)]
            c.camada("ELEM_PERFIL_ATERRO" if sinal > 0 else "ELEM_PERFIL_ESCAVACAO", COR_ATERRO if sinal > 0 else COR_ESCAVACAO)
            c.hachura(topo + base, "padrao" if sinal > 0 else "ansi37", 1.1)
        run, sinal = [], 0
    for r in linhas:
        if r[1] is None:
            fechar()
            continue
        dif = r[2] - r[1]
        sg = 1 if dif > 0.01 else -1 if dif < -0.01 else 0
        if sg == 0 or (sinal and sg != sinal):
            fechar()
        if sg != 0:
            sinal = sg
            run.append((r[0], r[2], r[1]))
    fechar()
    c.camada("ELEM_PERFIL_EXISTENTE", COR_EXISTENTE)
    c.poli([(X(r[0]), Y(r[1])) for r in linhas if r[1] is not None])
    c.camada("ELEM_PERFIL_GREIDE", COR_PROJETO)
    c.poli([(X(r[0]), Y(r[2])) for r in linhas])
    # limites do elemento e pontos notáveis
    c.camada("ELEM_PERFIL_LIMITES", "#E0B000")
    marcas = [("início", e.de), ("fim", e.ate)]
    if e.tipo == "curva":
        cv = e.el["curva"]
        marcas = [(n, cv[n]["progressiva"]) for n in ("TS", "SC", "CS", "ST")]
    for nome, s in marcas:
        if s0 <= s <= s1:
            c.linha((X(s), cy0), (X(s), cy1 + 1.5))
            c.texto((X(s), cy1 + 3.0), 2.2, nome, "cm")
    dentro = [(i, piv) for i, piv in enumerate(d["pivs"]) if s0 <= piv["progressiva"] <= s1]
    for i, piv in dentro:
        c.camada("ELEM_PERFIL_PIV", COR_PROJETO)
        c.linha((X(piv["progressiva"]), Y(piv["cota"])), (X(piv["progressiva"]), cy1))
        c.texto((X(piv["progressiva"]), cy1 + 3.0), 2.2, f"PIV {i + 1}", "cm")
    if dentro:
        detalhe = "   ".join(f"PIV {i + 1}: est. {fmt_est(piv['progressiva'])}, cota {v(piv['cota'], 2)} m, Lv {piv['lv']:.0f} m" for i, piv in dentro[:3])
        c.camada("ELEM_PERFIL_PIV", COR_PROJETO)
        c.texto((p[0] + 2, p[1] + 8.0), 2.1, detalhe, "em", cor=COR_PROJETO)
    # legenda
    c.camada("ELEM_PERFIL_LEGENDA", COR_EXISTENTE)
    c.linha((p[0] + 3, p[1] + 3.5), (p[0] + 12, p[1] + 3.5))
    c.camada("ELEM_PERFIL_LEGENDA", "#FFFFFF")
    c.texto((p[0] + 14, p[1] + 3.5), 2.2, "terreno existente (cota do trilho)", "em")
    c.camada("ELEM_PERFIL_LEGENDA", COR_PROJETO)
    c.linha((p[0] + 70, p[1] + 3.5), (p[0] + 79, p[1] + 3.5))
    c.camada("ELEM_PERFIL_LEGENDA", "#FFFFFF")
    c.texto((p[0] + 81, p[1] + 3.5), 2.2, "greide de projeto", "em")
    c.texto((p[0] + 115, p[1] + 3.5), 2.2, "vermelho: aterro   azul: escavação", "em")


def uso(valor, limite):
    return valor / limite * 100.0 if limite else 0.0


def cor_do_uso(pct, critico=100.0, atencao=90.0):
    return COR_NIVEL["CRÍTICO"] if pct > critico else COR_NIVEL["ATENÇÃO"] if pct >= atencao else COR_NIVEL["NORMAL"]


def linhas_da_curva(e):
    cv, r = e.el["curva"], e.d["regras"]
    el = e.el
    h_uso, i_uso = uso(cv["h_mm"], r["h_max_mm"]), uso(cv["i_mm"], r["i_max_mm"])
    folga = lambda x, ref: f"{(x / ref - 1) * 100:+.0f}%".replace(".", ",")
    return [
        ("Raio R (m)", f"{cv['raio']:.0f}", f"mínimo {v(cv['r_min'], 0)}", folga(cv["raio"], cv["r_min"]), COR_NIVEL["NORMAL"] if cv["raio"] >= 1.25 * cv["r_min"] else COR_NIVEL["ATENÇÃO"] if cv["raio"] >= cv["r_min"] else COR_NIVEL["CRÍTICO"]),
        ("Transição Ls (m)", f"{cv['ls']:.0f}", f"L_min {v(cv['l_min'], 1)}", folga(cv["ls"], cv["l_min"]), COR_NIVEL["NORMAL"] if cv["ls"] >= 1.03 * cv["l_min"] else COR_NIVEL["ATENÇÃO"] if cv["ls"] >= cv["l_min"] else COR_NIVEL["CRÍTICO"]),
        ("Trecho circular Lc (m)", v(cv["lc"], 1), f"mínimo {r['lc_min']:.0f}", "", COR_NIVEL["NORMAL"] if cv["lc"] >= r["lc_min"] else COR_NIVEL["ATENÇÃO"]),
        ("Deflexão Δ (°) / lado", f"{v(cv['delta_graus'], 2)} / {cv['lado']}", "", "", None),
        ("Tangente Ts / externa E (m)", f"{v(cv['ts_m'], 1)} / {v(cv['e_m'], 1)}", "", "", None),
        ("Superelevação h (mm)", f"{cv['h_mm']:.0f}", f"h_max {r['h_max_mm']:.0f}", f"{h_uso:.0f}% do limite", cor_do_uso(h_uso, 100.0, 95.0)),
        ("Equilíbrio he (mm)", v(cv["he_mm"], 1), "", "", None),
        ("Insuficiência I (mm)", v(cv["i_mm"], 1), f"I_max {r['i_max_mm']:.0f}", f"{i_uso:.0f}% do limite", cor_do_uso(i_uso, 100.0, 80.0)),
        ("Velocidade máx. da curva (km/h)", v(cv["v_max"], 1), f"V {r['V']:.0f}", folga(cv["v_max"], r["V"]), COR_NIVEL["NORMAL"] if cv["v_max"] >= 1.15 * r["V"] else COR_NIVEL["ATENÇÃO"] if cv["v_max"] >= r["V"] else COR_NIVEL["CRÍTICO"]),
        ("Aceleração não compensada (m/s²)", v(cv["anc"], 3), "", "", None),
        ("Rampa de superelevação", f"1:{1.0 / cv['rampa_efetiva']:.0f}" if cv["rampa_efetiva"] else "—", f"máx 1:{1.0 / r['rampa']:.0f}", "", None),
        ("Ripagem máx. / RMS (m)", f"{v(el['ripagem_m']['max'], 2)} / {v(el['ripagem_m']['rms'], 2)}", "", "", COR_NIVEL["NORMAL"] if el["ripagem_m"]["max"] < 0.5 else COR_NIVEL["ATENÇÃO"] if el["ripagem_m"]["max"] < 0.75 else COR_NIVEL["CRÍTICO"]),
        ("Levante mín / méd / máx (m)", f"{v(el['levante_m']['min'], 2)} / {v(el['levante_m']['medio'], 2)} / {v(el['levante_m']['max'], 2)}", "+ aterro  − escavação", "", None),
        ("Rampa do greide mín / máx (%)", f"{v(el['rampa_pct']['min'], 2)} / {v(el['rampa_pct']['max'], 2)}", "", "", None),
        ("Cotas mín / máx (m)", f"{v(el['cota_m']['min'], 2)} / {v(el['cota_m']['max'], 2)}", "", "", None),
        ("Pontos do levantamento", str(el["observacoes"]), "", "", None),
    ]


def linhas_da_tangente(e):
    el = e.el
    ant, pro = f"C{e.n - 1}" if e.n > 1 else "início", f"C{e.n}" if e.n <= e.d["n_curvas"] else "fim"
    az = lambda dirg: (90.0 - dirg) % 360.0
    r0 = min(e.d["al"], key=lambda q: abs(q[0] - e.de))
    r1 = min(e.d["al"], key=lambda q: abs(q[0] - e.ate))
    return [
        ("Comprimento (m)", v(el["comprimento"], 1), f"mín. entre curvas {e.d['criterios_tangente']:.0f}", "", COR_NIVEL["NORMAL"] if el["comprimento"] >= e.d["criterios_tangente"] or e.n == 1 or e.n > e.d["n_curvas"] else COR_NIVEL["ATENÇÃO"]),
        ("Entre as curvas", f"{ant} → {pro}", "", "", None),
        ("Estacas", f"{fmt_est(e.de)} a {fmt_est(e.ate)}", "", "", None),
        ("Azimute inicial / final (°)", f"{v(az(r0[3]), 2)} / {v(az(r1[3]), 2)}", "", "", None),
        ("Ripagem máx. / RMS (m)", f"{v(el['ripagem_m']['max'], 2)} / {v(el['ripagem_m']['rms'], 2)}", "", "", COR_NIVEL["NORMAL"] if el["ripagem_m"]["max"] < 0.5 else COR_NIVEL["ATENÇÃO"] if el["ripagem_m"]["max"] < 0.75 else COR_NIVEL["CRÍTICO"]),
        ("Levante mín / méd / máx (m)", f"{v(el['levante_m']['min'], 2)} / {v(el['levante_m']['medio'], 2)} / {v(el['levante_m']['max'], 2)}", "+ aterro  − escavação", "", None),
        ("Rampa do greide mín / máx (%)", f"{v(el['rampa_pct']['min'], 2)} / {v(el['rampa_pct']['max'], 2)}", "máx. 1,0 (atenção)", "", COR_NIVEL["NORMAL"] if max(abs(el["rampa_pct"]["min"]), abs(el["rampa_pct"]["max"])) < 1.0 else COR_NIVEL["ATENÇÃO"]),
        ("Cotas mín / máx (m)", f"{v(el['cota_m']['min'], 2)} / {v(el['cota_m']['max'], 2)}", "", "", None),
        ("Pontos do levantamento", str(el["observacoes"]), "mín. 3", "", COR_NIVEL["NORMAL"] if el["observacoes"] >= 3 else COR_NIVEL["ATENÇÃO"]),
        ("Dormentes (0,60 m)", f"≈ {int(el['comprimento'] / 0.6) + 1}", "", "", None),
        ("Trilhos (2 filas, m)", v(2 * el["comprimento"], 0), "", "", None),
    ]


def desenhar_dados(c, e):
    p = DADOS
    painel(c, DADOS)
    c.camada("ELEM_DADOS", "#FFFFFF")
    c.texto((p[0] + 2, p[3] + 2.0), 3.0, "DADOS DO ELEMENTO E FOLGA DOS LIMITES")
    linhas = linhas_da_curva(e) if e.tipo == "curva" else linhas_da_tangente(e)
    h = (p[3] - p[1] - 6.0) / max(len(linhas), 1)
    h = min(h, 4.9)
    y = p[3] - 3.0
    x_val, x_ref, x_fol = p[0] + 82.0, p[0] + 118.0, p[0] + 156.0
    for i, (nome, valor, ref, folga, cor) in enumerate(linhas):
        yy = y - i * h
        c.camada("ELEM_DADOS_LINHA", "#B0B0B0")
        c.linha((p[0], yy - h), (p[2], yy - h))
        c.camada("ELEM_DADOS", "#FFFFFF")
        c.texto((p[0] + 2, yy - h / 2.0), 2.6, nome, "em")
        c.texto((x_val, yy - h / 2.0), 2.6, valor, "em", cor=cor)
        if ref:
            c.texto((x_ref, yy - h / 2.0), 2.3, ref, "em", cor="#808080")
        if folga:
            c.texto((x_fol, yy - h / 2.0), 2.3, folga, "em", cor=cor)
        if cor:
            c.camada("ELEM_DADOS_ESTADO", cor)
            c.circulo((x_val - 3.0, yy - h / 2.0), 1.0)


def conselhos_para(motivo):
    t = motivo.lower()
    for chave, texto in CONSELHOS.items():
        if chave in t:
            return texto
    return None


def quebrar(texto, largura_mm, altura_mm):
    """Quebra de linha por contagem de caracteres (0,55 da altura por letra, em média)."""
    por_linha = max(int(largura_mm / (0.55 * altura_mm)), 10)
    linhas, atual = [], ""
    for palavra in texto.split():
        if len(atual) + len(palavra) + 1 > por_linha and atual:
            linhas.append(atual)
            atual = palavra
        else:
            atual = (atual + " " + palavra).strip()
    if atual:
        linhas.append(atual)
    return linhas


def desenhar_atencao(c, e):
    p = ATENCAO
    painel(c, ATENCAO)
    c.camada("ELEM_ATENCAO", "#FFFFFF")
    c.texto((p[0] + 2, p[3] + 2.0), 3.0, "PONTOS CRÍTICOS, DE ATENÇÃO E ZONAS DE ATERRO/ESCAVAÇÃO")
    y = p[3] - 5.0
    ht = 2.5
    largura = p[2] - p[0] - 8.0
    if not e.el["motivos"]:
        c.camada("ELEM_ATENCAO", COR_NIVEL["NORMAL"])
        c.texto((p[0] + 3, y), 2.8, "Sem pontos críticos nem de atenção: elemento dentro dos critérios.", "em")
        y -= 6.0
    usados = set()
    for m in e.el["motivos"]:
        cor = COR_NIVEL[m["nivel"]]
        linhas = quebrar(f"{m['nivel']}: {m['texto']}", largura, ht)
        for k, ln in enumerate(linhas):
            if y < p[1] + 3:
                break
            c.camada("ELEM_ATENCAO", cor)
            c.texto((p[0] + 3 + (0 if k == 0 else 3), y), ht, ln, "em", cor=cor)
            y -= ht * 1.35
        conselho = conselhos_para(m["texto"])
        if conselho and conselho not in usados and y > p[1] + 8:
            usados.add(conselho)
            for ln in quebrar("→ " + conselho, largura - 3, ht * 0.9):
                c.camada("ELEM_ATENCAO", "#808080")
                c.texto((p[0] + 6, y), ht * 0.9, ln, "em", cor="#808080")
                y -= ht * 1.25
        y -= 1.2
    zs = e.el["zonas"]
    if zs and y > p[1] + 8:
        c.camada("ELEM_ATENCAO", "#FFFFFF")
        c.texto((p[0] + 3, y), 2.7, "ZONAS DE TERRAPLENAGEM NO ELEMENTO (levante = greide − terreno):", "em")
        y -= 4.6
        for z in zs:
            if y < p[1] + 3:
                break
            cor = COR_ATERRO if z["natureza"] == "aterro" else COR_ESCAVACAO
            c.camada("ELEM_ATENCAO", cor)
            c.texto((p[0] + 3, y), 2.4, f"{z['natureza'].capitalize()}: est. {fmt_est(z['de'])} a {fmt_est(z['ate'])} ({z['ate'] - z['de']:.0f} m), máx. {v(z['maximo_m'], 2)} m em {fmt_est(z['onde'])} — {z['nivel']}", "em", cor=cor)
            y -= 3.5


def desenhar_titulo(c, e, total):
    nivel = e.el["nivel"]
    cor = COR_NIVEL[nivel]
    c.camada("ELEM_TITULO", "#FFFFFF")
    extra = ""
    if e.tipo == "curva":
        cv = e.el["curva"]
        extra = f" — R {cv['raio']:.0f} m — Ls {cv['ls']:.0f} m — Δ {v(cv['delta_graus'], 1)}° {cv['lado']}"
    c.texto((20.0, 276.0), 5.5, f"{e.nome}{extra}")
    c.texto((20.0, 270.0), 3.0, f"Estacas {fmt_est(e.de)} a {fmt_est(e.ate)} — km {e.de / 1000.0:.3f} a {e.ate / 1000.0:.3f} — {e.el['comprimento']:.0f} m".replace(".", ","))
    c.camada("ELEM_NIVEL", cor)
    c.ret((330.0, 268.0), (405.0, 284.0))
    c.texto((367.5, 276.0), 5.0, nivel, "cm", cor=cor)


def folha_do_elemento(mcp, e, total, carimbo):
    """Insere a A3 e desenha o elemento. Devolve o `indice` da folha."""
    escala_planta = Planta(e, PLANTA).esc
    campos = dict(carimbo)
    campos["titulo"] = f"{e.nome} — {e.el['nivel']}"
    campos["subtitulo"] = f"Estacas {fmt_est(e.de)} a {fmt_est(e.ate)}"
    r = mcp.call("cad_folha", {"acao": "inserir", "papel": "A3", "escala": escala_planta, "campos": campos})
    f = Folha(r["folha"])
    c = Fct(f)
    desenhar_titulo(c, e, total)
    desenhar_planta(c, e)
    desenhar_perfil(c, e)
    desenhar_dados(c, e)
    desenhar_atencao(c, e)
    c.enviar(mcp, e.curto)
    return f.indice

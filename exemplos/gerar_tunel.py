#!/usr/bin/env python3
"""Túnel ferroviário de bitola métrica (1,00 m) — mesma linguagem da curva ferroviária: uma folha só (aqui A1, 1:2000,
1 mm de papel = 2 m no desenho) com planta, perfil com e sem a montanha, vista frontal com e sem a montanha, seção tipo,
detalhe, quadros e carimbo. Trilhos e dormentes em tamanho real. Unidade: m.

Da imagem de satélite vêm só os dois portais (500,00 m em reta). O resto (curvas de entrada e saída, cotas do terreno,
o morro) é ADOTADO para estudo — a folha diz isso."""
import math, sys
import numpy as np
from mcp_cliente import Mcp

# ---------------------------------------------------------------- folha
ESC = 2000.0                                   # denominador da escala da planta
MM = ESC / 1000.0                              # metros de desenho por mm de papel
W, H = 841 * MM, 594 * MM                      # A1 deitada
TEXTO, COTA, EST, TANG = "#E0E0E0", "#C58A2E", "#40C0FF", "#B58CFF"

# ---------------------------------------------------------------- traçado (em metros, x leste / y norte, portal SW = origem)
TH0 = math.radians(36.87)                      # rumo do túnel (medido na imagem: 305 px por 229 px)
L_TUNEL = 500.0
C1 = dict(R=300.0, D=math.radians(9.0), sentido=-1)      # SW: pequena curva à direita (rumo de chegada 45,87° -> 36,87°)
C2 = dict(R=250.0, D=math.radians(27.0), sentido=+1)     # NE: pequena curva à esquerda (36,87° -> 63,87°)
L_INI, L_ENTRE, L_FIM = 30.0, 30.0, 30.0
ELEM = [("R", L_INI), ("C", C1), ("R", L_ENTRE), ("R", L_TUNEL), ("R", L_ENTRE), ("C", C2), ("R", L_FIM)]

def _montar():
    """Lista de trechos com (k0, k1, x0, y0, rumo0, curva|None), com o portal SW em (0,0)."""
    h = TH0 + C1["D"]                          # rumo inicial: a curva à direita o traz de volta a TH0
    x = y = 0.0; k = 0.0; tr = []
    for tipo, v in ELEM:
        if tipo == "R":
            tr.append((k, k + v, x, y, h, None)); x += v * math.cos(h); y += v * math.sin(h); k += v
        else:
            comp = v["R"] * v["D"]; tr.append((k, k + comp, x, y, h, v))
            s = v["sentido"]; cx = x - s * v["R"] * math.sin(h); cy = y + s * v["R"] * math.cos(h)
            h2 = h + s * v["D"]; x = cx + s * v["R"] * math.sin(h2); y = cy - s * v["R"] * math.cos(h2); h = h2; k += comp
    return tr, k
TRECHOS, K_TOTAL = _montar()
K_A = L_INI + C1["R"] * C1["D"] + L_ENTRE          # progressiva do portal SW
K_B = K_A + L_TUNEL
_xa, _ya = None, None

def pos(k):
    """(x, y, rumo) do eixo na progressiva k, com o portal SW deslocado para (0,0)."""
    for k0, k1, x0, y0, h0, c in TRECHOS:
        if k <= k1 + 1e-9:
            d = k - k0
            if c is None:
                return x0 + d * math.cos(h0) - _xa, y0 + d * math.sin(h0) - _ya, h0
            s = c["sentido"]; R = c["R"]
            cx = x0 - s * R * math.sin(h0); cy = y0 + s * R * math.cos(h0)
            h = h0 + s * d / R
            return cx + s * R * math.sin(h) - _xa, cy - s * R * math.cos(h) - _ya, h
    return pos(K_TOTAL)
# o portal SW fica em (0,0)
_xa = 0.0; _ya = 0.0
_xa, _ya, _ = pos(K_A)

def frente(x, y):
    """Coordenadas (s, t) no eixo do túnel prolongado: s ao longo, t para a esquerda."""
    c, s = math.cos(TH0), math.sin(TH0)
    return x * c + y * s, -x * s + y * c

# ---------------------------------------------------------------- vertical
GREIDE = 0.005                                 # 0,5 % subindo para NE
Z_A = 100.0                                    # cota do boleto do trilho no portal SW
def z_trilho(k): return Z_A + GREIDE * (k - K_A)

def _pchip(xs, ys):
    xs = np.array(xs, float); ys = np.array(ys, float); h = np.diff(xs); dl = np.diff(ys) / h
    d = np.zeros(len(xs))
    for i in range(1, len(xs) - 1):
        if dl[i - 1] * dl[i] > 0:
            w1, w2 = 2 * h[i] + h[i - 1], h[i] + 2 * h[i - 1]
            d[i] = (w1 + w2) / (w1 / dl[i - 1] + w2 / dl[i])
    d[0], d[-1] = dl[0], dl[-1]
    def f(x):
        x = min(max(x, xs[0]), xs[-1]); i = min(int(np.searchsorted(xs, x, side="right")) - 1, len(h) - 1)
        t = (x - xs[i]) / h[i]; t2, t3 = t * t, t * t * t
        return (2 * t3 - 3 * t2 + 1) * ys[i] + (t3 - 2 * t2 + t) * h[i] * d[i] + (-2 * t3 + 3 * t2) * ys[i + 1] + (t3 - t2) * h[i] * d[i + 1]
    return f
BASE = 99.0
Z_EIXO = _pchip([-200, -150, -60, -25, 0, 25, 80, 160, 240, 320, 400, 455, 490, 515, 545, 600, 700, 800],
                [99.4, 99.5, 100.2, 102.0, 111.0, 123.0, 142.0, 164.0, 176.0, 169.0, 152.0, 133.0, 120.0, 111.0, 105.0, 103.0, 102.0, 101.5])
def z_terreno(s, t):
    a = Z_EIXO(s) - BASE
    return BASE + a * math.exp(-(t / (30.0 + 0.9 * max(a, 0.0))) ** 2)
def z_terreno_xy(x, y):
    s, t = frente(x, y); return z_terreno(s, t)

# ---------------------------------------------------------------- seção do túnel (t horizontal, z acima do boleto do trilho)
BITOLA = 1.0
C_TRILHO = BITOLA / 2 + 0.0373                 # centro de cada trilho
H_TRILHO, BASE_TRILHO, BOLETO, ALMA = 0.1857, 0.1524, 0.0746, 0.019
ESPESSURA_DORMENTE, LASTRO_SOB = 0.17, 0.30
Z_LASTRO = -(H_TRILHO + ESPESSURA_DORMENTE + LASTRO_SOB)   # topo do concreto do invertido: -0,656
R_INT, R_EXT, Z_CENTRO = 2.75, 3.15, 2.60      # arco: centro a 2,60 m do boleto
Z_INV_EXT = Z_LASTRO - 0.50
Z_COROA_INT, Z_COROA_EXT = Z_CENTRO + R_INT, Z_CENTRO + R_EXT
ESP_DORM = 0.625

# ---------------------------------------------------------------- saída em .fct
out = ["unidade m"]
LAYERS = {}
CUR = ""
def n(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s
def camada(nome, cor):
    LAYERS.setdefault(nome, cor); out.append(f"camada {nome} {LAYERS[nome]}")
def cor_(c): return f" {c}" if c else ""
def L(a, b, c, d, cor=None): out.append(f"L {n(a)} {n(b)} {n(c)} {n(d)}{cor_(cor)}")
def Rr(x, y, w, h, giro=0, cor=None): out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)}" + (f" {n(giro)}" if giro else "") + cor_(cor))
def C(x, y, r, cor=None): out.append(f"C {n(x)} {n(y)} {n(r)}{cor_(cor)}")
def Ar(x, y, r, a0, a1, cor=None): out.append(f"A {n(x)} {n(y)} {n(r)} {n(a0)} {n(a1)}{cor_(cor)}")
def T(x, y, h, txt, rot=0, cor=None): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{cor_(cor)}')
def P(pts, f=False, cor=None): out.append("P " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + (" f" if f else "") + cor_(cor))
def Hs(lacos, cor=None): out.append("H solida " + " | ".join(" ".join(f"{n(x)} {n(y)}" for x, y in l) for l in lacos) + cor_(cor))
def virg(v, c=2): return f"{v:,.{c}f}".replace(",", "X").replace(".", ",").replace("X", ".")
def th(mm): return mm * MM                       # altura de texto: mm de papel -> m de desenho
def gms(rad):
    g = math.degrees(rad); d = int(g); m = int((g - d) * 60); s = ((g - d) * 60 - m) * 60
    return f"{d}°{m:02d}'{s:04.1f}''"
def rot(px, py, a): c, s = math.cos(a), math.sin(a); return px * c - py * s, px * s + py * c
def arco_pts(cx, cy, r, a0, a1, passo=6.0):
    m = max(2, int(abs(a1 - a0) / passo) + 1); return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / m)), cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / m))) for i in range(m + 1)]
def cota_h(x1, x2, y, txt, alt=4.0, esq=False):
    camada("COTAS", COTA); L(x1, y, x2, y); L(x1, y - alt / 2, x1, y + alt / 2); L(x2, y - alt / 2, x2, y + alt / 2)
    T((x1 + x2) / 2 - len(txt) * alt * 0.3, y + alt * 0.5, alt, txt)
def cota_v(x, y1, y2, txt, alt=4.0):
    camada("COTAS", COTA); L(x, y1, x, y2); L(x - alt / 2, y1, x + alt / 2, y1); L(x - alt / 2, y2, x + alt / 2, y2)
    T(x + alt * 0.7, (y1 + y2) / 2 - alt / 2, alt, txt)

# ================================================================ PRANCHETA (A1 a 1:2000) — moldura do escritório
# Medidas em mm de papel (A1 deitada, 841 x 594); M() leva para metros de desenho. Margem de 25 mm à esquerda (furação),
# 10 mm nos outros lados, marcas de centro nas bordas, linha divisória e a faixa inferior: NOTAS, LEGENDA, REVISÕES e carimbo.
def M(v): return v * MM
def rr(x, y, w, h, cor=None): Rr(M(x), M(y), M(w), M(h), 0, cor)
def ll(x1, y1, x2, y2): L(M(x1), M(y1), M(x2), M(y2))
def tt(x, y, h, txt, rot_=0, cor=TEXTO): T(M(x), M(y), M(h), txt, rot_, cor)
def centrado(x, y, h, txt, cor=TEXTO): T(M(x) - len(txt) * M(h) * 0.28, M(y), M(h), txt, 0, cor)
IX0, IX1, IY0, IY1 = 25.0, 831.0, 10.0, 584.0
Y_FAIXA = 99.0                                   # topo da faixa inferior
X_NOTAS, X_LEG, X_REV = 165.3, 505.7, 656.4      # divisórias verticais da faixa
camada("FOLHA", "#FFFFFF"); rr(0, 0, 841, 594)   # a folha (única forma na camada FOLHA)
camada("CARIMBO", TEXTO)
rr(IX0, IY0, IX1 - IX0, IY1 - IY0)
for x in (210.6, 341.0, 471.4, X_REV):           # marcas de centro (zonas) em cima e embaixo
    ll(x, IY1, x, 594); ll(x, 0, x, IY0)
ll(0, 300, IX0, 300); ll(IX1, 300, 841, 300)     # e nas laterais
ll(IX0, 341.8, IX1, 341.8)                       # divisória entre a parte de cima (planta e vistas) e a de baixo (perfis e seção)
ll(IX0, Y_FAIXA, IX1, Y_FAIXA)
for x in (X_NOTAS, X_LEG, X_REV): ll(x, IY0, x, Y_FAIXA)
ll(IX0, Y_FAIXA - 6, X_REV, Y_FAIXA - 6)
centrado((IX0 + X_NOTAS) / 2, Y_FAIXA - 4.5, 2.2, "NOTAS"); centrado((X_NOTAS + X_LEG) / 2, Y_FAIXA - 4.5, 2.2, "LEGENDA")
NOTAS = ["1. Portais e distância entre eles (500,00 m) medidos em imagem de satélite.", "2. Terreno, morro, cotas e curvas do traçado ADOTADOS para estudo;", "    fazer levantamento topográfico e sondagens antes de detalhar.",
         "3. Cotas em metros, no boleto do trilho; estacas de 20 m.", "4. Curvas circulares simples, sem espirais nem superelevação.", "5. Gabarito e seção esquemáticos: conferir a norma da ferrovia.",
         "6. Trilhos e dormentes em tamanho real (dê zoom no PDF)."]
for i, t_ in enumerate(NOTAS): tt(IX0 + 2, Y_FAIXA - 12 - i * 4.4, 1.9, t_)
# revisões
rw = 656.4 - 505.7; cols = [505.7, 505.7 + 9.6, 505.7 + 98.0, 505.7 + 110.0, 505.7 + 122.0, 505.7 + 135.0, 656.4]
for i in range(1, 8): ll(X_LEG + (X_REV - X_LEG), Y_FAIXA - 6.1 * i, X_REV - rw, Y_FAIXA - 6.1 * i)
ll(X_REV - rw, 56.3, X_REV, 56.3); ll(X_REV - rw, 50.2, X_REV, 50.2); ll(X_REV - rw, 44.1, X_REV, 44.1)
for x in cols[1:-1]: ll(x, 56.3 + 6.1 * 0 + 0.0, x, Y_FAIXA)
for x in cols[1:-1]: ll(x, 50.2, x, 56.3)
for x, txt in ((cols[0] + 2, "REV"), (cols[1] + 30, "DESCRIÇÃO"), (cols[2] + 1.5, "FINAL"), (cols[3] + 2.5, "INI."), (cols[4] + 2.5, "VIS."), (cols[5] + 4, "DATA")): tt(x, 52.0, 1.7, txt)
centrado((X_REV - rw + X_REV) / 2, 45.6, 2.2, "REVISÕES")
tt(cols[0] + 2.5, Y_FAIXA - 4.6, 1.8, "00"); tt(cols[1] + 3, Y_FAIXA - 4.6, 1.8, "EMISSÃO INICIAL - ESTUDO PRELIMINAR"); tt(cols[5] + 1.5, Y_FAIXA - 4.6, 1.8, "29/09/2026")
tt(X_REV - rw + 2, 39.0, 1.9, "RESPONSÁVEL TÉCNICO / EMPRESA:"); tt(X_REV - rw + 2, 33.5, 1.9, "(a definir)")
# carimbo
tx0, tw = X_REV, 831.0 - X_REV
for y in (80.0, 72.5, 64.8, 57.1, 49.4, 41.7, 33.0): ll(tx0, y, 831.0, y)
ll(tx0 + 88.5, 80.0, tx0 + 88.5, Y_FAIXA)
rr(tx0 + 32.7, 82.0, 31.9, 15.0); rr(tx0 + 120.4, 82.0, 22.3, 15.0)
for x in (tx0 + 28.7, tx0 + 94.9, tx0 + 160.2): ll(x, IY0, x, 41.7)
tt(tx0 + 1.5, 75.0, 1.6, "PROJETO", cor=TEXTO)
tt(tx0 + 1.5, 67.2, 2.8, "A DEFINIR (NOME DO CLIENTE)"); tt(tx0 + 1.5, 59.5, 2.8, "FASE: ESTUDO PRELIMINAR"); tt(tx0 + 1.5, 51.8, 2.8, "DISCIPLINA: FERROVIA - TÚNEL")
tt(tx0 + 1.5, 46.0, 2.6, "TÚNEL FERROVIÁRIO - BITOLA MÉTRICA (1,00 m)", cor=TEXTO); tt(tx0 + 1.5, 42.6, 1.9, "PLANTA, PERFIS, VISTAS FRONTAIS E SEÇÃO TIPO")
for x, txt in ((tx0 + 1.5, "ESCALA"), (tx0 + 30.0, "Nº DO DESENHO"), (tx0 + 96.5, "FOLHA"), (tx0 + 161.5, "REV.")): tt(x, 35.0, 1.5, txt)
for x, txt in ((tx0 + 1.5, "INDICADA"), (tx0 + 30.0, "TUN-01-PL-001"), (tx0 + 96.5, "01/01"), (tx0 + 165.0, "00")): tt(x, 20.0, 3.4, txt)
# legenda (amostra de cada camada)
camada("LEGENDA", TEXTO)
def amostra(nome, cor, i, col, desenhar=None, tipo=None):
    x0 = X_NOTAS + 4 + col * 170.0; y0 = Y_FAIXA - 15 - i * 11.5
    if desenhar: desenhar(x0, y0)
    else:
        camada(nome, cor); ll(x0, y0 + 1.2, x0 + 22, y0 + 1.2)
    camada("LEGENDA", TEXTO)
LEG = [("EIXO", "#FF6060", "eixo do traçado (progressiva)"), ("TRILHOS", "#E0E0E0", "trilhos 136 RE, bitola 1,00 m"), ("TUNEL_OCULTO", "#FFD84A", "túnel (paredes ocultas)"), ("ESTACAS", EST, "estaca de 20 m (rótulo a cada 100 m)"),
       ("TANGENTES", TANG, "tangentes e dados da curva"), ("CURVAS_NIVEL", "#8A7452", "curva de nível de 5 m"), ("CURVAS_MESTRAS", "#C0A060", "curva mestra de 25 m"), ("TERRENO", "#5FD35F", "terreno natural (perfil e silhueta)"),
       ("COTAS", COTA, "cotas e dimensões"), ("GABARITO", "#40FF90", "gabarito esquemático"), ("LASTRO", "#8C8C8C", "lastro"), ("SECAO", "#FFD84A", "revestimento do túnel")]
for k_, (nome, cor, txt) in enumerate(LEG):
    col, i = divmod(k_, 6)
    amostra(nome, cor, i, col)
    tt(X_NOTAS + 4 + col * 170.0 + 26, Y_FAIXA - 15 - i * 11.5, 2.0, txt)
def dormente_amostra(x0, y0):
    camada("DORMENTES", "#FFFFFF"); Rr(M(x0), M(y0), M(6), M(2.6), 0, "#B5835A")
camada("LEGENDA", TEXTO)
dormente_amostra(X_NOTAS + 4, Y_FAIXA - 15 - 6 * 11.5 + 0)
tt(X_NOTAS + 4 + 10, Y_FAIXA - 15 - 6 * 11.5, 2.0, "dormente 2,00 x 0,24 m (c/ 0,625 m)")
# a moldura não usa mais o título grande do topo (o carimbo já traz)

# ================================================================ PLANTA
JX0, JY0, JX1, JY1 = 62.0, 705.0, 870.0, 1140.0      # janela da planta (acima da divisória)
def caixa_do_eixo(rho):
    pts = []
    k = 0.0
    while k <= K_TOTAL:
        x, y, _ = pos(k); pts.append(rot(x, y, rho)); k += 5
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)
melhor = None
for g in range(-30, 1, 2):                      # o túnel sempre subindo da esquerda para a direita
    b = caixa_do_eixo(math.radians(g)); w, h_ = b[2] - b[0] + 70, b[3] - b[1] + 70
    folga = max(w / (JX1 - JX0), h_ / (JY1 - JY0))
    if melhor is None or folga < melhor[0]: melhor = (folga, g)
RHO = math.radians(melhor[1] if melhor else 0)
bx0, by0, bx1, by1 = caixa_do_eixo(RHO)
OFX = (JX0 + JX1) / 2 - (bx0 + bx1) / 2; OFY = (JY0 + JY1) / 2 - (by0 + by1) / 2
def PL(x, y):                                  # local (leste, norte) -> folha
    a, b = rot(x, y, RHO); return a + OFX, b + OFY
def dentro(p): return JX0 <= p[0] <= JX1 and JY0 <= p[1] <= JY1
def dentro_da_janela(x, y, larg=110.0, alt=14.0):
    return min(max(x, JX0 + 4), JX1 - larg), min(max(y, JY0 + 4), JY1 - alt)
def eixo(k, d=0.0):                            # ponto do eixo (deslocado d para a esquerda), na folha
    x, y, h = pos(k); return PL(x - d * math.sin(h), y + d * math.cos(h))

T(JX0, JY1 + th(3), th(5), "PLANTA - ESCALA 1:2000", cor=TEXTO); camada("TITULO", TEXTO)
camada("GUIAS", "#7F7F7F"); Rr(JX0, JY0, JX1 - JX0, JY1 - JY0)

# curvas de nível (marcha dos quadrados sobre o morro; a mesma função do perfil e das vistas)
def curvas_de_nivel(niveis, passo=4.0, s_min=-260.0, s_max=820.0, t_min=-300.0, t_max=300.0):
    ns, nt = int((s_max - s_min) / passo) + 1, int((t_max - t_min) / passo) + 1
    Z = np.array([[z_terreno(s_min + i * passo, t_min + j * passo) for j in range(nt)] for i in range(ns)])
    cs, sn = math.cos(TH0), math.sin(TH0)
    def mundo(i, j):
        s, t = s_min + i * passo, t_min + j * passo; return PL(s * cs - t * sn, s * sn + t * cs)
    res = {}
    for nivel in niveis:
        seg = []
        for i in range(ns - 1):
            for j in range(nt - 1):
                a, b, c, d = Z[i, j], Z[i + 1, j], Z[i + 1, j + 1], Z[i, j + 1]
                if min(a, b, c, d) > nivel or max(a, b, c, d) < nivel: continue
                cruz = []
                for eid, (v1, v2, p1, p2) in (
                    (("h", i, j), (a, b, (i, j), (i + 1, j))), (("v", i + 1, j), (b, c, (i + 1, j), (i + 1, j + 1))),
                    (("h", i, j + 1), (d, c, (i, j + 1), (i + 1, j + 1))), (("v", i, j), (a, d, (i, j), (i, j + 1)))):
                    if (v1 >= nivel) != (v2 >= nivel):
                        f = (nivel - v1) / (v2 - v1); q1, q2 = mundo(*p1), mundo(*p2)
                        cruz.append((eid, (q1[0] + f * (q2[0] - q1[0]), q1[1] + f * (q2[1] - q1[1]))))
                if len(cruz) == 2: seg.append((cruz[0], cruz[1]))
                elif len(cruz) == 4:
                    centro = (a + b + c + d) / 4
                    par = ((0, 1), (2, 3)) if (centro >= nivel) == (a >= nivel) else ((0, 3), (1, 2))
                    for u, v in par: seg.append((cruz[u], cruz[v]))
        seg = [s for s in seg if dentro(s[0][1]) and dentro(s[1][1])]
        viz, pto = {}, {}
        for (e1, p1), (e2, p2) in seg:
            pto[e1], pto[e2] = p1, p2; viz.setdefault(e1, []).append(e2); viz.setdefault(e2, []).append(e1)
        visto, linhas = set(), []
        for ini in list(viz):
            if ini in visto or len(viz[ini]) > 1: continue
            cam, atual, ant = [ini], ini, None
            visto.add(ini)
            while True:
                prox = [v for v in viz[atual] if v != ant and v not in visto]
                if not prox: break
                ant, atual = atual, prox[0]; visto.add(atual); cam.append(atual)
            linhas.append([pto[e] for e in cam])
        for ini in list(viz):                    # laços fechados
            if ini in visto: continue
            cam, atual, ant = [ini], ini, None
            visto.add(ini)
            while True:
                prox = [v for v in viz[atual] if v != ant and v not in visto]
                if not prox: break
                ant, atual = atual, prox[0]; visto.add(atual); cam.append(atual)
            linhas.append([pto[e] for e in cam] + [pto[cam[0]]])
        res[nivel] = linhas
    return res
NIVEIS = list(range(105, 180, 5))
for nivel, linhas in curvas_de_nivel(NIVEIS).items():
    mestra = nivel % 25 == 0
    camada("CURVAS_MESTRAS" if mestra else "CURVAS_NIVEL", "#C0A060" if mestra else "#8A7452")
    for pts in linhas:
        if len(pts) >= 2: P(pts)
    if mestra and linhas:
        pts = max(linhas, key=len); q = pts[len(pts) // 2]
        T(q[0] + 2, q[1] + 2, th(2.2), str(nivel), cor="#C0A060")

# via permanente: dormentes reais, trilhos, eixo, lastro
camada("DORMENTES", "#FFFFFF")
k = 0.0
while k <= K_TOTAL - 0.2:
    x, y, h = pos(k); cx, cy = PL(x, y); ang = h + RHO
    ox, oy = rot(-0.12, -1.0, ang)
    Rr(cx + ox, cy + oy, 0.24, 2.0, math.degrees(ang), "#B5835A"); k += ESP_DORM
camada("TRILHOS", "#E0E0E0")
for lado in (1, -1):
    for off in (BITOLA / 2, BITOLA / 2 + BOLETO, C_TRILHO - BASE_TRILHO / 2, C_TRILHO + BASE_TRILHO / 2):
        pts, kk = [], 0.0
        while kk < K_TOTAL: pts.append(eixo(kk, lado * off)); kk += 2.0
        pts.append(eixo(K_TOTAL, lado * off)); P(pts)
camada("EIXO", "#FF6060")
P([eixo(kk) for kk in np.arange(0, K_TOTAL, 2.0)] + [eixo(K_TOTAL)])
camada("LASTRO", "#8C8C8C")
for lado in (1, -1):
    pts = [eixo(kk, lado * 1.75) for kk in np.arange(0, K_A - 2, 2.0)] + [eixo(K_A - 2, lado * 1.75)]; P(pts)
    pts = [eixo(kk, lado * 1.75) for kk in np.arange(K_B + 2, K_TOTAL, 2.0)] + [eixo(K_TOTAL, lado * 1.75)]; P(pts)

# túnel em linha oculta (paredes internas e externas do revestimento) e portais
camada("TUNEL_OCULTO", "#FFD84A")
for lado in (1, -1):
    for d in (R_INT, R_EXT):
        a, b = eixo(K_A, lado * d), eixo(K_B, lado * d); L(*a, *b)
for kk in (K_A, K_B):
    a, b = eixo(kk, R_EXT + 6), eixo(kk, -(R_EXT + 6)); L(*a, *b)
camada("TEXTOS", TEXTO)
pa, pb = eixo(K_A, R_EXT + 26), eixo(K_B, R_EXT + 26)
ang_tun = math.degrees(TH0 + RHO)
cota_a, cota_b = eixo(K_A, 20), eixo(K_B, 20)
camada("COTAS", COTA); L(*cota_a, *cota_b)
for kk in (K_A, K_B):
    a, b = eixo(kk, 14), eixo(kk, 26); L(*a, *b)
mid = eixo((K_A + K_B) / 2, 20)
ox, oy = rot(-46, 3, math.radians(ang_tun)); T(mid[0] + ox, mid[1] + oy, th(3.2), "TÚNEL  L = 500,00 m", ang_tun)

# estacas de 20 m (rótulo a cada 100 m) e pontos notáveis
for i in range(0, int(K_TOTAL // 20) + 1):
    kk = i * 20.0
    if kk > K_TOTAL: break
    grande = (i % 5 == 0)
    a, b = eixo(kk, -(7 if grande else 4)), eixo(kk, 7 if grande else 4)
    camada("ESTACAS", EST); L(*a, *b)
    if grande:
        ox, oy = rot(-6, -18, math.radians(ang_tun)); q = eixo(kk, -7)
        T(q[0] + ox * 0 - 4, q[1] - 12, th(2.4), f"E {i}", cor=EST)
K_PC1, K_PT1 = L_INI, L_INI + C1["R"] * C1["D"]
K_PC2, K_PT2 = K_B + L_ENTRE, K_B + L_ENTRE + C2["R"] * C2["D"]
NOTAVEIS = [("INÍCIO", 0.0), ("PC1", K_PC1), ("PT1", K_PT1), ("PORTAL 1 (SW)", K_A), ("PORTAL 2 (NE)", K_B), ("PC2", K_PC2), ("PT2", K_PT2), ("FIM", K_TOTAL)]
LADO_DIST = {"INÍCIO": (1, 26), "PC1": (-1, 30), "PT1": (1, 44), "PORTAL 1 (SW)": (-1, 66), "PORTAL 2 (NE)": (1, 40), "PC2": (-1, 36), "PT2": (1, 40), "FIM": (-1, 26)}
camada("PONTOS", "#FFFFFF")
for nome, kk in NOTAVEIS:
    q = eixo(kk); C(q[0], q[1], 3.5)
    lado, dist = LADO_DIST[nome]
    p = eixo(kk, lado * dist)
    tx, ty = dentro_da_janela(p[0] - 20, p[1] + (2 if lado > 0 else -9), 90.0, 24.0)
    camada("TEXTOS", TEXTO); L(q[0], q[1], tx + 20, ty + 4); T(tx, ty, th(2.4), nome); T(tx, ty - th(2.8), th(2.0), f"km 0+{kk:06.2f}".replace(".", ","))
# tangentes (prolongadas até o PI) e dados das curvas
camada("TANGENTES", TANG)
for c, kpc, kpt in ((C1, K_PC1, K_PT1), (C2, K_PC2, K_PT2)):
    xa, ya, ha = pos(kpc); xb, yb, hb = pos(kpt)
    Tg = c["R"] * math.tan(c["D"] / 2); pi = (xa + Tg * math.cos(ha), ya + Tg * math.sin(ha))
    L(*PL(xa, ya), *PL(*pi)); L(*PL(xb, yb), *PL(*pi)); C(*PL(*pi), 3.0, TANG)
    ang_l = math.degrees(ha + RHO)
    lado_c = 1 if c is C1 else -1
    m = eixo((kpc + kpt) / 2, lado_c * 80)
    camada("ANGULOS", TANG)
    q = eixo((kpc + kpt) / 2); L(q[0], q[1], m[0], m[1], TANG)
    mx, my = dentro_da_janela(m[0] - 30, m[1] + (2 if lado_c > 0 else -9), 130.0, 16.0)
    T(mx, my, th(2.4), f"R = {virg(c['R'], 0)} m  Δ = {gms(c['D'])}", cor=TANG)
# norte e barra de escala
na = (JX0 + 60, JY1 - 60); camada("GUIAS", "#FFFFFF")
tip = (na[0] + 48 * math.cos(RHO + math.pi / 2), na[1] + 48 * math.sin(RHO + math.pi / 2)); L(*na, *tip)
for da in (150, -150):
    e = (tip[0] + 14 * math.cos(RHO + math.pi / 2 + math.radians(da)), tip[1] + 14 * math.sin(RHO + math.pi / 2 + math.radians(da))); L(*tip, *e)
camada("TEXTOS", TEXTO); T(tip[0] + 8 * math.cos(RHO + math.pi / 2) - 4, tip[1] + 8 * math.sin(RHO + math.pi / 2) - 4, th(4), "N")
bx, by = JX1 - 250, JY0 + 22
camada("GUIAS", "#FFFFFF")
for i in range(4):
    Rr(bx + i * 50, by, 50, 5)
    if i % 2 == 0: Hs([[(bx + i * 50, by), (bx + i * 50 + 50, by), (bx + i * 50 + 50, by + 5), (bx + i * 50, by + 5)]])
camada("TEXTOS", TEXTO)
for i, t_ in enumerate(("0", "50", "100", "150", "200 m")): T(bx + i * 50 - 3, by - 9, th(2.4), t_)
T(bx, by + 12, th(2.6), "ESCALA GRÁFICA 1:2000")
T(JX1 - 330, JY0 + 62, th(2.2), "Curvas de nível de 5 em 5 m (mestras de 25 em 25 m) - morro adotado para estudo.")
T(JX1 - 330, JY0 + 55, th(2.2), "Trilhos e dormentes em tamanho real: dê zoom para ver o detalhe.")

# ================================================================ PERFIL LONGITUDINAL — COM A MONTANHA
KP0, KP1 = 10.0, 780.0
PXO, PXE = 92.0, 1.0                            # x do perfil = PXO + (k - KP0)
def perfil_com():
    YB, ZB, E = 470.0, 92.0, 2.0
    def X(k): return PXO + (k - KP0)
    def Y(z): return YB + (z - ZB) * E
    camada("TITULO", TEXTO)
    T(PXO, 663, th(5), "PERFIL LONGITUDINAL - COM A MONTANHA", cor=TEXTO)
    T(PXO, 655, th(2.6), "ESCALA H 1:2000 - V 1:1000 (exagero vertical 2x)", cor=TEXTO)
    ks = np.arange(KP0, KP1 + 1e-6, 5.0)
    terr = [(X(k), Y(z_terreno_xy(*pos(k)[:2]))) for k in ks]
    cob = [(k, z_terreno_xy(*pos(k)[:2]) - z_trilho(k)) for k in ks]
    # massa do morro com o furo do túnel
    fundo = [(X(KP1), Y(ZB)), (X(KP0), Y(ZB))]
    tubo = [(X(K_A), Y(z_trilho(K_A) + Z_INV_EXT)), (X(K_B), Y(z_trilho(K_B) + Z_INV_EXT)), (X(K_B), Y(z_trilho(K_B) + Z_COROA_EXT)), (X(K_A), Y(z_trilho(K_A) + Z_COROA_EXT))]
    camada("ROCHA", "#5A4B3A"); Hs([terr + fundo, tubo])
    camada("TERRENO", "#5FD35F"); P(terr)
    # túnel: revestimento, vazio e greide
    camada("TUNEL", "#FFD84A")
    P(tubo, True)
    for zi in (Z_COROA_INT, Z_LASTRO):
        L(X(K_A), Y(z_trilho(K_A) + zi), X(K_B), Y(z_trilho(K_B) + zi))
    camada("TRILHOS", "#E0E0E0"); P([(X(k), Y(z_trilho(k))) for k in (KP0, K_A, K_B, KP1)])
    # eixo de cotas e malha de progressivas
    camada("COTAS", COTA)
    L(PXO, Y(ZB), X(KP1), Y(ZB))
    for z in range(100, 181, 10):
        L(PXO - 3, Y(z), PXO, Y(z)); T(PXO - 24, Y(z) - 2, th(2.2), f"{z}")
    L(PXO, Y(ZB), PXO, Y(182))
    T(PXO - 20, Y(182), th(2.2), "(m)")
    camada("ESTACAS", EST)
    for kk in range(100, 780, 100):
        L(X(kk), Y(ZB), X(kk), Y(ZB) - 4)
    # portais e cobertura máxima
    camada("PONTOS", "#FFFFFF")
    for kk, nome in ((K_A, "PORTAL 1"), (K_B, "PORTAL 2")):
        L(X(kk), Y(z_trilho(kk) + Z_COROA_EXT) + 4, X(kk), Y(z_trilho(kk) + Z_COROA_EXT) + 30)
        T(X(kk) - 16, Y(z_trilho(kk) + Z_COROA_EXT) + 34, th(2.4), nome, cor=TEXTO)
    kmax, cmax = max(cob, key=lambda a: a[1])
    camada("COTAS", COTA)
    zt = z_terreno_xy(*pos(kmax)[:2]); zc = z_trilho(kmax) + Z_COROA_EXT
    cota_v(X(kmax), Y(zc), Y(zt), f"cobertura máx. {virg(zt - zc, 1)} m")
    # quadro sob o perfil: progressiva, terreno, greide e alinhamento
    Rr(PXO, YB - 46, X(KP1) - PXO, 38)
    for y_ in (YB - 22, YB - 34): L(PXO, y_, X(KP1), y_)
    camada("TABELAS", TEXTO)
    T(PXO - 24, YB - 16, th(2.0), "km"); T(PXO - 24, YB - 28, th(2.0), "Terreno"); T(PXO - 24, YB - 40, th(2.0), "Greide")
    for kk in range(100, 780, 100):
        x_ = X(kk); L(x_, YB - 46, x_, YB - 8)
        T(x_ + 2, YB - 16, th(2.0), f"0+{kk:03d}"); T(x_ + 2, YB - 28, th(2.0), virg(z_terreno_xy(*pos(kk)[:2]), 2)); T(x_ + 2, YB - 40, th(2.0), virg(z_trilho(kk), 3))
    return
perfil_com()

# ================================================================ PERFIL — SEM A MONTANHA (túnel e via; exagero vertical 10x)
def perfil_sem():
    YB, ZB, E = 278.0, 96.0, 5.0
    def X(k): return PXO + (k - KP0)
    def Y(z): return YB + (z - ZB) * E
    camada("TITULO", TEXTO)
    T(PXO, 406, th(5), "PERFIL LONGITUDINAL - SEM A MONTANHA", cor=TEXTO)
    T(PXO, 398, th(2.6), "ESCALA H 1:2000 - V 1:400 (exagero vertical 5x); só o túnel e a via", cor=TEXTO)
    ext = [(X(K_A), Y(z_trilho(K_A) + Z_COROA_EXT)), (X(K_B), Y(z_trilho(K_B) + Z_COROA_EXT))]
    ext_b = [(X(K_B), Y(z_trilho(K_B) + Z_INV_EXT)), (X(K_A), Y(z_trilho(K_A) + Z_INV_EXT))]
    intr = [(X(K_A), Y(z_trilho(K_A) + Z_COROA_INT)), (X(K_B), Y(z_trilho(K_B) + Z_COROA_INT))]
    intr_b = [(X(K_B), Y(z_trilho(K_B) + Z_LASTRO)), (X(K_A), Y(z_trilho(K_A) + Z_LASTRO))]
    camada("CONCRETO", "#808080"); Hs([ext + ext_b, intr + intr_b])
    camada("TUNEL", "#FFD84A"); P(ext + ext_b, True); P(intr + intr_b, True)
    camada("LASTRO", "#8C8C8C"); P([(X(k), Y(z_trilho(k) - H_TRILHO - ESPESSURA_DORMENTE - LASTRO_SOB)) for k in (KP0, K_A)]); P([(X(k), Y(z_trilho(k) - H_TRILHO - ESPESSURA_DORMENTE - LASTRO_SOB)) for k in (K_B, KP1)])
    camada("TRILHOS", "#E0E0E0"); P([(X(k), Y(z_trilho(k))) for k in (KP0, K_A, K_B, KP1)])
    camada("EIXO", "#FF6060"); L(X(K_A), Y(z_trilho(K_A) + 2.6), X(K_B), Y(z_trilho(K_B) + 2.6))
    camada("COTAS", COTA)
    L(PXO, Y(ZB), X(KP1), Y(ZB))
    for z in (98, 100, 102, 104, 106, 108, 110, 112): L(PXO - 3, Y(z), PXO, Y(z)); T(PXO - 24, Y(z) - 2, th(2.2), f"{z}")
    L(PXO, Y(ZB), PXO, Y(114)); T(PXO - 20, Y(115), th(2.2), "(m)")
    cota_h(X(K_A), X(K_B), Y(z_trilho(K_B) + Z_COROA_EXT) + 34, "L = 500,00 m")
    camada("PONTOS", "#FFFFFF")
    for kk, z_, txt in ((K_A, z_trilho(K_A), "PORTAL 1 - trilho em 100,000"), (K_B, z_trilho(K_B), "PORTAL 2 - trilho em 102,500")):
        L(X(kk), Y(z_ + Z_COROA_EXT) + 3, X(kk), Y(z_ + Z_COROA_EXT) + 26)
    T(X(K_A) - 60, Y(z_trilho(K_A) + Z_COROA_EXT) + 42, th(2.4), "PORTAL 1 (SW) - trilho em 100,000", cor=TEXTO)
    T(X(K_B) - 30, Y(z_trilho(K_B) + Z_COROA_EXT) + 42, th(2.4), "PORTAL 2 (NE) - trilho em 102,500", cor=TEXTO)
    camada("NOTAS", TEXTO)
    T(X(K_A) + 120, YB + 10, th(2.6), "rampa contínua de 0,50 % (5 mm/m) subindo para NE, com drenagem para o portal SW")
    camada("ESTACAS", EST)
    for kk in range(100, 780, 100): L(X(kk), Y(ZB), X(kk), Y(ZB) - 4); T(X(kk) - 10, Y(ZB) - 11, th(2.2), f"km 0+{kk:03d}", cor=EST)
    # faixa do alinhamento em planta
    yf = YB - 44
    camada("TABELAS", TEXTO); Rr(PXO, yf - 16, X(KP1) - PXO, 30); T(PXO - 24, yf - 6, th(2.0), "Planta")
    camada("EIXO", "#FF6060"); L(X(KP0), yf, X(KP1), yf)
    camada("TANGENTES", TANG)
    for c, k0, k1 in ((C1, K_PC1, K_PT1), (C2, K_PC2, K_PT2)):
        s = c["sentido"]; pts = [(X(k0 + (k1 - k0) * i / 12), yf + s * 7 * math.sin(math.pi * i / 12)) for i in range(13)]; P(pts)
        T(X(k0), yf + (7 if s > 0 else -13), th(2.0), f"Curva {'à esquerda' if s > 0 else 'à direita'} R={virg(c['R'], 0)} m", cor=TANG)
perfil_sem()

# ================================================================ SEÇÃO TIPO (usada na seção e nas vistas frontais)
def secao(ox, oy, k, cheio_dentro=None, lastro=True, rotulos=False):
    """Seção do túnel com o boleto do trilho em (ox, oy); k = fator de ampliação (m de desenho por m real)."""
    X = lambda t: ox + t * k
    Y = lambda z: oy + z * k
    camada("SECAO", "#FFD84A")
    for r in (R_INT, R_EXT):
        Ar(X(0), Y(Z_CENTRO), r * k, 0, 180)
        L(X(-r), Y(Z_CENTRO), X(-r), Y(Z_INV_EXT if r == R_EXT else Z_LASTRO)); L(X(r), Y(Z_CENTRO), X(r), Y(Z_INV_EXT if r == R_EXT else Z_LASTRO))
    L(X(-R_EXT), Y(Z_INV_EXT), X(R_EXT), Y(Z_INV_EXT)); L(X(-R_INT), Y(Z_LASTRO), X(R_INT), Y(Z_LASTRO))
    camada("LASTRO", "#8C8C8C")
    if lastro:
        P([(X(-R_INT), Y(Z_LASTRO)), (X(R_INT), Y(Z_LASTRO)), (X(R_INT), Y(-H_TRILHO)), (X(1.0), Y(-H_TRILHO)), (X(1.0), Y(-H_TRILHO - ESPESSURA_DORMENTE)), (X(-1.0), Y(-H_TRILHO - ESPESSURA_DORMENTE)), (X(-1.0), Y(-H_TRILHO)), (X(-R_INT), Y(-H_TRILHO))], True)
    camada("DORMENTES", "#FFFFFF"); Rr(X(-1.0), Y(-H_TRILHO - ESPESSURA_DORMENTE), 2.0 * k, ESPESSURA_DORMENTE * k, 0, "#B5835A")
    camada("TRILHOS", "#E0E0E0")
    for lado in (1, -1):
        c = lado * C_TRILHO
        P([(X(c - BOLETO / 2), Y(0)), (X(c + BOLETO / 2), Y(0)), (X(c + BOLETO / 2), Y(-0.037)), (X(c + ALMA / 2), Y(-0.06)), (X(c + ALMA / 2), Y(-H_TRILHO + 0.026)), (X(c + BASE_TRILHO / 2), Y(-H_TRILHO + 0.026)),
           (X(c + BASE_TRILHO / 2), Y(-H_TRILHO)), (X(c - BASE_TRILHO / 2), Y(-H_TRILHO)), (X(c - BASE_TRILHO / 2), Y(-H_TRILHO + 0.026)), (X(c - ALMA / 2), Y(-H_TRILHO + 0.026)), (X(c - ALMA / 2), Y(-0.06)), (X(c - BOLETO / 2), Y(-0.037))], True)
    if cheio_dentro:
        pts = arco_pts(X(0), Y(Z_CENTRO), R_INT * k, 0, 180, 5) + [(X(-R_INT), Y(Z_LASTRO)), (X(R_INT), Y(Z_LASTRO))]
        camada("VAZIO", cheio_dentro); Hs([pts]); return
    camada("EIXO", "#FF6060"); L(X(0), Y(Z_LASTRO - 0.3), X(0), Y(Z_COROA_EXT + 0.4))

# ---- seção tipo (1:100 -> k = 20)
SX, SY, SK = 980.0, 460.0, 20.0
camada("TITULO", TEXTO); T(SX - 190, 640, th(4.2), "SEÇÃO TRANSVERSAL TIPO (1:100)", cor=TEXTO)
T(SX - 190, 633, th(2.4), "túnel de via única - bitola 1,00 m - trilho 136 RE", cor=TEXTO)
secao(SX, SY, SK)
# gabarito esquemático
camada("GABARITO", "#40FF90")
gab = [(-1.5, 0.05), (-1.5, 3.6), (-0.9, 4.5), (0.9, 4.5), (1.5, 3.6), (1.5, 0.05)]
P([(SX + t * SK, SY + z * SK) for t, z in gab], True)
camada("COTAS", COTA)
cota_h(SX - R_EXT * SK, SX + R_EXT * SK, SY + Z_COROA_EXT * SK + 22, "6,30")
cota_h(SX - R_INT * SK, SX + R_INT * SK, SY + Z_COROA_EXT * SK + 10, "5,50")
cota_v(SX + R_EXT * SK + 14, SY + Z_INV_EXT * SK, SY + Z_COROA_EXT * SK, "6,91")
cota_v(SX - R_EXT * SK - 30, SY + Z_LASTRO * SK, SY + Z_COROA_INT * SK, "6,01")
camada("NOTAS", TEXTO)
T(SX + R_EXT * SK + 24, SY + 2.6 * SK, th(2.2), "revestimento de concreto, e = 0,40 m")
T(SX + R_EXT * SK + 24, SY + 0.5 * SK, th(2.2), "lastro de 0,30 m sob o dormente")
T(SX + R_EXT * SK + 24, SY + 4.6 * SK, th(2.2), "gabarito esquemático (verde)", cor="#40FF90")
T(SX - 60, SY + Z_INV_EXT * SK - 26, th(2.2), "invertido em concreto, e = 0,50 m")

# ---- detalhe A: planta ampliada (1:100 -> k = 20)
DX, DY = 1230.0, 460.0
camada("TITULO", TEXTO); T(DX - 20, 640, th(4.2), "DETALHE A - PLANTA AMPLIADA (1:100)", cor=TEXTO)
T(DX - 20, 633, th(2.4), "trilhos 136 RE (base 152,4 / boleto 74,6), dormentes 2,00 x 0,24 m", cor=TEXTO)
kk = 20.0
camada("DORMENTES", "#FFFFFF")
for i in range(8): Rr(DX + i * ESP_DORM * kk - 0.12 * kk, DY - 1.0 * kk, 0.24 * kk, 2.0 * kk, 0, "#B5835A")
camada("TRILHOS", "#E0E0E0")
for lado in (1, -1):
    for off in (BITOLA / 2, BITOLA / 2 + BOLETO, C_TRILHO - BASE_TRILHO / 2, C_TRILHO + BASE_TRILHO / 2):
        L(DX - 20, DY + lado * off * kk, DX + 8 * ESP_DORM * kk, DY + lado * off * kk)
camada("EIXO", "#FF6060"); L(DX - 26, DY, DX + 8 * ESP_DORM * kk + 6, DY)
camada("COTAS", COTA); cota_v(DX + 8 * ESP_DORM * kk + 20, DY - 0.5 * kk, DY + 0.5 * kk, "bitola 1000"); cota_v(DX - 40, DY - 1.0 * kk, DY + 1.0 * kk, "2000")
cota_h(DX + 0.0, DX + ESP_DORM * kk, DY - 1.0 * kk - 14, "625 (espaçamento)")

# ================================================================ VISTAS FRONTAIS
FK = 2.0                                        # 1:1000
FCX, FY0, ZF = 1085.0, 735.0, 98.0
def FX(t): return FCX + t * FK
def FY(z): return FY0 + (z - ZF) * FK
def vista_com():
    camada("TITULO", TEXTO); T(FCX - 185, 930, th(4.2), "VISTA FRONTAL DO PORTAL 1 - COM A MONTANHA (1:1000)", cor=TEXTO)
    T(FCX - 185, 922, th(2.4), "olhando de SW para NE; silhueta do morro sobre o túnel", cor=TEXTO)
    ts = [t for t in np.arange(-45.0, 45.01, 1.5)]
    sil = [max(z_terreno(s, t) for s in np.arange(0.0, 525.0, 5.0)) for t in ts]
    pts = [(FX(t), FY(z)) for t, z in zip(ts, sil)]
    camada("ROCHA", "#5A4B3A"); Hs([pts + [(FX(45), FY(ZF)), (FX(-45), FY(ZF))]])
    camada("TERRENO", "#5FD35F"); P(pts)
    camada("CURVAS_NIVEL", "#8A7452")
    for z in range(100, 176, 10):
        run = []
        for t, s_ in zip(ts, sil):
            if s_ >= z: run.append((FX(t), FY(z)))
            elif run:
                if len(run) > 1: P([run[0], run[-1]])
                run = []
        if len(run) > 1: P([run[0], run[-1]])
    camada("COTAS", COTA)
    for z in range(100, 181, 10):
        L(FX(-45) - 6, FY(z), FX(-45), FY(z)); T(FX(-45) - 30, FY(z) - 2, th(2.2), f"{z}")
    L(FX(-45), FY(ZF), FX(-45), FY(180))
    zc = Z_A + 0                                # boleto no portal
    # o portal: solo em volta, vazio escuro e seção no boleto = (FCX, FY(Z_A))
    camada("VAZIO", "#101010")
    secao(FCX, FY(Z_A), FK, cheio_dentro="#101010")
    secao(FCX, FY(Z_A), FK)
    camada("TERRENO", "#5FD35F"); L(FX(-45), FY(Z_A + Z_LASTRO), FX(-R_EXT), FY(Z_A + Z_LASTRO)); L(FX(R_EXT), FY(Z_A + Z_LASTRO), FX(45), FY(Z_A + Z_LASTRO))
    camada("COTAS", COTA); T(FX(8), FY(Z_A + 9), th(2.4), "PORTAL 1 - trilho na cota 100,000")
    L(FX(6), FY(Z_A + 8.4), FX(R_EXT * 0.8), FY(Z_A + Z_COROA_EXT * 0.9))
    cota_h(FX(-45), FX(45), FY(ZF) - 14, "90 m (largura mostrada)")
vista_com()

FK2 = 8.0                                       # 1:250
GCX, GY0 = 1490.0, 745.0
def vista_sem():
    def X(t): return GCX + t * FK2
    def Y(z): return GY0 + z * FK2
    camada("TITULO", TEXTO); T(GCX - 175, 930, th(4.2), "VISTA FRONTAL DO PORTAL 1 - SEM A MONTANHA (1:250)", cor=TEXTO)
    T(GCX - 175, 922, th(2.4), "estrutura do portal: testa, alas e boca do túnel", cor=TEXTO)
    HT = 8.5
    camada("CONCRETO", "#808080")
    Hs([[(X(-5.6), Y(Z_INV_EXT)), (X(5.6), Y(Z_INV_EXT)), (X(5.6), Y(HT)), (X(-5.6), Y(HT))], arco_pts(X(0), Y(Z_CENTRO), R_INT * FK2, 0, 180, 5) + [(X(-R_INT), Y(Z_LASTRO)), (X(R_INT), Y(Z_LASTRO))]])
    camada("PORTAL", "#FFD84A")
    P([(X(-5.6), Y(Z_INV_EXT)), (X(-5.6), Y(HT)), (X(5.6), Y(HT)), (X(5.6), Y(Z_INV_EXT))])
    for lado in (1, -1):
        P([(X(lado * 5.6), Y(HT)), (X(lado * 12.5), Y(2.4)), (X(lado * 12.5), Y(Z_INV_EXT)), (X(lado * 5.6), Y(Z_INV_EXT))])
    L(X(-13.5), Y(Z_INV_EXT), X(13.5), Y(Z_INV_EXT))
    secao(GCX, GY0, FK2, cheio_dentro="#101010")
    secao(GCX, GY0, FK2)
    camada("COTAS", COTA)
    cota_h(X(-5.6), X(5.6), Y(HT) + 14, "11,20"); cota_h(X(-12.5), X(12.5), Y(HT) + 28, "25,00 (com as alas)")
    cota_v(X(12.5) + 14, Y(Z_INV_EXT), Y(HT), "9,66")
    camada("NOTAS", TEXTO); T(X(-12.5), Y(Z_INV_EXT) - 16, th(2.2), "testa em concreto, alas em leque; interior escuro = túnel")
vista_sem()

# ================================================================ QUADROS E NOTAS
camada("TABELAS", TEXTO)
QX, QY = 900.0, 1150.0
T(QX, QY, th(4.2), "ELEMENTOS DO TRAÇADO", cor=TEXTO)
elem = [
    "Bitola 1000 mm (métrica) - trilho 136 RE (base 152,4 / boleto 74,6)",
    "Dormentes 2,00 x 0,24 x 0,17 m, espaçados de 0,625 m (1600/km)",
    "Túnel de via única, L = 500,00 m em reta (medida na imagem)",
    f"Curva 1 (SW, à direita): R = 300 m, Δ = {gms(C1['D'])}, Lc = {virg(C1['R'] * C1['D'])} m, T = {virg(C1['R'] * math.tan(C1['D'] / 2))} m",
    f"Curva 2 (NE, à esquerda): R = 250 m, Δ = {gms(C2['D'])}, Lc = {virg(C2['R'] * C2['D'])} m, T = {virg(C2['R'] * math.tan(C2['D'] / 2))} m",
    "Curvas circulares simples, sem espirais nem superelevação (estudo prévio)",
    "Greide: rampa única de 0,50 % subindo de SW para NE",
    "Cota do boleto: portal 1 = 100,000 m; portal 2 = 102,500 m",
    "Revestimento e = 0,40 m; invertido e = 0,50 m; vão interno 5,50 m",
    "Seção adotada para estudo: conferir gabarito com a norma da ferrovia",
]
for i, t_ in enumerate(elem): T(QX, QY - 12 - i * 9, th(2.6), t_)
T(1290 - 30, QY, th(4.2), "QUADRO DE PROGRESSIVAS", cor=TEXTO)
for i, (nome, kk) in enumerate(NOTAVEIS):
    T(1260, QY - 12 - i * 9, th(2.6), f"{nome}"); T(1450, QY - 12 - i * 9, th(2.6), f"km 0+{kk:06.2f}".replace(".", ",")); T(1570, QY - 12 - i * 9, th(2.6), f"cota trilho {virg(z_trilho(kk), 3)}")
camada("NOTAS", TEXTO)
T(1260, QY - 12 - 8 * 9 - 4, th(2.4), f"Extensão total do traçado: {virg(K_TOTAL)} m (E = estaca de 20 m).")
T(1260, QY - 12 - 8 * 9 - 12, th(2.4), "Portais e distância: imagem de satélite (500,00 m).")
T(1260, QY - 12 - 8 * 9 - 20, th(2.4), "Terreno, morro, cotas e curvas: ADOTADOS - fazer levantamento e sondagem.")

# ================================================================ montar
BASE_TEXTO = "\n".join(out) + "\n"
ESTILOS = {"EIXO": "eixo", "TUNEL_OCULTO": "oculta", "TANGENTES": "tracejada", "CURVAS_NIVEL": "continua", "GABARITO": "tracejada", "GUIAS": "continua"}
ESPESSURAS = {"TRILHOS": 0.25, "TUNEL": 0.35, "TERRENO": 0.35, "CURVAS_MESTRAS": 0.25, "SECAO": 0.35, "PORTAL": 0.35, "FOLHA": 0.5}

def montar(fcdz, dxf):
    m = Mcp()
    m.call("cad_novo", {"unidade": "m"})
    m.call("cad_importar", {"texto": BASE_TEXTO})
    for nome, tipo in ESTILOS.items():
        if tipo != "continua": m.call("cad_camadas", {"acao": "propriedade", "nome": nome, "tipo_linha": tipo})
    for nome, e in ESPESSURAS.items(): m.call("cad_camadas", {"acao": "propriedade", "nome": nome, "espessura": e})
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1] if len(sys.argv) > 1 else "/home/maragon/Documentos/FeatherCAD/tunel_ferroviario_bitola_metrica"
    open(base + ".fct", "w").write(BASE_TEXTO)
    r = montar(base + ".fcdz", base + ".dxf")
    print(r["total_de_entidades"], r["por_tipo"])
    print("rotação da planta:", math.degrees(RHO), "°; portal A em k =", K_A, "; total", K_TOTAL)

#!/usr/bin/env python3
"""Casa de teste (referencia do usuario): planta baixa e diagrama unifilar NBR 5444, em 2 folhas A3 a 1:75 (cm)."""
import math, sys
from mcp_cliente import Mcp

ESC = 50
W, H = 2100, 1485            # A3 a 1:50, em cm
K = 1.0
PASSO = 2300
X0, Y0 = 340, 430            # origem da planta dentro da folha
DATA = "29/09/2026"

# ------------------------------------------------------------------ base em .fct
out = ["unidade cm"]
LAYERS, CUR = {}, ""
def n(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s
def camada(nome, cor):
    global CUR
    LAYERS.setdefault(nome, cor)
    out.append(f"camada {nome} {LAYERS[nome]}")
    CUR = "" if cor == LAYERS[nome] else " " + cor
def L(a, b, c, d): out.append(f"L {n(a)} {n(b)} {n(c)} {n(d)}{CUR}")
def R(x, y, w, h): out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)}{CUR}")
def C(x, y, r): out.append(f"C {n(x)} {n(y)} {n(r)}{CUR}")
def A(x, y, r, a1, a2): out.append(f"A {n(x)} {n(y)} {n(r)} {n(a1)} {n(a2)}{CUR}")
def T(x, y, h, txt, rot=0): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{CUR}')
def virgula(v, c=1): return f"{v:.{c}f}".replace(".", ",")
def milhar(v): return f"{v:,}".replace(",", ".")

# ------------------------------------------------------------------ geometria da casa (cm, origem no canto inferior esquerdo)
def segmentos(a, b, aberturas):
    """Trechos de [a,b] que sobram depois de tirar as aberturas."""
    pts, cur = [], a
    for x1, x2 in sorted(aberturas):
        if x1 > cur: pts.append((cur, x1))
        cur = max(cur, x2)
    if cur < b: pts.append((cur, b))
    return pts

def parede_h(X, Y, y, x1, x2, esp, aberturas=()):
    for a, b in segmentos(x1, x2, aberturas): R(X + a, Y + y - esp / 2, b - a, esp)
def parede_v(X, Y, x, y1, y2, esp, aberturas=()):
    for a, b in segmentos(y1, y2, aberturas): R(X + x - esp / 2, Y + a, esp, b - a)

JANELAS_H = [(15, (100, 240)), (730 - 7.5, (100, 250)), (730 - 7.5, (400, 480)), (730 - 7.5, (620, 800)), (730 - 7.5, (940, 1030)), (7.5, (380, 600)), (7.5, (760, 960))]
# (y do eixo, (x1,x2)); as duas primeiras são norte da casa/sul da sala
JANELAS_V = [(7.5, (120, 250)), (7.5, (480, 610)), (1062.5, (60, 140)), (1062.5, (480, 600))]
def casa(X, Y, nomes=True):
    camada("PAREDES", "#9a9a9a")
    # externas (15)
    parede_v(X, Y, 7.5, 56, 730, 15, [(256, 346), (120, 250), (480, 610)])
    parede_h(X, Y, 722.5, 0, 1070, 15, [(100, 250), (400, 480), (620, 800), (940, 1030)])
    parede_v(X, Y, 1062.5, 0, 730, 15, [(60, 140), (480, 600)])
    parede_h(X, Y, 7.5, 345, 1070, 15, [(365, 435), (480, 660), (800, 960)])
    parede_h(X, Y, 63.5, 0, 345, 15, [(100, 240)])
    # internas (10)
    parede_v(X, Y, 345, 56, 730, 10, [(250, 340), (395, 475)])
    parede_v(X, Y, 445, 0, 230, 10)
    parede_h(X, Y, 363, 10, 345, 10)
    parede_h(X, Y, 490, 345, 545, 10, [(400, 480)])
    parede_v(X, Y, 545, 363, 722, 10)
    parede_h(X, Y, 363, 545, 1070, 10, [(570, 650), (930, 1010)])
    parede_v(X, Y, 900, 363, 722, 10)
    parede_v(X, Y, 705, 0, 230, 10, [(60, 160)])
    parede_v(X, Y, 705, 290, 363, 10)
    parede_h(X, Y, 230, 445, 705, 10, [(500, 580)])
    camada("JANELAS", "#3aa0ff")
    for y, (a, b) in [(63.5, (100, 240)), (722.5, (100, 250)), (722.5, (400, 480)), (722.5, (620, 800)), (722.5, (940, 1030)), (7.5, (480, 660)), (7.5, (800, 960))]:
        R(X + a, Y + y - 7.5, b - a, 15); L(X + a, Y + y, X + b, Y + y)
    for x, (a, b) in JANELAS_V:
        R(X + x - 7.5, Y + a, 15, b - a); L(X + x, Y + a, X + x, Y + b)
    camada("PORTAS", "#20c0b0")
    for hx, hy, r, a1, a2, folha in [(15, 346, 90, 270, 360, 0), (345, 250, 90, 0, 90, 0), (345, 475, 80, 180, 270, 180),
                                      (480, 490, 80, 90, 180, 90), (650, 363, 80, 90, 180, 90), (1010, 363, 80, 90, 180, 90), (435, 15, 70, 90, 180, 90)]:
        L(X + hx, Y + hy, X + hx + r * math.cos(math.radians(folha)), Y + hy + r * math.sin(math.radians(folha)))
        A(X + hx, Y + hy, r, a1, a2)
    if nomes:
        camada("TEXTOS", "#e6e6e6")
        for x, y, t in [(40, 690, "DORMITÓRIO 1"), (360, 695, "BANHEIRO"), (580, 690, "DORMITÓRIO 2"), (915, 690, "A. SERVIÇO"),
                        (110, 100, "SALA"), (530, 60, "COPA"), (820, 60, "COZINHA"), (352, 150, "CIRC.")]:
            T(X + x, Y + y, 16 * K, t)

AMBIENTES = [("Dormitório 1", 15, 340, 371, 715), ("Banheiro", 350, 540, 495, 715), ("Dormitório 2", 550, 895, 371, 715),
             ("A. de serviço", 905, 1055, 371, 715), ("Sala", 15, 340, 71, 356), ("Copa", 455, 700, 15, 225),
             ("Cozinha", 715, 1055, 15, 358), ("Hall", 350, 700, 235, 358), ("Circulação", 350, 440, 15, 230)]

def cotas(X, Y):
    camada("COTAS", "#f0a030")
    L(X, Y - 80, X + 1070, Y - 80); L(X, Y - 92, X, Y - 68); L(X + 1070, Y - 92, X + 1070, Y - 68); T(X + 500, Y - 72, 14 * K, "1070")
    L(X - 80, Y, X - 80, Y + 730); L(X - 92, Y, X - 68, Y); L(X - 92, Y + 730, X - 68, Y + 730); T(X - 90, Y + 340, 14 * K, "730", 90)

def tabela(x0, ytopo, colunas, linhas, alt=30, h=12):
    larg = sum(w for _, w in colunas); nl = len(linhas) + 1
    camada("TABELAS", "#c0c0c0")
    for i in range(nl + 1): L(x0, ytopo - i * alt, x0 + larg, ytopo - i * alt)
    x = x0
    for _, w in colunas + [("", 0)]:
        L(x, ytopo, x, ytopo - nl * alt); x += w
    camada("TEXTOS", "#e6e6e6")
    x = x0
    for t, w in colunas: T(x + 8, ytopo - alt + alt / 3, h, t); x += w
    for j, lin in enumerate(linhas):
        x = x0
        for (t, w), v in zip(colunas, lin): T(x + 8, ytopo - (j + 2) * alt + alt / 3, h, v); x += w

def textos(x, y, linhas, h=12, passo=24):
    camada("TEXTOS", "#e6e6e6")
    for i, t in enumerate(linhas): T(x, y - i * passo, h, t)

def carimbo(ox, num, total, titulo):
    camada("FOLHA", "#ffffff")
    R(ox, 0, W, H); R(ox + 50, 50, W - 100, H - 100)
    bx, by = ox + W - 50 - 1000, 50
    R(bx, by, 1000, 250)
    L(bx, by + 90, bx + 1000, by + 90); L(bx, by + 170, bx + 1000, by + 170); L(bx + 760, by, bx + 760, by + 170)
    camada("CARIMBO", "#e0e0e0")
    T(bx + 15, by + 200, 22, "CASA DE TESTE - PROJETO ELÉTRICO (REFERÊNCIA DO USUÁRIO)")
    T(bx + 15, by + 120, 18, titulo)
    T(bx + 15, by + 45, 16, "FeatherOffice / FeatherCAD - unidade: cm")
    T(bx + 775, by + 125, 16, f"FOLHA {num:02d}/{total:02d}")
    T(bx + 775, by + 45, 13, f"ESCALA 1:50   DATA {DATA}")
    T(bx + 15, by + 15, 11, "Dimensionamento preliminar: conferir com responsável técnico e concessionária.")

# ------------------------------------------------------------------ eletrica
QD = (50, 363)
LUZES = [  # x, y, comando, circuito, potencia
    (160, 210, "a", 1, 100),   # sala
    (160, 546, "f", 1, 160),   # dormitorio 1
    (450, 606, "g", 1, 100),   # banheiro
    (440, 430, "h", 1, 100),   # hall
    (700, 560, "i", 1, 160),   # dormitorio 2
    (575, 120, "b", 2, 100),   # copa
    (860, 216, "c", 2, 160),   # cozinha
    (956, 540, "j", 2, 100),   # area de servico
]
ARANDELA = (1055, 310, "d", 2, 100)
INTERRUPTORES = [("a", 30, 270), ("f", 330, 455), ("g", 425, 500), ("h", 395, 500), ("i", 585, 375), ("j", 935, 375), ("b", 455, 215), ("c", 720, 300), ("d", 1040, 270)]
TOMADAS = [  # x, y, parede, nivel, circuito, potencia(VA)
    (15, 654, "oeste", "baixa", 3, 100), (15, 455, "oeste", "baixa", 3, 100), (164, 715, "norte", "baixa", 3, 100), (340, 540, "leste", "baixa", 3, 100),
    (15, 300, "oeste", "baixa", 3, 100), (15, 120, "oeste", "baixa", 3, 100), (150, 71, "sul", "baixa", 3, 100), (300, 71, "sul", "baixa", 3, 100),
    (556, 690, "oeste", "baixa", 4, 100), (700, 715, "norte", "baixa", 4, 100), (890, 690, "leste", "baixa", 4, 100), (830, 371, "sul", "baixa", 4, 100),
    (475, 15, "sul", "media", 5, 600), (590, 15, "sul", "media", 5, 600), (690, 15, "sul", "media", 5, 600),
    (350, 300, "oeste", "baixa", 6, 100), (640, 235, "sul", "baixa", 6, 100),
    (760, 15, "sul", "media", 7, 600), (880, 15, "sul", "media", 7, 600), (1055, 60, "leste", "media", 7, 600),
    (710, 320, "oeste", "baixa", 8, 100), (1055, 250, "leste", "baixa", 8, 100), (900, 358, "norte", "baixa", 8, 100),
    (1055, 690, "leste", "media", 9, 600), (905, 600, "oeste", "media", 9, 600), (1055, 460, "leste", "media", 9, 600),
    (905, 650, "oeste", "media", 10, 1000),
    (545, 640, "oeste", "alta", 11, 5500),
    (980, 15, "sul", "alta", 12, 600),
]
NOMES = {1: "Iluminação social", 2: "Iluminação serviço", 3: "TUG Sala e Dormitório 1", 4: "TUG Dormitório 2", 5: "TUG Copa", 6: "TUG Hall",
         7: "TUG Cozinha", 8: "TUG Hall / Cozinha / Serviço", 9: "TUG Área de serviço", 10: "TUE Tanque / máquina", 11: "TUE Chuveiro (banheiro)", 12: "TUE Cozinha (geladeira)"}
CARGA = {c: 0 for c in NOMES}
for *_, cmd, c, p in LUZES: CARGA[c] += p
CARGA[ARANDELA[3]] += ARANDELA[4]
for *_, c, p in TOMADAS: CARGA[c] += p
TENSAO = {c: (220 if c == 11 else 127) for c in NOMES}
# fases: chuveiro nas duas; o resto, gulosamente na fase mais leve
FASE, soma = {11: "F1+F2"}, {"F1": 0, "F2": 0}
for c in sorted((c for c in NOMES if c != 11), key=lambda c: -CARGA[c]):
    f = min(soma, key=soma.get); FASE[c] = f; soma[f] += CARGA[c]
def disj_de(i):
    for d in (10, 16, 20, 25, 32, 40):
        if i * 1.0 <= d: return d
    return 50
SECAO = {}
for c in NOMES:
    i = CARGA[c] / TENSAO[c]; c_disj = disj_de(i)
    SECAO[c] = 1.5 if c in (1, 2) else (6 if c == 11 else (4 if c_disj >= 25 else 2.5))
def sec(s): return virgula(s) if s != int(s) else str(int(s))

def F(c, q=1): return dict(circuito=c, tipo="fase", secao=SECAO[c], quantidade=q)
def N_(c): return dict(circuito=c, tipo="neutro", secao=SECAO[c])
def Rt(c, q=1): return dict(circuito=c, tipo="retorno", secao=SECAO[c], quantidade=q)

def ordenar(pontos, inicio):
    rest, cur, ord_ = list(pontos), inicio, []
    while rest:
        p = min(rest, key=lambda q: math.dist(cur, q)); rest.remove(p); ord_.append(p); cur = p
    return ord_

def rotas():
    """(pontos, condutores, terra, extras) de todos os eletrodutos, em cm da planta. Radiais curvas; troncos das tomadas no piso (tracejado)."""
    r = []
    L1 = {"a": (160, 210), "f": (160, 546), "g": (450, 606), "h": (440, 430), "i": (700, 560), "b": (575, 120), "c": (860, 216), "j": (956, 540)}
    def cv(k): return dict(curva=k)
    r += [([QD, L1["a"]], [F(1), N_(1)], True, cv(0.10)), ([QD, L1["f"]], [F(1), N_(1)], True, cv(-0.10)), ([QD, L1["h"]], [F(1), N_(1)], True, cv(0.14)),
          ([L1["h"], L1["g"]], [F(1), N_(1)], True, cv(-0.15)), ([L1["h"], L1["i"]], [F(1), N_(1)], True, cv(0.15)),
          ([QD, L1["b"]], [F(2), N_(2)], True, cv(-0.14)), ([L1["b"], L1["c"]], [F(2), N_(2)], True, cv(0.15)), ([L1["c"], L1["j"]], [F(2), N_(2)], True, cv(-0.12)),
          ([L1["c"], (1055, 310)], [F(2), N_(2)], True, cv(0.12))]
    ligacao = {"a": ((160, 210), (30, 270)), "f": ((160, 546), (330, 455)), "g": ((450, 606), (425, 500)), "h": ((440, 430), (395, 500)),
               "i": ((700, 560), (585, 375)), "j": ((956, 540), (935, 375)), "b": ((575, 120), (455, 215)), "c": ((860, 216), (720, 300)), "d": ((1055, 310), (1040, 270))}
    circ = {"a": 1, "f": 1, "g": 1, "h": 1, "i": 1, "j": 2, "b": 2, "c": 2, "d": 2}
    for k, (luz, sw) in ligacao.items():
        r.append(([luz, sw], [F(circ[k]), Rt(circ[k])], False, {}))
    # troncos compartilhados no piso (tracejados): A leva 5, 6, 7 e 8 ao hall; B leva 4, 9, 10 e 11 aos quartos e à área de serviço
    JA, JB = (420, 262), (485, 440)
    def cond_de(c): return [F(c, 2)] if c == 11 else [F(c), N_(c)]
    grupos = [(JA, (5, 6, 7, 8), 0.16), (JB, (4, 9, 10, 11), -0.16)]
    for J, cs, curva in grupos:
        cond = [x for c in cs for x in cond_de(c)]
        r.append(([QD, J], cond, True, dict(tipo="piso", curva=curva, diametro=32)))
        for c in cs:
            ordem = ordenar([(x, y) for x, y, _, _, cc, _ in TOMADAS if cc == c], J)
            r.append(([J, ordem[0]], cond_de(c), True, dict(curva=0.08 if c % 2 else -0.08)))
            for a, b in zip(ordem, ordem[1:]):
                r.append(([a, b], cond_de(c), True, dict(rotulo=False)))
    # circuito 3 (sala e dormitório 1) sai direto do quadro
    ordem = ordenar([(x, y) for x, y, _, _, cc, _ in TOMADAS if cc == 3], QD)
    r.append(([QD, ordem[0]], cond_de(3), True, {}))
    for a, b in zip(ordem, ordem[1:]):
        r.append(([a, b], cond_de(3), True, dict(rotulo=False)))
    return r

# fases do multifilar trifasico (A, B, C); o chuveiro (220 V) entre as duas fases mais leves
FASE3, soma3 = {}, {"A": 0.0, "B": 0.0, "C": 0.0}
for c in sorted(NOMES, key=lambda c: -CARGA[c]):
    if c == 11:
        duas = sorted(soma3, key=soma3.get)[:2]
        FASE3[c] = "+".join(duas)
        for f in duas: soma3[f] += CARGA[c] / 220
    else:
        f = min(soma3, key=soma3.get); FASE3[c] = f; soma3[f] += CARGA[c] / 127
def disjuntor_txt(c):
    i = CARGA[c] / TENSAO[c]
    return f"{'2P' if c == 11 else '1P'}-{disj_de(i)}A"
LOCAL3 = {c: NOMES[c].replace("TUG ", "TUG ").replace("Iluminação", "Ilum.") for c in NOMES}
ESQ3, DIR3 = [1, 2, 3, 4, 5, 6], [7, 8, 9, 10, 11, 12]
ROW = 50
YR = {c: 950 - ROW * (i + 0.5) for i, c in enumerate(ESQ3)}
YR.update({c: 950 - ROW * (i + 0.5) for i, c in enumerate(DIR3)})

def montar(fcdz, dxf=None):
    tot = sum(CARGA.values())
    # ---- folha 1: arquitetura
    carimbo(0, 1, 3, "PLANTA BAIXA - ARQUITETURA")
    casa(X0, Y0); cotas(X0, Y0)
    camada("TEXTOS", "#e6e6e6")
    for nome, x1, x2, y1, y2 in AMBIENTES:
        T(X0 + x1 + 10, Y0 + y1 + 10, 13, f"{virgula((x2 - x1) * (y2 - y1) / 10000, 2)} m²")
    T(1420, 1400, 22, "QUADRO DE AMBIENTES")
    tabela(1420, 1370, [("Ambiente", 240), ("Medidas (m)", 200), ("Área (m²)", 120)],
           [[nm, f"{virgula((x2 - x1) / 100, 2)} x {virgula((y2 - y1) / 100, 2)}", virgula((x2 - x1) * (y2 - y1) / 10000, 2)] for nm, x1, x2, y1, y2 in AMBIENTES])
    # ---- folha 2: unifilar (só a base; símbolos vêm depois)
    o2 = PASSO
    carimbo(o2, 2, 3, "INSTALAÇÃO ELÉTRICA - DIAGRAMA UNIFILAR (NBR 5444)")
    casa(o2 + X0, Y0)
    xr = o2 + 1450
    textos(xr, 480, ["Ponto de luz: letra = interruptor que comanda; -n- = circuito;", "100 / 160 = potência (W). Tomada: número = circuito.",
                     "Eletroduto: números = circuitos que passam; #seção; Ø = diâmetro.", "Cada traço = um condutor (fase, neutro, retorno, terra)."], 11, 22)
    # ---- folha 3: multifilar trifásico
    o3 = 2 * PASSO
    carimbo(o3, 3, 3, "DIAGRAMA MULTIFILAR TRIFÁSICO E QUADRO DE CIRCUITOS")
    XA, XB, XC, XN, XT = o3 + 900, o3 + 960, o3 + 1020, o3 + 1080, o3 + 1140
    XBUS = {"A": XA, "B": XB, "C": XC}
    YT, YB = 1390, 560
    textos(o3 + 100, 1420, ["DIAGRAMA MULTIFILAR TRIFÁSICO (127/220 V: 3 fases + neutro + terra)"], 22)
    camada("QD-BARRAS", "#e0e0e0")
    for x, nm in [(XA, "A"), (XB, "B"), (XC, "C"), (XN, "N"), (XT, "T")]:
        L(x, YT, x, YB); T(x - 4, YT + 12, 16, nm); T(x - 4, YB - 28, 16, nm)
    camada("QD-DIAGRAMA", "#e0e0e0")
    R(XA - 25, 1010, XN - XA + 50, 90)            # IDR tetrapolar
    textos(XT + 40, 1050, ["IDR tetrapolar 63 A / 30 mA"], 12)
    textos(XT + 40, 1290, ["Disjuntor geral tripolar 63 A"], 12)
    R(XT + 60, 1170, 150, 70); textos(XT + 90, 1198, ["DPS classe II 20 kA"], 11)
    for yy, xx in [(1230, XA), (1210, XB), (1190, XC), (1170, XN)]: L(xx, yy, XT + 60, yy)
    L(XT, 1150, XT + 60, 1150) if False else None
    camada("TABELAS", "#c0c0c0")
    def tab_lado(xs, titulos, ks, campos):
        ys = [950 + ROW * 0 - ROW * i for i in range(len(ks) + 1)]
        top = 1000
        L(xs[0], top, xs[-1], top)
        for y in ys: L(xs[0], y, xs[-1], y)
        for x in xs: L(x, top, x, ys[-1])
        camada("TEXTOS", "#e6e6e6")
        for x, t in zip(xs, titulos): T(x + 5, top - 32, 10, t)
        for j, k in enumerate(ks):
            for x, f in zip(xs, campos): T(x + 5, ys[j] - 32, 11, f(k))
        camada("TABELAS", "#c0c0c0")
    cs = [lambda k: LOCAL3[k], lambda k: "#" + sec(SECAO[k]), lambda k: disjuntor_txt(k), lambda k: f"C{k}"]
    tab_lado([o3 + 100, o3 + 350, o3 + 410, o3 + 480, o3 + 530], ["LOCAL", "SEÇÃO", "PROTEÇÃO", "CIRC."], ESQ3, cs)
    tab_lado([o3 + 1400, o3 + 1450, o3 + 1520, o3 + 1580, o3 + 1830], ["CIRC.", "PROTEÇÃO", "SEÇÃO", "LOCAL"], DIR3, list(reversed(cs)))
    camada("QD-CIRCUITOS", "#e0e0e0")
    for c in ESQ3 + DIR3:
        fases = [FASE3[c]] if "+" not in FASE3[c] else FASE3[c].split("+")
        ys = [YR[c]] if len(fases) == 1 else [YR[c] + 6, YR[c] - 6]
        for f, yy in zip(fases, ys):
            if c in ESQ3: L(o3 + 530, yy, XBUS[f], yy)
            else: L(XBUS[f], yy, o3 + 1400, yy)
    # quadro de circuitos (embaixo à esquerda)
    textos(o3 + 100, 520, ["QUADRO DE CIRCUITOS"], 18)
    linhas = []
    for c in NOMES:
        i = CARGA[c] / TENSAO[c]
        linhas.append([f"C{c}", NOMES[c], milhar(CARGA[c]), f"{TENSAO[c]} V", FASE3[c], virgula(i), disjuntor_txt(c), sec(SECAO[c]) + " mm²"])
    tabela(o3 + 100, 495, [("Circ.", 55), ("Descrição", 290), ("VA", 70), ("Tensão", 65), ("Fase", 70), ("I (A)", 55), ("Disj.", 80), ("Seção", 80)], linhas, alt=27, h=11)
    textos(o3 + 100, 495 - 27 * 14 - 10, [f"Carga instalada: {milhar(tot)} VA ({virgula(tot / 1000)} kVA).  Correntes: A {virgula(soma3['A'])} A  |  B {virgula(soma3['B'])} A  |  C {virgula(soma3['C'])} A."], 12, 22)

    m = Mcp()
    m.call("cad_novo", {"unidade": "cm"})
    for nome, cor in [("ELE-ILUMINACAO", "#ffcc00"), ("ELE-INTERRUPTORES", "#ff50ff"), ("ELE-TOMADAS", "#30d030"), ("ELE-ELETRODUTOS", "#ff8a5c"), ("ELE-QUADRO", "#ff8000"), ("ELE-LEGENDA", "#e6e6e6"), ("QD-MULTIFILAR", "#e0e0e0")]:
        m.call("cad_camadas", {"acao": "criar", "nome": nome, "cor": cor})
    m.call("cad_importar", {"texto": "\n".join(out) + "\n"})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "PAREDES", "espessura": 0.5})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "ELE-ELETRODUTOS", "espessura": 0.25})
    X, Y = o2 + X0, Y0
    def sim(nome, ponto, cam, **kw): m.call("cad_simbolo", {"nome": nome, "ponto": list(ponto), "camada": cam, "escala": ESC, **kw})
    for pts, cond, terra, extra in rotas():
        m.call("cad_eletroduto", {"pontos": [[X + x, Y + y] for x, y in pts], "condutores": cond, "terra": terra, "camada": "ELE-ELETRODUTOS", "escala": ESC, "espessura": 0.35, **extra})
    m.call("cad_eletroduto", {"pontos": [[X + 50, Y + 357], [X - 60, Y + 300], [X - 230, Y + 254]], "condutores": [dict(tipo="fase", secao=16, quantidade=2), dict(tipo="neutro", secao=16)],
                              "tipo": "piso", "diametro": 25, "secao_terra": 16, "camada": "ELE-ELETRODUTOS", "escala": ESC, "espessura": 0.35, "curva": 0.12})
    sim("caixa_medidor", (X - 245, Y + 254), "ELE-QUADRO")
    for x, y, cmd, c, p in LUZES: sim("ponto_luz_teto", (X + x, Y + y), "ELE-ILUMINACAO", comando=cmd, circuito=c, potencia=p)
    x, y, cmd, c, p = ARANDELA
    sim("arandela", (X + x, Y + y), "ELE-ILUMINACAO", comando=cmd, circuito=c, potencia=p, rotacao=180)
    for cmd, x, y in INTERRUPTORES: sim("interruptor_simples", (X + x, Y + y), "ELE-INTERRUPTORES", comando=cmd)
    for x, y, parede, nivel, c, p in TOMADAS:
        sim(f"tomada_{nivel}", (X + x, Y + y), "ELE-TOMADAS", parede=parede, circuito=c, potencia=f"{p}VA" if p < 1000 else f"{p}W")
    for J in ((420, 262), (485, 440)): sim("caixa_passagem_piso", (X + J[0], Y + J[1]), "ELE-QUADRO")
    sim("quadro_geral_embutido", (X + QD[0], Y + QD[1] - 6), "ELE-QUADRO", potencia=f"{virgula(tot / 1000)}kVA")
    # legenda (folha 2)
    leg = [("tomada_baixa", "Tomada baixa (30 cm)", {}), ("tomada_media", "Tomada média (110 cm)", {}), ("tomada_alta", "Tomada alta (200 cm)", {}),
           ("ponto_luz_teto", "Ponto de luz no teto", dict(comando="a", circuito=1, potencia=100)), ("arandela", "Arandela (na parede)", dict(comando="d", circuito=2, potencia=100)),
           ("interruptor_simples", "Interruptor simples", dict(comando="a")), ("quadro_geral_embutido", "Quadro geral, embutido", {}), ("caixa_medidor", "Caixa para medidor", {})]
    yl = 1300
    T_ = lambda x, y, txt, h=12: m.call("cad_criar", {"formas": [{"tipo": "texto", "ponto": [x, y], "altura": h, "conteudo": txt}], "camada": "ELE-LEGENDA"})
    T_(xr, yl + 60, "LEGENDA - SIMBOLOGIA (NBR 5444)", 18)
    for i, (nome, txt, kw) in enumerate(leg):
        y = yl - i * 62
        sim(nome, (xr + 40, y + (18 if nome.startswith("tomada") else 0)), "ELE-LEGENDA", **({"parede": "norte"} if nome.startswith("tomada") else {}), **kw)
        T_(xr + 150, y - 4, txt)
    y0 = yl - len(leg) * 62 - 10
    for i, (txt, kw, cond) in enumerate([("Eletroduto no teto/parede", dict(tipo="teto", diametro=16), [dict(tipo="fase", secao=1.5)]),
                                         ("Eletroduto no piso (tracejado)", dict(tipo="piso", diametro=25), [dict(tipo="fase", secao=1.5)]),
                                         ("Condutor fase", {}, [dict(tipo="fase", secao=1.5)]), ("Condutor neutro", {}, [dict(tipo="neutro", secao=1.5)]),
                                         ("Condutor de retorno", {}, [dict(tipo="retorno", secao=1.5)]), ("Condutor terra", {}, [dict(tipo="terra", secao=1.5)])]):
        y = y0 - i * 42
        m.call("cad_eletroduto", {"pontos": [[xr, y], [xr + 90, y]], "condutores": cond, "terra": False, "rotulo": "diametro" in kw, "camada": "ELE-LEGENDA", "escala": ESC, **kw})
        T_(xr + 150, y - 4, txt)
    # multifilar (folha 3)
    def sm(nome, x, y, **kw): sim(nome, (x, y), "QD-MULTIFILAR", **kw)
    for x in (XA, XB, XC): sm("disjuntor", x, 1290, rotacao=90)
    for x in (XA, XB, XC, XN): sm("disjuntor", x, 1055, rotacao=90)
    for xx, yy in [(XA, 1230), (XB, 1210), (XC, 1190), (XN, 1170)]: sm("no_de_ligacao", xx, yy)
    for c in ESQ3 + DIR3:
        fases = FASE3[c].split("+")
        ys = [YR[c]] if len(fases) == 1 else [YR[c] + 6, YR[c] - 6]
        for f, yy in zip(fases, ys):
            sm("no_de_ligacao", XBUS[f], yy)
            sm("disjuntor", o3 + 700 if c in ESQ3 else o3 + 1290, yy)
    m.call("cad_eletroduto", {"pontos": [[o3 + 700, 1345], [XA - 5, 1345]], "condutores": [dict(tipo="fase", secao=16, quantidade=3), dict(tipo="neutro", secao=16)], "secao_terra": 16, "camada": "QD-MULTIFILAR", "escala": ESC, "diametro": 32})
    T_(o3 + 620, 1345, "Entrada", 12)
    T_(XT + 40, 600, "T: barra de terra exclusiva (circuitos com DR)", 12)
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    if dxf: m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1]
    r = montar(base + ".fcdz", base + ".dxf")
    print(r["total_de_entidades"], r["por_tipo"])
    print({c: (CARGA[c], FASE[c], sec(SECAO[c])) for c in NOMES}, soma)

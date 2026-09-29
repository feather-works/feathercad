#!/usr/bin/env python3
"""Projeto da casa em 4 folhas A3 (1:50, cm): a base sai em .fct e os simbolos NBR 5444 vem do MCP (cad_simbolo, cad_eletroduto)."""
import math, sys, json
from mcp_cliente import Mcp

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
def P(pts, f=False): out.append("P " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + (" f" if f else "") + CUR)
def cheio(pts): out.append("H solida " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + CUR)

W, H, PASSO = 2100, 1485, 2300
PX, PY = 300, 420
DATA = "29/09/2026"
F1, F2, F12 = "#ff4040", "#4a90ff", "#d040d0"
def virgula(v, c=1): return f"{v:.{c}f}".replace(".", ",")
def sec(s): return virgula(s) if s != int(s) else str(int(s))

# ------------------------------------------------------------------ circuitos e queda de tensao
CIRC = {
    1: dict(nome="Iluminação (5 pontos x 100 VA)", local="Iluminação (todos)", va=500, v=127, fase="F1", cor=F1, disj="10 A", disj_curto="1P-10A", min_s=1.5),
    2: dict(nome="TUG Sala + Quarto 1 (8 tomadas)", local="TUG Sala e Quarto 1", va=800, v=127, fase="F2", cor=F2, disj="16 A", disj_curto="1P-16A", min_s=2.5),
    3: dict(nome="TUG Cozinha (5 tomadas)", local="TUG Cozinha", va=2000, v=127, fase="F1", cor=F1, disj="20 A", disj_curto="1P-20A", min_s=2.5),
    4: dict(nome="TUG Banho + Quarto 2 (7 tomadas)", local="TUG Banho e Quarto 2", va=1200, v=127, fase="F2", cor=F2, disj="16 A", disj_curto="1P-16A", min_s=2.5),
    5: dict(nome="Chuveiro (TUE 5500 W)", local="Chuveiro", va=5500, v=220, fase="F1+F2", cor=F12, disj="32 A 2P", disj_curto="2P-32A", min_s=6),
}
PIOR = {
    1: [(15, 362), (40, 362), (40, 600), (175, 600), (175, 620), (475, 620), (475, 600), (800, 600)],
    2: [(15, 328), (30, 328), (30, 30), (490, 30)],
    3: [(15, 345), (420, 345), (440, 345), (440, 260), (600, 260), (600, 30), (975, 30), (975, 350)],
    4: [(15, 362), (40, 362), (40, 430), (365, 430), (365, 775), (610, 775), (610, 450), (975, 450), (975, 775), (940, 775)],
    5: [(15, 345), (420, 345), (420, 720), (560, 720), (560, 770)],
}
RHO, VERT, SECOES = 0.0225, 3.5, [1.5, 2.5, 4, 6, 10, 16]
def comp(pts): return sum(math.dist(pts[i], pts[i + 1]) for i in range(len(pts) - 1))
for k, c in CIRC.items():
    c["i"] = c["va"] / c["v"]
    c["l"] = comp(PIOR[k]) / 100 + VERT
    dv = lambda s, c=c: 2 * RHO * c["l"] * c["i"] / s / c["v"] * 100
    c["dv0"] = dv(c["min_s"])
    c["s"] = next(s for s in SECOES if s >= c["min_s"] and dv(s) <= 4.0)
    c["dv"] = dv(c["s"])
PIOR_C = max(CIRC, key=lambda k: CIRC[k]["dv"])
F1_VA = sum(c["va"] for c in CIRC.values() if c["fase"] == "F1")
F2_VA = sum(c["va"] for c in CIRC.values() if c["fase"] == "F2")
I_CH = CIRC[5]["i"]

# ------------------------------------------------------------------ eletrodutos fisicos (cada trecho = uma chamada cad_eletroduto)
def F(c, s, q=1): return dict(circuito=c, tipo="fase", secao=s, quantidade=q)
def N(c, s): return dict(circuito=c, tipo="neutro", secao=s)
def Rt(c, s, q=1): return dict(circuito=c, tipo="retorno", secao=s, quantidade=q)
S = {k: CIRC[k]["s"] for k in CIRC}
def luz(pts, cond, terra=True): return dict(pontos=pts, condutores=cond, terra=terra)
TRECHOS = [
    # QD -> leste (C1, C3, C5)
    luz([(15, 345), (250, 345)], [F(1, S[1]), N(1, S[1]), F(3, S[3]), N(3, S[3]), F(5, S[5], 2)]),
    luz([(250, 345), (420, 345)], [F(3, S[3]), N(3, S[3]), F(5, S[5], 2)]),
    luz([(420, 345), (420, 720), (560, 720), (560, 770)], [F(5, S[5], 2)]),
    luz([(420, 345), (440, 345), (440, 260), (600, 260), (600, 30)], [F(3, S[3]), N(3, S[3])]),
    luz([(600, 30), (975, 30), (975, 350)], [F(3, S[3]), N(3, S[3])]),
    luz([(600, 30), (520, 30)], [F(3, S[3]), N(3, S[3])]),
    # QD -> oeste/norte (C1, C2, C4)
    luz([(15, 362), (40, 362), (40, 430)], [F(1, S[1]), N(1, S[1]), F(2, S[2]), N(2, S[2]), F(4, S[4]), N(4, S[4])]),
    luz([(40, 430), (40, 600)], [F(1, S[1]), N(1, S[1]), F(2, S[2]), N(2, S[2])]),
    luz([(40, 600), (40, 775), (200, 775)], [F(2, S[2]), N(2, S[2])]),
    luz([(40, 600), (175, 600)], [F(1, S[1]), N(1, S[1])]),
    luz([(40, 450), (340, 450)], [F(2, S[2]), N(2, S[2])]),
    luz([(40, 430), (365, 430), (365, 775), (610, 775), (610, 450), (975, 450), (975, 775), (940, 775)], [F(4, S[4]), N(4, S[4])]),
    luz([(15, 328), (30, 328), (30, 30), (490, 30)], [F(2, S[2]), N(2, S[2])]),
    # iluminacao
    luz([(250, 345), (250, 200)], [F(1, S[1]), N(1, S[1])]),
    luz([(250, 200), (750, 200)], [F(1, S[1]), N(1, S[1])]),
    luz([(175, 600), (175, 620), (475, 620), (475, 600)], [F(1, S[1]), N(1, S[1])]),
    luz([(475, 600), (800, 600)], [F(1, S[1]), N(1, S[1])]),
    # ligacao dos interruptores (fase que vai, retorno que volta)
    luz([(250, 200), (180, 200), (180, 36)], [F(1, S[1])], False),
    luz([(190, 30), (190, 120), (470, 120), (470, 320), (484, 320)], [Rt(1, S[1], 2)], False),
    luz([(490, 326), (490, 350), (375, 350), (375, 385), (371, 385)], [Rt(1, S[1], 2)], False),
    luz([(250, 188), (360, 188), (360, 379)], [Rt(1, S[1])], False),
    luz([(175, 600), (240, 600), (240, 421)], [F(1, S[1])], False),
    luz([(246, 415), (300, 415), (300, 527), (329, 527)], [Rt(1, S[1], 2)], False),
    luz([(335, 521), (335, 570), (175, 570), (175, 588)], [Rt(1, S[1])], False),
    luz([(475, 600), (475, 500), (390, 500), (390, 421)], [F(1, S[1]), Rt(1, S[1])], False),
    luz([(800, 600), (800, 500), (690, 500), (690, 421)], [F(1, S[1]), Rt(1, S[1])], False),
    luz([(750, 200), (750, 80), (521, 80)], [F(1, S[1]), Rt(1, S[1])], False),
]
LUZES = [(250, 200, "a"), (175, 600, "b"), (475, 600, "c"), (800, 600, "d"), (750, 200, "e")]
INTERRUPTORES = [("paralelo", 180, 30, "a"), ("intermediario", 490, 320, "a"), ("paralelo", 365, 385, "a"),
                 ("paralelo", 240, 415, "b"), ("paralelo", 335, 527, "b"), ("simples", 390, 415, "c"),
                 ("simples", 690, 415, "d"), ("simples", 515, 80, "e")]
TOMADAS = [  # x, y (no rosto da parede), parede, nivel, circuito, potencia
    (15, 250, "oeste", "baixa", 2, "100VA"), (15, 100, "oeste", "baixa", 2, "100VA"), (400, 15, "sul", "baixa", 2, "100VA"), (490, 15, "sul", "baixa", 2, "100VA"),
    (15, 450, "oeste", "baixa", 2, "100VA"), (15, 750, "oeste", "baixa", 2, "100VA"), (200, 785, "norte", "baixa", 2, "100VA"), (345, 450, "leste", "baixa", 2, "100VA"),
    (520, 15, "sul", "media", 3, "600VA"), (700, 15, "sul", "media", 3, "600VA"), (900, 15, "sul", "media", 3, "600VA"),
    (985, 90, "leste", "baixa", 3, "100VA"), (985, 350, "leste", "baixa", 3, "100VA"),
    (355, 600, "oeste", "media", 4, "600VA"), (475, 785, "norte", "baixa", 4, "100VA"),
    (605, 450, "oeste", "baixa", 4, "100VA"), (605, 750, "oeste", "baixa", 4, "100VA"), (985, 450, "leste", "baixa", 4, "100VA"),
    (985, 750, "leste", "baixa", 4, "100VA"), (940, 785, "norte", "baixa", 4, "100VA"),
    (560, 785, "norte", "alta", 5, "5500W"),
]

# ------------------------------------------------------------------ pecas de folha
def carimbo(ox, num, titulo, escala="1:50"):
    camada("FOLHA", "#ffffff")
    R(ox, 0, W, H); R(ox + 50, 50, W - 100, H - 100)
    bx, by = ox + W - 50 - 1000, 50
    R(bx, by, 1000, 250)
    L(bx, by + 90, bx + 1000, by + 90); L(bx, by + 170, bx + 1000, by + 170); L(bx + 760, by, bx + 760, by + 170)
    camada("CARIMBO", "#e0e0e0")
    T(bx + 15, by + 200, 22, "PROJETO RESIDENCIAL - PLANTA BAIXA E INSTALAÇÃO ELÉTRICA")
    T(bx + 15, by + 120, 18, titulo)
    T(bx + 15, by + 45, 16, "FeatherOffice / FeatherCAD - unidade: cm")
    T(bx + 775, by + 125, 16, f"FOLHA {num:02d}/04")
    T(bx + 775, by + 45, 13, f"ESCALA {escala}   DATA {DATA}")
    T(bx + 15, by + 15, 11, "Dimensionamento preliminar: conferir com responsável técnico e concessionária.")

def base(X, Y, nomes=True):
    camada("PAREDES", "#9a9a9a")
    for a, b in [(0, 200), (290, 380), (480, 650), (800, 1000)]: R(X + a, Y, b - a, 15)
    for a, b in [(0, 100), (250, 430), (520, 750), (900, 1000)]: R(X + a, Y + 785, b - a, 15)
    for a, b in [(15, 150), (300, 500), (650, 785)]:
        R(X, Y + a, 15, b - a); R(X + 985, Y + a, 15, b - a)
    for a, b in [(15, 250), (330, 400), (480, 700), (780, 985)]: R(X + a, Y + 395, b - a, 10)
    R(X + 345, Y + 405, 10, 380); R(X + 595, Y + 405, 10, 380); R(X + 495, Y + 15, 10, 85); R(X + 495, Y + 300, 10, 95)
    camada("JANELAS", "#3aa0ff")
    for a, b in [(380, 480), (650, 800)]: R(X + a, Y, b - a, 15); L(X + a, Y + 7.5, X + b, Y + 7.5)
    for a, b in [(100, 250), (430, 520), (750, 900)]: R(X + a, Y + 785, b - a, 15); L(X + a, Y + 792.5, X + b, Y + 792.5)
    for a, b in [(150, 300), (500, 650)]:
        R(X, Y + a, 15, b - a); L(X + 7.5, Y + a, X + 7.5, Y + b)
        R(X + 985, Y + a, 15, b - a); L(X + 992.5, Y + a, X + 992.5, Y + b)
    camada("PORTAS", "#20c0b0")
    for hx, hy, r in [(200, 15, 90), (250, 405, 80), (400, 405, 80), (700, 405, 80)]:
        L(X + hx, Y + hy, X + hx, Y + hy + r); A(X + hx, Y + hy, r, 0, 90)
    if nomes:
        camada("TEXTOS", "#e6e6e6")
        for x, y, t in [(190, 250, "SALA"), (690, 250, "COZINHA"), (70, 680, "QUARTO 1"), (400, 680, "BANHO"), (690, 680, "QUARTO 2")]:
            T(X + x, Y + y, 18, t)

def cotas(X, Y):
    camada("COTAS", "#f0a030")
    L(X, Y - 70, X + 1000, Y - 70); L(X, Y - 80, X, Y - 60); L(X + 1000, Y - 80, X + 1000, Y - 60); T(X + 470, Y - 62, 14, "1000")
    L(X, Y - 35, X + 1000, Y - 35)
    for x in (0, 500, 1000): L(X + x, Y - 43, X + x, Y - 27)
    T(X + 235, Y - 28, 12, "500"); T(X + 735, Y - 28, 12, "500")
    L(X - 70, Y, X - 70, Y + 800); L(X - 80, Y, X - 60, Y); L(X - 80, Y + 800, X - 60, Y + 800); T(X - 78, Y + 380, 14, "800", 90)
    L(X - 35, Y, X - 35, Y + 800)
    for y in (0, 400, 800): L(X - 43, Y + y, X - 27, Y + y)
    T(X - 42, Y + 190, 12, "400", 90); T(X - 42, Y + 590, 12, "400", 90)
    L(X, Y + 850, X + 1000, Y + 850)
    for x in (0, 350, 600, 1000): L(X + x, Y + 842, X + x, Y + 858)
    T(X + 165, Y + 860, 12, "350"); T(X + 465, Y + 860, 12, "250"); T(X + 765, Y + 860, 12, "400")

def tabela(x0, ytopo, colunas, linhas, alt=30, h=12):
    larg = sum(w for _, w in colunas)
    nl = len(linhas) + 1
    camada("TABELAS", "#c0c0c0")
    for i in range(nl + 1): L(x0, ytopo - i * alt, x0 + larg, ytopo - i * alt)
    x = x0
    for _, w in colunas + [("", 0)]:
        L(x, ytopo, x, ytopo - nl * alt); x += w
    camada("TEXTOS", "#e6e6e6")
    x = x0
    for t, w in colunas: T(x + 6, ytopo - alt + alt / 3, h, t); x += w
    for j, lin in enumerate(linhas):
        x = x0
        for (t, w), v in zip(colunas, lin): T(x + 6, ytopo - (j + 2) * alt + alt / 3, h, v); x += w

def textos(x, y, linhas, h=12, passo=26):
    camada("TEXTOS", "#e6e6e6")
    for i, t in enumerate(linhas): T(x, y - i * passo, h, t)

def offset(pts, d):
    """Polilinha deslocada d para a esquerda (cantos em esquadro ou qualquer angulo, com esquina de miter)."""
    segs = []
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        l = math.hypot(dx, dy)
        nx, ny = -dy / l * d, dx / l * d
        segs.append(((a[0] + nx, a[1] + ny), (b[0] + nx, b[1] + ny)))
    r = [segs[0][0]]
    for s1, s2 in zip(segs, segs[1:]):
        (x1, y1), (x2, y2), (x3, y3), (x4, y4) = s1[0], s1[1], s2[0], s2[1]
        den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4)
        if abs(den) < 1e-9: r.append(s1[1]); continue
        t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den
        r.append((x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
    r.append(segs[-1][1])
    return r

# ================================================================== BASE
# FOLHA 1: arquitetura
o = 0
carimbo(o, 1, "PLANTA BAIXA - ARQUITETURA")
base(o + 150, PY); cotas(o + 150, PY)
textos(o + 150 + 190, PY + 225, ["18,24 m²"], 12); textos(o + 150 + 690, PY + 225, ["18,24 m²"], 12)
textos(o + 150 + 70, PY + 655, ["12,54 m²"], 12); textos(o + 150 + 400, PY + 655, ["9,12 m²"], 12); textos(o + 150 + 690, PY + 655, ["14,44 m²"], 12)
textos(o + 1300, 1330, ["QUADRO DE AMBIENTES"], 22)
tabela(o + 1300, 1300, [("Ambiente", 220), ("Medidas (m)", 200), ("Área (m²)", 150)],
       [["Sala", "4,80 x 3,80", "18,24"], ["Cozinha", "4,80 x 3,80", "18,24"], ["Quarto 1", "3,30 x 3,80", "12,54"], ["Banho", "2,40 x 3,80", "9,12"], ["Quarto 2", "3,80 x 3,80", "14,44"], ["Total interno", "", "72,58"]])
textos(o + 1300, 1010, ["QUADRO DE ESQUADRIAS"], 22)
tabela(o + 1300, 980, [("Tipo", 90), ("Descrição", 330), ("Larg. x alt. (cm)", 190), ("Qtd", 60)],
       [["P1", "Porta de entrada (sala)", "90 x 210", "1"], ["P2", "Porta interna (quartos e banho)", "80 x 210", "3"], ["J1", "Janela (peitoril 110)", "150 x 100", "7"],
        ["J2", "Janela da sala (peitoril 110)", "100 x 100", "1"], ["J3", "Janela do banho (peitoril 160)", "90 x 60", "1"]])
textos(o + 1300, 670, ["Paredes externas 15 cm e internas 10 cm. Cotas em cm."], 12)

# FOLHA 2: instalacao eletrica (NBR 5444)
o2 = PASSO
carimbo(o2, 2, "INSTALAÇÃO ELÉTRICA - ELETRODUTOS, CONDUTORES E CIRCUITOS (NBR 5444)")
X2, Y2 = o2 + PX, PY
base(X2, Y2)
XR = o2 + 1420  # coluna da direita
textos(XR, 1370, ["LEGENDA - SIMBOLOGIA (NBR 5444)"], 18)
textos(XR, 400, ["Ponto de luz: letra = interruptor que comanda; -1- = circuito; 100W = potência.",
                 "Tomada: número = circuito; potência em VA. Eletroduto: números em cima = circuitos que",
                 "passam por ele; embaixo = seção (mm²) quando diferente de 1,5; Ø = diâmetro (mm).",
                 "Cada traço no eletroduto é um condutor: fase, neutro (com bandeira), retorno, terra (T).",
                 "Sala: 3 pontos de comando (S3 - S4 - S3). Quarto 1: 2 pontos (S3 - S3)."], 11, 20)

# FOLHA 3: fases e queda de tensao
o3 = 2 * PASSO
carimbo(o3, 3, "INSTALAÇÃO ELÉTRICA - FASES E QUEDA DE TENSÃO")
X3, Y3 = o3 + PX, PY
base(X3, Y3)
camada("PIOR-PONTO", "#ff2020")
px_, py_ = PIOR[PIOR_C][-1]
C(X3 + px_, Y3 + py_, 28); L(X3 + px_, Y3 + py_ + 28, X3 + px_ - 20, Y3 + py_ + 60)
T(X3 + px_ - 215, Y3 + py_ + 64, 12, f"PIOR PONTO C{PIOR_C}: dV {virgula(CIRC[PIOR_C]['dv'])} %")
XR3 = o3 + 1420
textos(XR3, 1370, ["LEGENDA - FASES"], 18)
for i, (cor, txt) in enumerate([(F1, "Fase F1 (C1, C3)"), (F2, "Fase F2 (C2, C4)"), (F12, "F1 + F2, 220 V (C5)")]):
    camada("LEGENDA-FASES", cor); L(XR3, 1320 - i * 38, XR3 + 40, 1320 - i * 38); textos(XR3 + 55, 1316 - i * 38, [txt], 12)
camada("PIOR-PONTO", "#ff2020"); C(XR3 + 20, 1320 - 3 * 38 + 4, 12); textos(XR3 + 55, 1316 - 3 * 38, ["Pior ponto (maior queda de tensão)"], 12)
# circuitos coloridos, cada um deslocado dentro do trecho que compartilha
for tr in TRECHOS:
    cs = sorted({c["circuito"] for c in tr["condutores"] if c.get("circuito") is not None})
    for i, k in enumerate(cs):
        d = (i - (len(cs) - 1) / 2) * 5
        cor = CIRC[k]["cor"]
        camada(f"C{k}-{CIRC[k]['fase'].replace('+', '')}", cor)
        P([(X3 + x, Y3 + y) for x, y in offset(tr["pontos"], d)])
textos(XR3, 1120, ["QUEDA DE TENSÃO POR CIRCUITO (carga toda no fim = pior caso)"], 16)
tabela(XR3, 1090, [("Circ.", 55), ("Fase", 65), ("I (A)", 60), ("L (m)", 60), ("S ini.", 60), ("dV ini.", 80), ("S adot.", 90), ("dV adot.", 90)],
       [[f"C{k}", c["fase"], virgula(c["i"]), virgula(c["l"]), sec(c["min_s"]), virgula(c["dv0"]) + " %", sec(c["s"]) + " mm²", virgula(c["dv"]) + " %"] for k, c in CIRC.items()])
textos(XR3, 860, [f"dV = 2 x {RHO} x L x I / (S x V)  (cobre 70 °C; L = trecho do QD ao ponto + {virgula(VERT)} m de descidas).",
                  "Limite: 4 % nos circuitos terminais (NBR 5410: 7 % da entrada).",
                  f"Com {sec(CIRC[3]['min_s'])} mm² a cozinha (C3) passaria de 4 % ({virgula(CIRC[3]['dv0'])} %): adotado {sec(CIRC[3]['s'])} mm².",
                  f"Pior ponto final: C{PIOR_C}, dV = {virgula(CIRC[PIOR_C]['dv'])} %.",
                  "Cada circuito segue o seu caminho; no trecho compartilhado as linhas vão lado a lado."], 11, 24)

# FOLHA 4: multifilar, cargas, quadro
o4 = 3 * PASSO
carimbo(o4, 4, "QUADRO DE DISTRIBUIÇÃO - DIAGRAMA MULTIFILAR E CARGAS", "S/E")
XA, XB, XN, XT = o4 + 600, o4 + 660, o4 + 720, o4 + 780
YT, YB = 1360, 640
textos(o4 + 100, 1400, ["DIAGRAMA MULTIFILAR DO QD (bifásico 127/220 V: 2 fases + neutro + terra)"], 22)
camada("QD-BARRAS", "#e0e0e0")
for x, nm in [(XA, "A"), (XB, "B"), (XN, "N"), (XT, "T")]:
    L(x, YT, x, YB); textos(x - 4, YT + 12, [nm], 16); textos(x - 4, YB - 28, [nm], 16)
camada("QD-DIAGRAMA", "#e0e0e0")
R(XA - 25, 1070, XN - XA + 50, 80)           # IDR
textos(XT + 40, 1105, ["IDR 63 A / 30 mA (2F + N)"], 12)
textos(XT + 40, 1290, ["Disjuntor geral 50 A bipolar"], 12)
R(XT + 60, 1170, 140, 70); textos(XT + 90, 1198, ["DPS classe II 20 kA"], 11)
for y_, x_ in [(1230, XA), (1210, XB), (1190, XN)]: L(x_, y_, XT + 60, y_)
L(XT, 1170, XT + 60, 1170)
textos(o4 + 425, 1322, ["Entrada"], 12)
# tabelas laterais alinhadas nas linhas dos circuitos
ESQ, DIR = [1, 3, 5], [2, 4]
camada("TABELAS", "#c0c0c0")
def tab_lado(xs, titulos, ks, campos):
    for i, (x0, x1) in enumerate(zip(xs, xs[1:])): pass
    ys = [1000 - 60 * i for i in range(len(ks) + 2)]
    for y in ys: L(xs[0], y, xs[-1], y)
    for x in xs: L(x, ys[0], x, ys[-1])
    camada("TEXTOS", "#e6e6e6")
    for x, t in zip(xs, titulos): T(x + 5, ys[0] - 36, 10, t)
    for j, k in enumerate(ks):
        for x, f in zip(xs, campos):
            T(x + 5, ys[j + 1] - 36, 11, f(k))
    camada("TABELAS", "#c0c0c0")
tab_lado([o4 + 100, o4 + 280, o4 + 335, o4 + 390, o4 + 440], ["LOCAL", "SEÇÃO", "PROT.", "CIRC."], ESQ,
         [lambda k: CIRC[k]["local"], lambda k: "#" + sec(CIRC[k]["s"]), lambda k: CIRC[k]["disj_curto"], lambda k: f"C{k}"])
tab_lado([o4 + 940, o4 + 990, o4 + 1045, o4 + 1100, o4 + 1280], ["CIRC.", "PROT.", "SEÇÃO", "LOCAL"], DIR,
         [lambda k: f"C{k}", lambda k: CIRC[k]["disj_curto"], lambda k: "#" + sec(CIRC[k]["s"]), lambda k: CIRC[k]["local"]])
camada("QD-CIRCUITOS", "#e0e0e0")
Y_C = {1: 910, 3: 850, 5: 790, 2: 910, 4: 850}
for k in ESQ:
    y = Y_C[k]
    if k == 5:
        L(o4 + 440, y + 5, XA, y + 5); L(o4 + 440, y - 5, XB, y - 5)
    else:
        L(o4 + 440, y, XA, y)
for k in DIR:
    y = Y_C[k]; L(XB, y, o4 + 940, y)
# cargas
textos(o4 + 100, 590, ["QUADRO DE CARGAS"], 20)
tabela(o4 + 100, 560, [("Circ.", 60), ("Descrição", 300), ("Pot (VA)", 80), ("Tensão", 70), ("Fase", 70), ("I (A)", 60), ("Disj.", 90), ("Condutor", 90), ("L (m)", 60), ("dV %", 60)],
       [[f"C{k}", c["nome"], str(c["va"]), f"{c['v']} V", c["fase"], virgula(c["i"]), c["disj"], sec(c["s"]) + " mm²", virgula(c["l"]), virgula(c["dv"])] for k, c in CIRC.items()], alt=28, h=11)
tot = sum(c["va"] for c in CIRC.values())
i1, i2 = F1_VA / 127 + I_CH, F2_VA / 127 + I_CH
milhar = lambda v: f"{v:,}".replace(",", ".")
textos(o4 + 100, 350, [f"Carga instalada: {milhar(tot)} VA ({virgula(tot / 1000)} kVA)",
                       f"F1: C1 + C3 = {milhar(F1_VA)} VA ({virgula(F1_VA / 127)} A) + chuveiro {virgula(I_CH)} A = {virgula(i1)} A",
                       f"F2: C2 + C4 = {milhar(F2_VA)} VA ({virgula(F2_VA / 127)} A) + chuveiro {virgula(I_CH)} A = {virgula(i2)} A",
                       "Geral: disjuntor 50 A bipolar + IDR 63 A / 30 mA + DPS classe II nas duas fases.",
                       f"Desequilíbrio entre fases (127 V): {virgula(abs(F1_VA - F2_VA) / max(F1_VA, F2_VA) * 100, 0)} %."], 13, 24)
# quadro em vista frontal (1:2,5)
qx, qy = o4 + 1400, 350
textos(qx, qy + 940, ["QUADRO DE DISTRIBUIÇÃO - VISTA FRONTAL (1:2,5)"], 18)
textos(qx, qy + 915, ["Caixa 32 x 45 cm, trilho DIN, 2 fileiras, embutida na sala (h = 150 cm)"], 11)
camada("QD-CAIXA", "#b0b0b0"); R(qx, qy, 640, 900); R(qx + 30, qy + 30, 580, 840)
def disj(x, y, mod, rot, cor, cor2=None):
    larg = 35 * mod
    camada("QD-DISJUNTOR", "#e8e8e8"); R(x, y, larg, 170)
    camada("QD-FASE", cor); R(x, y + 156, larg if not cor2 else larg / 2, 14)
    if cor2: camada("QD-FASE", cor2); R(x + larg / 2, y + 156, larg / 2, 14)
    camada("QD-DISJUNTOR", "#e8e8e8"); R(x + larg / 2 - 8, y + 70, 16, 45)
    camada("TEXTOS", "#e6e6e6"); T(x + larg / 2 + 6, y + 8, 12, rot, 90)
def fileira(y, itens):
    camada("QD-CAIXA", "#b0b0b0"); L(qx + 50, y + 85, qx + 590, y + 85)
    x = qx + 60
    for it in itens:
        if it[0] == "vazio":
            camada("QD-CAIXA", "#606060"); R(x, y, 35, 170); camada("TEXTOS", "#808080"); T(x + 8, y + 8, 12, "RESERVA", 90); x += 38
        else:
            disj(x, y, it[1], it[0], it[2], it[3] if len(it) > 3 else None); x += 35 * it[1] + 3
textos(qx + 50, qy + 810, ["PROTEÇÃO GERAL"], 14)
fileira(qy + 620, [("GERAL 50A 2P", 2, F1, F2), ("IDR 63A 30mA", 3, F1, F2), ("DPS F1", 1, F1), ("DPS F2", 1, F2)])
textos(qx + 50, qy + 550, ["CIRCUITOS"], 14)
fileira(qy + 350, [("C1 10A", 1, F1), ("C2 16A", 1, F2), ("C3 20A", 1, F1), ("C4 16A", 1, F2), ("C5 32A 2P", 2, F1, F2), ("vazio",), ("vazio",), ("vazio",)])
camada("QD-BARRAS", "#a0a0a0"); R(qx + 60, qy + 250, 460, 22)
camada("QD-BARRAS", "#30d030"); R(qx + 60, qy + 190, 460, 22)
textos(qx + 60, qy + 278, ["BARRA DE NEUTRO (N)"], 12); textos(qx + 60, qy + 218, ["BARRA DE TERRA (PE)"], 12)
camada("QD-BARRAS", "#e0e0e0")
for i in range(12): C(qx + 80 + i * 36, qy + 261, 6); C(qx + 80 + i * 36, qy + 201, 6)
textos(qx + 60, qy + 100, ["Entrada: F1, F2, N, PE (por cima)  |  Saídas: eletrodutos por baixo"], 12)

BASE_FCT = "\n".join(out) + "\n"

# ================================================================== SIMBOLOS E ELETRODUTOS PELO MCP
def montar(destino_fcdz, destino_dxf=None):
    m = Mcp()
    m.call("cad_novo", {"unidade": "cm"})
    for nome, cor in [("ELE-ILUMINACAO", "#ffcc00"), ("ELE-INTERRUPTORES", "#ff50ff"), ("ELE-TOMADAS", "#30d030"), ("ELE-ELETRODUTOS", "#ff8a5c"),
                      ("ELE-QUADRO", "#ff8000"), ("ELE-LEGENDA", "#e6e6e6"), ("QD-MULTIFILAR", "#e0e0e0")]:
        m.call("cad_camadas", {"acao": "criar", "nome": nome, "cor": cor})
    m.call("cad_importar", {"texto": BASE_FCT})

    def sim(nome, ponto, camada, **kw): m.call("cad_simbolo", {"nome": nome, "ponto": list(ponto), "camada": camada, **kw})
    def dut(camada, pontos, cond, X, Y, **kw): m.call("cad_eletroduto", {"pontos": [[X + x, Y + y] for x, y in pontos], "condutores": cond, "camada": camada, **kw})

    def pontos(X, Y, rotulos):
        for x, y, cmd in LUZES:
            kw = dict(comando=cmd, circuito=1, potencia=100) if rotulos else {}
            sim("ponto_luz_teto", (X + x, Y + y), "ELE-ILUMINACAO", **kw)
        for tipo, x, y, cmd in INTERRUPTORES:
            kw = dict(comando=cmd) if rotulos else {}
            sim(f"interruptor_{tipo}", (X + x, Y + y), "ELE-INTERRUPTORES", **kw)
        for x, y, parede, nivel, circ, pot in TOMADAS:
            kw = dict(circuito=circ, potencia=pot) if rotulos else {}
            sim(f"tomada_{nivel}", (X + x, Y + y), "ELE-TOMADAS", parede=parede, **kw)
        sim("quadro_geral_embutido", (X + 1, Y + 345), "ELE-QUADRO", rotacao=270, **({"potencia": "10kVA"} if rotulos else {}))

    # folha 2: tudo em NBR 5444
    for tr in TRECHOS:
        dut("ELE-ELETRODUTOS", tr["pontos"], tr["condutores"], X2, Y2, terra=tr["terra"])
    dut("ELE-ELETRODUTOS", [(0, 345), (-100, 345)], [F(None, 10, 2), N(None, 10)], X2, Y2, tipo="piso", diametro=32, secao_terra=10)
    sim("caixa_medidor", (X2 - 112, Y2 + 345), "ELE-QUADRO")
    m.call("cad_criar", {"formas": [{"tipo": "texto", "ponto": [X2 - 200, Y2 + 380], "altura": 10, "conteudo": "Alimentador (rede): 2F + N + T  #10 mm² Ø32"}], "camada": "ELE-LEGENDA"})
    pontos(X2, Y2, True)
    # folha 3: simbolos sem rotulos, sobre os circuitos coloridos
    pontos(X3, Y3, False)
    # legenda da folha 2 (mesmos simbolos)
    leg = [("tomada_baixa", "Tomada baixa (30 cm)", {}), ("tomada_media", "Tomada média (110 cm)", {}), ("tomada_alta", "Tomada alta (200 cm)", {}),
           ("tomada_piso", "Tomada no piso", {}), ("ponto_luz_teto", "Ponto de luz no teto", dict(comando="a", circuito=1, potencia=100)),
           ("ponto_luz_embutido", "Ponto de luz embutido", dict(comando="a", circuito=1, potencia=100)),
           ("interruptor_simples", "Interruptor simples", dict(comando="a")), ("interruptor_paralelo", "Interruptor paralelo (three-way)", dict(comando="a")),
           ("interruptor_intermediario", "Interruptor intermediário (four-way)", dict(comando="a")), ("quadro_geral_embutido", "Quadro geral de luz e força, embutido", {}),
           ("caixa_medidor", "Caixa para medidor", {})]
    for i, (nome, txt, kw) in enumerate(leg):
        y = 1320 - i * 50
        if nome.startswith("tomada"): sim(nome, (XR + 30, y + 18), "ELE-LEGENDA", parede="norte", **kw)
        elif nome.startswith("quadro"): sim(nome, (XR + 30, y - 8), "ELE-LEGENDA", **kw)
        else: sim(nome, (XR + 30, y), "ELE-LEGENDA", **kw)
        m.call("cad_criar", {"formas": [{"tipo": "texto", "ponto": [XR + 130, y - 4], "altura": 12, "conteudo": txt}], "camada": "ELE-LEGENDA"})
    # legenda dos condutores: um eletroduto de exemplo por tipo
    y0 = 1320 - len(leg) * 50 - 30
    exemplos = [("Eletroduto no teto/parede, Ø25", dict(tipo="teto", diametro=25), [F(None, 1.5)]),
                ("Eletroduto no piso (tracejado), Ø25", dict(tipo="piso", diametro=25), [F(None, 1.5)]),
                ("Condutor fase", {}, [F(None, 1.5)]), ("Condutor neutro", {}, [N(None, 1.5)]),
                ("Condutor de retorno", {}, [Rt(None, 1.5)]), ("Condutor terra", {}, [dict(tipo="terra", secao=1.5)])]
    for i, (txt, kw, cond) in enumerate(exemplos):
        y = y0 - i * 40
        m.call("cad_eletroduto", {"pontos": [[XR, y], [XR + 90, y]], "condutores": cond, "camada": "ELE-LEGENDA", "terra": False, "rotulo": kw.get("diametro") is not None, **kw})
        m.call("cad_criar", {"formas": [{"tipo": "texto", "ponto": [XR + 130, y - 4], "altura": 12, "conteudo": txt}], "camada": "ELE-LEGENDA"})

    # folha 4: multifilar com simbolos do MCP
    def sm(nome, x, y, **kw): sim(nome, (x, y), "QD-MULTIFILAR", **kw)
    for x in (XA, XB):
        sm("disjuntor", x, 1290, rotacao=90)
    for x in (XA, XB, XN):
        sm("disjuntor", x, 1110, rotacao=90)
    for x, y in [(XA, 1230), (XB, 1210), (XN, 1190), (XT, 1170)]: sm("no_de_ligacao", x, y)
    for k in ESQ:
        y = Y_C[k]
        if k == 5:
            sm("no_de_ligacao", XA, y + 5); sm("no_de_ligacao", XB, y - 5)
            sm("disjuntor", o4 + 540, y + 5); sm("disjuntor", o4 + 540, y - 5)
        else:
            sm("no_de_ligacao", XA, y); sm("disjuntor", o4 + 540, y)
    for k in DIR:
        y = Y_C[k]; sm("no_de_ligacao", XB, y); sm("disjuntor", o4 + 880, y)
    m.call("cad_eletroduto", {"pontos": [[o4 + 480, 1325], [XA - 5, 1325]], "condutores": [F(None, 10, 2), N(None, 10)], "secao_terra": 10, "camada": "QD-MULTIFILAR", "diametro": 32})

    m.call("cad_salvar", {"caminho": destino_fcdz, "sobrescrever": True})
    if destino_dxf: m.call("cad_salvar", {"caminho": destino_dxf, "sobrescrever": True})
    resumo = m.call("cad_resumo")
    m.fechar()
    return resumo

if __name__ == "__main__":
    open(sys.argv[1] + ".base.fct", "w").write(BASE_FCT)
    print(montar(sys.argv[1] + ".fcdz", sys.argv[1] + ".dxf"))
    print("pior:", PIOR_C, {k: (round(c['l'], 1), round(c['dv0'], 2), c['s'], round(c['dv'], 2)) for k, c in CIRC.items()})

#!/usr/bin/env python3
"""Partida estrela-triangulo classica (K1 rede, K2 triangulo, K3 estrela, rele de tempo KT): forca e comando, mm, folha A3 1:1."""
import math, sys
from mcp_cliente import Mcp

out = ["unidade mm"]
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
def T(x, y, h, txt, rot=0): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{CUR}')
def cheio(pts): out.append("H solida " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + CUR)

TH = 2.6
FORCA, COMANDO, TEXTO = "#e8e8e8", "#7fd0ff", "#e6e6e6"

def ponto(x, y, r=0.9):
    cheio([(x + r * math.cos(k * math.pi / 4), y + r * math.sin(k * math.pi / 4)) for k in range(8)])

def tracejada(x1, y1, x2, y2, tr=2.0, vao=1.2):
    d = math.hypot(x2 - x1, y2 - y1)
    ux, uy = (x2 - x1) / d, (y2 - y1) / d
    p = 0.0
    while p < d:
        q = min(p + tr, d)
        L(x1 + ux * p, y1 + uy * p, x1 + ux * q, y1 + uy * q)
        p += tr + vao

def contato(x, yt, yb, no=True, top="", bot="", nome="", lado=1):
    h = yt - yb
    a, b = yt - 0.3 * h, yb + 0.3 * h
    L(x, yt, x, a); L(x, b, x, yb)
    if no: L(x, a, x + 3.5 * lado, b + 1.6)
    else:
        L(x, a, x + 1.2 * lado, b + 0.4)
        L(x + 2.4 * lado, b + 1.8, x - 1.2 * lado, b + 1.8)
    if top: T(x + (1.6 if lado > 0 else -5.5), yt - 3.2, TH * 0.9, top)
    if bot: T(x + (1.6 if lado > 0 else -5.5), yb + 0.8, TH * 0.9, bot)
    if nome: T(x - (6.8 if lado > 0 else -3.0), (yt + yb) / 2 - 1, TH, nome)

def bobina(x, yt, yb, nome, w=12):
    R(x - w / 2, yb, w, yt - yb)
    T(x - 3.2, (yt + yb) / 2 - 1.2, TH, nome)
    L(x, yt, x, yt + 8); L(x, yb, x, yb - 8)
    T(x + 1.4, yt + 2.4, TH * 0.85, "A1"); T(x + 1.4, yb - 4.4, TH * 0.85, "A2")

def fusivel(x, yt, yb):
    L(x, yt + 8, x, yt); R(x - 2, yb, 4, yt - yb); L(x, yb, x, yb - 8)

def contator(xs, ytop, ybot, nome, dx_label=-12):
    for x, t, b in zip(xs, "135", "246"):
        C(x, ytop, 1.0); C(x, ybot, 1.0)
        contato(x, ytop - 1.0, ybot + 1.0, no=True)
        T(x - 3.4, ytop + 1.6, TH * 0.85, t); T(x - 3.4, ybot - 4, TH * 0.85, b)
    tracejada(xs[0] + 1.6, (ytop + ybot) / 2 + 0.5, xs[-1] + 1.6, (ytop + ybot) / 2 + 0.5)
    T(xs[0] + dx_label, (ytop + ybot) / 2 - 1, TH, nome)

# ---------------------------------------------------------------- folha
camada("FOLHA", "#ffffff")
R(0, 0, 420, 297); R(10, 10, 400, 277)
R(220, 10, 190, 40); L(220, 30, 410, 30); L(340, 10, 340, 30)
camada("CARIMBO", TEXTO)
T(225, 40, 4.2, "DIAGRAMA ELÉTRICO - PARTIDA ESTRELA-TRIÂNGULO")
T(225, 33, 3.0, "Circuito de força e circuito de comando (K1, K2, K3 e KT)")
T(225, 21, 3.0, "FOLHA 1/1"); T(345, 21, 3.0, "ESCALA S/E")
T(225, 13, 2.6, "FeatherOffice / FeatherCAD - unidade: mm   29/09/2026"); T(345, 13, 2.6, "Motor trifásico 3~, 6 terminais")

# ---------------------------------------------------------------- FORÇA
camada("FORÇA", FORCA)
T(30, 280, 3.6, "CIRCUITO DE FORÇA")
X = [50, 62, 74]
for x, nm in zip(X, ("L1", "L2", "L3")):
    T(x - 2.0, 271, TH, nm); L(x, 270, x, 262)
    fusivel(x, 262, 248)
    L(x, 240, x, 206)
L(X[0], 248, X[0], 240); L(X[1], 248, X[1], 240); L(X[2], 248, X[2], 240)
T(34, 253, TH, "F1")
contator(X, 205, 188, "K1")
for x in X: L(x, 187, x, 170)
# FT1: relé térmico
R(44, 152, 36, 18)
for x in X: R(x - 2.2, 154, 4.4, 14); L(x - 2.2, 156, x + 2.2, 166)
T(30, 159, TH, "FT1")
for x, t, b in zip(X, "135", "246"):
    T(x + 1.2, 171.2, TH * 0.8, t); T(x + 1.2, 149.8, TH * 0.8, b)
# nós de saída de FT1 (U1, V1, W1) e ligação ao motor
NY = {X[0]: 135, X[1]: 131, X[2]: 127}
for x in X: L(x, 152, x, 112)
for x in X: ponto(x, NY[x])
L(X[0], 112, 56, 105.4); L(X[2], 112, 68, 105.4)
# motor
MX, MY = 62, 95
C(MX, MY, 10); T(MX - 2.8, MY + 1, 5, "M"); T(MX - 4.2, MY - 5.5, 3.2, "3~")
T(43, 109, TH * 0.9, "U1"); T(59.5, 109, TH * 0.9, "V1"); T(76, 109, TH * 0.9, "W1")
L(62, 112, 62, 105)
# terminais de baixo: W2 (esq.), U2 (meio), V2 (dir.)
L(56, 85.4, 50, 80); L(62, 85, 62, 80); L(68, 85.4, 74, 80)
T(43, 77, TH * 0.9, "W2"); T(64, 79, TH * 0.9, "U2"); T(76, 77, TH * 0.9, "V2")
L(50, 80, 50, 76); L(62, 80, 62, 72); L(74, 80, 74, 68)
# K2 (triângulo): U1->W2, V1->U2, W1->V2
K2X, K3X = [100, 112, 124], [150, 162, 174]
for x, (nx, ny), kx in zip(X, [(X[0], NY[X[0]]), (X[1], NY[X[1]]), (X[2], NY[X[2]])], K2X):
    L(nx, ny, kx, ny); L(kx, ny, kx, 121)
contator(K2X, 120, 103, "K2")
Y_TAP = [76, 72, 68]
for kx, ty, mx in zip(K2X, Y_TAP, X):
    L(kx, 102, kx, ty); ponto(kx, ty)
# taps dos terminais 4,5,6 até K3 (estrela)
for mx, ty, kx in zip(X, Y_TAP, K3X):
    L(mx, ty, kx, ty); ponto(mx, ty)
    L(kx, ty, kx, 65)
contator(K3X, 64, 47, "K3")
for x in K3X: L(x, 46, x, 42)
L(K3X[0], 42, K3X[2], 42); ponto(K3X[1], 42)
T(178, 41, TH, "ponto de estrela")
T(85, 140, TH * 0.9, "delta: U1-W2, V1-U2, W1-V2")

# ---------------------------------------------------------------- COMANDO
camada("COMANDO", COMANDO)
T(245, 280, 3.6, "CIRCUITO DE COMANDO")
X1, XK, X3, X4, X5 = 270, 300, 320, 350, 383.5
L(262, 275, 350, 275); ponto(X1, 275); T(352, 273, TH, "Fase (127 V)")
L(262, 62, 400, 62); T(402, 60, TH * 0.9, "N")
# F7 (NF), S0 (NF)
L(X1, 275, X1, 265); contato(X1, 265, 250, no=False, top="95", bot="96", nome="F7", lado=-1)
L(X1, 250, X1, 246); contato(X1, 246, 236, no=False, top="1", bot="2", nome="S0  E", lado=-1)
L(X1, 236, X1, 232); T(X1 + 2, 229.5, TH * 0.9, "3")
L(X1, 232, XK, 232); ponto(X1, 232); ponto(XK, 232)
# S1 (NA) em paralelo com o selo K1 (13-14)
contato(X1, 232, 212, no=True, top="3", bot="4", nome="S1  E", lado=-1)
contato(XK, 232, 212, no=True, top="13", bot="14", nome="K1", lado=1)
L(X1, 212, X1, 205); L(XK, 212, XK, 205); L(X1, 205, X5, 205)
for x in (X1, XK, X3, X4, X5): ponto(x, 205)
# coluna KT (temporizador)
L(X1, 205, X1, 96); bobina(X1, 96, 84, "KT")
# coluna K3: K2 NF (21-22) + KT NF temporizado (55-56) -> K3
L(X3, 205, X3, 196); contato(X3, 196, 176, no=False, top="21", bot="22", nome="K2", lado=1)
L(X3, 176, X3, 160); contato(X3, 160, 140, no=False, top="55", bot="56", nome="KT", lado=1)
L(X3, 140, X3, 96); bobina(X3, 96, 84, "K3")
# coluna K2: K3 NF (21-22) + KT NA temporizado (67-68) -> K2
L(X4, 205, X4, 196); contato(X4, 196, 176, no=False, top="21", bot="22", nome="K3", lado=1)
L(X4, 176, X4, 160); contato(X4, 160, 140, no=True, top="67", bot="68", nome="KT", lado=1)
L(X4, 140, X4, 96); bobina(X4, 96, 84, "K2")
# coluna K1: K3 NA (13-14) ou K2 NA (13-14) em paralelo -> K1
L(X5, 205, X5, 190); L(X5 - 8.5, 190, X5 + 8.5, 190); ponto(X5, 190)
L(X5 - 8.5, 190, X5 - 8.5, 182); L(X5 + 8.5, 190, X5 + 8.5, 182)
contato(X5 - 8.5, 182, 162, no=True, top="13", bot="14", nome="K3", lado=-1)
contato(X5 + 8.5, 182, 162, no=True, top="13", bot="14", nome="K2", lado=1)
L(X5 - 8.5, 162, X5 - 8.5, 156); L(X5 + 8.5, 162, X5 + 8.5, 156); L(X5 - 8.5, 156, X5 + 8.5, 156); ponto(X5, 156)
L(X5, 156, X5, 96); bobina(X5, 96, 84, "K1")
for x in (X1, X3, X4, X5): L(x, 76, x, 62); ponto(x, 62)

# ---------------------------------------------------------------- legenda
camada("TEXTOS", TEXTO)
T(15, 30, 2.6, "Sequência: S1 liga K1, KT e K3 (estrela). Vencido o tempo de KT, K3 desliga e K2 liga (triângulo).")
T(15, 25, 2.6, "K2 e K3 se intertravam pelos contatos NF 21-22. F7 protege o comando; FT1, o motor.")
T(15, 20, 2.6, "Numeração dos contatos e bobinas (A1 alto / A2 baixo) conforme os relés usuais.")

BASE = "\n".join(out) + "\n"

def montar(fcdz, dxf):
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": BASE})
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1]
    r = montar(base + ".fcdz", base + ".dxf")
    print(r["total_de_entidades"], r["por_tipo"])

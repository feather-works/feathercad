#!/usr/bin/env python3
"""Diagrama de forca e comando (estrela-triangulo, conforme a referencia do usuario), em mm, folha A3 1:1."""
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
def A(x, y, r, a1, a2): out.append(f"A {n(x)} {n(y)} {n(r)} {n(a1)} {n(a2)}{CUR}")
def T(x, y, h, txt, rot=0): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{CUR}')
def cheio(pts): out.append("H solida " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + CUR)

TH = 2.6       # altura de texto (mm, papel 1:1)
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

def contato(x, yt, yb, no=True, top="", bot="", nome="", lado=1, marca=True):
    """Contato vertical: terminais em yt (cima) e yb (baixo). NA: alavanca solta; NF: alavanca encostada + traço."""
    h = yt - yb
    a, b = yt - 0.3 * h, yb + 0.3 * h
    L(x, yt, x, a); L(x, b, x, yb)
    if no:
        L(x, a, x + 3.5 * lado, b + 1.6)
    else:
        L(x, a, x + 1.2 * lado, b + 0.4)
        L(x + 2.4 * lado, b + 1.8, x - 1.2 * lado, b + 1.8)   # traço do contato NF
    if top: T(x + (1.6 * lado if lado > 0 else -5.5), yt - 3.2, TH * 0.9, top)
    if bot: T(x + (1.6 * lado if lado > 0 else -5.5), yb + 0.8, TH * 0.9, bot)
    if nome: T(x - (6.8 if lado > 0 else -3.0), (yt + yb) / 2 - 1, TH, nome)

def bobina(x, yt, yb, nome, w=14):
    R(x - w / 2, yb, w, yt - yb)
    T(x - 3.2, (yt + yb) / 2 - 1.2, TH, nome)
    L(x, yt, x, yt + 8); L(x, yb, x, yb - 8)
    T(x + 1.4, yt + 2.4, TH * 0.85, "A2"); T(x + 1.4, yb - 4.4, TH * 0.85, "A1")

def fusivel(x, yt, yb):
    L(x, yt + 8, x, yt); R(x - 2, yb, 4, yt - yb); L(x, yb, x, yb - 8)

def enrolamento(x, yt, n_esp=4, r=2.6):
    L(x, yt, x, yt - 2)
    y = yt - 2
    for _ in range(n_esp):
        A(x, y - r, r, 270, 90)
        y -= 2 * r
    L(x, y, x, y - 2)
    return y - 2

# ---------------------------------------------------------------- folha
camada("FOLHA", "#ffffff")
R(0, 0, 420, 297); R(10, 10, 400, 277)
R(220, 10, 190, 40); L(220, 30, 410, 30); L(340, 10, 340, 30)
camada("CARIMBO", TEXTO)
T(225, 40, 4.2, "DIAGRAMA ELÉTRICO - PARTIDA (REFERÊNCIA COM T1)")
T(225, 33, 3.0, "Reprodução do diagrama de referência (enrolamentos T1)")
T(225, 21, 3.0, "FOLHA 1/1"); T(345, 21, 3.0, "ESCALA S/E"); T(225, 13, 2.6, "FeatherOffice / FeatherCAD - unidade: mm   29/09/2026"); T(345, 13, 2.6, "Motor trifásico 3~")

# ---------------------------------------------------------------- FORÇA
camada("FORÇA", FORCA)
T(30, 280, 3.6, "CIRCUITO DE FORÇA")
XR, XS, XT = 50, 62, 74
for x, nm in [(XR, "R"), (XS, "S"), (XT, "T")]:
    T(x - 1.2, 271, TH, nm); L(x, 270, x, 262)
    fusivel(x, 262, 248)
    L(x, 240, x, 240)
T(34, 253, TH, "F1")
# derivacoes para K2 (R fica mais alta e vai mais longe: sem cruzar)
K2X = [150, 170, 190]
for (x, y, xk) in [(XR, 235, K2X[2]), (XS, 227, K2X[1]), (XT, 219, K2X[0])]:
    L(x, 248, x, 206)
    ponto(x, y)
    L(x, y, xk, y); L(xk, y, xk, 206)
# K1: contatos principais (50,62,74)
def contator(xs, ytop, ybot, nome, dx_label=-16):
    for x, t, b in zip(xs, ["1", "3", "5"], ["2", "4", "6"]):
        C(x, ytop, 1.0); C(x, ybot, 1.0)
        contato(x, ytop - 1.0, ybot + 1.0, no=True, top="", bot="")
        T(x - 3.4, ytop + 1.6, TH * 0.85, t); T(x - 3.4, ybot - 4, TH * 0.85, b)
    tracejada(xs[0] + 1.6, (ytop + ybot) / 2 + 0.5, xs[-1] + 1.6, (ytop + ybot) / 2 + 0.5)
    T(xs[0] + dx_label, (ytop + ybot) / 2 - 1, TH, nome)
contator([XR, XS, XT], 205, 188, "K1")
contator(K2X, 205, 188, "K2", dx_label=-12)
# saidas de K1 (nos) e retorno ao relé térmico
for x in (XR, XS, XT): L(x, 187, x, 172)
# FT1 (relé térmico)
R(44, 152, 36, 18)
for x in (XR, XS, XT):
    R(x - 2.2, 154, 4.4, 14); L(x - 2.2, 156, x + 2.2, 166)
T(30, 159, TH, "FT1")
for x, t, b in zip((XR, XS, XT), "135", "246"):
    T(x + 1.2, 171.2, TH * 0.8, t); T(x + 1.2, 149.8, TH * 0.8, b)
L(XS, 152, XS, 95)
for x, xc in ((XR, 56), (XT, 68)):
    L(x, 152, x, 100); L(x, 100, xc, 94.6)
# derivacoes de K1 para os taps dos enrolamentos T1
ponto(XR, 181); ponto(XS, 177); ponto(XT, 173)
# T1 (três enrolamentos)
ends = []
for x, (xa, ya) in zip(K2X, [(XR, 181), (XS, 177), (XT, 173)]):
    L(x, 187, x, 170)
    y_fim = enrolamento(x, 170)
    ends.append((x, y_fim))
    L(xa, ya, x, ya)
    ponto(x, ya)
T(140, 158, TH, "T1")
# K3 (fecha o ponto de estrela nos enrolamentos)
for x, yf in ends: L(x, yf, x, 125)
contator(K2X, 125, 108, "K3", dx_label=-12)
for x in K2X: L(x, 107, x, 100)
L(K2X[0], 100, K2X[2], 100); ponto(K2X[1], 100)
# motor
C(62, 85, 10); T(59.2, 86, 5, "M"); T(58, 79.5, 3.2, "3~")

# ---------------------------------------------------------------- COMANDO
camada("COMANDO", COMANDO)
T(245, 280, 3.6, "CIRCUITO DE COMANDO")
XL, X1, X2, X3, X4 = 250, 270, 300, 330, 360
L(XL, 275, 350, 275); ponto(X1, 275)
L(XL, 62, XL, 275)                # retorno à esquerda
L(XL, 62, 380, 62)
# chave de sobrecarga F7 (contato NF do relé térmico)
L(X1, 275, X1, 265); contato(X1, 265, 250, no=False, top="95", bot="96", nome="F7", lado=-1)
T(X1 - 12, 256, TH * 0.9, "M")
# S0 (botão NF)
L(X1, 250, X1, 246); contato(X1, 246, 236, no=False, top="1", bot="2", nome="S0  E", lado=-1)
L(X1, 236, X1, 232); T(X1 + 2, 229.5, TH * 0.9, "3")
# barra em y=232
L(X1, 232, 380, 232); ponto(X1, 232); ponto(X2, 232); ponto(X3, 232)
# S1 (NA) + selo K2 (13-14) + K3 (13-14), em paralelo até y=205
contato(X1, 232, 212, no=True, top="3", bot="4", nome="S1  E", lado=-1)
contato(X2, 232, 212, no=True, top="13", bot="14", nome="K2", lado=1)
contato(X3, 232, 212, no=True, top="13", bot="14", nome="K3", lado=1)
contato(380, 232, 212, no=True, top="43", bot="44", nome="K2", lado=-1)
L(X1, 212, X1, 205); L(X2, 212, X2, 205); L(X3, 212, X3, 205); L(X1, 205, X3, 205)
ponto(X1, 205); ponto(X2, 205); ponto(X3, 205)
# ---- colunas: K6 (X1), K3 (X2), K1 (X3), K2 (X4)
L(X1, 205, X1, 96)
bobina(X1, 96, 84, "K6", 12)
# K6 temporizado 17-18 -> coluna K3 (com K1 NF 21-22)
L(X2, 205, X2, 196); contato(X2, 196, 172, no=True, top="17", bot="18", nome="K6", lado=1)
L(X2, 172, X2, 152); contato(X2, 152, 134, no=False, top="21", bot="22", nome="K1", lado=1)
L(X2, 134, X2, 96); bobina(X2, 96, 84, "K3", 12)
# K6 temporizado 27-28 (NF) -> coluna K1 (com K3 NF 21-22)
L(X3, 205, X3, 196); contato(X3, 196, 172, no=False, top="27", bot="28", nome="K6", lado=1)
L(X3, 172, X3, 152); contato(X3, 152, 134, no=False, top="21", bot="22", nome="K3", lado=1)
L(X3, 134, X3, 96); bobina(X3, 96, 84, "K1", 12)
# K2 (triângulo): alimenta pelo nó da direita, com K1 NF 31-32
L(X3, 205, X4, 205); ponto(X3, 205)
L(X4, 205, X4, 152); contato(X4, 152, 134, no=False, top="31", bot="32", nome="K1", lado=1)
L(X4, 134, X4, 96); bobina(X4, 96, 84, "K2", 12)
# nós do retorno inferior
for x in (X1, X2, X3, X4): L(x, 76, x, 62); ponto(x, 62)
# chave de F7 indicada no lado esquerdo e observação
T(15, 15, 2.6, "Bobinas: A2 (alto) / A1 (baixo). Contatos NA/NF conforme a numeração.")

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

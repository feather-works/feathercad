#!/usr/bin/env python3
"""Terreno do sitio (contorno da foto de satelite) em metros, folha A3 a 1:1000 (1 mm de papel = 1 m)."""
import math, sys
from mcp_cliente import Mcp

# ---------------------------------------------------------------- contorno tirado da foto (pixels, y para baixo)
PX = [(435, 25), (555, 180), (519, 222), (477, 307), (540, 385), (308, 620), (80, 428), (270, 127)]
FRENTE_M = 72.5                              # medida da frente (lado P1-P2), dada pelo usuario
S = FRENTE_M / math.dist(PX[0], PX[1])      # metros por pixel
NOME_V = [f"V{i + 1}" for i in range(8)]

def para_m(p):  # pixel -> metros, y para cima
    return (p[0] * S, -p[1] * S)
V = [para_m(p) for p in PX]
minx = min(v[0] for v in V); maxy = max(v[1] for v in V)
OX, OY = 30, 268                             # onde o canto superior esquerdo do terreno cai na folha
V = [(x - minx + OX, y - maxy + OY) for x, y in V]

def area(pts):
    return abs(sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))) / 2
LADOS = [math.dist(V[i], V[(i + 1) % 8]) for i in range(8)]
PERIM, AREA = sum(LADOS), area(V)

# construcoes que aparecem na foto dentro do contorno: (nome, centro px, largura px, altura px, giro graus)
CONSTR = [("Loja de artesanato", (413, 93), 42, 24, 33), ("Marcenaria", (292, 128), 28, 24, 33)]
PISCINA = ((453, 155), 20, 13, 41)

# ---------------------------------------------------------------- .fct
out = ["unidade m"]
LAYERS, CUR = {}, ""
def n(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s
def camada(nome, cor):
    global CUR
    LAYERS.setdefault(nome, cor)
    out.append(f"camada {nome} {LAYERS[nome]}")
    CUR = "" if cor == LAYERS[nome] else " " + cor
def L(a, b, c, d): out.append(f"L {n(a)} {n(b)} {n(c)} {n(d)}{CUR}")
def Rr(x, y, w, h): out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)}{CUR}")
def Rg(x, y, w, h, g): out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)} {n(g)}{CUR}")
def C(x, y, r): out.append(f"C {n(x)} {n(y)} {n(r)}{CUR}")
def T(x, y, h, txt, rot=0): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{CUR}')
def P(pts, f=False): out.append("P " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + (" f" if f else "") + CUR)
def virgula(v, c=2): return f"{v:,.{c}f}".replace(",", "X").replace(".", ",").replace("X", ".")
def cheio(pts): out.append("H solida " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + CUR)

TEXTO = "#e6e6e6"
# folha A3 (mm de papel = m no desenho a 1:1000)
camada("FOLHA", "#ffffff"); Rr(0, 0, 420, 297); Rr(10, 10, 400, 277)
Rr(220, 10, 190, 40); L(220, 30, 410, 30); L(340, 10, 340, 30)
camada("CARIMBO", TEXTO)
T(225, 40, 4.2, "TERRENO DO SÍTIO - LEVANTAMENTO DO CONTORNO")
T(225, 33, 3.0, "Planta de situação com as edificações existentes")
T(225, 21, 3.0, "FOLHA 1/1"); T(345, 21, 3.0, "ESCALA 1:1000")
T(225, 13, 2.6, "FeatherOffice / FeatherCAD - unidade: m   29/09/2026"); T(345, 13, 2.6, "Contorno a partir de imagem de satélite")

# vias: avenida (frente) e rua lateral
def unit(a, b):
    d = math.dist(a, b); return ((b[0] - a[0]) / d, (b[1] - a[1]) / d)
u = unit(V[0], V[1])                         # ao longo da frente
sgn = 1 if area([V[0], V[1], V[2]]) else 1
# normal para fora (para a direita da frente, lado da avenida)
nrm = (u[1], -u[0])
if (nrm[0] * (V[3][0] - V[0][0]) + nrm[1] * (V[3][1] - V[0][1])) > 0: nrm = (-nrm[0], -nrm[1])   # aponta para longe do interior
camada("VIAS", "#7f7f7f")
def desl(p, d): return (p[0] + nrm[0] * d, p[1] + nrm[1] * d)
A0 = (V[0][0] - u[0] * 12, V[0][1] - u[1] * 12); A1 = (V[1][0] + u[0] * 75, V[1][1] + u[1] * 75)
for d in (3, 16): L(*desl(A0, d), *desl(A1, d))
camada("TEXTOS", TEXTO)
mid = ((V[0][0] + V[1][0]) / 2, (V[0][1] + V[1][1]) / 2)
ang_frente = math.degrees(math.atan2(u[1], u[0]))
pos_av = desl(mid, 8.5)
T(pos_av[0] - 10 * u[0] - 0.8 * nrm[0], pos_av[1] - 10 * u[1] - 1.4, 3.0, "AVENIDA (frente)", ang_frente)
camada("TEXTOS", TEXTO)
b = desl(V[0], 24)
pass

# terreno
camada("TERRENO", "#5fd35f")
P(V, True)
for i, v in enumerate(V):
    C(v[0], v[1], 1.0)
    camada("TEXTOS", TEXTO)
    # rotulo do vertice, para fora do polígono
    cx = sum(p[0] for p in V) / 8; cy = sum(p[1] for p in V) / 8
    d = unit((cx, cy), v)
    T(v[0] + d[0] * 3 - 1.5, v[1] + d[1] * 3 - 1, 2.6, NOME_V[i])
    camada("TERRENO", "#5fd35f")

# cotas dos lados (texto alinhado ao lado, para fora)
camada("COTAS", "#f0a030")
cx = sum(p[0] for p in V) / 8; cy = sum(p[1] for p in V) / 8
for i in range(8):
    a, b = V[i], V[(i + 1) % 8]
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    uu = unit(a, b)
    ang = math.degrees(math.atan2(uu[1], uu[0]))
    if ang > 90 or ang <= -90: ang += 180
    nn = (-uu[1], uu[0])
    if nn[0] * (m[0] - cx) + nn[1] * (m[1] - cy) < 0: nn = (-nn[0], -nn[1])
    txt = f"{virgula(LADOS[i])} m"
    off = 3.2 if LADOS[i] > 30 else 3.2
    px_, py_ = m[0] + nn[0] * off, m[1] + nn[1] * off
    # alinha o texto pelo centro
    hw = len(txt) * 2.6 * 0.3
    tx = px_ - math.cos(math.radians(ang)) * hw + (-math.sin(math.radians(ang))) * 0
    ty = py_ - math.sin(math.radians(ang)) * hw
    if LADOS[i] > 15:
        T(tx, ty, 2.6, txt, ang)

# construcoes (retangulos girados) e piscina
def retang(cpx, w_px, h_px, giro):
    cxm, cym = (lambda p: (p[0] * S - minx + OX, -p[1] * S - maxy + OY))(cpx)
    w, h = w_px * S, h_px * S
    g = math.radians(giro)
    # canto inferior esquerdo, a partir do centro
    x0 = cxm - (w / 2) * math.cos(g) + (h / 2) * math.sin(g)
    y0 = cym - (w / 2) * math.sin(g) - (h / 2) * math.cos(g)
    return x0, y0, w, h, giro, cxm, cym
camada("EDIFICACOES", "#ff8a5c")
for nome, c, w, h, g in CONSTR:
    x0, y0, ww, hh, gg, cxm, cym = retang(c, w, h, g)
    Rg(x0, y0, ww, hh, gg)
    camada("TEXTOS", TEXTO); T(cxm - len(nome) * 0.42, cym - 0.8, 1.8, nome); camada("EDIFICACOES", "#ff8a5c")
# ---- duas casas (foto ampliada): eixo a 40,7 graus, escala 0,0865 m/px tirada das marcas de 20 m e 40 m
ANG = math.radians(40.7)
U = (math.cos(ANG), math.sin(ANG))            # ao longo da faixa das casas (sobe para a direita)
N = (math.sin(ANG), -math.cos(ANG))           # através da faixa (para o sudeste)
def em_metros(p): return (p[0] * S - minx + OX, -p[1] * S - maxy + OY)
CENTRO_VERMELHA = em_metros((495, 200))
def casa_rot(nome, centro, ao_longo, atraves, cor="#ff8a5c", dims=True):
    cx_, cy_ = centro
    cantos = []
    for a, b in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
        cantos.append((cx_ + U[0] * a * ao_longo / 2 + N[0] * b * atraves / 2, cy_ + U[1] * a * ao_longo / 2 + N[1] * b * atraves / 2))
    camada("EDIFICACOES", cor); P(cantos, True)
    camada("TEXTOS", TEXTO)
    if dims:
        T(cx_ - len(nome) * 0.42, cy_ + 0.8, 1.8, nome)
        T(cx_ - 4.2, cy_ - 1.8, 1.8, f"{virgula(ao_longo, 1)} x {virgula(atraves, 1)} m")
    else:  # peça pequena: rótulo ao lado, fora do retângulo
        T(cx_ + N[0] * (atraves / 2 + 1.5) - 1.5, cy_ + N[1] * (atraves / 2 + 1.5) - 2.0, 1.8, nome)
    return cantos
def desloca(c, d_ao_longo, d_atraves): return (c[0] + U[0] * d_ao_longo + N[0] * d_atraves, c[1] + U[1] * d_ao_longo + N[1] * d_atraves)
casa_rot("Casa 2 (telhado vermelho)", CENTRO_VERMELHA, 8.1, 11.9)
casa_rot("Casa 1 (comprida)", desloca(CENTRO_VERMELHA, -25.0, -1.1), 32.0, 11.2)
casa_rot("Anexo", desloca(CENTRO_VERMELHA, -6.55, -1.0), 4.9, 7.8, "#ffb38a", dims=False)

c, w, h, g = PISCINA
x0, y0, ww, hh, gg, cxm, cym = retang(c, w, h, g)
camada("PISCINA", "#3aa0ff"); Rg(x0, y0, ww, hh, gg)
camada("TEXTOS", TEXTO); T(cxm - 4, cym - 1, 2.2, "Piscina")

# norte e escala grafica
camada("NORTE", TEXTO)
nx, ny = 385, 150
L(nx, ny - 14, nx, ny + 14); P([(nx - 4, ny + 6), (nx, ny + 14), (nx + 4, ny + 6)]); C(nx, ny, 9)
T(nx - 2, ny + 17, 4.5, "N")
camada("ESCALA", TEXTO)
ex, ey = 240, 62
for i in range(4):
    Rr(ex + i * 12.5, ey, 12.5, 2);
cheio([(ex, ey), (ex + 12.5, ey), (ex + 12.5, ey + 2), (ex, ey + 2)]); cheio([(ex + 25, ey), (ex + 37.5, ey), (ex + 37.5, ey + 2), (ex + 25, ey + 2)])
for i, t in enumerate(["0", "12,5", "25", "37,5", "50 m"]): T(ex + i * 12.5 - 1.5, ey + 3.5, 2.4, t)
T(ex, ey - 5, 2.6, "Escala gráfica (1:1000)")

# quadro de lados
camada("TABELAS", "#c0c0c0")
tx0, ty0 = 236, 274
cols = [("Lado", 22), ("Vértices", 42), ("Comprimento (m)", 44), ("Obs.", 62)]
larg = sum(w for _, w in cols)
linhas = []
obs = {0: "FRENTE (avenida)", 1: "avenida", 2: "quebra", 3: "quebra", 4: "divisa lateral", 5: "divisa (fundos)", 6: "divisa", 7: "divisa"}
for i in range(8):
    linhas.append([f"{i + 1}", f"{NOME_V[i]} - {NOME_V[(i + 1) % 8]}", virgula(LADOS[i]), obs[i]])
linhas.append(["", "PERÍMETRO", virgula(PERIM), ""]); linhas.append(["", "ÁREA (m²)", virgula(AREA), f"{virgula(AREA / 10000, 4)} ha"])
alt = 7
nl = len(linhas) + 1
for i in range(nl + 1): L(tx0, ty0 - i * alt, tx0 + larg, ty0 - i * alt)
x = tx0
for _, w in cols + [("", 0)]:
    L(x, ty0, x, ty0 - nl * alt); x += w
camada("TEXTOS", TEXTO)
x = tx0
for t, w in cols: T(x + 1.5, ty0 - alt + 2.2, 2.4, t); x += w
for j, lin in enumerate(linhas):
    x = tx0
    for (t, w), v in zip(cols, lin): T(x + 1.5, ty0 - (j + 2) * alt + 2.2, 2.4, v); x += w
T(tx0, ty0 - nl * alt - 6, 2.4, "Frente = 72,50 m (informada). O 606,00 m do mapa é o perímetro; aqui o perímetro vale a soma dos lados.")
T(tx0, ty0 - nl * alt - 10, 2.4, "Contorno tirado da imagem de satélite; construções em posição aproximada.")
T(tx0, ty0 - nl * alt - 18, 2.4, "Casas 1 e 2: medidas da foto ampliada (0,0865 m/px, marcas de 20 m e 40 m); eixo a 40,7°.")
T(tx0, ty0 - nl * alt - 14, 2.4, "Frente para a avenida, a nordeste; a via lateral (R. das Ferra…) cruza a avenida perto do vértice V1.")

BASE = "\n".join(out) + "\n"

def montar(fcdz, dxf):
    m = Mcp()
    m.call("cad_novo", {"unidade": "m"})
    m.call("cad_importar", {"texto": BASE})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "TERRENO", "espessura": 0.5})
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1]
    r = montar(base + ".fcdz", base + ".dxf")
    print(r["total_de_entidades"], r["por_tipo"])
    print("escala m/px", round(S, 4), "lados", [round(l, 2) for l in LADOS], "perímetro", round(PERIM, 2), "área", round(AREA, 1), "m²")

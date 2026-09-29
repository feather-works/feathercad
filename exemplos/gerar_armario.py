#!/usr/bin/env python3
"""Armario de cozinha integrada com area de servico (versao 5): 4 pranchas + 3 folhas de tabelas, A3 a 1:20, em mm."""
import math, os, sys
from mcp_cliente import Mcp

W, H, PASSO = 8400, 5940, 9000     # A3 a 1:20 (mm de desenho); distancia entre folhas
TXT, PEQ, TIT = 42, 34, 84         # alturas de texto: normal, pequena, titulo (mm de desenho)
SETA = 60                          # comprimento da ponta de seta

out = ["unidade mm"]
LAYERS, CUR = {}, ""
def n(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s
IMPRESSAO = os.environ.get("IMPRESSAO") == "1"   # cópia para papel: linhas escuras, preenchimentos claros
FILL_IMP = {"R1": "#9a9a9a", "APARELHOS": "#c4c4c4", "PEDRA2": "#dcdcdc", "CABECALHO": "#c8c8c8"}
def cor_imp(nome, cor):
    if nome in FILL_IMP: return FILL_IMP[nome]
    if nome == "COTAS": return "#a04800"
    if nome in ("TABELAS", "PEDRA"): return "#555555"
    return "#000000"
def camada(nome, cor):
    global CUR
    if IMPRESSAO: cor = cor_imp(nome, cor)
    LAYERS.setdefault(nome, cor)
    out.append(f"camada {nome} {LAYERS[nome]}")
    CUR = "" if cor == LAYERS[nome] else " " + cor
def L(a, b, c, d): out.append(f"L {n(a)} {n(b)} {n(c)} {n(d)}{CUR}")
def R(x, y, w, h): out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)}{CUR}")
def T(x, y, h, txt, rot=0): out.append(f'T {n(x)} {n(y)} {n(h)} {n(rot)} "{txt}"{CUR}')
def cheio(x, y, w, h): out.append(f"H solida {n(x)} {n(y)} {n(x + w)} {n(y)} {n(x + w)} {n(y + h)} {n(x)} {n(y + h)}{CUR}")
def largura(txt, h): return len(txt) * h * 0.56
def centro(x, y, txt, h=TXT, rot=0):
    """Texto centrado no ponto (x, y), na horizontal (ou a 90 graus)."""
    w = largura(txt, h)
    if rot == 0: T(x - w / 2, y - h / 3, h, txt)
    else: T(x + h / 3, y - w / 2, h, txt, 90)

# ---------------------------------------------------------------- cotas
def seta(x, y, dx, dy):
    """Ponta de seta aberta em (x, y) apontando na direcao (dx, dy)."""
    nx, ny = -dy, dx
    L(x, y, x - dx * SETA + nx * SETA / 3, y - dy * SETA + ny * SETA / 3)
    L(x, y, x - dx * SETA - nx * SETA / 3, y - dy * SETA - ny * SETA / 3)
def cota_h(x1, x2, y, txt, h=PEQ, ext=0, sobre=True):
    camada("COTAS", "#f0a030")
    L(x1, y, x2, y)
    if abs(x2 - x1) > 3 * SETA:
        seta(x1, y, -1, 0); seta(x2, y, 1, 0)
    else:
        seta(x1, y, 1, 0); seta(x2, y, -1, 0)
    if ext:
        L(x1, y - ext / 2, x1, y + ext / 2); L(x2, y - ext / 2, x2, y + ext / 2)
    camada("TEXTOS", "#e6e6e6")
    centro((x1 + x2) / 2, y + h * 0.9 if sobre else y - h * 0.9, txt, h)
def cota_v(x, y1, y2, txt, h=PEQ, ext=0, lado=-1):
    camada("COTAS", "#f0a030")
    L(x, y1, x, y2)
    if abs(y2 - y1) > 3 * SETA:
        seta(x, y1, 0, -1); seta(x, y2, 0, 1)
    else:
        seta(x, y1, 0, 1); seta(x, y2, 0, -1)
    if ext:
        L(x - ext / 2, y1, x + ext / 2, y1); L(x - ext / 2, y2, x + ext / 2, y2)
    camada("TEXTOS", "#e6e6e6")
    centro(x + lado * h * 0.9, (y1 + y2) / 2, txt, h, 90)

# ---------------------------------------------------------------- folha
def folha(i, titulo_carimbo, sub_carimbo=""):
    ox = i * PASSO
    camada("FOLHA", "#ffffff")
    R(ox, 0, W, H); R(ox + 200, 200, W - 400, H - 400)
    bx, by = ox + W - 200 - 3800, 200
    R(bx, by, 3800, 800)
    L(bx, by + 400, bx + 3800, by + 400); L(bx + 2400, by, bx + 2400, by + 400)
    camada("CARIMBO", "#e0e0e0")
    T(bx + 100, by + 620, 78, "COZINHA INTEGRADA COM AREA DE SERVICO - VERSAO 5 (FINAL)")
    T(bx + 100, by + 450, 56, titulo_carimbo)
    T(bx + 100, by + 250, 52, "FeatherOffice / FeatherCAD - unidade: mm")
    T(bx + 2500, by + 250, 52, f"FOLHA {i + 1:02d}/07   ESCALA 1:20")
    T(bx + 100, by + 90, 40, sub_carimbo or "Cotas em mm - regra zero: nao arredondar, nao recalcular.")
    return ox

SUB = "Cozinha integrada com area de servico - VERSAO 5 (final) - R1 + I1-I7 + M2-M7 + S1-S7 + Z1 | pe-direito real 2540mm | cotas em mm"

def caixa(x, y, w, h, nome="", cam="MODULOS", cor="#e6e6e6", tam=TXT, rot=0):
    camada(cam, cor); R(x, y, w, h)
    if nome:
        camada("TEXTOS", "#e6e6e6"); centro(x + w / 2, y + h / 2, nome, tam, rot)

# ================================================================ PRANCHA 1: vista frontal
MOD = {  # nome: (x1, x2)  (mm, a partir da extremidade oeste)
    "R1": (0, 68), "I1": (68, 848), "I2": (866, 1376), "I3": (1394, 1994), "I4": (2012, 2918), "I5": (2936, 3576), "I6": (3594, 4224), "I7": (4242, 4682)}
def prancha1():
    ox = folha(0, "PRANCHA 1/4 - VISTA FRONTAL - PAREDE NORTE (V5)", SUB)
    X, Y = ox + 1900, 2700
    # borda geral
    camada("CONTORNO", "#e6e6e6"); R(X, Y, 4700, 2540)
    # R1
    camada("R1", "#5f5f5f"); cheio(X, Y, 68, 2540); camada("CONTORNO", "#e6e6e6"); R(X, Y, 68, 2540)
    camada("TEXTOS", "#e6e6e6"); centro(X + 34, Y + 1270, "R1", TXT, 90)
    # geladeira + S1
    camada("APARELHOS", "#6f6f6f"); cheio(X + 68, Y, 780, 2000)
    camada("CONTORNO", "#e6e6e6"); R(X + 68, Y, 780, 2000)
    camada("TEXTOS", "#e6e6e6")
    for k, t in enumerate(["geladeira", "BRE66AK", "(sem rodape)"]): centro(X + 458, Y + 1030 - k * 60, t, PEQ)
    caixa(X + 68, Y + 2018, 780, 522, "S1", tam=54)
    # inferiores (150-900), medianos (1600-1950) e superiores (1950-2540)
    for nome in ("I2", "I3", "I6", "I7"):
        x1, x2 = MOD[nome]; caixa(X + x1, Y + 150, x2 - x1, 750, nome, tam=54)
    for nome, mn, sn in (("I2", "M2", "S2"), ("I3", "M3", "S3"), ("I6", "M6", "S6"), ("I7", "M7", "S7")):
        x1, x2 = MOD[nome]; caixa(X + x1, Y + 1600, x2 - x1, 350, mn); caixa(X + x1, Y + 1950, x2 - x1, 590, sn, tam=54)
    # I4: gavetas I4A-C e porta-temperos I4D (tracejados)
    x1, x2 = MOD["I4"]
    camada("CONTORNO", "#e6e6e6"); R(X + x1, Y + 150, x2 - x1, 750)
    camada("TRACEJADO", "#e6e6e6")
    R(X + x1, Y + 150, 756, 250); R(X + x1, Y + 400, 756, 250); R(X + x1, Y + 650, 756, 250); R(X + x2 - 150, Y + 150, 150, 750)
    camada("TEXTOS", "#e6e6e6")
    centro(X + x1 + 378, Y + 275, "I4A", PEQ); centro(X + x1 + 378, Y + 525, "I4B", PEQ); centro(X + x1 + 378, Y + 775, "I4C", PEQ)
    centro(X + x2 - 75, Y + 525, "I4D", PEQ, 90); centro(X + x1 + 378, Y + 860, "I4", 54)
    camada("APARELHOS", "#6f6f6f"); cheio(X + x1, Y + 1600, 480, 350)
    camada("TEXTOS", "#e6e6e6"); centro(X + x1 + 240, Y + 1810, "M4A", PEQ); centro(X + x1 + 240, Y + 1740, "micro", PEQ)
    caixa(X + x1 + 498, Y + 1600, 408, 350, "M4B")
    caixa(X + x1, Y + 1950, x2 - x1, 590, "S4", tam=54)
    cota_h(X + x1, X + x1 + 480, Y + 2260, "480", ext=100); cota_h(X + x1 + 498, X + x2, Y + 2260, "408", ext=100)
    # I5: cooktop, forno, depurador
    x1, x2 = MOD["I5"]
    caixa(X + x1, Y + 130, x2 - x1, 770, "", "MODULOS")
    camada("APARELHOS", "#6f6f6f"); cheio(X + 2956, Y + 130, 600, 600); cheio(X + 2956, Y + 920, 600, 35)
    camada("CONTORNO", "#e6e6e6"); R(X + 2956, Y + 130, 600, 600); R(X + 2956, Y + 920, 600, 35)
    camada("TEXTOS", "#e6e6e6"); centro(X + 3256, Y + 800, "I5", 54); centro(X + 3256, Y + 300, "forno", PEQ); centro(X + 3256, Y + 240, "OE8EA", PEQ)
    camada("APARELHOS", "#6f6f6f"); cheio(X + x1, Y + 1600, x2 - x1, 350)
    caixa(X + x1, Y + 1600, x2 - x1, 350, "depurador (M5)", "MODULOS", tam=PEQ)
    caixa(X + x1, Y + 1950, x2 - x1, 590, "S5", tam=54)
    # Z1: gaveta de rodape (tracejada)
    camada("TRACEJADO", "#e6e6e6"); R(X + 1394, Y, 1524, 150)
    # pedra 1 (tampo) e pedra 2 (frontao)
    camada("PEDRA", "#c8c8c8"); cheio(X + 866, Y + 900, 3816, 20); R(X + 866, Y + 900, 3816, 20)
    camada("PEDRA2", "#4d4d4d"); cheio(X + 866, Y + 920, 3816, 680)
    camada("CONTORNO", "#e6e6e6"); R(X + 866, Y + 920, 3816, 680)
    camada("TEXTOS", "#e6e6e6")
    centro(X + 2774, Y + 1330, "pedra 2 - painel vertical (frontao), ver PRANCHA 3", TXT)
    centro(X + 3256, Y + 1110, "cooktop KE4GC", PEQ)
    centro(X + 2774, Y + 1000, "pedra unica 866-4682, prof. 670 uniforme (rebaixo italiano I2/I3/I4/I7)", PEQ)
    # cotas: esquerda
    cota_v(X - 1100, Y, Y + 2540, "2540 (pe-direito real)", ext=100)
    cota_v(X - 650, Y + 1950, Y + 2540, "590 (prof 550)", ext=100); cota_v(X - 650, Y + 1600, Y + 1950, "350 (prof 400)", ext=100)
    cota_v(X - 300, Y + 2018, Y + 2540, "522 (S1, prof 650)", ext=100)
    # cotas: embaixo
    for nome, (x1, x2) in MOD.items():
        cota_h(X + x1, X + x2, Y - 300, f"{x2 - x1}" + (" (R1)" if nome == "R1" else ""), ext=120)
    cota_h(X + 2956, X + 3556, Y - 700, "600 (forno/cooktop/depurador)", ext=120)
    cota_h(X + 2768, X + 2918, Y - 1000, "150 (I4D)", ext=120)
    cota_h(X + 1394, X + 2918, Y - 1000 + 0, "1524 (Z1 - gaveta de rodape)", ext=120, sobre=False)
    cota_h(X, X + 4700, Y - 1400, "4700", ext=120)
    camada("TEXTOS", "#e6e6e6")
    T(X, 1050, TIT, "PRANCHA 1 / 4 - VISTA FRONTAL - PAREDE NORTE (V5)"); T(X, 780, 30, SUB)

# ================================================================ PRANCHA 2: pedra 1
def prancha2():
    ox = folha(1, "PRANCHA 2/4 - PEDRA 1 (BANCADA - RECORTES)", SUB)
    X, Y = ox + 1500, 2500
    camada("TEXTOS", "#e6e6e6"); centro(X + 1908, Y + 1350, "PEDRA - PECA UNICA (866-4682 = 3816mm) - PROF. 670mm UNIFORME", TIT * 0.85)
    cota_h(X, X + 3816, Y + 1000, "3816 (pedra total)", ext=120)
    camada("PEDRA", "#c8c8c8"); R(X, Y, 3816, 670)
    # rebaixo italiano (tracejado, recuo 20) nas colunas molhadas; lisa nas secas
    camada("TRACEJADO", "#e6e6e6")
    for a, b in ((0, 528), (528, 1146), (1146, 2070), (3376, 3816)): L(X + a, Y + 650, X + b, Y + 650)
    camada("TEXTOS", "#e6e6e6")
    for a, b in ((0, 528), (528, 1146), (1146, 2070), (3376, 3816)):
        centro(X + (a + b) / 2, Y + 600, "rebaixo italiano", PEQ); centro(X + (a + b) / 2, Y + 555, "(recuo 20, prof 5)", PEQ)
    centro(X + 2723, Y + 600, "superficie lisa", PEQ); centro(X + 2723, Y + 555, "(sem rebaixo)", PEQ)
    # recortes
    for nome, x, yb, w, h in (("pia", 548, 140, 560, 340), ("cooktop", 2090, 70, 600, 450), ("tanque", 3386, 160, 400, 350)):
        camada("APARELHOS", "#6f6f6f"); cheio(X + x, Y + yb, w, h)
        camada("CONTORNO", "#e6e6e6"); R(X + x, Y + yb, w, h)
        camada("TEXTOS", "#e6e6e6"); centro(X + x + w / 2, Y + yb + h / 2 + 30, nome, TXT); centro(X + x + w / 2, Y + yb + h / 2 - 30, f"{w}x{h}", TXT)
    # cotas verticais das distancias (de cima e de baixo)
    for x, sup, inf, y_topo, y_base in ((548 - 180, 190, 140, 480, 140), (2090 - 180, 150, 70, 520, 70), (3386 - 180, 160, 160, 510, 160)):
        cota_v(X + x, Y + y_topo, Y + 670, f"{sup}", ext=100); cota_v(X + x, Y, Y + y_base, f"{inf}", ext=100)
    # cotas horizontais das larguras e distancias
    cota_h(X + 548, X + 1108, Y - 300, "560", ext=100); cota_h(X + 2090, X + 2690, Y - 300, "600", ext=100); cota_h(X + 3386, X + 3786, Y - 300, "400", ext=100)
    cota_h(X, X + 548, Y - 600, "548 (pedra->pia)", ext=100); cota_h(X + 1108, X + 2090, Y - 600, "982 (pia->cooktop)", ext=100)
    cota_h(X + 2690, X + 3386, Y - 600, "696 (cooktop->tanque)", ext=100); cota_h(X + 3786, X + 3816, Y - 600, "30 (tanque->pedra)", ext=100, sobre=True)
    cota_h(X + 3376, X + 3386, Y - 900, "10 (I6->tanque)", ext=100); cota_h(X + 3786, X + 3816, Y - 900, "30 (tanque->janela)", ext=100)
    camada("TEXTOS", "#e6e6e6")
    T(X, Y - 1250, PEQ, "frente = borda com saliencia de 20mm (zona molhada) / flush (zona seca) | fundo = parede")
    T(ox + 1500, 1050, TIT, "PRANCHA 2 / 4 - PEDRA 1 (BANCADA - RECORTES: PIA, COOKTOP, TANQUE)"); T(ox + 1500, 780, 30, SUB)

# ================================================================ PRANCHA 3: pedra 2
def prancha3():
    ox = folha(2, "PRANCHA 3/4 - PEDRA 2 (PAINEL VERTICAL DO FRONTAO)", SUB)
    X, Y = ox + 1500, 2800
    camada("TEXTOS", "#e6e6e6"); centro(X + 1908, Y + 1000, "PEDRA 2 - PAINEL VERTICAL (REVESTE O FRONTAO, 920-1600)", TIT * 0.85)
    camada("PEDRA2", "#4d4d4d"); cheio(X, Y, 3816, 680)
    camada("CONTORNO", "#e6e6e6"); R(X, Y, 3816, 680)
    partes = [("I2", 0, 528), ("I3", 528, 1146), ("I4", 1146, 2070), ("I5", 2070, 2728), ("I6", 2728, 3376), ("I7", 3376, 3816)]
    for nome, a, b in partes:
        camada("CONTORNO", "#e6e6e6"); L(X + a, Y - 120, X + a, Y + 800) if a else None
        camada("TEXTOS", "#e6e6e6"); centro(X + (a + b) / 2, Y + 340, nome, 54)
    L(X + 3816, Y - 120, X + 3816, Y + 800)
    cota_v(X - 350, Y, Y + 680, "680", ext=100)
    cota_h(X, X + 3816, Y - 400, "3816 (I2 a I7)", ext=100)
    camada("TEXTOS", "#e6e6e6")
    for k, t in enumerate(["Espessura: espessura padrao de chapa - a definir com a marmoraria",
                           "Peca continua (866-4682) - posicao do corte/emenda a definir com a marmoraria.",
                           "Limites verticais: 920 (topo da pedra 1) a 1600 (inicio da faixa M dos aereos)."]):
        T(X, Y - 900 - k * 150, TXT, t)
    T(ox + 1500, 1050, TIT, "PRANCHA 3 / 4 - PEDRA 2 (PAINEL VERTICAL DO FRONTAO)"); T(ox + 1500, 780, 30, SUB)

# ================================================================ PRANCHA 4: vistas laterais
def escala_altura(X, Y, x):
    """Regua de alturas a esquerda: 0, 150, 900/920, 1600, 1950, 2540."""
    camada("COTAS", "#f0a030"); L(x, Y, x, Y + 2540)
    camada("TEXTOS", "#e6e6e6")
    for v, t in ((0, "0"), (150, "150"), (910, "900/920"), (1600, "1600"), (1950, "1950"), (2540, "2540")):
        camada("COTAS", "#f0a030"); L(x - 40, Y + v, x + 40, Y + v)
        camada("TEXTOS", "#e6e6e6"); centro(x - 90, Y + v + 30, t, PEQ, 90)
def circ(x, y, prof):
    camada("TEXTOS", "#e6e6e6"); T(x, y, PEQ, f"circ."); T(x, y - 45, PEQ, f"{1460 - prof}")
def prancha4():
    ox = folha(3, "PRANCHA 4/4 - VISTA LATERAL - PROFUNDIDADES POR SECAO", SUB)
    Y = 1900
    # ---- R1
    X = ox + 1300
    escala_altura(X - 250, Y, X - 250)
    camada("R1", "#5f5f5f"); cheio(X, Y, 670, 2540); camada("CONTORNO", "#e6e6e6"); R(X, Y, 670, 2540)
    camada("TEXTOS", "#e6e6e6"); centro(X + 335, Y + 1300, "670", TXT); centro(X + 335, Y + 1240, "R1 (fundo cego)", TXT)
    camada("TEXTOS", "#e6e6e6"); T(X + 100, 1500, 70, "R1")
    # ---- I1 / S1
    X = ox + 3050
    escala_altura(X - 250, Y, X - 250)
    caixa(X, Y + 2018, 650, 522, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 325, Y + 2320, "650", TXT); centro(X + 325, Y + 2260, "S1", TXT)
    circ(X + 700, Y + 2400, 650)
    camada("APARELHOS", "#7c7c7c"); cheio(X, Y, 800, 2000); camada("CONTORNO", "#e6e6e6"); R(X, Y, 800, 2000)
    camada("TEXTOS", "#e6e6e6"); centro(X + 400, Y + 1000, "800", TXT); centro(X + 400, Y + 940, "geladeira", TXT); centro(X + 400, Y + 880, "(sem rodape)", TXT)
    T(X + 850, Y + 950, PEQ, "circ."); T(X + 850, Y + 905, PEQ, "660"); T(X + 850, Y + 860, PEQ, "mais estreito")
    cota_v(X + 1150, Y, Y + 2100, "2100 (nicho, sem rodape)", ext=100, lado=1)
    camada("TEXTOS", "#e6e6e6"); T(X, 1500, 70, "I1 / S1 (excecao)")
    # ---- I2 / I6
    X = ox + 4900
    escala_altura(X - 250, Y, X - 250)
    caixa(X, Y + 1950, 550, 590, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 275, Y + 2280, "550", TXT); centro(X + 275, Y + 2220, "faixa S", TXT)
    caixa(X, Y + 1600, 400, 350, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 200, Y + 1800, "400", TXT); centro(X + 200, Y + 1740, "faixa M", TXT)
    camada("TRACEJADO", "#e6e6e6"); L(X, Y + 920, X, Y + 1600)
    camada("APARELHOS", "#7c7c7c"); cheio(X, Y, 650, 900); camada("CONTORNO", "#e6e6e6"); R(X, Y, 650, 900)
    camada("PEDRA", "#c8c8c8"); cheio(X, Y + 900, 670, 20); R(X, Y + 900, 670, 20)
    camada("TEXTOS", "#e6e6e6"); centro(X + 325, Y + 500, "650", TXT); centro(X + 325, Y + 440, "nicho I2/I6", TXT); centro(X + 325, Y + 380, "(sem rodape)", TXT)
    T(X + 720, Y + 500, PEQ, "circ."); T(X + 720, Y + 455, PEQ, "810")
    cota_v(X + 1000, Y, Y + 900, "900 (sem rodape)", ext=100, lado=1)
    camada("TEXTOS", "#e6e6e6"); T(X, 1500, 70, "I2 / I6 (excecao)")
    # ---- perfil generico
    X = ox + 6750
    escala_altura(X - 250, Y, X - 250)
    caixa(X, Y + 1950, 550, 590, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 275, Y + 2280, "550", TXT); centro(X + 275, Y + 2220, "faixa S", TXT)
    T(X + 620, Y + 2400, PEQ, "circ."); T(X + 620, Y + 2355, PEQ, "910")
    caixa(X, Y + 1600, 400, 350, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 200, Y + 1800, "400", TXT); centro(X + 200, Y + 1740, "faixa M", TXT)
    T(X + 470, Y + 1780, PEQ, "circ."); T(X + 470, Y + 1735, PEQ, "1060")
    camada("TRACEJADO", "#e6e6e6"); L(X, Y + 920, X, Y + 1600)
    cota_v(X + 250, Y + 920, Y + 1600, "680", ext=100, lado=1)
    camada("TEXTOS", "#e6e6e6"); T(X + 320, Y + 1300, PEQ, "frontao livre"); T(X + 320, Y + 1255, PEQ, "(sem marcenaria, I->M)")
    camada("PEDRA", "#c8c8c8"); cheio(X, Y + 900, 670, 20); R(X, Y + 900, 670, 20)
    T(X + 730, Y + 905, PEQ, "circ."); T(X + 730, Y + 860, PEQ, "790")
    caixa(X, Y + 150, 650, 750, "", "MODULOS"); camada("TEXTOS", "#e6e6e6"); centro(X + 325, Y + 560, "650", 54); centro(X + 325, Y + 470, "faixa I", 54)
    T(X + 720, Y + 500, PEQ, "circ."); T(X + 720, Y + 455, PEQ, "810")
    camada("TRACEJADO", "#e6e6e6"); R(X, Y, 600, 150)
    camada("TEXTOS", "#e6e6e6"); centro(X + 300, Y + 100, "600", PEQ); centro(X + 300, Y + 50, "rodape", PEQ)
    cota_v(X - 60, Y + 150, Y + 900, "750 (faixa I)", ext=100, lado=1); cota_v(X - 60, Y, Y + 150, "150 (rodape)", ext=100, lado=1)
    camada("TEXTOS", "#e6e6e6"); T(X, 1500, 70, "PERFIL GENERICO (I3, I4, I7)")
    T(ox + 1300, 1150, PEQ, "Excecoes de profundidade: I5 tem rodape rebaixado ate 130mm (mantendo 600mm de profundidade). Zona molhada (I2/I3/I4/I7) tem pingadeira e rebaixo italiano na pedra (ver PRANCHA 2);")
    T(ox + 1300, 1090, PEQ, "zona seca (I5/I6) tem a mesma profundidade de pedra (670mm), sem esses detalhes.")
    T(ox + 1300, 940, TIT, "PRANCHA 4 / 4 - VISTA LATERAL - PROFUNDIDADES POR SECAO"); T(ox + 1300, 700, 30, SUB)

# ================================================================ tabelas (folhas 5 a 7)
def tabela(x0, ytopo, colunas, linhas, alt=150, h=48, cab="#e6e6e6"):
    larg = sum(w for _, w in colunas); nl = len(linhas) + 1
    camada("TABELAS", "#c0c0c0")
    for i in range(nl + 1): L(x0, ytopo - i * alt, x0 + larg, ytopo - i * alt)
    x = x0
    for _, w in colunas + [("", 0)]:
        L(x, ytopo, x, ytopo - nl * alt); x += w
    camada("CABECALHO", "#333333")
    cheio(x0, ytopo - alt, larg, alt)
    x = x0
    camada("TEXTOS", "#e6e6e6")
    for t, w in colunas: T(x + 30, ytopo - alt + alt / 3, h, t); x += w
    for j, lin in enumerate(linhas):
        x = x0
        for (t, w), v in zip(colunas, lin): T(x + 30, ytopo - (j + 2) * alt + alt / 3, h, v); x += w

MODULOS = [
    ["R1", "Ripado de madeira, extremidade oeste", "68", "0-2540", "670", "Fixo, fundo cego, mesma visao dos demais modulos"],
    ["I1", "Coluna geladeira", "780", "0-2540 (nicho 0-2000 + S1 2018-2540)", "650 (790 na face)", "Nicho aberto + porta vertical acima (S1), dobradica esquerda"],
    ["I2", "Lava-loucas sob bancada", "510", "150-900 (750)", "650", "Nicho aberto (frente do proprio aparelho)"],
    ["I3", "Gabinete pia", "600", "150-900 (750)", "650", "Porta vertical, dobradica direita, lixeira dupla extraivel"],
    ["I4", "Gavetas + porta-temperos", "906", "150-900 (750)", "650", "I4A/I4B/I4C - 3 gavetas de 252; I4D - porta-temperos vertical (150), encostado em I5"],
    ["I5", "Cooktop + forno", "640", "0-900 (rodape rebaixado 130)", "650", "Forno e cooktop centralizados no eixo 3256, 20mm de folga cada lado"],
    ["I6", "Nicho lava e seca", "630", "150-900 (750)", "650", "Nicho aberto, maquina apoiada no piso"],
    ["I7", "Gabinete tanque", "440", "150-900 (750)", "650", "2 folhas em bascula (abrem para baixo); compartimento de roupa suja"],
    ["Z1", "Gaveta de rodape (sob I3+I4)", "1524 externos", "0-150 (rodape)", "600", "Corredica 600; se nao vencer o vao, dividir em duas de 762"],
    ["S1", "Sobre I1 (excecao, sem faixa M)", "780", "2018-2540 (522)", "650", "Porta unica, dobradica esquerda"],
    ["M2", "Sobre I2", "510", "1600-1950 (350)", "400", "generico"],
    ["S2", "Sobre M2", "510", "1950-2540 (590)", "550", "generico"],
    ["M3", "Sobre I3 (escorredor)", "600", "1600-1950 (350)", "400", "Basculante Aventos HK, escorredor duplo embutido"],
    ["S3", "Sobre M3", "600", "1950-2540 (590)", "550", "Porta lateral, dobradica esquerda"],
    ["M4A", "Sobre I4 - nicho micro-ondas", "480", "1600-1950 (350)", "400", "Aberto na frente, fundo vazado para ventilacao"],
    ["M4B", "Sobre I4, ao lado do micro", "408", "1600-1950 (350)", "400", "Porta, dobradica direita"],
    ["S4", "Sobre M4A/M4B, largura cheia", "906", "1950-2540 (590)", "550", "Porta, dobradica esquerda"],
    ["M5", "Sobre I5 - depurador", "640", "1600-1950 (350)", "400", "Depurador (600), centralizado no eixo do cooktop, 20mm folga cada lado"],
    ["S5", "Sobre M5, largura cheia", "640", "1950-2540 (590)", "550", "Porta, dobradica esquerda"],
    ["M6", "Sobre I6", "630", "1600-1950 (350)", "400", "generico"],
    ["S6", "Sobre M6", "630", "1950-2540 (590)", "550", "generico"],
    ["M7", "Sobre I7", "440", "1600-1950 (350)", "400", "generico"],
    ["S7", "Sobre M7", "440", "1950-2540 (590)", "550", "generico"],
]
FERRAGENS = [
    ["Dobradicas retas", "Portas verticais comuns (S1, S3, S4, S5, S6, S7, I3, I7)", "conforme contagem de portas"],
    ["Dobradica basculante Aventos HK", "M3 - escorredor sobre a cuba", "1 jogo"],
    ["Corredicas ocultas", "I4A, I4B, I4C (gavetas 252; prof. 650)", "3 jogos"],
    ["Corredica vertical / puxador embutido", "I4D - porta-temperos (150mm, encostado em I5)", "1 jogo"],
    ["Corredica reforcada 600 mm", "Z1 - gaveta de rodape (1524mm externos)", "1 jogo (avaliar carga; dividir em 2x762 se necessario)"],
    ["Dobradica bascula (flap, abre p/ baixo)", "I7 - compartimento tanque / roupa suja", "1 jogo"],
    ["Puxadores", "Todas as portas e gavetas - modelo a definir", "a definir"],
    ["Suporte de prateleira", "Prateleira/divisoes internas dos modulos M e S", "conforme projeto"],
    ["Grelhas de ventilacao", "I1 - entrada 780x100 no rodape, saida 780x60 (2640-2700)", "2 pecas"],
    ["Sifao / engate flexivel", "I7 - area central reservada a hidraulica do tanque", "conforme instalacao"],
]
ELETRO = [
    ["Geladeira Brastemp BRE66AK", "1943 x 705 x 740 mm (AxLxP)"], ["Cooktop Electrolux KE4GC", "mesa 520 x 390 mm - recorte de pedra 600 x 450 mm"],
    ["Forno Electrolux OE8EA 80L", "nicho 600 x 600 x 600 mm - A CONFIRMAR"], ["Micro-ondas Consul CM020BF", "455 x 264 x 353 mm (LxAxP), bancada"],
    ["Lava-loucas 45 cm", "448 x 845 mm (LxA)"], ["Lava e seca LG CV5012PC4", "850 x 600 x 620 mm"],
    ["Depurador Fischer Slim 60", "192 x 600 x 280 mm, embutido"], ["Tanque inox", "400 x 350 mm (recorte de pedra)"],
]
NOTAS = [
    "Regra zero: todas as cotas sao levantamento fechado e conferido; nao arredondar, nao recalcular.",
    "Pe-direito real confirmado em 2540mm (substitui os 2700mm do levantamento original).",
    "Janela leste 1000x1000, peitoril 1100: valor representativo, nao medido - confirmar em obra.",
    "Forno Electrolux OE8EA 80L: nicho 600x600x600 A CONFIRMAR junto ao fabricante antes da fabricacao.",
    "R1 (ripado, 68mm) substitui a antiga lateral de 18mm entre a extremidade oeste e I1; mesma peca do painel ripado original, agora integrada a cadeia de marcenaria, ate o teto real (2540mm).",
    "Pedra: peca unica de 866-4682 (3816mm) - exige emenda (posicao a definir com a marmoraria). Profundidade uniforme de 670mm em toda a extensao, sem degrau na borda frontal.",
    "Rebaixo italiano (escorredor esculpido na superficie da pedra) apenas nas colunas molhadas (I2, I3, I4, I7): recuado 20mm da borda frontal, profundidade de rebaixo de 5mm. Colunas secas (I5, I6) permanecem lisas.",
    "Cuba (pia, 560x340) recentralizada no eixo de I3 (x=1694) apos o crescimento de I2; 140mm da borda frontal da pedra (120mm da linha da marcenaria + 20mm da saia molhada).",
    "Cooktop (recorte 600x450) e forno (600mm) centralizados no eixo de I5 (x=3256), com 20mm de folga de cada lado dos 640mm do modulo. Depurador (M5) segue o mesmo eixo.",
    "Tanque (400x350): 10mm da divisa com I6, 30mm da parede leste (janela); centralizado na profundidade.",
    "Z1 (gaveta de rodape) atravessa I3 e I4 (1394-2918, 1524mm externos); dividir em duas de 762mm se a ferragem nao vencer o vao unico.",
    "Nao especificar materiais, cores ou acabamentos - ainda nao definidos.",
    "Nao desenhar marcenaria nas paredes sul e leste.",
]
def pagina_tabelas():
    ox = folha(4, "TABELA DE MODULOS", "Prancha tecnica executiva - cozinha integrada com area de servico")
    x0 = ox + 650
    camada("TEXTOS", "#e6e6e6")
    T(x0, 5380, 100, "Prancha tecnica executiva - cozinha integrada com area de servico")
    T(x0, 5200, 46, "VERSAO 5 (final) - nomenclatura R1 / I1-I7 / M2-M7 / S1-S7 / Z1 - pe-direito real 2540mm - pedra unica com rebaixo italiano nas colunas molhadas")
    T(x0, 5100, 46, "R1 = ripado (extremidade oeste) | I = inferior (150-900) | M = mediano (1600-1950, exceto coluna 1) | S = superior (1950-2540) | Z1 = gaveta de rodape. Regra zero: cotas de levantamento fechado.")
    T(x0, 4900, 76, "Tabela de modulos")
    tabela(x0, 4820, [("Modulo", 330), ("Funcao", 1600), ("Vao (mm)", 480), ("Altura (mm)", 1250), ("Prof. (mm)", 520), ("Abertura / observacao", 3120)], MODULOS, alt=150, h=44)
    ox = folha(5, "TABELAS DE FERRAGENS E ELETRODOMESTICOS", "Prancha tecnica executiva - cozinha integrada com area de servico")
    x0 = ox + 650
    camada("TEXTOS", "#e6e6e6"); T(x0, 5300, 76, "Tabela de ferragens")
    tabela(x0, 5220, [("Ferragem", 2250), ("Aplicacao", 3000), ("Quantidade / observacao", 2050)], FERRAGENS, alt=150, h=44)
    camada("TEXTOS", "#e6e6e6"); T(x0, 3200, 76, "Eletrodomesticos considerados")
    tabela(x0, 3120, [("Aparelho", 2800), ("Dimensoes", 4200)], ELETRO, alt=150, h=44)
    ox = folha(6, "NOTAS DE EXECUCAO", "Prancha tecnica executiva - cozinha integrada com area de servico")
    x0 = ox + 650
    camada("TEXTOS", "#e6e6e6"); T(x0, 5350, 90, "Notas de execucao")
    for k, t in enumerate(NOTAS): T(x0, 5050 - k * 210, 50, "- " + t)

prancha1(); prancha2(); prancha3(); prancha4(); pagina_tabelas()
BASE = "\n".join(out) + "\n"

def montar(fcdz, dxf):
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": BASE})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "TRACEJADO", "tipo_linha": "tracejada"})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "CONTORNO", "espessura": 0.35})
    m.call("cad_camadas", {"acao": "propriedade", "nome": "PEDRA", "espessura": 0.3})
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1]
    r = montar(base + ".fcdz", base + ".dxf")
    print(r["total_de_entidades"], r["por_tipo"])

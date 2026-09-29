"""Pack de elétrica do FeatherCAD, versão 0: dispositivos de quadro de distribuição desenhados em TAMANHO REAL (mm, vista frontal):
disjuntor modular (1 a 4 polos), DR, DPS, trilho DIN, barra de neutro/terra, barramento tipo pente, caixa com prensa-cabos e cabos
coloridos. Cada função põe o dispositivo com o canto inferior esquerdo em (x, y) e aceita `k` (ampliação, para folhas de detalhe).

Saída: texto compacto `.fct` (uma linha por objeto). Cores do CAD escuro: o "preto" da tinta é #FFFFFF (ACI 7), que vira preto no papel."""
import math

MODULO = 17.5          # largura de um polo (mm)
ALTURA = 90.0          # altura de um dispositivo modular (mm)
TRILHO = 35.0          # largura do trilho DIN (mm)
BULGE_90 = math.tan(math.radians(90) / 4)
LINHA, FINO, TEXTO = "#FFFFFF", "#9AA0A6", "#B8BEC4"
FUNDO = "#1B2027"      # o que tapa o que está atrás (trilho): igual ao fundo escuro; no papel o preenchimento escuro clareia
FASE, NEUTRO, TERRA, PRETO = "#FF2A2A", "#2F6BFF", "#22B14C", "#FFFFFF"


def n(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


class Desenho:
    """Acumula as linhas do `.fct`."""

    def __init__(self):
        self.out = ["unidade mm"]
        self.camadas = {}
        self.atual = None

    def camada(self, nome, cor):
        self.camadas.setdefault(nome, cor)
        if self.atual != nome:
            self.out.append(f"camada {nome} {self.camadas[nome]}")
            self.atual = nome

    def L(self, a, b, c, d, cor=None):
        self.out.append(f"L {n(a)} {n(b)} {n(c)} {n(d)}" + (f" {cor}" if cor else ""))

    def R(self, x, y, w, h, cor=None):
        self.out.append(f"R {n(x)} {n(y)} {n(w)} {n(h)}" + (f" {cor}" if cor else ""))

    def C(self, x, y, r, cor=None):
        self.out.append(f"C {n(x)} {n(y)} {n(r)}" + (f" {cor}" if cor else ""))

    def A(self, x, y, r, a0, a1, cor=None):
        self.out.append(f"A {n(x)} {n(y)} {n(r)} {n(a0)} {n(a1)}" + (f" {cor}" if cor else ""))

    def T(self, x, y, h, txt, cor=None, giro=0):
        self.out.append(f'T {n(x)} {n(y)} {n(h)} {n(giro)} "{txt}"' + (f" {cor}" if cor else ""))

    def P(self, pts, fechada=False, cor=None):
        self.out.append("P " + " ".join(f"{n(p[0])} {n(p[1])}" if len(p) == 2 else f"~{n(p[2])} {n(p[0])} {n(p[1])}" for p in pts) + (" f" if fechada else "") + (f" {cor}" if cor else ""))

    def H(self, pts, cor=None):
        self.out.append("H solida " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + (f" {cor}" if cor else ""))

    def texto(self):
        return "\n".join(self.out) + "\n"

    # ---- formas de apoio
    def rr(self, x, y, w, h, r, cor=None):
        """Retângulo de cantos arredondados (raio r)."""
        r = min(r, w / 2, h / 2)
        b = BULGE_90
        pts = [(x + r, y), (x + w - r, y, b), (x + w, y + r), (x + w, y + h - r, b), (x + w - r, y + h), (x + r, y + h, b), (x, y + h - r), (x, y + r, b)]
        # cada ponto com bulge liga ao próximo por arco; o formato quer o ~b DEPOIS do ponto de partida do arco
        seq = []
        for i, p in enumerate(pts):
            seq.append((p[0], p[1]))
            if len(p) == 3:
                seq.append(("b", p[2]))
        linha = "P "
        for item in seq:
            linha += (f"~{n(item[1])} " if item[0] == "b" else f"{n(item[0])} {n(item[1])} ")
        self.out.append(linha + "f" + (f" {cor}" if cor else ""))

    def fundo(self, x, y, w, h):
        self.camada("FUNDO_DISPOSITIVOS", FUNDO)
        self.H([(x, y), (x + w, y), (x + w, y + h), (x, y + h)], FUNDO)

    def parafuso(self, cx, cy, k=1.0, r=3.6, cor=LINHA):
        self.C(cx, cy, r * k, cor)
        self.C(cx, cy, r * 0.62 * k, FINO)
        d = r * 0.55 * k
        self.L(cx - d, cy, cx + d, cy, FINO)
        self.L(cx, cy - d, cx, cy + d, FINO)

    def cabo(self, pts, cor, largura=3.0, k=1.0):
        """Cabo: duas linhas paralelas coloridas (como na vista do escritório), com pontas retas. Só segmentos retos."""
        w = largura * k / 2
        lados = ([], [])
        for i, (x, y) in enumerate(pts):
            if i == 0:
                dx, dy = pts[1][0] - x, pts[1][1] - y
            elif i == len(pts) - 1:
                dx, dy = x - pts[i - 1][0], y - pts[i - 1][1]
            else:
                dx, dy = pts[i + 1][0] - pts[i - 1][0], pts[i + 1][1] - pts[i - 1][1]
            d = math.hypot(dx, dy) or 1.0
            nx, ny = -dy / d * w, dx / d * w
            lados[0].append((x + nx, y + ny))
            lados[1].append((x - nx, y - ny))
        self.P(lados[0], False, cor)
        self.P(lados[1], False, cor)
        self.L(*lados[0][0], *lados[1][0], cor)
        self.L(*lados[0][-1], *lados[1][-1], cor)


# ======================================================================== dispositivos

def disjuntor(d, x, y, polos=1, corrente="C16", icc="6kA", k=1.0, cabos=False):
    """Disjuntor modular: corpo, bornes com parafuso em cima e embaixo, janela da alavanca, corrente e capacidade de interrupção."""
    w = polos * MODULO * k
    h = ALTURA * k
    d.fundo(x, y, w, h)
    d.camada("DISJUNTOR", LINHA)
    d.rr(x, y, w, h, 2.0 * k)
    d.L(x + 1.2 * k, y + 22 * k, x + w - 1.2 * k, y + 22 * k, FINO)
    d.L(x + 1.2 * k, y + h - 22 * k, x + w - 1.2 * k, y + h - 22 * k, FINO)
    for i in range(polos):
        cx = x + (i + 0.5) * MODULO * k
        d.parafuso(cx, y + h - 11 * k, k)
        d.parafuso(cx, y + 11 * k, k)
        if i:
            d.L(x + i * MODULO * k, y + h - 22 * k, x + i * MODULO * k, y + h, FINO)
            d.L(x + i * MODULO * k, y, x + i * MODULO * k, y + 22 * k, FINO)
    # janela e alavanca
    cx, cy = x + w / 2, y + h / 2
    if polos == 1:
        d.rr(cx - 4 * k, cy - 8 * k, 8 * k, 16 * k, 2 * k, FINO)
        d.rr(cx - 2.6 * k, cy - 4 * k, 5.2 * k, 8 * k, 1 * k, FINO)
        d.T(cx - 4.4 * k, cy + 15 * k, 3.6 * k, corrente, TEXTO)
        d.T(cx - 4.4 * k, cy - 20 * k, 3.6 * k, icc, TEXTO)
    else:
        jw = min(w - 6 * k, 22 * k)
        d.rr(cx - jw / 2, cy - 15 * k, jw, 30 * k, 4 * k, FINO)
        d.rr(cx - jw * 0.36, cy - 9 * k, jw * 0.72, 18 * k, 2.5 * k, FINO)
        d.L(cx - jw * 0.36, cy + 1 * k, cx + jw * 0.36, cy + 1 * k, FINO)
        d.T(cx - 5 * k, cy + 19 * k, 4 * k, corrente, TEXTO)
        d.T(cx - 6 * k, cy - 24 * k, 4 * k, icc, TEXTO)
    if cabos:
        for i in range(polos):
            cxp = x + (i + 0.5) * MODULO * k
            d.camada("CABOS", FASE)
            d.cabo([(cxp, y + h - 11 * k), (cxp, y + h + 30 * k)], FASE, 2.6, k)
            d.cabo([(cxp, y + 11 * k), (cxp, y - 30 * k)], FASE, 2.6, k)
    return (x, y, w, h)


def dr(d, x, y, polos=4, k=1.0):
    """Interruptor diferencial (DR): como o disjuntor, com o botão de teste (T) e a janela de estado."""
    w = polos * MODULO * k
    h = ALTURA * k
    d.fundo(x, y, w, h)
    d.camada("DR", LINHA)
    d.rr(x, y, w, h, 2.0 * k)
    d.L(x + 1.2 * k, y + 22 * k, x + w - 1.2 * k, y + 22 * k, FINO)
    d.L(x + 1.2 * k, y + h - 22 * k, x + w - 1.2 * k, y + h - 22 * k, FINO)
    for i in range(polos):
        cx = x + (i + 0.5) * MODULO * k
        d.parafuso(cx, y + h - 11 * k, k)
        d.parafuso(cx, y + 11 * k, k)
        d.L(x + i * MODULO * k, y + h - 22 * k, x + i * MODULO * k, y + h, FINO) if i else None
        d.L(x + i * MODULO * k, y, x + i * MODULO * k, y + 22 * k, FINO) if i else None
    # botão de teste (T) à esquerda, alavanca à direita, janela de estado no meio
    d.R(x + 4 * k, y + h - 42 * k, 12 * k, 14 * k, FINO)
    d.T(x + 8.2 * k, y + h - 40 * k, 6 * k, "T", TEXTO)
    d.rr(x + w * 0.5 - 5 * k, y + 32 * k, 10 * k, 20 * k, 2 * k, FINO)
    d.R(x + w * 0.5 - 3 * k, y + 36 * k, 6 * k, 8 * k, FINO)
    d.R(x + 4 * k, y + 30 * k, 14 * k, 8 * k, FINO)
    d.R(x + w - 26 * k, y + h - 50 * k, 22 * k, 20 * k, FINO)
    d.L(x + w - 26 * k, y + h - 44 * k, x + w - 4 * k, y + h - 44 * k, FINO)
    d.T(x + w - 24 * k, y + h - 48 * k, 3 * k, "30mA", TEXTO)
    return (x, y, w, h)


def dps(d, x, y, k=1.0, cabos=False):
    """Protetor contra surtos (DPS): um módulo, com a janela de estado vermelha/verde e a etiqueta."""
    w = MODULO * k
    h = ALTURA * k
    d.fundo(x, y, w, h)
    d.camada("DPS", LINHA)
    d.rr(x, y, w, h, 2.0 * k)
    d.L(x + 1.2 * k, y + 22 * k, x + w - 1.2 * k, y + 22 * k, FINO)
    d.L(x + 1.2 * k, y + h - 22 * k, x + w - 1.2 * k, y + h - 22 * k, FINO)
    cx = x + w / 2
    d.parafuso(cx, y + h - 11 * k, k)
    d.parafuso(cx, y + 11 * k, k)
    d.R(x + 3 * k, y + 55 * k, w - 6 * k, 8 * k, FINO)
    d.H([(x + 3.6 * k, y + 55.6 * k), (x + w - 3.6 * k, y + 55.6 * k), (x + w - 3.6 * k, y + 62.4 * k), (x + 3.6 * k, y + 62.4 * k)], FASE)
    d.R(x + 3 * k, y + 28 * k, w - 6 * k, 24 * k, FINO)
    d.T(x + 4.6 * k, y + 38 * k, 3 * k, "DPS", TEXTO)
    d.H([(x + 2 * k, y + 24 * k), (x + w - 2 * k, y + 24 * k), (x + w - 2 * k, y + 26.4 * k), (x + 2 * k, y + 26.4 * k)], TERRA)
    if cabos:
        d.camada("CABOS", FASE)
        d.cabo([(cx, y + h - 11 * k), (cx, y + h + 30 * k)], FASE, 2.6, k)
    return (x, y, w, h)


def trilho_din(d, x, y, comprimento, k=1.0):
    """Trilho DIN de 35 mm, com os furos alongados."""
    d.camada("TRILHO", FINO)
    d.R(x, y, comprimento * k, TRILHO * k)
    d.L(x, y + 7.5 * k, x + comprimento * k, y + 7.5 * k)
    d.L(x, y + (TRILHO - 7.5) * k, x + comprimento * k, y + (TRILHO - 7.5) * k)
    passo = 25.0
    i = 0
    while 8 + i * passo + 10 < comprimento - 4:
        d.rr(x + (8 + i * passo) * k, y + 12 * k, 10 * k, 11 * k, 4.5 * k)
        i += 1


def barra_de_bornes(d, x, y, altura, cor, vertical=True, k=1.0):
    """Barra de neutro (azul) ou de terra (verde) com bornes de parafuso, em pé."""
    d.camada("BARRA_NEUTRO" if cor == NEUTRO else "BARRA_TERRA", cor)
    w = 16 * k
    if vertical:
        d.R(x, y, w, altura * k, cor)
        d.H([(x + 1.2 * k, y + 1.2 * k), (x + w - 1.2 * k, y + 1.2 * k), (x + w - 1.2 * k, y + altura * k - 1.2 * k), (x + 1.2 * k, y + altura * k - 1.2 * k)], cor)
        passo = 9.5 * k
        m = int((altura * k - 6 * k) // passo)
        for i in range(m):
            cy = y + 5 * k + i * passo + passo / 2
            d.C(x + w / 2, cy, 2.6 * k, "#FFFFFF")
            d.L(x + w / 2 - 1.8 * k, cy, x + w / 2 + 1.8 * k, cy, "#FFFFFF")


def pente(d, x, y, polos, k=1.0, cor=LINHA):
    """Barramento tipo pente: barra horizontal com um dente por polo (o 'pente' dos disjuntores)."""
    d.camada("PENTE", cor)
    w = polos * MODULO * k
    d.R(x, y + 12 * k, w, 6 * k, cor)
    for i in range(polos):
        cx = x + (i + 0.5) * MODULO * k
        d.P([(cx - 3.2 * k, y + 12 * k), (cx - 3.2 * k, y + 3 * k), (cx + 3.2 * k, y + 3 * k), (cx + 3.2 * k, y + 12 * k)], False, cor)
        d.L(cx - 1.4 * k, y + 5 * k, cx + 1.4 * k, y + 5 * k, cor)


def caixa(d, x, y, largura, altura, glandulas=(), k=1.0):
    """Caixa do quadro: moldura arredondada, placa de montagem, quatro parafusos nos cantos e os prensa-cabos embaixo."""
    d.camada("CAIXA", LINHA)
    d.rr(x, y, largura * k, altura * k, 8 * k)
    d.rr(x + 8 * k, y + 8 * k, (largura - 16) * k, (altura - 16) * k, 3 * k, FINO)
    for cx, cy in ((x + 14 * k, y + altura * k - 14 * k), (x + (largura - 14) * k, y + altura * k - 14 * k), (x + 14 * k, y + 14 * k), (x + (largura - 14) * k, y + 14 * k)):
        d.parafuso(cx, cy, k, 4.0)
    d.camada("GLANDULAS", FINO)
    for cx in glandulas:
        d.rr(x + cx * k - 8 * k, y - 12 * k, 16 * k, 12 * k, 2 * k, FINO)
        d.L(x + cx * k - 5 * k, y - 3 * k, x + cx * k + 5 * k, y - 3 * k, FINO)
        d.L(x + cx * k - 5 * k, y - 6 * k, x + cx * k + 5 * k, y - 6 * k, FINO)


# ======================================================================== peças de folha

def detalhe_disjuntores(d, x, y):
    """A folha 'DETALHE DISJUNTORES': unipolar, bipolar e tripolar em ampliação de 2:1, com legendas de entrada e saída."""
    K = 2.0
    AMARELO = "#FFE04A"
    larg, alt = 220.0, 380.0
    d.camada("FOLHA", "#FFFFFF")
    d.R(x, y, 3 * larg, alt + 34)
    d.camada("TITULO", AMARELO)
    d.T(x, y + alt + 44, 16, "DETALHE DISJUNTORES", AMARELO)
    for i, (polos, nome, entrada, saida) in enumerate(((1, "UNIPOLAR", "ENTRADA DO|CABO/FASE", "SAÍDA DO|CABO/FASE"), (2, "BIPOLAR", "ENTRADA DOS|CABOS/FASE", "SAÍDA DOS|CABOS/FASE"), (3, "TRIPOLAR", "ENTRADA DOS|CABOS/FASE", "SAÍDA DOS|CABOS/FASE"))):
        px = x + i * larg
        d.camada("FOLHA", "#FFFFFF")
        d.R(px, y, larg, alt + 34)
        d.L(px, y + 34, px + larg, y + 34)
        d.camada("TITULO", "#FFFFFF")
        d.T(px + larg / 2 - len(nome) * 6.4, y + 10, 14, nome, "#FFFFFF")
        w = polos * MODULO * K
        dx = px + (larg - w) / 2
        dy = y + 34 + (alt - ALTURA * K) / 2
        disjuntor(d, dx, dy, polos, "XXA", "XXKA" if polos > 1 else "XKA", K, cabos=True)
        d.camada("ROTULOS", AMARELO)
        a, b = entrada.split("|")
        d.T(px + larg / 2 - len(a) * 3.2, y + alt + 14, 8, a, AMARELO)
        d.T(px + larg / 2 - len(b) * 3.2, y + alt + 3, 8, b, AMARELO)
        a, b = saida.split("|")
        d.T(px + larg / 2 - len(a) * 3.2, y + 34 + 16, 8, a, AMARELO)
        d.T(px + larg / 2 - len(b) * 3.2, y + 34 + 5, 8, b, AMARELO)

"""Apoio das folhas do projeto ferroviário: desenhar em mm de papel e mandar ao MCP no formato compacto (`.fct`).

`Folha` converte mm de papel (canto inferior esquerdo da folha) em unidades do desenho (m); `Fct` junta as linhas de texto do
formato compacto e `enviar` as manda por `cad_importar`. As contas de engenharia (criticidade, superelevação, greide) vêm
prontas do MCP; aqui só se desenha.
"""
import math
import os
import tempfile

ESCALAS_BONITAS = [100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 20000, 25000, 50000]


def v(x, casas=1):
    """Número com vírgula decimal."""
    if x is None:
        return "—"
    return f"{x:.{casas}f}".replace(".", ",")


def escala_que_cabe(largura_m, altura_m, painel_mm):
    """Menor escala bonita (denominador) em que `largura_m` × `altura_m` cabem em `painel_mm` = (largura, altura)."""
    precisa = max(largura_m / painel_mm[0], altura_m / painel_mm[1]) * 1000.0
    return next((e for e in ESCALAS_BONITAS if e >= precisa), ESCALAS_BONITAS[-1])


class Folha:
    """Uma folha inserida pelo MCP (`cad_folha inserir`): converte mm de papel em unidades do desenho."""

    def __init__(self, info):
        self.info = info
        self.indice = info["indice"]
        self.largura = info["largura_mm"]
        self.altura = info["altura_mm"]
        self.escala = info["escala"]
        self.k = self.escala / 1000.0
        self.ox = info["min"]["x"]
        self.oy = info["min"]["y"]

    def p(self, x, y):
        return (self.ox + x * self.k, self.oy + y * self.k)


class Fct:
    """Constrói o texto do formato compacto com coordenadas em mm de papel."""

    def __init__(self, folha):
        self.f = folha
        self.linhas = []
        self.cor_atual = None

    def camada(self, nome, cor="#FFFFFF", espessura=None):
        extra = f" esp={espessura}" if espessura else ""
        self.cor_atual = cor
        self.linhas.append(f"camada {nome} {cor}{extra}")

    def _c(self, cor):
        """Sufixo de cor: a da forma, senão a da camada atual (a cor da 1ª declaração da camada é a que vale no arquivo)."""
        cor = cor or self.cor_atual
        return f" {cor}" if cor else ""

    def _n(self, mm):
        x, y = self.f.p(*mm)
        return f"{x:.3f} {y:.3f}"

    def linha(self, a, b, cor=None):
        self.linhas.append(f"L {self._n(a)} {self._n(b)}" + self._c(cor))

    def poli(self, pts, fechada=False, cor=None):
        if len(pts) < 2:
            return
        self.linhas.append("P " + " ".join(self._n(p) for p in pts) + (" f" if fechada else "") + self._c(cor))

    def ret(self, a, b, cor=None):
        self.poli([a, (b[0], a[1]), b, (a[0], b[1])], True, cor)

    def circulo(self, c, raio_mm, cor=None):
        x, y = self.f.p(*c)
        self.linhas.append(f"C {x:.3f} {y:.3f} {raio_mm * self.f.k:.3f}" + self._c(cor))

    def texto(self, p, altura_mm, txt, alinh="e", giro=0.0, cor=None):
        """`alinh`: e/c/d na horizontal e b/m/t/f na vertical (ex.: `c`, `dm`)."""
        t = txt.replace("\\", "\\\\").replace('"', '\\"')
        a = "" if alinh == "eb" else f" {alinh}"
        self.linhas.append(f'T {self._n(p)} {altura_mm * self.f.k:.4f} {giro:.2f} "{t}"{a}' + self._c(cor))

    def hachura(self, pts, padrao="padrao", espaco_mm=1.2, cor=None):
        """Região hachurada; `espaco_mm` é o espaçamento das linhas no papel."""
        if len(pts) < 3:
            return
        xs = [self.f.p(*p)[0] for p in pts]
        ys = [self.f.p(*p)[1] for p in pts]
        dim = max(max(xs) - min(xs), max(ys) - min(ys), 1e-9)
        escala = espaco_mm * self.f.k * 40.0 / dim
        nome = padrao if padrao == "solida" else f"{padrao}*{escala:.5f}"
        self.linhas.append(f"H {nome} " + " ".join(self._n(p) for p in pts) + self._c(cor))

    def texto_fct(self):
        return "\n".join(self.linhas) + "\n"

    def enviar(self, mcp, nome="folha"):
        """Manda tudo ao desenho (um passo de desfazer). Devolve o nº de formas criadas."""
        if not self.linhas:
            return 0
        caminho = os.path.join(tempfile.gettempdir(), f"feathercad_{os.getpid()}_{nome}.fct")
        with open(caminho, "w") as f:
            f.write("unidade m\n" + self.texto_fct())
        r = mcp.call("cad_importar", {"caminho": caminho})
        os.remove(caminho)
        return r.get("criadas", 0) if isinstance(r, dict) else 0


def recortar_segmento(a, b, r):
    """Liang–Barsky: parte do segmento a→b dentro do retângulo r = (x0, y0, x1, y1), ou None."""
    (x0, y0), (x1, y1) = a, b
    dx, dy = x1 - x0, y1 - y0
    t0, t1 = 0.0, 1.0
    for p, q in ((-dx, x0 - r[0]), (dx, r[2] - x0), (-dy, y0 - r[1]), (dy, r[3] - y0)):
        if abs(p) < 1e-12:
            if q < 0:
                return None
            continue
        t = q / p
        if p < 0:
            if t > t1:
                return None
            t0 = max(t0, t)
        else:
            if t < t0:
                return None
            t1 = min(t1, t)
    return (x0 + t0 * dx, y0 + t0 * dy), (x0 + t1 * dx, y0 + t1 * dy)


def recortar_poli(pts, r):
    """Partes da polilinha dentro do retângulo, como lista de polilinhas."""
    partes, atual = [], []
    for a, b in zip(pts, pts[1:]):
        s = recortar_segmento(a, b, r)
        if s is None:
            if atual:
                partes.append(atual)
                atual = []
            continue
        p, q = s
        if atual and math.hypot(atual[-1][0] - p[0], atual[-1][1] - p[1]) > 1e-6:
            partes.append(atual)
            atual = []
        if not atual:
            atual.append(p)
        atual.append(q)
    if atual:
        partes.append(atual)
    return partes


def paralela(pts, d):
    """Paralela à esquerda (d > 0) de uma polilinha, pelas normais das direções vizinhas."""
    saida = []
    n = len(pts)
    for i, (x, y) in enumerate(pts):
        a = pts[max(i - 1, 0)]
        b = pts[min(i + 1, n - 1)]
        t = math.atan2(b[1] - a[1], b[0] - a[0])
        saida.append((x - math.sin(t) * d, y + math.cos(t) * d))
    return saida


def interpolar(xs, ys, x):
    """Interpolação linear em listas ordenadas; None fora do intervalo."""
    if not xs or x < xs[0] or x > xs[-1]:
        return None
    lo, hi = 0, len(xs) - 1
    while hi - lo > 1:
        meio = (lo + hi) // 2
        if xs[meio] <= x:
            lo = meio
        else:
            hi = meio
    t = (x - xs[lo]) / max(xs[hi] - xs[lo], 1e-12)
    return ys[lo] + (ys[hi] - ys[lo]) * t


def tabela(c, x, y_topo, colunas, linhas, h_linha=5.0, h_texto=2.6, titulo=None, cores=None, cor_cabecalho="#808080"):
    """Tabela com grade em mm de papel. `colunas`: lista de (título, largura_mm, alinhamento `e|c|d`); `linhas`: lista de listas de
    texto; `cores`: opcional, uma lista de cores por linha (ou por célula). Devolve o y da base da tabela."""
    y = y_topo
    larg = sum(col[1] for col in colunas)
    if titulo:
        c.texto((x + larg / 2.0, y - h_linha / 2.0), h_texto * 1.15, titulo, "cm")
        c.ret((x, y - h_linha), (x + larg, y))
        y -= h_linha
    xs = [x]
    for col in colunas:
        xs.append(xs[-1] + col[1])
    cab = y
    for i, col in enumerate(colunas):
        px, al = _celula(xs[i], xs[i + 1], col[2])
        c.texto((px, y - h_linha / 2.0), h_texto, col[0], al + "m", cor=cor_cabecalho)
    c.ret((x, y - h_linha), (x + larg, y))
    y -= h_linha
    for r, linha in enumerate(linhas):
        for i, col in enumerate(colunas):
            if i >= len(linha):
                continue
            cor = None
            if cores and r < len(cores) and cores[r]:
                cor = cores[r][i] if isinstance(cores[r], (list, tuple)) else cores[r]
            px, al = _celula(xs[i], xs[i + 1], col[2])
            c.texto((px, y - h_linha / 2.0), h_texto, str(linha[i]), al + "m", cor=cor)
        c.linha((x, y - h_linha), (x + larg, y - h_linha))
        y -= h_linha
    for xx in xs:
        c.linha((xx, cab), (xx, y))
    c.linha((x, cab), (x + larg, cab))
    return y


def _celula(x0, x1, alinh):
    """Âncora do texto numa célula: (x, alinhamento horizontal)."""
    if alinh == "c":
        return ((x0 + x1) / 2.0, "c")
    if alinh == "d":
        return (x1 - 1.2, "d")
    return (x0 + 1.2, "e")

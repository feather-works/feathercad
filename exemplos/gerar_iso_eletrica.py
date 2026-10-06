"""Isométrico de uma instalação elétrica residencial (estilo do 3D de projeto elétrico): casa em traço fino, eletrodutos laranja grossos no
teto com descidas até os pontos (tomadas e interruptores em caixas pretas, com chamada e rótulo), luminárias em círculo, quadro, duto
enterrado tracejado até o poste de entrada com a caixa de medição.

Eixos (iso.py): E1 desce para a direita, E2 sobe para a direita, Z sobe; medidas em METROS.

    python3 gerar_iso_eletrica.py [saida_base]   # padrão ~/Documentos/FeatherCAD/instalacao_eletrica_isometrico
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from iso import Iso, n
from mcp_cliente import Mcp

PD = 2.8  # pé-direito
ZT = 2.7  # eletrodutos no teto
H = 0.3  # altura do texto dos rótulos (m)

I = Iso(k=1.0)
P = I.p


class Cena:
    def __init__(self):
        self.conduto = set()  # segmentos únicos (pares de pontos 3D) do eletroduto
        self.rotulos = []

    def seg(self, a, b):
        a, b = tuple(round(v, 3) for v in a), tuple(round(v, 3) for v in b)
        if a != b:
            self.conduto.add((a, b) if a < b else (b, a))

    def caminho(self, pts):
        for a, b in zip(pts, pts[1:]):
            self.seg(a, b)


def parede(a, b, z0=0.0, z1=PD):
    """Parede de (e1,e2) a (e1,e2): aresta de baixo, de cima e as verticais das pontas."""
    I.camada("ARQ-Paredes", "#8A8A8A")
    I.linha(P(a[0], a[1], z0), P(b[0], b[1], z0))
    I.linha(P(a[0], a[1], z1), P(b[0], b[1], z1))
    I.linha(P(a[0], a[1], z0), P(a[0], a[1], z1))
    I.linha(P(b[0], b[1], z0), P(b[0], b[1], z1))


def abertura(eixo, fixo, a, b, z0, z1):
    """Janela/porta num plano de parede: `eixo` = 'e2' (parede com e2 constante) ou 'e1'."""
    if eixo == "e2":
        pts = [P(a, fixo, z0), P(b, fixo, z0), P(b, fixo, z1), P(a, fixo, z1)]
    else:
        pts = [P(fixo, a, z0), P(fixo, b, z0), P(fixo, b, z1), P(fixo, a, z1)]
    I.camada("ARQ-Esquadrias", "#8A8A8A")
    I.poli(pts, True)


PAREDES = [((0, 0), (12, 0)), ((12, 0), (12, 7)), ((12, 7), (0, 7)), ((0, 7), (0, 0)),
           ((5, 0), (5, 7)), ((5, 3.5), (12, 3.5)), ((8.5, 0), (8.5, 3.5)), ((7, 3.5), (7, 7))]
# (eixo da parede, valor fixo, de, até, z0, z1, tipo)
ABERTURAS = [("e2", 0, 1.0, 3.6, 0.9, 2.1, "janela"), ("e2", 0, 9.6, 11.0, 0.9, 2.1, "janela"), ("e2", 0, 3.7, 4.7, 0.0, 2.1, "porta"),
             ("e2", 7, 9.0, 10.4, 0.9, 2.1, "janela"), ("e2", 7, 5.4, 6.4, 1.4, 2.1, "janela"), ("e1", 12, 1.0, 2.4, 0.9, 2.1, "janela"),
             ("e1", 12, 4.5, 6.0, 0.9, 2.1, "janela"), ("e1", 5, 2.9, 3.7, 0.0, 2.1, "porta"), ("e1", 8.5, 2.4, 3.2, 0.0, 2.1, "porta"),
             ("e1", 7, 3.9, 4.7, 0.0, 2.1, "porta")]


def construcao():
    # lote e muro (traço fino, como no 3D de referência)
    I.camada("ARQ-Terreno", "#B5B5B5")
    I.poli([P(-3, -3, 0), P(21, -3, 0), P(21, 10, 0), P(-3, 10, 0)], True)
    I.camada("ARQ-Muro", "#A0A0A0")
    for a, b in (((-3, -3), (15, -3)), ((15, -3), (15, 10)), ((15, 10), (-3, 10)), ((-3, 10), (-3, -3))):
        I.linha(P(a[0], a[1], 0), P(b[0], b[1], 0))
        I.linha(P(a[0], a[1], 1.0), P(b[0], b[1], 1.0))
        I.linha(P(a[0], a[1], 0), P(a[0], a[1], 1.0))
    for a, b in PAREDES:
        parede(a, b)
    for eixo, fixo, a, b, z0, z1, _ in ABERTURAS:
        abertura(eixo, fixo, a, b, z0, z1)


# (nome, (e1, e2), z, rótulo, tipo)   tipo: tomada | interruptor | alta
PONTOS = [
    ("S1", (4.95, 3.2), 1.2, "S1  a  C1", "interruptor"), ("S2", (5.05, 3.9), 1.2, "S2  b  C1", "interruptor"),
    ("S3", (5.05, 4.4), 1.2, "S3  c  C1", "interruptor"), ("S4", (7.05, 4.4), 1.2, "S4  d  C1", "interruptor"),
    ("S5", (8.55, 3.0), 1.2, "S5  e  C1", "interruptor"),
    ("T1", (0.05, 1.5), 0.3, "TUG 100VA  C2", "tomada"), ("T2", (0.05, 5.5), 0.3, "TUG 100VA  C2", "tomada"),
    ("T3", (2.5, 6.95), 1.1, "TUG 600VA  C3", "tomada"), ("T4", (6.2, 0.05), 1.1, "TUG 600VA  C3", "tomada"),
    ("T5", (7.6, 0.05), 1.1, "TUG 600VA  C3", "tomada"), ("T6", (8.45, 1.5), 0.3, "TUG 600VA  C3", "tomada"),
    ("T7", (5.05, 6.0), 1.1, "TUG 600VA  C3", "tomada"), ("T8", (7.05, 6.2), 0.3, "TUG 100VA  C2", "tomada"),
    ("T9", (11.95, 5.0), 0.3, "TUG 100VA  C2", "tomada"), ("T10", (9.5, 6.95), 0.3, "TUG 100VA  C2", "tomada"),
    ("T11", (8.55, 1.0), 0.3, "TUG 100VA  C2", "tomada"), ("T12", (11.95, 1.5), 0.3, "TUG 100VA  C2", "tomada"),
    ("T13", (10.0, 0.05), 0.3, "TUG 100VA  C2", "tomada"),
    ("A1", (0.05, 3.5), 2.2, "TUE ar-cond. 1350W  C5", "alta"), ("A2", (11.95, 6.2), 2.2, "TUE ar-cond. 1350W  C5", "alta"),
    ("A3", (11.95, 0.8), 2.2, "TUE ar-cond. 1350W  C5", "alta"), ("CH", (6.95, 5.5), 2.2, "TUE chuveiro 5400W  C6", "alta"),
]
LUMINARIAS = [("L1", (2.5, 3.5), "L1  100W  C1"), ("L2", (6.75, 1.75), "L2  100W  C1"), ("L3", (6.0, 5.25), "L3  60W  C1"),
              ("L4", (9.5, 5.25), "L4  100W  C1"), ("L5", (10.25, 1.75), "L5  100W  C1")]
ESPINHA = 3.4  # e2 do tronco no teto
QD = (5.0, 0.3)


def cubo_preto(e1, e2, z, tam=0.14, alt=0.2):
    I.caixa(e1 - tam / 2, e1 + tam / 2, e2 - tam / 2, e2 + tam / 2, z - alt / 2, z + alt / 2, topo="#000000", frente="#000000", esquerda="#000000")


def rotulo(cena, e1, e2, z, texto, i, lado=None):
    """Pede uma chamada para o ponto 3D; a posição final sai de `colocar_rotulos` (sem sobrepor outro rótulo)."""
    cena.rotulos.append((P(e1, e2, z), texto, lado if lado is not None else (1 if i % 2 == 0 else -1)))


def colocar_rotulos(cena):
    """Coloca cada chamada perto do ponto (para cima, à direita ou à esquerda) subindo até não bater em outro texto."""
    caixas = []
    batem = lambda c: any(not (c[2] + 0.2 < o[0] or c[0] > o[2] + 0.2 or c[3] + 0.15 < o[1] or c[1] > o[3] + 0.15) for o in caixas)
    for q, texto, pref in sorted(cena.rotulos, key=lambda r: -r[0][1]):
        larg = 0.6 * H * len(texto)
        achou = None
        for k in range(60):
            for lado in (pref, -pref):
                t = (q[0] + 0.6 * lado, q[1] + 0.7 + 0.42 * k)
                c = (t[0], t[1], t[0] + larg, t[1] + H + 0.1) if lado > 0 else (t[0] - larg, t[1], t[0], t[1] + H + 0.1)
                if not batem(c):
                    achou = (t, c, lado)
                    break
            if achou:
                break
        t, c, lado = achou or ((q[0] + 0.6, q[1] + 0.7), (q[0], q[1], q[0], q[1]), 1)
        caixas.append(c)
        I.camada("EL-Rotulos", "#FFFFFF")
        I.linha(q, t)
        I.linha(t, (c[2] if lado > 0 else c[0], t[1]))
        I.camada("EL-Textos", "#FFFFFF")
        I.texto(c[0], t[1] + 0.06, H, texto)


def instalacao(cena):
    # quadro de distribuição (cubo preto na parede, com rótulo) e a descida ao teto
    e1, e2 = QD
    I.caixa(e1 - 0.2, e1 + 0.2, e2 - 0.06, e2 + 0.06, 1.2, 1.75, topo="#000000", frente="#000000", esquerda="#000000")
    cena.caminho([(e1, e2, 1.75), (e1, e2, ZT), (e1, ESPINHA, ZT)])
    # tronco no teto e ramais às luminárias
    cena.caminho([(0.6, ESPINHA, ZT), (11.4, ESPINHA, ZT)])
    for nome, (l1, l2), _ in LUMINARIAS:
        cena.caminho([(l1, ESPINHA, ZT), (l1, l2, ZT)])
    # descidas aos pontos
    for nome, (p1, p2), z, _, tipo in PONTOS:
        cena.caminho([(p1, ESPINHA, ZT), (p1, p2, ZT), (p1, p2, z + 0.1)])
    # desenha os eletrodutos (laranja grosso) como linhas únicas
    I.camada("EL-Eletrodutos", "#D9791A")
    for a, b in sorted(cena.conduto):
        I.linha(P(*a), P(*b))
    # pontos: caixas pretas
    for nome, (p1, p2), z, rot, tipo in PONTOS:
        cubo_preto(p1, p2, z)
    # luminárias: círculo com X no teto
    for nome, (l1, l2), _ in LUMINARIAS:
        I.camada("EL-Luminarias", "#FFFFFF")
        I.poli(I.elipse_pts(l1, l2, ZT, 0.28), True)
        a, b, c, d = P(l1 - 0.2, l2 - 0.2, ZT), P(l1 + 0.2, l2 + 0.2, ZT), P(l1 - 0.2, l2 + 0.2, ZT), P(l1 + 0.2, l2 - 0.2, ZT)
        I.linha(a, b)
        I.linha(c, d)
    # rótulos
    for i, (nome, (p1, p2), z, rot, tipo) in enumerate(PONTOS):
        rotulo(cena, p1, p2, z, rot, i)
    for i, (nome, (l1, l2), rot) in enumerate(LUMINARIAS):
        rotulo(cena, l1, l2, ZT + 0.05, rot, i + 1)
    rotulo(cena, QD[0], QD[1], 1.5, "QD  quadro de distribuição", 3)


def entrada(cena):
    """Duto enterrado do quadro ao poste (tracejado) e o poste com a caixa de medição."""
    e1, e2 = QD
    pb = (18.0, -0.5)
    I.camada("EL-Enterrado", "#444444")
    pts = [P(e1, e2, 1.2), P(e1, e2, -0.5), P(e1, -0.5, -0.5), P(pb[0], pb[1], -0.5), P(pb[0], pb[1], 0.0)]
    I.poli(pts, False)
    # poste (retângulo fino de 6 m), braço e caixa de medição
    I.camada("EL-Poste", "#6B6B6B")
    for dz, cor in ((0.0, None),):
        I.poli([P(pb[0] - 0.12, pb[1], 0), P(pb[0] + 0.12, pb[1], 0), P(pb[0] + 0.12, pb[1], 6.0), P(pb[0] - 0.12, pb[1], 6.0)], True)
    I.linha(P(pb[0] - 0.5, pb[1], 5.6), P(pb[0] + 0.5, pb[1], 5.6))
    I.linha(P(pb[0], pb[1], 0.0), P(pb[0], pb[1], -0.3))
    I.caixa(pb[0] - 0.3, pb[0] + 0.3, pb[1] - 0.2, pb[1] + 0.2, 1.3, 2.0, topo="#F4F4F4", frente="#E0E0E0", esquerda="#CFCFCF")
    I.camada("EL-Eletrodutos", "#D9791A")
    I.linha(P(pb[0], pb[1], 2.0), P(pb[0], pb[1], 5.6))
    for texto, q, lado in (("POSTE DE ENTRADA", P(pb[0], pb[1], 5.0), 1), ("CAIXA DE MEDIÇÃO", P(pb[0] + 0.3, pb[1], 1.7), 1),
                           ("ELETRODUTO ENTERRADO Ø25 mm", P(12.0, -0.5, -0.5), -1), ("ATERRAMENTO: HASTE 2,40 m", P(pb[0], pb[1], -0.3), 1)):
        cena.rotulos.append((q, texto, lado))


def titulo_e_legenda():
    x0, y0 = P(-3, 10, 0)[0] - 2.0, P(-3, 10, 0)[1] + 4.5
    I.camada("EL-Textos", "#FFFFFF")
    I.texto(x0, y0, 0.7, "INSTALAÇÃO ELÉTRICA - VISTA ISOMÉTRICA")
    I.texto(x0, y0 - 1.0, 0.35, "Residência de 84 m² - eletrodutos no teto, descidas aos pontos e entrada de energia em poste. Sem escala.")
    ly = y0 - 2.6
    I.camada("EL-Eletrodutos", "#D9791A")
    I.linha((x0, ly), (x0 + 1.4, ly))
    I.camada("EL-Textos", "#FFFFFF")
    I.texto(x0 + 1.8, ly - 0.1, 0.3, "Eletroduto embutido no teto e nas paredes")
    I.camada("EL-Enterrado", "#444444")
    I.linha((x0, ly - 0.8), (x0 + 1.4, ly - 0.8))
    I.camada("EL-Textos", "#FFFFFF")
    I.texto(x0 + 1.8, ly - 0.9, 0.3, "Eletroduto enterrado (vala)")


def exportar_cena(cena, caminho):
    """A mesma cena em 3D (metros), para o Blender: paredes, aberturas, pontos, luminárias, eletrodutos e a entrada de energia."""
    import json
    pb = [18.0, -0.5]
    dados = {
        "pe_direito": PD, "zt": ZT, "paredes": PAREDES, "aberturas": ABERTURAS,
        "pontos": [{"nome": n_, "e": list(pos), "z": z, "rotulo": r, "tipo": t} for n_, pos, z, r, t in PONTOS],
        "luminarias": [{"nome": n_, "e": list(pos), "rotulo": r} for n_, pos, r in LUMINARIAS],
        "conduto": [[list(a), list(b)] for a, b in sorted(cena.conduto)],
        "quadro": list(QD), "poste": pb,
        "enterrado": [[QD[0], QD[1], 1.2], [QD[0], QD[1], -0.5], [QD[0], -0.5, -0.5], [pb[0], pb[1], -0.5], [pb[0], pb[1], 0.0]],
        "terreno": [-3, -3, 21, 10], "muro": [-3, -3, 15, 10],
    }
    json.dump(dados, open(caminho, "w"), ensure_ascii=False, indent=1)


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.expanduser("~"), "Documentos", "FeatherCAD", "instalacao_eletrica_isometrico")
    cena = Cena()
    construcao()
    instalacao(cena)
    entrada(cena)
    colocar_rotulos(cena)
    titulo_e_legenda()
    fct = "unidade m\n" + "\n".join(I.out) + "\n"
    exportar_cena(cena, os.path.join(os.path.expanduser("~"), "Documentos", "FeatherCAD", "modelo3d", "cena.json"))
    caminho = base + ".fct"
    open(caminho, "w").write(fct)
    m = Mcp()
    m.call("cad_abrir", {"caminho": caminho})
    for nome, esp, tipo in (("EL-Eletrodutos", 1.0, None), ("EL-Enterrado", 0.5, "tracejada"), ("EL-Poste", 0.35, None), ("ARQ-Paredes", 0.18, None),
                            ("ARQ-Esquadrias", 0.13, None), ("ARQ-Muro", 0.13, None), ("ARQ-Terreno", 0.13, None), ("EL-Rotulos", 0.13, None)):
        a = {"acao": "propriedade", "nome": nome, "espessura": esp}
        if tipo:
            a["tipo_linha"] = tipo
        m.call("cad_camadas", a)
    for ext in (".fcdz", ".dxf"):
        m.call("cad_salvar", {"caminho": base + ext, "sobrescrever": True})
    m.call("cad_pdf", {"caminho": base + ".pdf", "papel": "A3", "orientacao": "horizontal", "cores": True, "sobrescrever": True})
    r = m.call("cad_ver", {"largura": 2400, "grade": False, "rotulos": False, "arquivo": "/tmp/iso_eletrica.png"})
    print(r)
    print("gravado em", base)
    m.fechar()


if __name__ == "__main__":
    main()

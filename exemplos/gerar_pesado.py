#!/usr/bin/env python3
"""Gera um projeto DXF EXTREMAMENTE pesado, para testar o desempenho do FeatherCAD (zoom, pan, cursor, abrir).

Uso:
    python3 exemplos/gerar_pesado.py [saida.dxf] [--escala N]

`--escala 1` (padrão): ~250 mil objetos, ~700 mil segmentos (arquivo de uns 40 MB).
`--escala 4`: ~1 milhão de objetos (uns 160 MB). `--escala 0.25`: leve, para conferir o gerador.

É uma "cidade": ruas em grade (linhas), quadras (polilinhas fechadas, algumas com cantos arredondados por bulge),
árvores (blocos ARVORE e círculos), poços (arcos), curvas de nível (polilinhas longas), placas e rótulos (TEXT,
uns com giro). Tudo determinístico (semente fixa): o mesmo comando gera o mesmo arquivo.
Não commitar o resultado (é grande); gere de novo quando precisar.
"""

import argparse
import math
import random
import sys

CAMADAS = [
    ("RUAS", 8),
    ("QUADRAS", 3),
    ("EDIFICACOES", 1),
    ("ARVORES", 3),
    ("CURVAS_NIVEL", 30),
    ("TEXTOS", 7),
    ("POCOS", 5),
    ("EIXOS", 2),
]


class Escritor:
    """Escreve pares código/valor do DXF em blocos grandes (rápido mesmo com milhões de linhas)."""

    def __init__(self, caminho):
        self.f = open(caminho, "w", encoding="utf-8", newline="\n", buffering=1 << 20)
        self.n = 0

    def par(self, codigo, valor):
        self.f.write(f"{codigo}\n{valor}\n")

    def entidade(self, tipo, camada, *pares):
        self.f.write(f"0\n{tipo}\n8\n{camada}\n")
        for codigo, valor in pares:
            self.f.write(f"{codigo}\n{valor}\n")
        self.n += 1

    def linha(self, camada, x1, y1, x2, y2):
        self.entidade("LINE", camada, (10, f"{x1:.3f}"), (20, f"{y1:.3f}"), (30, 0), (11, f"{x2:.3f}"), (21, f"{y2:.3f}"), (31, 0))

    def circulo(self, camada, x, y, r):
        self.entidade("CIRCLE", camada, (10, f"{x:.3f}"), (20, f"{y:.3f}"), (30, 0), (40, f"{r:.3f}"))

    def arco(self, camada, x, y, r, a1, a2):
        self.entidade("ARC", camada, (10, f"{x:.3f}"), (20, f"{y:.3f}"), (30, 0), (40, f"{r:.3f}"), (50, f"{a1:.2f}"), (51, f"{a2:.2f}"))

    def texto(self, camada, x, y, altura, conteudo, giro=0.0):
        self.entidade("TEXT", camada, (10, f"{x:.3f}"), (20, f"{y:.3f}"), (30, 0), (40, f"{altura:.2f}"), (1, conteudo), (50, f"{giro:.1f}"))

    def insercao(self, camada, nome, x, y, escala=1.0, giro=0.0):
        self.entidade("INSERT", camada, (2, nome), (10, f"{x:.3f}"), (20, f"{y:.3f}"), (30, 0), (41, f"{escala:.3f}"), (42, f"{escala:.3f}"), (50, f"{giro:.1f}"))

    def polilinha(self, camada, vertices, fechada=False):
        """POLYLINE 2D do R12: vértices (x, y) ou (x, y, bulge)."""
        self.f.write(f"0\nPOLYLINE\n8\n{camada}\n66\n1\n10\n0\n20\n0\n30\n0\n70\n{1 if fechada else 0}\n")
        for v in vertices:
            bulge = f"42\n{v[2]:.4f}\n" if len(v) > 2 and v[2] else ""
            self.f.write(f"0\nVERTEX\n8\n{camada}\n10\n{v[0]:.3f}\n20\n{v[1]:.3f}\n30\n0\n{bulge}")
        self.f.write(f"0\nSEQEND\n8\n{camada}\n")
        self.n += 1


def cabecalho(e):
    e.par(0, "SECTION"); e.par(2, "HEADER"); e.par(9, "$ACADVER"); e.par(1, "AC1009"); e.par(0, "ENDSEC")
    e.par(0, "SECTION"); e.par(2, "TABLES"); e.par(0, "TABLE"); e.par(2, "LAYER"); e.par(70, len(CAMADAS))
    for nome, cor in CAMADAS:
        e.par(0, "LAYER"); e.par(2, nome); e.par(70, 0); e.par(62, cor); e.par(6, "CONTINUOUS")
    e.par(0, "ENDTAB"); e.par(0, "ENDSEC")
    # o bloco ARVORE: copa (círculo), tronco (cruz) e um arco de sombra
    e.par(0, "SECTION"); e.par(2, "BLOCKS")
    e.par(0, "BLOCK"); e.par(8, "0"); e.par(2, "ARVORE"); e.par(70, 0); e.par(10, 0); e.par(20, 0); e.par(30, 0); e.par(3, "ARVORE")
    e.circulo("0", 0, 0, 3.0)
    e.linha("0", -1.2, 0, 1.2, 0)
    e.linha("0", 0, -1.2, 0, 1.2)
    e.arco("0", 0, 0, 4.0, 200, 340)
    e.par(0, "ENDBLK"); e.par(8, "0")
    e.par(0, "ENDSEC")


def cidade(e, escala, rnd):
    lado = max(8, int(round(200 * math.sqrt(escala))))  # quadras por lado
    passo = 60.0  # metros entre eixos de rua
    total = lado * passo
    e.par(0, "SECTION"); e.par(2, "ENTITIES")

    # ruas: cada rua é cortada em trechos de uma quadra (muitos segmentos, como num arquivo de levantamento real)
    for i in range(lado + 1):
        p = i * passo
        for j in range(lado):
            q = j * passo
            e.linha("RUAS", p, q, p, q + passo)       # rua vertical
            e.linha("RUAS", q, p, q + passo, p)       # rua horizontal
        if i % 10 == 0:
            e.linha("EIXOS", p, -20, p, total + 20)
            e.linha("EIXOS", -20, p, total + 20, p)
            e.texto("TEXTOS", p, -35, 6, f"EIXO {i // 10}")
            e.texto("TEXTOS", -60, p, 6, f"EIXO {chr(65 + (i // 10) % 26)}")

    # quadras: lote com edificação (polilinha fechada); alguns cantos arredondados por bulge
    nomes = ["Rua das Flores", "Av. Central", "Rua do Comércio", "Travessa Sul", "Alameda dos Ipês", "Rua da Estação"]
    for i in range(lado):
        for j in range(lado):
            x, y = i * passo + 6, j * passo + 6
            w, h = passo - 12, passo - 12
            e.polilinha("QUADRAS", [(x, y), (x + w, y), (x + w, y + h), (x, y + h)], fechada=True)
            if rnd.random() < 0.85:
                m = rnd.uniform(4, 9)
                if rnd.random() < 0.3:
                    b = 0.4142  # tan(90°/4): canto de 90° arredondado (o bulge vale para o segmento que SAI do vértice)
                    v = [
                        (x + m + 4, y + m, 0),
                        (x + w - m, y + m, 0),
                        (x + w - m, y + h - m - 6, b),   # canto superior direito, raio 6
                        (x + w - m - 6, y + h - m, 0),
                        (x + m, y + h - m, 0),
                        (x + m, y + m + 4, b),            # canto inferior esquerdo, raio 4 (fecha no primeiro vértice)
                    ]
                    e.polilinha("EDIFICACOES", v, fechada=True)
                else:
                    e.polilinha("EDIFICACOES", [(x + m, y + m), (x + w - m, y + m), (x + w - m, y + h - m), (x + m, y + h - m)], fechada=True)
                # árvores: blocos e alguns círculos soltos
                for _ in range(rnd.randint(0, 2)):
                    e.insercao("ARVORES", "ARVORE", x + rnd.uniform(1, w - 1), y + rnd.uniform(1, h - 1), rnd.uniform(0.6, 1.4), rnd.uniform(0, 360))
            if rnd.random() < 0.08:
                e.circulo("POCOS", x + w / 2, y + h / 2, rnd.uniform(1.0, 2.5))
                e.arco("POCOS", x + w / 2, y + h / 2, rnd.uniform(3.0, 5.0), rnd.uniform(0, 180), rnd.uniform(200, 359))
            if (i * lado + j) % 7 == 0:
                e.texto("TEXTOS", x + 3, y + h / 2, 2.5, f"Q{i:03d}-{j:03d}  Lote {rnd.randint(1, 40)}  {rnd.randint(80, 900)} m2", rnd.choice([0.0, 0.0, 90.0, 15.0]))
        if i % 5 == 0:
            e.texto("TEXTOS", i * passo + 3, total / 2, 5, rnd.choice(nomes), 90.0)

    # curvas de nível: polilinhas longas e suaves atravessando a cidade (muitos vértices)
    n_curvas = max(20, int(round(300 * escala)))
    for k in range(n_curvas):
        y0 = rnd.uniform(0, total)
        amp = rnd.uniform(20, 120)
        fase = rnd.uniform(0, 6.28)
        freq = rnd.uniform(1.0, 4.0)
        pontos = 400
        v = []
        for t in range(pontos + 1):
            x = total * t / pontos
            v.append((x, y0 + amp * math.sin(fase + freq * 6.2832 * t / pontos) + 10 * math.sin(37 * t / pontos)))
        e.polilinha("CURVAS_NIVEL", v)
        e.texto("TEXTOS", total * 0.5, v[pontos // 2][1] + 3, 4, f"{100 + k * 5},00")

    e.par(0, "ENDSEC"); e.par(0, "EOF")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("saida", nargs="?", default="exemplos/projeto_pesado.dxf")
    ap.add_argument("--escala", type=float, default=1.0, help="1 = ~250 mil objetos; 4 = ~1 milhão; 0.25 = leve")
    a = ap.parse_args()
    rnd = random.Random(20260930)
    e = Escritor(a.saida)
    cabecalho(e)
    cidade(e, a.escala, rnd)
    e.f.close()
    print(f"{a.saida}: {e.n} objetos gravados (fora os do bloco ARVORE)", file=sys.stderr)


if __name__ == "__main__":
    main()

"""Projeto luminotécnico v3 (galpão 40 x 20 m, 24 h): gerado pelo MCP, isolado (não mexe no aplicativo aberto).

O CÁLCULO é do Rust (`cad_luminotecnico`, `cad_circuitos`, `cad_notas_padrao`, `cad_carimbo`); este script só decide o leiaute:
4 folhas A1 em 1:100 (planta de iluminação, mapa de lux e cálculo, corte e dimerização, quadros/unifilar/materiais).

    python3 luminotecnico_v3.py [saida_base]        # padrão: ~/Documentos/FeatherCAD/projeto_luminotecnico_galpao_v3
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_cliente import Mcp

ESC = 100
K = ESC / 1000.0  # metros do desenho por mm de papel
C_AMB, L_AMB, H_AMB = 40.0, 20.0, 6.0
LARGURA_FOLHA = 100.0  # distância entre folhas (m)

CAMADAS = {  # nome: (cor, espessura mm, tipo_linha)
    "ARQ-Paredes": ("#7F8C8D", 0.5, None), "ARQ-Esquadrias": ("#B07A5A", 0.25, None), "ARQ-Pilares": ("#8D8D8D", 0.35, None),
    "ARQ-Eixos": ("#9AA5A6", 0.13, "eixo"), "ARQ-Cotas": ("#8D8D8D", 0.13, None), "ARQ-Estrutura": ("#7F8C8D", 0.35, None),
    "EL-Luminarias": ("#E69500", 0.25, None), "EL-Claraboias": ("#1E88E5", 0.25, None), "EL-Fotocelula": ("#1E88E5", 0.25, None),
    "EL-Quadro": ("#D84315", 0.35, None), "EL-Condutos": ("#C0392B", 0.35, None), "EL-Emergencia": ("#2E9E4F", 0.35, None),
    "EL-Textos": ("#FFFFFF", 0.13, None), "EL-Feixes": ("#E69500", 0.13, "tracejada"), "EL-Feixes-Natural": ("#1E88E5", 0.13, "tracejada"),
    "LUX": ("#2E9E8F", 0.0, None), "DI-Tabelas": ("#FFFFFF", 0.13, None), "DI-Linhas do diagrama": ("#FFFFFF", 0.25, None),
    "DI-Disjuntor": ("#FFFFFF", 0.25, None), "DI-Textos do diagrama": ("#FFFFFF", 0.13, None),
    "F-Notas": ("#FFFFFF", 0.13, None), "F-Legenda": ("#FFFFFF", 0.13, None), "F-Titulos": ("#FFFFFF", 0.25, None),
}


class Lote:
    """Acumula formas por camada e as grava de uma vez (cad_criar)."""

    def __init__(self, m):
        self.m, self.f = m, {}

    def add(self, camada, forma, cor=None):
        self.f.setdefault((camada, cor), []).append(forma)

    def linha(self, camada, a, b, cor=None):
        self.add(camada, {"tipo": "linha", "a": list(a), "b": list(b)}, cor)

    def poli(self, camada, pts, fechada=False, cor=None):
        self.add(camada, {"tipo": "polilinha", "pontos": [list(p) for p in pts], "fechada": fechada}, cor)

    def ret(self, camada, a, b, cor=None):
        self.add(camada, {"tipo": "retangulo", "canto1": list(a), "canto2": list(b)}, cor)

    def circ(self, camada, c, r, cor=None):
        self.add(camada, {"tipo": "circulo", "centro": list(c), "raio": r}, cor)

    def texto(self, camada, p, s, h=0.25, centro=False, cor=None):
        if centro:
            p = (p[0] - 0.5 * 0.6 * h * len(s), p[1])
        self.add(camada, {"tipo": "texto", "ponto": [p[0], p[1]], "altura": h, "conteudo": s}, cor)

    def cheio(self, camada, pts, cor=None):
        self.add(camada, {"tipo": "hachura", "pontos": [list(p) for p in pts]}, cor)

    def gravar(self):
        for (camada, cor), formas in self.f.items():
            args = {"formas": formas, "camada": camada}
            if cor:
                args["cor"] = cor
            self.m.call("cad_criar", args)
        self.f = {}


def papel_para_doc(o, xp, yp):
    return (o[0] + xp * K, o[1] + yp * K)


def fmt(v, d=2):
    return f"{v:,.{d}f}".replace(",", "X").replace(".", ",").replace("X", ".")


def _fmt_cota(v):
    return fmt(v)


def cota_cadeia_h(L, xs, y, camada="ARQ-Cotas", h=0.2, fator=1.0):
    """Cota horizontal encadeada: linha única, um tracinho por ponto, o valor de cada trecho no meio (valores reais = distância / fator)."""
    L.linha(camada, (xs[0], y), (xs[-1], y))
    for x in xs:
        L.linha(camada, (x - 0.15, y - 0.15), (x + 0.15, y + 0.15))
        L.linha(camada, (x, y - 0.35), (x, y + 0.35))
    for a, b in zip(xs, xs[1:]):
        L.texto("EL-Textos", ((a + b) / 2, y + 0.25), _fmt_cota((b - a) / fator), h, centro=True)


def cota_cadeia_v(L, ys, x, camada="ARQ-Cotas", h=0.2, fator=1.0):
    L.linha(camada, (x, ys[0]), (x, ys[-1]))
    for y in ys:
        L.linha(camada, (x - 0.15, y - 0.15), (x + 0.15, y + 0.15))
        L.linha(camada, (x - 0.35, y), (x + 0.35, y))
    for a, b in zip(ys, ys[1:]):
        t = _fmt_cota((b - a) / fator)
        L.texto("EL-Textos", (x - 0.3 - 0.6 * h * len(t), (a + b) / 2 - h / 2), t, h)


def cota_h(L, x1, x2, y, **kw):
    cota_cadeia_h(L, [x1, x2], y, **kw)


def cota_v(L, y1, y2, x, **kw):
    cota_cadeia_v(L, [y1, y2], x, **kw)


def paredes(L, p0):
    """Paredes de 0,20 m com portão (sul), porta (norte) e porta (leste); retorna as esquadrias para rotular."""
    x0, y0 = p0
    c, l, e = C_AMB, L_AMB, 0.2

    def seg(a, b):
        L.linha("ARQ-Paredes", (x0 + a[0], y0 + a[1]), (x0 + b[0], y0 + b[1]))

    gaps = {"S": (17.0, 23.0), "N": (3.4, 4.6), "L": (9.4, 10.6)}
    for off, (ya, yb) in ((-e, (-e, 0)), (0.0, (0, 0))):
        pass
    # sul
    for yy, x_ini, x_fim in ((-e, -e, c + e), (0.0, 0.0, c)):
        seg((x_ini, yy), (gaps["S"][0], yy))
        seg((gaps["S"][1], yy), (x_fim, yy))
    for xx in gaps["S"]:
        seg((xx, -e), (xx, 0.0))
    # norte
    for yy, x_ini, x_fim in ((l + e, -e, c + e), (l, 0.0, c)):
        seg((x_ini, yy), (gaps["N"][0], yy))
        seg((gaps["N"][1], yy), (x_fim, yy))
    for xx in gaps["N"]:
        seg((xx, l), (xx, l + e))
    # oeste
    seg((-e, -e), (-e, l + e))
    seg((0.0, 0.0), (0.0, l))
    # leste
    for xx, y_ini, y_fim in ((c + e, -e, l + e), (c, 0.0, l)):
        seg((xx, y_ini), (xx, gaps["L"][0]))
        seg((xx, gaps["L"][1]), (xx, y_fim))
    for yy in gaps["L"]:
        seg((c, yy), (c + e, yy))
    # esquadrias: folha do portão (tracejada na camada de esquadrias) e giro das portas
    L.ret("ARQ-Esquadrias", (x0 + 17.0, y0 - 0.1), (x0 + 23.0, y0 + 0.1))
    L.ret("ARQ-Esquadrias", (x0 + 3.4, y0 + l - 0.05), (x0 + 4.6, y0 + l + 0.05))
    L.ret("ARQ-Esquadrias", (x0 + c - 0.05, y0 + 9.4), (x0 + c + 0.05, y0 + 10.6))
    L.texto("EL-Textos", (x0 + 17.6, y0 - 1.15), "Portão 6,00 m", 0.2)
    L.texto("EL-Textos", (x0 + 2.4, y0 + l + 0.55), "Porta 1,20 m", 0.2)
    L.texto("EL-Textos", (x0 + c + 0.5, y0 + 10.6 + 0.1), "Porta 1,20 m", 0.2)


def eixos_e_pilares(L, p0):
    x0, y0 = p0
    for i in range(6):
        x = i * 8.0
        px = min(max(x, 0.2), C_AMB - 0.2)
        L.linha("ARQ-Eixos", (x0 + x, y0 - 1.3), (x0 + x, y0 + L_AMB + 1.5))
        L.ret("ARQ-Pilares", (x0 + px - 0.2, y0), (x0 + px + 0.2, y0 + 0.4))
        L.ret("ARQ-Pilares", (x0 + px - 0.2, y0 + L_AMB - 0.4), (x0 + px + 0.2, y0 + L_AMB))
        L.circ("ARQ-Eixos", (x0 + x, y0 + L_AMB + 2.0), 0.55, "#8D8D8D")
        L.texto("EL-Textos", (x0 + x, y0 + L_AMB + 2.0 - 0.1), f"P{i + 1}", 0.2, centro=True)


def cotas_da_planta(L, p0, xs, ys):
    x0, y0 = p0
    cota_cadeia_h(L, [x0 + i * 8.0 for i in range(6)], y0 + L_AMB + 3.4)
    cota_h(L, x0, x0 + C_AMB, y0 + L_AMB + 4.8)
    cota_cadeia_h(L, [x0 + v for v in [0.0] + xs + [C_AMB]], y0 - 2.6)
    cota_h(L, x0, x0 + C_AMB, y0 - 4.0)
    cota_cadeia_v(L, [y0 + v for v in [0.0] + ys + [L_AMB]], x0 - 2.0)
    cota_v(L, y0, y0 + L_AMB, x0 - 3.6)


def ambiente(luz):
    return {"comprimento": C_AMB, "largura": L_AMB, "pe_direito": H_AMB, "plano_trabalho": 0.8, "refletancias": [0.5, 0.5, 0.2]}


def calcular(m):
    base = {"ambiente": ambiente(None), "iluminancia_lux": 300, "fator_manutencao": 0.8, "grade_passo": 1, "espacamento_max_hm": 1.5,
            "luminaria": {"fluxo_lm": 21000, "potencia_w": 150, "altura_montagem": 6.0, "fator_utilizacao": 0.72, "expoente": 1.0},
            "arranjo": {"linhas": 3, "colunas": 7}}
    return base


def configurar_camadas(m):
    for nome, (cor, esp, tipo) in CAMADAS.items():
        a = {"acao": "criar", "nome": nome, "cor": cor}
        if esp:
            a["espessura"] = esp
        if tipo:
            a["tipo_linha"] = tipo
        m.call("cad_camadas", a)


def inserir_folha(m, i, titulo, subtitulo, total):
    o = (i * LARGURA_FOLHA, 0.0)
    campos = {"cliente": "GALPÃO INDUSTRIAL 20 x 40 m - OPERAÇÃO 24 h", "fase": "PROJETO EXECUTIVO", "disciplina": "ELÉTRICA - ILUMINAÇÃO",
              "titulo": titulo, "subtitulo": subtitulo, "numero": f"ELE-{i + 1:02d}", "folha": f"{i + 1}/{total}", "revisao": "R00",
              "revisao_descricao": "EMISSÃO INICIAL", "responsavel": "PROJETISTA: A PREENCHER"}
    m.call("cad_folha", {"acao": "inserir", "papel": "A1", "escala": ESC, "campos": campos, "ponto": list(o)})
    r = m.call("cad_carimbo", {"indice": i, "linhas": ["CREA/CAU: A PREENCHER", "ART/RRT Nº: A PREENCHER",
                                                       "PROPRIETÁRIO: A PREENCHER", "ÁREA: 800,00 m²   DATA: 04/10/2026"]})
    return o, r["faixa"]


def bloco_na_caixa(m, caixa, itens, altura=1.8, colunas=1, camada="F-Notas"):
    x0, y1 = caixa["min"][0] + 1.5 * K, caixa["max"][1] - 1.5 * K
    larg = caixa["largura_mm_papel"] - 3.0
    a = {"origem": [x0, y1], "largura": larg, "altura_texto": altura, "escala": ESC, "itens": itens, "camada": camada}
    if colunas > 1:
        a["colunas"] = colunas
        a["altura_max"] = caixa["altura_mm_papel"] - 3.0
    r = m.call("cad_texto_bloco", a)
    return r


def tabela(m, origem, titulo, colunas, linhas, **kw):
    a = {"origem": list(origem), "escala": ESC, "titulo": titulo, "colunas": colunas, "linhas": linhas, "altura_linha": 4.5,
         "altura_texto": 2.0, "camada": "DI-Tabelas", "virgula": True}
    a.update(kw)
    return m.call("cad_tabela", a)


def comprimentos_dos_circuitos(posicoes):
    """Do QDL (x=0,6; y=10) até a última luminária da fileira, pelo tronco em x=1,5 e subida de 4,5 m do quadro ao eletroduto."""
    linhas = sorted({round(y, 3) for _, y in posicoes})
    res = []
    for y in linhas:
        xs = [x for x, yy in posicoes if abs(yy - y) < 1e-6]
        res.append(round(0.9 + abs(y - 10.0) + (max(xs) - 1.5) + 4.5, 1))
    return linhas, res


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.expanduser("~"), "Documentos", "FeatherCAD", "projeto_luminotecnico_galpao_v3")
    base = os.path.abspath(base)
    m = Mcp()
    m.call("cad_modo", {"projeto": True})
    m.call("cad_novo", {"unidade": "m"})
    configurar_camadas(m)

    # ---------------- cálculo (Rust) ----------------
    lum = m.call("cad_luminotecnico", {**calcular(m)})
    pos = [tuple(p) for p in lum["posicoes_m"]]
    pos.sort(key=lambda p: (round(p[1], 3), p[0]))
    linhas_y, comps = comprimentos_dos_circuitos(pos)
    n_por_fileira = len(pos) // len(linhas_y)
    circuitos = []
    for i, (y, comp) in enumerate(zip(linhas_y, comps)):
        ini, fim = i * n_por_fileira + 1, (i + 1) * n_por_fileira
        circuitos.append({"id": f"C{i + 1}", "descricao": f"Fileira {i + 1} (L{ini:02d}-L{fim:02d})", "tipo": "iluminacao",
                          "potencia_w": n_por_fileira * 150, "comprimento_m": comp})
    perimetro = 2 * (38.4 + 18.4)
    circuitos.append({"id": f"C{len(circuitos) + 1}", "descricao": "Emergência: 8 blocos + 3 saídas", "tipo": "iluminacao", "potencia_w": 100,
                      "fp": 1.0, "comprimento_m": round(perimetro + 1.5, 1)})
    dados_circ = {"quadro": {"nome": "QDL", "tensao_fn": 220, "tensao_ff": 380, "fases": 3, "alimentador_m": 15, "agrupamento": 4, "disjuntor_minimo_a": 10},
                  "circuitos": circuitos, "fatores_demanda": {"iluminação": 1.0}}
    calc = m.call("cad_circuitos", dados_circ)
    secoes = {c["id"]: c["secao_mm2"] for c in calc["circuitos"]}

    # ---------------- folhas ----------------
    o1, f1 = inserir_folha(m, 0, "PLANTA DE ILUMINAÇÃO", "Luminárias, claraboias, eletrodutos e emergência - 1:100", 4)
    o2, f2 = inserir_folha(m, 1, "ILUMINÂNCIA E CÁLCULO", "Método dos lumens e mapa de lux - 1:100", 4)
    o3, f3 = inserir_folha(m, 2, "CORTE E DIMERIZAÇÃO", "Corte transversal e esquema de dimerização", 4)
    o4, f4 = inserir_folha(m, 3, "QUADROS, UNIFILAR E MATERIAIS", "QDL, diagrama unifilar, lista de materiais e notas", 4)

    folha1(m, o1, f1, pos, linhas_y, lum, calc, secoes)
    folha2(m, o2, f2, pos, lum)
    folha3(m, o3, f3, pos)
    lista = folha4(m, o4, f4, dados_circ, lum, calc)

    # ---------------- confere e grava ----------------
    ver = m.call("cad_verificar", {"leiaute": True, "limite": 25})
    print(json.dumps(ver, ensure_ascii=False)[:2500])
    for ext in (".fcdz", ".dxf"):
        m.call("cad_salvar", {"caminho": base + ext, "sobrescrever": True})
    m.call("cad_pdf", {"caminho": base + ".pdf", "papel": "auto", "cores": True, "sobrescrever": True})
    for i in range(4):
        w = {"x1": i * LARGURA_FOLHA - 1, "y1": -1, "x2": i * LARGURA_FOLHA + 85.2, "y2": 60.5}
        m.call("cad_ver", {"janela": w, "largura": 2400, "grade": False, "rotulos": False, "arquivo": f"/tmp/lum_v3_f{i + 1}.png"})
    print("avisos do cálculo:", calc["avisos"], lum["avisos"])
    print("gravado em", base)
    m.fechar()


# ============================ FOLHA 1 ============================
def folha1(m, o, faixa, pos, linhas_y, lum, calc, secoes):
    L = Lote(m)
    p0 = (o[0] + 10.0, o[1] + 20.0)
    paredes(L, p0)
    eixos_e_pilares(L, p0)
    xs = sorted({round(x, 3) for x, _ in pos})
    cotas_da_planta(L, p0, xs, linhas_y)
    L.texto("F-Titulos", (p0[0], p0[1] + L_AMB + 7.4), "PLANTA DE ILUMINAÇÃO - ESC. 1:100", 0.5)
    L.texto("EL-Textos", (p0[0], p0[1] + L_AMB + 6.4), "Galpão 40,00 x 20,00 m - pé-direito 6,00 m - 800 m² - funcionamento 24 h", 0.25)
    # QDL
    qa, qb = (p0[0] + 0.1, p0[1] + 9.55), (p0[0] + 0.6, p0[1] + 10.45)
    L.ret("EL-Quadro", qa, qb)
    L.cheio("EL-Quadro", [qa, (qb[0], qa[1]), qb, (qa[0], qb[1])])
    L.texto("EL-Textos", (p0[0] + 0.12, p0[1] + 11.0), "QDL", 0.25)
    # claraboias e fotocélulas
    cl = [(6.0, 6.667), (14.0, 6.667), (26.0, 6.667), (34.0, 6.667), (6.0, 13.333), (14.0, 13.333), (26.0, 13.333), (34.0, 13.333)]
    for i, (cx, cy) in enumerate(cl):
        a, b = (p0[0] + cx - 1.5, p0[1] + cy - 1.0), (p0[0] + cx + 1.5, p0[1] + cy + 1.0)
        L.ret("EL-Claraboias", a, b)
        L.linha("EL-Claraboias", a, b)
        L.linha("EL-Claraboias", (a[0], b[1]), (b[0], a[1]))
        L.texto("EL-Textos", (p0[0] + cx, p0[1] + cy - 1.0 - 0.3), f"CL{i + 1}", 0.18, centro=True)
    L.gravar()
    for i, (cx, cy, nome) in enumerate([(20.0, 6.667, "FC1"), (20.0, 13.333, "FC2")]):
        m.call("cad_simbolo", {"nome": "fotocelula", "ponto": [p0[0] + cx, p0[1] + cy], "escala": ESC, "camada": "EL-Fotocelula", "comando": nome})
    # eletrodutos (marcas de condutor e seção vêm do cálculo)
    tronco_x = p0[0] + 1.5
    for i, y in enumerate(linhas_y):
        n = i + 1
        xs_f = [x for x, yy in pos if abs(yy - y) < 1e-6]
        s = secoes[f"C{n}"]
        m.call("cad_eletroduto", {"pontos": [[tronco_x, p0[1] + y], [p0[0] + max(xs_f) + 0.6, p0[1] + y]], "escala": ESC, "camada": "EL-Condutos", "posicao": 0.62,
                                  "condutores": [{"circuito": n, "tipo": "fase", "secao": s}, {"circuito": n, "tipo": "neutro", "secao": s}], "secao_terra": s})
    st = max(secoes[f"C{i + 1}"] for i in range(len(linhas_y)))
    m.call("cad_eletroduto", {"pontos": [[tronco_x, p0[1] + linhas_y[0]], [tronco_x, p0[1] + linhas_y[-1]]], "escala": ESC, "camada": "EL-Condutos", "posicao": 0.2,
                              "condutores": [{"circuito": i + 1, "tipo": t, "secao": secoes[f"C{i + 1}"]} for i in range(len(linhas_y)) for t in ("fase", "neutro")], "secao_terra": st})
    m.call("cad_criar", {"camada": "EL-Condutos", "formas": [{"tipo": "linha", "a": [p0[0] + 0.6, p0[1] + 10.0], "b": [tronco_x, p0[1] + 10.0]}]})
    loop = [[p0[0] + 0.6, p0[1] + 10.0], [p0[0] + 0.8, p0[1] + 10.0], [p0[0] + 0.8, p0[1] + 0.8], [p0[0] + 39.2, p0[1] + 0.8], [p0[0] + 39.2, p0[1] + 19.2],
            [p0[0] + 0.8, p0[1] + 19.2], [p0[0] + 0.8, p0[1] + 10.0]]
    s4 = secoes[f"C{len(linhas_y) + 1}"]
    m.call("cad_eletroduto", {"pontos": loop, "escala": ESC, "camada": "EL-Emergencia", "posicao": 0.22, "condutores": [
        {"circuito": len(linhas_y) + 1, "tipo": "fase", "secao": s4}, {"circuito": len(linhas_y) + 1, "tipo": "neutro", "secao": s4}], "secao_terra": s4})
    # luminárias com tag (L01…) e circuito
    for i, (x, y) in enumerate(pos):
        c = linhas_y.index(next(v for v in linhas_y if abs(v - y) < 1e-6)) + 1
        m.call("cad_simbolo", {"nome": "luminaria_led", "ponto": [p0[0] + x, p0[1] + y], "escala": ESC, "camada": "EL-Luminarias", "circuito": f"C{c}", "comando": f"L{i + 1:02d}"})
    # emergência: 8 blocos e 3 saídas sobre o circuito perimetral
    blocos = [(10.0, 1.7), (30.0, 1.7), (10.0, 18.3), (30.0, 18.3), (1.7, 5.0), (1.7, 15.0), (38.3, 5.0), (38.3, 15.0)]
    saidas = [(20.0, 1.9), (4.0, 18.1), (37.9, 10.0)]
    for x, y in blocos:
        m.call("cad_simbolo", {"nome": "bloco_emergencia", "ponto": [p0[0] + x, p0[1] + y], "escala": ESC, "camada": "EL-Emergencia"})
    for x, y in saidas:
        m.call("cad_simbolo", {"nome": "saida_emergencia", "ponto": [p0[0] + x, p0[1] + y], "escala": ESC, "camada": "EL-Emergencia", "rotacao": 90 if x > 30 else 0})
    # tabelas à direita
    tx, ty = o[0] + 57.0, o[1] + 54.0
    dados = [["Área / pé-direito", "800 m²  /  6,00 m"], ["Iluminância de projeto (Em)", "300 lux - NBR ISO/CIE 8995-1"],
             ["Luminárias", f"{lum['luminarias']} x LED high bay 150 W, 21.000 lm, 5000 K, IP65"], ["Potência instalada", f"{fmt(lum['potencia_instalada_w'], 0)} W ({fmt(lum['densidade_w_m2'])} W/m²)"],
             ["Iluminância média calculada", f"{fmt(lum['iluminancia_media_metodo_lux'], 0)} lux (método dos lumens)"],
             ["Uniformidade (com as paredes)", f"{fmt(lum['grade']['uniformidade'])} (mín. 0,60) - ver folha 2"],
             ["Luz natural", "8 claraboias 3,0 x 2,0 m (48 m²) - não entra no cálculo"], ["Operação", "24 h - à noite 100 % artificial; dimerizado de dia"]]
    r = tabela(m, (tx, ty), "DADOS DO PROJETO", [{"titulo": "Item", "largura": 52}, {"titulo": "Valor", "largura": 133}], dados)
    ty2 = ty - r["altura_total"] - 4 * K * 1.0
    ql = []
    n_f = len(pos) // len(linhas_y)
    for i in range(len(linhas_y)):
        ql.append([f"L{i * n_f + 1:02d}-L{(i + 1) * n_f:02d}", n_f, f"LED high bay 150 W - fileira {i + 1}", f"C{i + 1}", "FC1" if i == 0 else ("FC2" if i == 2 else "FC1+FC2")])
    ql += [["CL1-CL8", 8, "Claraboia 3,0 x 2,0 m, telha translúcida", "-", "-"], ["FC1-FC2", 2, "Fotocélula de dimerização 0-10 V / DALI", "-", "QDL"],
           ["E1-E8", 8, "Bloco autônomo de emergência LED, 1 h", f"C{len(linhas_y) + 1}", "-"], ["S1-S3", 3, "Sinalização de saída de emergência", f"C{len(linhas_y) + 1}", "-"]]
    tabela(m, (tx, ty2), "QUADRO DE LUMINÁRIAS E COMANDO", [{"titulo": "Tag", "largura": 24}, {"titulo": "Qtd", "largura": 12, "alinhamento": "centro"},
                                                             {"titulo": "Descrição", "largura": 105}, {"titulo": "Circ.", "largura": 18, "alinhamento": "centro"},
                                                             {"titulo": "Comando", "largura": 26, "alinhamento": "centro"}], ql)
    # notas (dentro da caixa NOTAS)
    notas = m.call("cad_notas_padrao", {"conjuntos": ["geral"], "eletroduto": "DE AÇO GALVANIZADO APARENTE Ø3/4\"", "fiacao": "#1,5 mm² (ver quadro de cargas)"})
    bloco_na_caixa(m, faixa["notas"], notas["itens"], altura=1.45)
    legenda_f1(m, faixa["legenda"])


def legenda_f1(m, caixa):
    L = Lote(m)
    x0, ytop = caixa["min"][0], caixa["max"][1]
    colw = caixa["largura"] / 2.0
    itens = [("luminaria_led", "Luminária LED high bay 150 W / 21.000 lm (L = tag, C = circuito)"), ("claraboia", "Claraboia 3,0 x 2,0 m - telha translúcida (CL = tag)"),
             ("fotocelula", "Fotocélula de dimerização por luz natural (FC)"), ("quadro", "QDL - quadro de distribuição de luz"),
             ("bloco_emergencia", "Bloco autônomo de emergência LED, autonomia 1 h"), ("saida_emergencia", "Sinalização de saída de emergência"),
             ("conduto", "Eletroduto aparente de aço - fase, neutro e terra (marcas: ver NBR 5444)"), ("conduto_em", "Eletroduto do circuito de emergência"),
             ("pilar", "Pilar do pórtico (P1 a P6)"), ("eixo", "Eixo do pórtico")]
    for i, (nome, texto) in enumerate(itens):
        col, lin = divmod(i, 5)
        x = x0 + col * colw + 4.0 * K
        y = ytop - (7.0 + lin * 10.5) * K
        if nome in ("luminaria_led", "fotocelula", "bloco_emergencia", "saida_emergencia"):
            cam = {"luminaria_led": "EL-Luminarias", "fotocelula": "EL-Fotocelula"}.get(nome, "EL-Emergencia")
            m.call("cad_simbolo", {"nome": nome, "ponto": [x + 5 * K, y], "escala": ESC, "camada": cam})
        elif nome == "claraboia":
            L.ret("EL-Claraboias", (x, y - 2.0 * K), (x + 9 * K, y + 2.0 * K))
            L.linha("EL-Claraboias", (x, y - 2.0 * K), (x + 9 * K, y + 2.0 * K))
            L.linha("EL-Claraboias", (x, y + 2.0 * K), (x + 9 * K, y - 2.0 * K))
        elif nome == "quadro":
            L.ret("EL-Quadro", (x + 2 * K, y - 2.5 * K), (x + 6 * K, y + 2.5 * K))
            L.cheio("EL-Quadro", [(x + 2 * K, y - 2.5 * K), (x + 6 * K, y - 2.5 * K), (x + 6 * K, y + 2.5 * K), (x + 2 * K, y + 2.5 * K)])
        elif nome in ("conduto", "conduto_em"):
            L.linha("EL-Condutos" if nome == "conduto" else "EL-Emergencia", (x, y), (x + 10 * K, y))
        elif nome == "pilar":
            L.ret("ARQ-Pilares", (x + 2.5 * K, y - 2 * K), (x + 6.5 * K, y + 2 * K))
        elif nome == "eixo":
            L.linha("ARQ-Eixos", (x, y), (x + 10 * K, y))
        L.texto("F-Legenda", (x + 13 * K, y - 0.8 * K), texto, 1.8 * K)
    L.gravar()


# ============================ FOLHA 2 ============================
def folha2(m, o, faixa, pos, lum):
    L = Lote(m)
    p0 = (o[0] + 10.0, o[1] + 20.0)
    r = m.call("cad_luminotecnico", {**calcular(m), "desenhar_mapa": {"origem": list(p0), "camada": "LUX"}})
    for i in (0,):
        paredes(L, p0)
    for x, y in pos:
        L.circ("EL-Luminarias", (p0[0] + x, p0[1] + y), 0.25)
        L.linha("EL-Luminarias", (p0[0] + x - 0.17, p0[1] + y - 0.17), (p0[0] + x + 0.17, p0[1] + y + 0.17))
        L.linha("EL-Luminarias", (p0[0] + x - 0.17, p0[1] + y + 0.17), (p0[0] + x + 0.17, p0[1] + y - 0.17))
    cotas_da_planta(L, p0, sorted({round(x, 3) for x, _ in pos}), sorted({round(y, 3) for _, y in pos}))
    L.texto("F-Titulos", (p0[0], p0[1] + L_AMB + 7.4), "MAPA DE ILUMINÂNCIA - ESC. 1:100", 0.5)
    L.texto("EL-Textos", (p0[0], p0[1] + L_AMB + 6.4), "Plano de trabalho a 0,80 m - luz artificial (luz direta + 1ª reflexão nas paredes) - sem luz natural", 0.25)
    L.gravar()
    # legenda das faixas (ao lado do mapa)
    lx, ly = p0[0] + C_AMB + 5.0, p0[1] + L_AMB
    L.texto("F-Titulos", (lx, ly + 1.0), "LUX (meta 300 lux)", 0.3)
    nomes = ["abaixo de 150 lux", "150 a 225 lux", "225 a 300 lux", "300 a 375 lux", "acima de 375 lux"]
    for i, f in enumerate(r["mapa"]["faixas"]):
        y = ly - i * 1.1
        L.ret("LUX", (lx, y - 0.4), (lx + 1.2, y + 0.4), f["cor"])
        L.cheio("LUX", [(lx, y - 0.4), (lx + 1.2, y - 0.4), (lx + 1.2, y + 0.4), (lx, y + 0.4)], f["cor"])
        L.texto("EL-Textos", (lx + 1.6, y - 0.12), nomes[i], 0.22)
    L.gravar()
    # tabela do cálculo
    g = lum["grade"]
    dados = [["Dimensões (C x L x H)", "40,0 x 20,0 x 6,0 m (área 800 m²)"], ["Atividade / norma", "Galpão industrial - NBR ISO/CIE 8995-1"],
             ["Iluminância mantida (Em)", "300 lux no plano de trabalho (h = 0,80 m)"], ["Altura útil (hm)", f"{fmt(lum['altura_util_hm_m'], 1)} m (montagem a 6,0 m)"],
             ["Índice do local K", f"{fmt(lum['indice_do_local_k'])} = (C·L) / (hm·(C+L))"],
             ["Fator de utilização (u)", f"{fmt(lum['fator_utilizacao'])} ({lum['fator_utilizacao_origem']})"], ["Fator de manutenção (d)", fmt(lum["fator_manutencao"])],
             ["Fluxo total necessário", f"{fmt(lum['fluxo_total_necessario_lm'], 0)} lm = E·A / (u·d)"],
             ["Luminárias (mínimo / adotado)", f"{lum['luminarias_minimo']} / {lum['luminarias']} x LED 150 W, 21.000 lm"],
             ["Arranjo", f"{lum['arranjo']['linhas_ao_longo_da_largura']} linhas x {lum['arranjo']['colunas_ao_longo_do_comprimento']} colunas"],
             ["Espaçamento", f"{fmt(lum['espacamento_m']['entre_colunas'])} x {fmt(lum['espacamento_m']['entre_linhas'])} m (e/hm {fmt(lum['relacao_e_hm']['colunas'])} e {fmt(lum['relacao_e_hm']['linhas'])})"],
             ["Iluminância média (lumens)", f"{fmt(lum['iluminancia_media_metodo_lux'], 0)} lux"],
             ["Grade com paredes (ρ = 0,5)", f"média {fmt(g['media_lux'], 0)}, mín. {fmt(g['minima_lux'], 0)}, máx. {fmt(g['maxima_lux'], 0)} lux, Uo = {fmt(g['uniformidade'])}"],
             ["Só luz direta (conservador)", f"média {fmt(g['so_direta']['media_lux'], 0)}, mín. {fmt(g['so_direta']['minima_lux'], 0)} lux, Uo = {fmt(g['so_direta']['uniformidade'])}"],
             ["Potência instalada", f"{fmt(lum['potencia_instalada_w'], 0)} W ({fmt(lum['densidade_w_m2'])} W/m²)"],
             ["Eficiência energética", f"{fmt(lum['eficiencia_lux_por_w_m2'], 1)} lux por W/m²"]]
    tabela(m, (o[0] + 57.0, o[1] + 54.0), "CÁLCULO LUMINOTÉCNICO - MÉTODO DOS LUMENS", [{"titulo": "Parâmetro", "largura": 62}, {"titulo": "Valor", "largura": 123}], dados)
    itens = [{"tipo": "nota", "texto": f"{i + 1}. {t.upper()}"} for i, t in enumerate(lum["premissas"])]
    itens += [{"tipo": "nota", "texto": f"{len(itens) + 1 + i}. ATENÇÃO: {t}"} for i, t in enumerate(lum["avisos"])]
    bloco_na_caixa(m, faixa["notas"], itens, altura=1.45)
    bloco_na_caixa(m, faixa["legenda"], [{"tipo": "nota", "texto": "MAPA DE LUX: cada célula de 1 x 1 m mostra a iluminância no plano de trabalho. Verde = meta atendida (300 a 375 lux), amarelo = entre 225 e 300 lux, laranja e vermelho = abaixo; azul = acima de 375 lux."},
                                          {"tipo": "nota", "texto": "A luz das paredes entra por imagem de 1ª ordem (refletância 0,5); a linha 'só luz direta' é o limite inferior. A conferência final é feita com a fotometria (.ies/.ldt) do fabricante, no DIALux ou Relux."}], altura=1.8, camada="F-Legenda")


# ============================ FOLHA 3 ============================
def folha3(m, o, faixa, pos):
    L = Lote(m)
    Z = 2.0  # o corte é desenhado em 1:50 (geometria x2); as cotas mostram os valores reais
    x0, y0 = o[0] + 6.0, o[1] + 20.0

    def P(x, y):
        return (x0 + Z * x, y0 + Z * y)

    def roof(x):
        return H_AMB + 1.0 - abs(x - 10.0) / 10.0

    def seg(cam, a_, b_):
        L.linha(cam, P(*a_), P(*b_))

    seg("ARQ-Estrutura", (0, 0), (0, H_AMB)); seg("ARQ-Estrutura", (20, 0), (20, H_AMB))
    seg("ARQ-Estrutura", (-0.3, H_AMB - 0.05), (10, H_AMB + 1.0)); seg("ARQ-Estrutura", (10, H_AMB + 1.0), (20.3, H_AMB - 0.05))
    seg("ARQ-Estrutura", (0, H_AMB), (20, H_AMB)); seg("ARQ-Estrutura", (0, H_AMB), (10, H_AMB + 1.0)); seg("ARQ-Estrutura", (20, H_AMB), (10, H_AMB + 1.0))
    for xm in (5.0, 15.0):  # montantes da tesoura
        seg("ARQ-Estrutura", (xm, H_AMB), (xm, roof(xm)))
    seg("ARQ-Paredes", (-1.0, 0), (21.0, 0)); seg("ARQ-Cotas", (-1.0, 0.8), (21.0, 0.8))
    L.texto("EL-Textos", P(21.2, 0.8 - 0.1), "plano de trabalho", 0.22)
    ys = sorted({round(y, 3) for _, y in pos})
    for y in ys:  # luminárias penduradas no banzo inferior e seu facho (45°)
        cx, cy = y, H_AMB - 0.35
        L.ret("EL-Luminarias", P(cx - 0.35, cy - 0.15), P(cx + 0.35, cy + 0.15))
        seg("EL-Luminarias", (cx, cy + 0.15), (cx, H_AMB))
        seg("EL-Feixes", (cx - 0.3, cy - 0.15), (cx - 0.3 - (cy - 0.95), 0.8)); seg("EL-Feixes", (cx + 0.3, cy - 0.15), (cx + 0.3 + (cy - 0.95), 0.8))
    for cx in (6.667, 13.333):  # claraboias na cobertura e a luz natural
        seg("EL-Claraboias", (cx - 1.0, roof(cx - 1.0) + 0.1), (cx + 1.0, roof(cx + 1.0) + 0.1)); seg("EL-Claraboias", (cx - 1.0, roof(cx - 1.0)), (cx + 1.0, roof(cx + 1.0)))
        seg("EL-Feixes-Natural", (cx - 1.0, roof(cx - 1.0) - 0.1), (cx - 3.0, 0.8)); seg("EL-Feixes-Natural", (cx + 1.0, roof(cx + 1.0) - 0.1), (cx + 3.0, 0.8))
    for cx, nome in ((6.667, "FC1"), (13.333, "FC2")):
        L.circ("EL-Fotocelula", P(cx, H_AMB - 0.9), 0.25)
        L.texto("EL-Textos", P(cx - 0.45, H_AMB - 1.55), nome, 0.22)
    cota_cadeia_v(L, [y0, y0 + Z * 0.8, y0 + Z * H_AMB], x0 - 1.5, fator=Z)
    cota_v(L, y0, y0 + Z * H_AMB, x0 - 3.2, fator=Z)
    cota_h(L, x0, x0 + Z * 20.0, y0 - 1.6, fator=Z)
    cota_cadeia_h(L, [x0 + Z * v for v in [0.0] + ys + [20.0]], y0 - 3.0, fator=Z)
    for txt, yy, alvo in (("Fotocélula FC - dimerização 0-10 V / DALI", 9.2, (6.667, H_AMB - 0.9)), ("Claraboia 3,0 x 2,0 m - telha translúcida difusa", 7.9, (13.333, roof(13.333))),
                          ("Luminária LED high bay 150 W / 21.000 lm (h = 6,0 m)", 6.6, (10.0, H_AMB - 0.35))):
        L.texto("EL-Textos", P(21.5, yy), txt, 0.24)
        L.linha("ARQ-Cotas", P(21.4, yy + 0.1), P(*alvo))
    L.texto("F-Titulos", (x0, y0 + Z * 9.3), "CORTE TRANSVERSAL - ESC. 1:50", 0.5)
    L.texto("EL-Textos", (x0, y0 + Z * 8.6), "Vista pela largura de 20 m: luz artificial (laranja) e luz natural (azul), facho esquemático", 0.25)
    # esquema de dimerização (sem escala)
    ex, ey = o[0] + 8.0, o[1] + 42.0
    L.texto("F-Titulos", (ex, ey + 14.0), "ESQUEMA DE DIMERIZAÇÃO - SEM ESCALA", 0.5)
    DRV = (ex + 8.5, ey + 3.0)
    for nome, y in (("FC1", ey + 9.0), ("FC2", ey + 1.0)):
        L.circ("EL-Fotocelula", (ex + 1.0, y + 0.8), 0.9)
        L.texto("EL-Textos", (ex + 0.35, y + 0.65), nome, 0.22)
        L.linha("DI-Linhas do diagrama", (ex + 1.9, y + 0.8), (DRV[0], y + 0.8))
    L.ret("DI-Linhas do diagrama", (DRV[0], DRV[1] - 1.0), (DRV[0] + 6.5, DRV[1] + 8.4))
    for i, t in enumerate(("Interface de", "dimerização", "0-10 V / DALI")):
        L.texto("EL-Textos", (DRV[0] + 0.4, DRV[1] + 6.4 - i * 0.8), t, 0.22)
    for nome, fonte, y in (("C1", "FC1", ey + 9.0), ("C2", "FC1+FC2", ey + 5.0), ("C3", "FC2", ey + 1.0)):
        L.ret("EL-Luminarias", (ex + 19.0, y), (ex + 29.0, y + 1.6))
        L.texto("EL-Textos", (ex + 19.4, y + 0.55), f"{nome}: 7 x 150 W ({fonte})", 0.22)
        L.linha("DI-Linhas do diagrama", (DRV[0] + 6.5, y + 0.8), (ex + 19.0, y + 0.8))
    L.texto("EL-Textos", (ex, ey - 1.2), "Sem luz natural os drivers vão a 100 %. Chave manual no QDL força 100 %.", 0.22)
    # detalhe da fixação da luminária (sem escala)
    dx, dy = o[0] + 50.0, o[1] + 42.0
    L.texto("F-Titulos", (dx, dy + 12.0), "DETALHE DE FIXAÇÃO DA LUMINÁRIA - SEM ESCALA", 0.5)
    L.linha("ARQ-Estrutura", (dx, dy + 10.0), (dx + 24.0, dy + 10.0)); L.texto("EL-Textos", (dx + 0.2, dy + 10.4), "banzo inferior da tesoura", 0.22)
    L.linha("EL-Condutos", (dx + 3.0, dy + 9.2), (dx + 12.0, dy + 9.2)); L.linha("EL-Condutos", (dx + 3.0, dy + 8.6), (dx + 12.0, dy + 8.6))
    L.texto("EL-Textos", (dx + 3.0, dy + 7.9), "eletroduto de aço Ø3/4\" (aparente)", 0.22)
    L.ret("EL-Condutos", (dx + 12.0, dy + 8.3), (dx + 14.0, dy + 9.5)); L.texto("EL-Textos", (dx + 14.4, dy + 8.7), "caixa de passagem", 0.22)
    L.linha("EL-Luminarias", (dx + 13.0, dy + 8.3), (dx + 13.0, dy + 5.8)); L.linha("EL-Luminarias", (dx + 13.3, dy + 8.3), (dx + 13.3, dy + 5.8))
    L.texto("EL-Textos", (dx + 14.4, dy + 7.2), "cabo flexível 3 x #1,5 mm² (F, N, PE)", 0.22)
    L.poli("EL-Luminarias", [(dx + 10.0, dy + 5.8), (dx + 16.4, dy + 5.8), (dx + 15.0, dy + 3.6), (dx + 11.4, dy + 3.6)], True)
    L.texto("EL-Textos", (dx + 17.0, dy + 4.6), "luminária LED high bay", 0.22)
    L.linha("EL-Emergencia", (dx + 10.6, dy + 5.8), (dx + 10.6, dy + 6.4)); L.linha("EL-Emergencia", (dx + 10.6, dy + 6.4), (dx + 12.6, dy + 6.4))
    L.texto("EL-Textos", (dx + 17.0, dy + 3.6), "PE ligado ao corpo da luminária", 0.22)
    L.gravar()
    notas = [{"tipo": "nota", "texto": "1. A DIMERIZAÇÃO É POR ZONA: FC1 COMANDA A FILEIRA 1 (C1), FC2 A FILEIRA 3 (C3) E A FILEIRA 2 (C2) SEGUE A MÉDIA DAS DUAS."},
             {"tipo": "nota", "texto": "2. SEM LUZ NATURAL (NOITE) AS 21 LUMINÁRIAS FUNCIONAM A 100 %; O PROJETO NÃO CONTA COM A LUZ NATURAL PARA ATINGIR 300 LUX."},
             {"tipo": "nota", "texto": "3. AS CLARABOIAS FICAM ENTRE AS FILEIRAS, SOBRE OS CORREDORES, E USAM TELHA TRANSLÚCIDA DIFUSA PARA EVITAR OFUSCAMENTO E GANHO DE CALOR."},
             {"tipo": "nota", "texto": "4. AS LUMINÁRIAS SÃO FIXADAS NO BANZO INFERIOR DAS TESOURAS (H = 6,00 m) E ATERRADAS; O FACHO DO DESENHO É ESQUEMÁTICO."}]
    bloco_na_caixa(m, faixa["notas"], notas, altura=1.45)
    bloco_na_caixa(m, faixa["legenda"], [{"tipo": "nota", "texto": "LARANJA: LUZ ARTIFICIAL (LUMINÁRIA E FACHO).  AZUL: LUZ NATURAL (CLARABOIA E FOTOCÉLULA).  VERMELHO: ELETRODUTO.  VERDE: PROTEÇÃO (PE).  CINZA: ESTRUTURA, PISO E COTAS."}], altura=1.8, camada="F-Legenda")


# ============================ FOLHA 4 ============================
def folha4(m, o, faixa, dados_circ, lum, calc):
    a = papel_para_doc
    r = m.call("cad_circuitos", {**dados_circ, "desenhar": {
        "quadro_de_cargas": {"origem": list(a(o, 40, 568)), "escala": ESC, "altura_texto": 2.5, "altura_linha": 6.0},
        "demanda": {"origem": list(a(o, 40, 495)), "escala": ESC, "altura_texto": 2.5, "altura_linha": 6.0},
        "unifilar": {"origem": list(a(o, 50, 430)), "escala": ESC, "altura_texto": 2.6}}})
    # lista de materiais, tudo calculado do desenho e do cálculo
    cs = calc["circuitos"]
    metros = {}
    for c in cs:
        metros[c["secao_mm2"]] = metros.get(c["secao_mm2"], 0.0) + c["comprimento_m"] * 3  # fase + neutro + terra
    disj = {}
    for c in cs:
        disj[c["disjuntor_a"]] = disj.get(c["disjuntor_a"], 0) + 1
    itens = [["Luminária LED high bay 150 W, 21.000 lm, 5000 K, IRC ≥ 80, IP65", "un", lum["luminarias"]],
             ["Claraboia de telha translúcida difusa 3,0 x 2,0 m", "un", 8], ["Fotocélula para dimerização 0-10 V / DALI", "un", 2],
             ["Bloco autônomo de emergência LED, autonomia 1 h", "un", 8], ["Sinalização de saída de emergência LED", "un", 3],
             ["Quadro de distribuição de luz QDL com barramento, trilho DIN e barreira", "un", 1]]
    for amp, q in sorted(disj.items()):
        itens.append([f"Disjuntor termomagnético monopolar curva C {amp:.0f} A", "un", q])
    itens += [["DPS classe II no QDL (275 V)", "un", 3], ["Interruptor-seccionador geral tripolar do QDL", "un", 1]]
    for s, mt in sorted(metros.items()):
        itens.append([f"Cabo de cobre flexível 450/750 V, {fmt(s, 1)} mm² (fase, neutro e terra)", "m", round(mt)])
    itens += [["Eletroduto de aço galvanizado aparente Ø3/4\" com acessórios (tronco, fileiras e emergência)", "m", round(3 * 37.1 + 13.4 + 1 + 2 * (38.4 + 18.4))],
              ["Haste de aterramento 5/8\" x 2,40 m com caixa de inspeção", "un", 1]]
    linhas = [[i + 1, d, u, q, "a definir"] for i, (d, u, q) in enumerate(itens)]
    tabela(m, a(o, 520, 568), "LISTA DE MATERIAIS", [{"titulo": "Item", "largura": 11, "alinhamento": "centro"}, {"titulo": "Descrição", "largura": 160},
                                                    {"titulo": "Un", "largura": 9, "alinhamento": "centro"}, {"titulo": "Qtd", "largura": 15, "alinhamento": "direita", "decimais": 0},
                                                    {"titulo": "Ref. (SINAPI)", "largura": 26, "alinhamento": "centro"}], linhas, altura_texto=2.4, altura_linha=6.0)
    # notas normativas (colunas) e advertência
    notas = m.call("cad_notas_padrao", {"conjuntos": ["geral", "quadros", "aterramento", "iluminacao", "emergencia"], "eletroduto": "DE AÇO GALVANIZADO APARENTE Ø3/4\"",
                                       "fiacao": "#1,5 mm² (circuitos de iluminação, ver quadro de cargas)"})
    ox, oy = a(o, 330, 470)
    m.call("cad_texto_bloco", {"origem": [ox, oy], "largura": 480, "altura_texto": 2.3, "escala": ESC, "colunas": 2, "altura_max": 340 - 130,
                               "itens": [{"tipo": "titulo", "texto": "NOTAS GERAIS"}] + notas["itens"], "camada": "F-Notas"})
    adv = [{"tipo": "titulo", "texto": "ADVERTÊNCIA"},
           {"tipo": "nota", "texto": "1. QUANDO UM DISJUNTOR OU DR ATUA, DESLIGANDO UM CIRCUITO, HÁ SOBRECARGA OU CURTO-CIRCUITO. NÃO TROQUE POR OUTRO DE MAIOR CORRENTE: A TROCA EXIGE ANTES A TROCA DOS CONDUTORES, POR PROFISSIONAL QUALIFICADO."},
           {"tipo": "nota", "texto": "2. NUNCA DESATIVE NEM REMOVA A PROTEÇÃO CONTRA CHOQUES (DR). ESTA ADVERTÊNCIA DEVE SER FIXADA, EM MATERIAL INDELÉVEL, NA PORTA DO QDL."}]
    ax, ay = a(o, 40, 345)
    m.call("cad_criar", {"camada": "F-Notas", "formas": [{"tipo": "retangulo", "canto1": [ax - 1.0 * K, ay + 4.5 * K], "canto2": [ax + 214 * K, ay - 36 * K]}]})
    m.call("cad_texto_bloco", {"origem": [ax, ay], "largura": 210, "altura_texto": 2.0, "escala": ESC, "itens": adv, "camada": "F-Notas"})
    prem = [{"tipo": "nota", "texto": f"{i + 1}. {t.upper()}"} for i, t in enumerate(r["premissas"])]
    bloco_na_caixa(m, faixa["notas"], prem, altura=1.35)
    bloco_na_caixa(m, faixa["legenda"], [{"tipo": "nota", "texto": "UNIFILAR: CADA SAÍDA TEM DISJUNTOR (A), SEÇÃO DO CONDUTOR EM mm² (ABAIXO DA LINHA), LETRA DA FASE (ACIMA) E A CARGA (W) COM O Nº E A DESCRIÇÃO DO CIRCUITO."},
                                          {"tipo": "nota", "texto": "O ALIMENTADOR DO QDL É TRIFÁSICO (3F+N+T). OS CIRCUITOS DE ILUMINAÇÃO SÃO FASE-NEUTRO EM 220 V, UM POR FASE; O QUADRO TEM DR NAS TOMADAS, QUE NESTE PROJETO NÃO EXISTEM."},
                                          {"tipo": "nota", "texto": "STATUS OK = CAPACIDADE DE CONDUÇÃO ≥ DISJUNTOR E QUEDA DE TENSÃO DENTRO DO LIMITE; REVER = AJUSTAR SEÇÃO, COMPRIMENTO OU DIVISÃO DOS CIRCUITOS."}], altura=1.8, camada="F-Legenda")
    return linhas


if __name__ == "__main__":
    main()

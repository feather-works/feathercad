#!/usr/bin/env python3
"""Cabine de medição e transformação de consumidor, 13,8 kV / 380-220 V, 300 kVA (Dyn1) — conjunto de 6 pranchas A1 para avaliação
por um especialista: (1) unifilar geral, (2) planta da cabine, (3) cortes, (4) malha de aterramento, (5) proteção e comando,
(6) memorial, lista de materiais e pendências. Desenhado em mm de papel (1:1); as plantas e cortes usam escala própria indicada.

TODOS os valores são PREMISSAS de anteprojeto (nada veio de estudo de curto-circuito nem da concessionária): estão marcados como
"a confirmar" no desenho. Sem telemetria, tudo local (MCP do FeatherCAD)."""
import math, sys
from pack_eletrica import Desenho, n, transformador_lateral, transformador_frontal
from mcp_cliente import Mcp

INK, MT, BT, TERRA, TIT, DIM, CINZA = "#FFFFFF", "#D02020", "#1F6FD0", "#2E8B57", "#A87400", "#606060", "#B0B0B0"
OFF = 1000.0                        # deslocamento entre as pranchas no desenho (mm)
UTIL = (35.0, 109.0, 821.0, 574.0)  # área útil da folha A1 (x0, y0, x1, y1)

# dados do sistema
S_KVA, V_MT, V_BT, ZPCT = 300.0, 13.8, 0.38, 4.5
I_MT = S_KVA / (math.sqrt(3) * V_MT)
I_BT = S_KVA / (math.sqrt(3) * V_BT)
ICC_TRAFO = I_BT / (ZPCT / 100)


class Pg:
    """Uma prancha: desenha em mm de papel; `dx` desloca para a posição dela no desenho."""

    def __init__(self, dw, i):
        self.d, self.dx = dw, i * OFF

    def camada(self, nome, cor):
        self.d.camada(nome, cor)

    def L(self, a, b, c, d, cor=INK):
        self.d.L(a + self.dx, b, c + self.dx, d, cor)

    def R(self, x, y, w, h, cor=INK):
        self.d.R(x + self.dx, y, w, h, cor)

    def C(self, x, y, r, cor=INK):
        self.d.C(x + self.dx, y, r, cor)

    def A(self, x, y, r, a0, a1, cor=INK):
        self.d.A(x + self.dx, y, r, a0, a1, cor)

    def T(self, x, y, h, txt, cor=INK, giro=0):
        self.d.T(x + self.dx, y, h, txt, cor, giro)

    def P(self, pts, fechada=False, cor=INK):
        self.d.P([(x + self.dx, y) for x, y in pts], fechada, cor)

    def H(self, pts, cor=INK):
        self.d.H([(x + self.dx, y) for x, y in pts], cor)

    # ---- apoio
    def tracejada(self, x1, y1, x2, y2, cor=DIM, traco=3.0, vao=2.0):
        d = math.hypot(x2 - x1, y2 - y1)
        if d < 1e-6:
            return
        ux, uy = (x2 - x1) / d, (y2 - y1) / d
        t = 0.0
        while t < d:
            e = min(t + traco, d)
            self.L(x1 + ux * t, y1 + uy * t, x1 + ux * e, y1 + uy * e, cor)
            t += traco + vao

    def retang_tracejado(self, x, y, w, h, cor=DIM):
        for a, b, c, dd in ((x, y, x + w, y), (x + w, y, x + w, y + h), (x + w, y + h, x, y + h), (x, y + h, x, y)):
            self.tracejada(a, b, c, dd, cor)

    def linhas(self, x, y, itens, h=2.3, passo=None, cor=INK):
        passo = passo or h * 1.7
        for i, t in enumerate(itens):
            self.T(x, y - i * passo, h, t, cor)

    def titulo(self, txt):
        self.camada("TITULOS", TIT)
        self.T(UTIL[0] + 4, UTIL[3] - 8, 4.5, txt, TIT)

    def tabela(self, x, y, larguras, linhas, h=2.2, alt=5.0, cab=True, cor=INK):
        """Grade com linhas de texto; `y` é o topo. A 1ª linha é o cabeçalho."""
        W = sum(larguras)
        H = alt * len(linhas)
        self.R(x, y - H, W, H, cor)
        for i in range(1, len(linhas)):
            self.L(x, y - i * alt, x + W, y - i * alt, cor if (i == 1 and cab) else DIM)
        cx = x
        for w in larguras[:-1]:
            cx += w
            self.L(cx, y, cx, y - H, DIM)
        for i, lin in enumerate(linhas):
            cx = x
            for w, t in zip(larguras, lin):
                self.T(cx + 1.2, y - (i + 1) * alt + (alt - h) / 2 + 0.2, h, t, TIT if (cab and i == 0) else INK)
                cx += w

    def balao(self, x, y, num, cor=INK, r=3.0):
        self.C(x, y, r, cor)
        self.T(x - 0.85 * len(str(num)) * 0.9, y - 1.1, 2.3, str(num), cor)

    # ---- símbolos unifilares (linha principal horizontal em y)
    def fusivel(self, x, y, cor):
        self.L(x - 6, y, x - 4, y, cor); self.R(x - 4, y - 1.6, 8, 3.2, cor); self.L(x - 4, y, x + 4, y, cor); self.L(x + 4, y, x + 6, y, cor)

    def seccionadora(self, x, y, cor, terra=False):
        self.L(x - 6, y, x - 3.5, y, cor); self.C(x - 3.2, y, 0.7, cor)
        self.L(x - 3.2, y, x + 3.0, y + 4.5, cor); self.C(x + 3.5, y, 0.7, cor); self.L(x + 4.2, y, x + 6, y, cor)
        if terra:
            self.L(x, y - 1, x, y - 6, TERRA); self.terra(x, y - 6)

    def disjuntor(self, x, y, cor, s=3.5):
        self.L(x - 6, y, x - s, y, cor); self.R(x - s, y - s, 2 * s, 2 * s, cor); self.L(x - s, y - s, x + s, y + s, cor)
        self.L(x - s, y + s, x + s, y - s, cor); self.L(x + s, y, x + 6, y, cor)

    def tc(self, x, y, cor):
        self.C(x, y, 3.0, cor); self.L(x - 6, y, x + 6, y, cor)

    def tp(self, x, y, cor):
        self.L(x, y, x, y - 7, cor); self.C(x, y - 10, 3.0, cor); self.C(x, y - 13.5, 3.0, cor); self.L(x, y - 16.5, x, y - 20, cor)

    def para_raios(self, x, y, cor):
        self.L(x, y, x, y - 6, cor); self.R(x - 1.6, y - 14, 3.2, 8, cor); self.L(x - 1.2, y - 12, x, y - 9, cor); self.L(x + 1.2, y - 12, x, y - 9, cor)
        self.L(x, y - 14, x, y - 17, TERRA); self.terra(x, y - 17)

    def terra(self, x, y):
        self.L(x, y, x, y - 2, TERRA); self.L(x - 4, y - 2, x + 4, y - 2, TERRA); self.L(x - 2.6, y - 3.6, x + 2.6, y - 3.6, TERRA)
        self.L(x - 1.2, y - 5.2, x + 1.2, y - 5.2, TERRA)

    def trafo(self, x, y, cor):
        self.L(x - 14, y, x - 8, y, cor); self.C(x - 4, y, 8, cor); self.C(x + 4, y, 8, cor); self.L(x + 12, y, x + 18, y, cor)

    def cabo(self, x1, x2, y, cor, n=3):
        self.L(x1, y, x2, y, cor)
        m = (x1 + x2) / 2
        for k in range(n):
            self.L(m - 1.5 + k * 1.5, y - 2, m - 0.5 + k * 1.5, y + 2, cor)

    def rele(self, x, y, w, h, txt, cor=INK):
        self.R(x, y, w, h, cor)
        for i, t in enumerate(txt):
            self.T(x + 1.5, y + h - 4.2 - i * 3.6, 2.2, t, cor)

    def cota(self, x1, y1, x2, y2, txt, off=6.0, cor=DIM, h=2.4):
        """Cota linear horizontal ou vertical, afastada `off` (mm de papel)."""
        if abs(y2 - y1) < 1e-6:
            yy = y1 + off
            self.L(x1, y1, x1, yy + 1.5 * (1 if off > 0 else -1), cor); self.L(x2, y2, x2, yy + 1.5 * (1 if off > 0 else -1), cor)
            self.L(x1, yy, x2, yy, cor)
            self.L(x1 - 1.5, yy - 1.5, x1 + 1.5, yy + 1.5, cor); self.L(x2 - 1.5, yy - 1.5, x2 + 1.5, yy + 1.5, cor)
            self.T((x1 + x2) / 2 - len(txt) * h * 0.28, yy + 1.0, h, txt, cor)
        else:
            xx = x1 + off
            self.L(x1, y1, xx + 1.5 * (1 if off > 0 else -1), y1, cor); self.L(x2, y2, xx + 1.5 * (1 if off > 0 else -1), y2, cor)
            self.L(xx, y1, xx, y2, cor)
            self.L(xx - 1.5, y1 - 1.5, xx + 1.5, y1 + 1.5, cor); self.L(xx - 1.5, y2 - 1.5, xx + 1.5, y2 + 1.5, cor)
            self.T(xx + 1.2, (y1 + y2) / 2 - len(txt) * h * 0.28, h, txt, cor, 90)


class Sub(Pg):
    """Vista de uma prancha com origem (ox, oy) e escala s: os símbolos (desenhados em ~12 mm) saem ampliados no ponto pedido."""

    def __init__(self, pai, ox, oy, s):
        self.pai, self.o, self.s = pai, (ox, oy), s

    def camada(self, nome, cor):
        self.pai.camada(nome, cor)

    def L(self, a, b, c, d, cor=INK):
        o, s = self.o, self.s
        self.pai.L(o[0] + a * s, o[1] + b * s, o[0] + c * s, o[1] + d * s, cor)

    def R(self, x, y, w, h, cor=INK):
        o, s = self.o, self.s
        self.pai.R(o[0] + x * s, o[1] + y * s, w * s, h * s, cor)

    def C(self, x, y, r, cor=INK):
        o, s = self.o, self.s
        self.pai.C(o[0] + x * s, o[1] + y * s, r * s, cor)

    def A(self, x, y, r, a0, a1, cor=INK):
        o, s = self.o, self.s
        self.pai.A(o[0] + x * s, o[1] + y * s, r * s, a0, a1, cor)

    def T(self, x, y, h, txt, cor=INK, giro=0):
        o, s = self.o, self.s
        self.pai.T(o[0] + x * s, o[1] + y * s, h * s, txt, cor, giro)

    def P(self, pts, fechada=False, cor=INK):
        o, s = self.o, self.s
        self.pai.P([(o[0] + x * s, o[1] + y * s) for x, y in pts], fechada, cor)


def sub(self, x, y, s=1.0):
    return Sub(self, x, y, s)


Pg.sub = sub


# ====================================================================== 1. UNIFILAR
def p1(dw):
    p = Pg(dw, 0)
    p.titulo("1. DIAGRAMA UNIFILAR GERAL - CABINE DE MEDIÇÃO E TRANSFORMAÇÃO 13,8 kV / 380-220 V - 300 kVA")
    y0, K = 466.0, 1.6
    sy = lambda x: p.sub(x, y0, K)

    def spec(x, linhas, cor=INK, dy=0.0):
        n_ = len(linhas)
        for i, t in enumerate(linhas):
            p.T(x - 9, y0 + 27 + dy + (n_ - 1 - i) * 3.7, 2.5 if i else 2.7, t, cor if i == 0 else DIM)

    # ---- linha MT
    p.camada("UF-MT", MT)
    p.L(45, y0 - 16, 45, y0 + 16, MT); p.L(48, y0 - 16, 48, y0 + 16, MT)
    p.T(38, y0 - 26, 3.0, "REDE DE", INK); p.T(38, y0 - 30.5, 3.0, "DISTRIBUIÇÃO", INK); p.T(38, y0 - 35, 2.5, "13,8 kV - 60 Hz - 3F", DIM); p.T(38, y0 - 39, 2.5, "Icc: a obter", DIM); p.T(38, y0 - 43, 2.5, "da concessionária", DIM)
    p.L(48, y0, 553, y0, MT)
    sy(72).fusivel(0, 0, MT); p.balao(72, y0 + 17, 1)
    spec(72, ["CHAVE FUSÍVEL", "15 kV / 100 A", "elo 15K a 25K"], dy=4)
    sy(102).para_raios(0, 0, MT); p.balao(112, y0 - 20, 2)
    spec(104, ["PÁRA-RAIOS", "ZnO 12 kV/10 kA", "poste e cabine"], dy=4)
    p.cabo(130, 190, y0, MT)
    p.T(128, y0 + 6, 2.4, "RAMAL SUBTERRÂNEO", MT); p.T(128, y0 + 10, 2.4, "3 x 1 x 50 mm² EPR 8,7/15 kV", MT)
    p.T(128, y0 - 8, 2.2, "eletroduto Ø100 enterrado", DIM)
    p.R(155, y0 - 5, 10, 10, MT); p.T(153, y0 - 12, 2.4, "CP 0,80x0,80", MT)
    p.camada("LIMITE", DIM)
    p.tracejada(122, 300, 122, 552, DIM, 4, 2); p.T(123, 545, 2.6, "LIMITE DA", DIM); p.T(123, 541.5, 2.6, "PROPRIEDADE", DIM)
    p.retang_tracejado(200, 300, 615, 250, DIM)
    for x0, w, nome in ((200, 130, "MEDIÇÃO (concessionária)"), (330, 195, "PROTEÇÃO E MANOBRA MT"), (525, 100, "TRANSFORMAÇÃO"), (625, 190, "QGBT - BAIXA TENSÃO")):
        p.T(x0 + 3, 543, 3.0, nome, TIT)
        if x0 > 200:
            p.tracejada(x0, 300, x0, 550, DIM, 3, 2)
    p.camada("UF-MT", MT)
    p.P([(211, y0 + 5), (219, y0 + 5), (215, y0 - 5)], True, MT); p.balao(215, y0 + 17, 3)
    spec(215, ["MUFLA 15 kV", "3 un. internas"], dy=4)
    sy(247).tc(0, 0, MT); p.balao(247, y0 + 17, 4)
    spec(247, ["TC MEDIÇÃO", "15-5 A cl. 0,3", "3 un."], dy=4)
    sy(292).tp(0, 0, MT); p.balao(305, y0 - 20, 5)
    p.T(297, y0 - 42, 2.4, "TP 13800/115 V", MT); p.T(297, y0 - 46, 2.4, "2 un. (V)", DIM)
    sy(358).seccionadora(0, 0, MT, terra=True); p.balao(358, y0 + 17, 7)
    spec(358, ["SECCIONADORA", "15 kV / 400 A", "c/ lâmina de terra"], dy=4)
    sy(408).disjuntor(0, 0, MT); p.balao(408, y0 + 17, 8)
    spec(408, ["DISJUNTOR MT", "vácuo 15 kV 400 A", "12,5 kA c/ bobina"], dy=4)
    sy(452).tc(0, 0, MT); p.balao(452, y0 + 17, 9)
    spec(452, ["TC PROTEÇÃO", "30-5 A 10B (3 un.)"], dy=4)
    p.cabo(478, 520, y0, MT); p.T(472, y0 + 6, 2.4, "3 x 1 x 25 mm² EPR", MT); p.T(472, y0 + 10, 2.4, "em eletrocalha", MT)
    sy(577).trafo(0, 0, MT); p.balao(577, y0 + 26, 11)
    spec(577, ["TRANSFORMADOR", "300 kVA - Dyn1", "13,8k / 380-220 V", "Z = 4,5 %  tap +-2x2,5 %"], dy=10)
    p.balao(547, y0 - 22, 12); p.T(531, y0 - 32, 2.4, "26/49 alarme e trip", DIM)
    # ---- medição (abaixo da linha)
    p.camada("UF-MED", DIM)
    p.R(232, 405, 58, 18, INK); p.T(235, 415, 2.5, "BLOCO DE AFERIÇÃO", INK); p.T(235, 409, 2.2, "(curto-circuita TCs)", DIM)
    p.tracejada(247, y0 - 5, 247, 423, DIM); p.tracejada(292, y0 - 34, 292, 423, DIM)
    p.tracejada(261, 405, 261, 388, DIM)
    p.C(261, 377, 10, INK); p.T(254, 375.5, 3.0, "kWh", INK); p.balao(280, 372, 6)
    p.T(212, 352, 2.5, "MEDIDOR ELETRÔNICO 4Q", INK); p.T(212, 348, 2.5, "kWh / kvarh - fornecido", DIM); p.T(212, 344.5, 2.5, "e lacrado pela concessionária", DIM)
    # ---- proteção
    p.camada("UF-PROT", TERRA)
    p.R(396, 378, 84, 56, INK); p.balao(488, 430, 9)
    for i, t in enumerate(("RELÉ DIGITAL", "50/51  50N/51N", "27/59 (opcional)", "entradas IA IB IC IN", "saída de TRIP (NA)")):
        p.T(399, 424 - i * 9.6, 2.9 if i == 0 else 2.6, t, INK if i == 0 else DIM)
    p.tracejada(408, 434, 408, y0 - 5.6, TERRA); p.tracejada(452, y0 - 5, 452, 434, TERRA)
    p.R(396, 336, 50, 34, INK); p.balao(454, 366, 10)
    for i, t in enumerate(("FONTE AUX.", "125 Vcc", "retificador +", "baterias")):
        p.T(399, 362 - i * 7.4, 2.6, t, INK if i == 0 else DIM)
    p.tracejada(421, 370, 421, 378, TERRA)
    p.T(462, 356, 2.4, "TC > relé > bobina de abertura Y1", TERRA)
    p.T(462, 352, 2.4, "(circuito detalhado na prancha 5)", DIM)
    # ---- BT
    p.camada("UF-BT", BT)
    p.L(603.8, y0, 648, y0, BT)
    sy(655).disjuntor(0, 0, BT); p.balao(655, y0 + 17, 13)
    spec(655, ["DJ GERAL BT", "500 A regulável", "Icu >= 25 kA"], dy=4)
    p.L(664.6, y0, 676, y0, BT)
    p.L(676, y0 + 5, 806, y0 + 5, BT); p.L(676, y0 - 5, 806, y0 - 5, BT); p.L(676, y0 - 5, 676, y0 + 5, BT); p.L(806, y0 - 5, 806, y0 + 5, BT)
    p.balao(741, y0 + 17, 14); p.T(716, y0 + 28, 2.5, "BARRAMENTO Cu 50 x 5 mm - 630 A", BT); p.T(716, y0 + 24, 2.5, "Icc barramento a confirmar", DIM)
    saidas = [("QD-1", "PRODUÇÃO", "160 A"), ("QD-2", "ILUMINAÇÃO", "63 A"), ("QD-3", "AR COND.", "100 A"), ("QD-4", "USINA FV *", "63 A"),
              ("BC", "CAPACITORES", "45 kvar"), ("DPS", "DPS CL.I/II", "40 kA"), ("MM", "MULTIMED.", "-")]
    for k, (tag, nome, dj) in enumerate(saidas):
        x = 690 + k * 19
        p.L(x, y0 - 5, x, y0 - 22, BT)
        p.R(x - 3.2, y0 - 29.4, 6.4, 6.4, BT); p.L(x - 3.2, y0 - 29.4, x + 3.2, y0 - 23, BT); p.L(x - 3.2, y0 - 23, x + 3.2, y0 - 29.4, BT)
        p.L(x, y0 - 29.4, x, 352, BT)
        p.T(x + 4.0, 356, 2.6, f"{tag}  {nome}  {dj}", INK, 90)
    p.balao(745, y0 - 42, 15)
    p.T(690, 342, 2.4, "* USINA FV: sujeita a parecer de acesso da concessionária", DIM)
    # ---- terra
    p.camada("UF-TERRA", TERRA)
    p.L(102, 330, 806, 330, TERRA)
    p.L(102, y0 - 35, 102, 330, TERRA); p.L(358, y0 - 18, 358, 330, TERRA); p.L(568, y0 - 14, 568, 330, TERRA); p.L(614, y0 - 14, 614, 330, TERRA)
    p.T(204, 322, 3.0, "BARRA DE TERRA DA CABINE - ligada à malha (prancha 4)", TERRA)
    p.T(548, 334, 2.3, "carcaça", TERRA); p.T(617, 334, 2.3, "neutro BT", TERRA)
    # ---- tabelas
    p.camada("TABELA", INK)
    eq = [("1", "Chave fusível 15 kV 100 A, elo 15K a 25K (concessionária)"), ("2", "Pára-raios ZnO 12 kV / 10 kA (poste e cabine)"),
          ("3", "Mufla terminal interna 15 kV (3 un.)"), ("4", "TC medição 15-5 A cl. 0,3 (3 un.)"),
          ("5", "TP medição 13800/115 V (2 un., ligação V)"), ("6", "Bloco de aferição + medidor eletrônico 4Q"),
          ("7", "Seccionadora tripolar 15 kV 400 A c/ lâmina de terra"), ("8", "Disjuntor MT a vácuo 15 kV 400 A 12,5 kA c/ bobina"),
          ("9", "TC proteção 30-5 A 10B (3 un.) e relé 50/51 50N/51N"), ("10", "Fonte auxiliar 125 Vcc (retificador + baterias)"),
          ("11", "Transformador 300 kVA Dyn1, óleo, Z 4,5 %, tap +-2x2,5 %"), ("12", "Termômetro / relé térmico 26/49 (alarme e trip)"),
          ("13", "Disjuntor geral BT 500 A regulável, Icu >= 25 kA"), ("14", "Barramento QGBT Cu 50 x 5 mm (630 A), 25 kA"),
          ("15", "Alimentadores QD-1 a QD-4 e banco de capacitores 45 kvar"), ("16", "DPS cl. I/II 40 kA e multimedidor de grandezas")]
    p.tabela(38, 290, [11, 358], [["Nº", "EQUIPAMENTO / ESPECIFICAÇÃO (1/2)"]] + [list(t) for t in eq[:8]], h=2.6, alt=6.4)
    p.tabela(415, 290, [11, 358], [["Nº", "EQUIPAMENTO / ESPECIFICAÇÃO (2/2)"]] + [list(t) for t in eq[8:]], h=2.6, alt=6.4)
    p.tabela(38, 226, [26, 80, 30, 60, 110, 170], [
        ["ALIM.", "DESTINO", "Ib (A) *", "DISJUNTOR", "CABO (mm²) EPR 0,6/1 kV", "OBSERVAÇÃO"],
        ["QD-1", "Produção", "~120", "160 A 3P", "3 x 70 + 1 x 35", "queda de tensão a verificar"],
        ["QD-2", "Iluminação", "~45", "63 A 3P", "3 x 16 + 16", ""],
        ["QD-3", "Ar condicionado", "~75", "100 A 3P", "3 x 35 + 25", "partida a verificar"],
        ["QD-4", "Usina fotovoltaica", "~45", "63 A 3P", "3 x 16 + 16", "interligação: parecer de acesso"],
        ["BC", "Banco de capacitores", "68", "100 A + contatores", "3 x 25 + 16", "45 kvar - cos φ >= 0,92"],
    ][:6], h=2.6, alt=6.4)
    p.T(38, 178, 2.5, "* Correntes ILUSTRATIVAS: a demanda real da instalação não foi informada. Valores de anteprojeto a confirmar com a concessionária e o estudo de curto-circuito/coordenação.", DIM)
    # legenda de símbolos (embaixo)
    p.camada("SIMBOLOS", INK)
    p.T(38, 160, 3.0, "SÍMBOLOS UTILIZADOS", TIT)
    itens = [("fusivel", "Fusível / chave fusível"), ("seccionadora", "Seccionadora"), ("disjuntor", "Disjuntor"), ("tc", "TC (transf. de corrente)"),
             ("tp", "TP (transf. de potencial)"), ("para_raios", "Pára-raios"), ("trafo", "Transformador"), ("terra", "Aterramento")]
    for i, (nome, txt) in enumerate(itens):
        xx = 62 + i * 94
        v = p.sub(xx, 134, 1.3)
        if nome == "seccionadora":
            v.seccionadora(0, 0, MT)
        elif nome == "tp":
            v.tp(0, 6, MT)
        elif nome == "para_raios":
            v.para_raios(0, 8, MT)
        elif nome == "terra":
            v.terra(0, 6)
        elif nome == "trafo":
            v.trafo(0, 0, MT)
        else:
            getattr(v, nome)(0, 0, MT if nome != "disjuntor" else BT)
        p.T(xx - 20, 117, 2.5, txt, DIM)
    notas(p, ["In MT = 12,6 A ; In BT = 456 A ; Icc BT (barr. infinito) = 10,1 kA.", "Icc MT, elo fusível e ajustes do relé: carta da concessionária.",
              "Normas: NBR 14039 (MT), NBR 5410, NBR 5419, NR-10.", "Medição indireta em MT conforme norma técnica local."],
          ["Linha vermelha = MT 13,8 kV", "Linha azul = BT 380/220 V", "Linha verde = aterramento / proteção", "Tracejado = limites / ligações secundárias"])


def notas(p, notas_, legenda):
    p.camada("NOTAS", INK)
    p.linhas(30, 87.0, [f"{i + 1}. {t}" for i, t in enumerate(notas_)], 2.0, 4.6)
    p.linhas(172, 87.0, legenda, 2.0, 4.6)


# ====================================================================== 2. PLANTA DA CABINE
def p2(dw):
    p = Pg(dw, 1)
    p.titulo("2. PLANTA BAIXA DA CABINE - ESCALA 1:20")
    S = 50.0                        # mm de papel por metro (1:20)
    ox, oy = 85.0, 190.0
    X = lambda m: ox + m * S
    Y = lambda m: oy + m * S
    W, H, E = 6.6, 5.0, 0.15
    # ---- paredes (alvenaria hachurada)
    p.camada("PAREDES", INK)
    for (x, y, w, h) in ((-E, -E, W + 2 * E, E), (-E, H, W + 2 * E, E), (-E, 0, E, H), (W, 0, E, H)):
        p.R(X(x), Y(y), w * S, h * S, INK); hachura(p, X(x), Y(y), w * S, h * S, 2.4)
    for xd in (2.0, 4.7):                                   # divisórias
        p.R(X(xd), Y(0), E * S, H * S, INK); hachura(p, X(xd), Y(0), E * S, H * S, 2.4)
    p.R(X(0), Y(2.5), 2.0 * S, E * S, INK); hachura(p, X(0), Y(2.5), 2.0 * S, E * S, 2.4)
    # ---- vãos (interrompem as paredes) e folhas de porta com arco
    p.camada("PORTAS", TIT)
    def vao(x, y, w, h):
        p.R(X(x), Y(y), w * S, h * S, "#FFFFFF" if False else INK)
    def folha(hx, hy, comp, ang_fecha, ang_abre, cor=TIT):
        cx, cy = X(hx), Y(hy)
        r = comp * S
        p.L(cx, cy, cx + r * math.cos(math.radians(ang_abre)), cy + r * math.sin(math.radians(ang_abre)), cor)
        a0, a1 = sorted((ang_fecha, ang_abre))
        if a1 - a0 > 180:
            a0, a1 = a1, a0 + 360
        p.A(cx, cy, r, a0, a1, cor)
    # P1 (parede norte): x 0,60 a 1,50, abre para fora (cima)
    p.L(X(0.6), Y(H), X(1.5), Y(H), TIT); folha(0.6, H, 0.9, 0, 90)
    p.T(X(1.6), Y(H) + 30, 2.6, "P1 0,90 x 2,10 (medição)", TIT)
    # P2 (parede sul): 2 folhas de 0,85 (x 2,50 a 4,20)
    p.L(X(2.5), Y(0), X(4.2), Y(0), TIT); folha(2.5, 0, 0.85, 0, -90); folha(4.2, 0, 0.85, 180, 270)
    p.T(X(4.3), Y(0) - 40, 2.6, "P2 1,70 x 2,40 (2 folhas, c/ veneziana)", TIT)
    # P3 (parede oeste): y 0,60 a 1,50
    p.L(X(0), Y(0.6), X(0), Y(1.5), TIT); folha(0, 0.6, 0.9, 90, 180)
    p.T(X(0) - 52, Y(0.6) - 8, 2.6, "P3 0,90 x 2,10", TIT)
    # P4 (parede leste): 2 folhas de 0,60 (y 1,60 a 2,80)
    p.L(X(W), Y(1.6), X(W), Y(2.8), TIT); folha(W, 1.6, 0.6, 90, 0); folha(W, 2.8, 0.6, -90, 0)
    p.T(X(W) + 4, Y(3.55), 2.6, "P4 1,20 x 2,10 (QGBT)", TIT)
    # ---- equipamentos
    def eq(x, y, w, h, nome, cor, t=2.4, n=None, num=None):
        p.R(X(x), Y(y), w * S, h * S, cor)
        if nome:
            p.T(X(x) + 2, Y(y) + h * S / 2 - 1, t, nome, cor)
        if num:
            p.balao(X(x) + w * S / 2, Y(y) + h * S + 5, num, cor, 3.0)
    p.camada("EQUIPAMENTOS", MT)
    eq(0.25, 3.95, 0.7, 0.5, "MED", INK, 2.3, num=1)
    eq(1.05, 2.85, 0.8, 1.3, "TC/TP", MT, 2.3, num=2)
    eq(0.55, 1.35, 0.9, 1.0, "SECC", MT, 2.3, num=3)
    eq(0.55, 0.15, 0.9, 1.1, "DISJ MT", MT, 2.3, num=4)
    eq(1.55, 1.7, 0.4, 0.6, "125Vcc", INK, 1.7, num=5)
    for (bx, by, bw) in ((0.55, 1.35, 0.9), (0.55, 0.15, 0.9)):                       # frente dos cubículos + folga de manobra
        p.tracejada(X(bx + bw), Y(by), X(bx + bw), Y(by + 1.0), DIM, 2, 2)
    p.tracejada(X(1.5), Y(0.15), X(1.5), Y(2.35), DIM, 2, 2)
    p.T(X(1.6), Y(0.3), 2.0, "manobra >= 1,50 m", DIM, 90)
    # transformador (vista superior): tanque, radiadores, trilhos, buchas
    p.camada("TRAFO", MT)
    p.retang_tracejado(X(2.3), Y(1.65), 2.2 * S, 1.7 * S, DIM)
    p.T(X(2.32), Y(1.65) - 5, 2.1, "bacia de contenção 2,20 x 1,70", DIM)
    for yr in (2.12, 2.88):
        p.R(X(2.5), Y(yr), 2.0 * S, 0.04 * S, MT)                                     # trilhos
    p.R(X(2.85), Y(2.0), 1.1 * S, 1.0 * S, MT)
    for x0 in (2.65, 3.95):
        p.R(X(x0), Y(2.1), 0.2 * S, 0.8 * S, MT)
        for j in range(1, 4):
            p.L(X(x0 + j * 0.05), Y(2.1), X(x0 + j * 0.05), Y(2.9), MT)
    for xb in (2.98, 3.28, 3.58):
        p.C(X(xb), Y(2.65), 1.9, MT); p.C(X(xb), Y(2.65), 0.6, MT)
    for xb in (3.68, 3.77, 3.86, 3.95):
        p.C(X(xb) - 5, Y(2.3), 1.3, BT)
    p.T(X(2.95), Y(2.45) - 1, 2.6, "TRAFO 300 kVA", MT); p.balao(X(3.4), Y(3.0) + 5, 6, MT, 3.0); p.balao(X(4.5), Y(3.35) + 4, 7, INK, 3.0)
    # QGBT + banco de capacitores
    p.camada("EQUIPAMENTOS", BT)
    eq(4.95, 4.1, 1.6, 0.8, "QGBT", BT, 2.8, num=8)
    p.tracejada(X(4.95), Y(4.1), X(6.55), Y(4.1), DIM, 2, 2)
    p.L(X(5.3), Y(4.1), X(5.3) + 0, Y(4.1) - 0.6 * S, DIM)
    p.A(X(5.3), Y(4.1), 0.6 * S, 180, 270, DIM)
    eq(5.75, 0.35, 0.8, 0.6, "BC 45 kvar", BT, 1.9, num=9)
    p.T(X(5.0), Y(1.45), 2.0, "corredor >= 1,00 m", DIM)
    # ---- canaleta e eletrocalha
    p.camada("CANALETA", TERRA)
    p.tracejada(X(1.9), Y(3.6), X(1.9), Y(0.9), TERRA, 3, 2); p.tracejada(X(1.9), Y(0.9), X(5.1), Y(0.9), TERRA, 3, 2); p.tracejada(X(5.1), Y(0.9), X(5.1), Y(4.1), TERRA, 3, 2)
    p.T(X(2.4), Y(0.9) - 5, 2.1, "canaleta 0,40 x 0,40 (MT e BT separados)", TERRA); p.balao(X(3.0), Y(0.9) + 6, 10, TERRA, 3.0)
    p.tracejada(X(1.0), Y(2.75), X(3.4), Y(2.75), MT, 4, 1); p.balao(X(2.2), Y(3.5), 11, MT, 3.0); p.L(X(2.2), Y(3.5) - 3, X(2.2), Y(2.75), MT)
    # ---- acessórios: iluminação, extintor, placa, emergência, aterramento
    p.camada("ACESSORIOS", INK)
    for (lx, ly) in ((1.0, 4.4), (1.0, 1.25), (3.4, 4.0), (3.4, 0.75), (5.8, 3.0)):
        p.C(X(lx), Y(ly), 3.2, INK); p.L(X(lx) - 3.2, Y(ly), X(lx) + 3.2, Y(ly), INK); p.L(X(lx), Y(ly) - 3.2, X(lx), Y(ly) + 3.2, INK)
    p.balao(X(5.8) + 11, Y(3.0), 12, INK, 3.0)
    p.R(X(6.45), Y(0.15), 0.1 * S, 0.1 * S, MT); p.balao(X(6.3) - 4, Y(0.15) - 6, 13, MT, 3.0)
    p.R(X(2.15), Y(4.4), 0.03 * S, 0.5 * S, TIT); p.balao(X(2.15) + 8, Y(4.65), 14, TIT, 3.0)
    for (ex, ey) in ((0.15, 3.3), (6.4, 3.4)):
        p.C(X(ex), Y(ey), 2.0, TERRA); p.L(X(ex) - 2, Y(ey) - 2, X(ex) + 2, Y(ey) + 2, TERRA); p.L(X(ex) - 2, Y(ey) + 2, X(ex) + 2, Y(ey) - 2, TERRA)
    p.T(X(0.3), Y(3.3) + 4, 2.0, "BT-1", TERRA); p.T(X(6.05), Y(3.4) + 4, 2.0, "BT-2", TERRA); p.balao(X(0.15) + 9, Y(3.3) - 8, 16, TERRA, 3.0)
    # ventilação
    p.camada("VENTILAÇÃO", DIM)
    veneziana(p, X(2.9), Y(H) - 0.0, 1.0 * S, 0.15 * S, DIM)
    p.T(X(2.75), Y(H) + 10, 2.4, "V2 superior 0,60 x 0,40 (h = 2,90 m)", DIM)
    p.T(X(4.4), Y(0) - 15, 2.4, "V1 nas folhas da P2 (entrada de ar)", DIM)
    # nomes de compartimentos
    p.camada("NOMES", TIT)
    p.T(X(0.08), Y(2.62), 2.6, "MEDIÇÃO", TIT); p.T(X(0.08), Y(2.39) - 4, 2.6, "PROTEÇÃO MT", TIT)
    p.T(X(2.3), Y(4.55), 2.8, "TRANSFORMAÇÃO", TIT); p.T(X(4.95), Y(3.75), 2.8, "QGBT / CAPACITORES", TIT)
    # cortes AA e BB
    p.camada("CORTES", INK)
    p.tracejada(X(-0.7), Y(1.2), X(W + 0.8), Y(1.2), INK, 6, 2)
    p.tracejada(X(3.4), Y(-0.9), X(3.4), Y(H + 0.7), INK, 6, 2)
    for (px, py, ang) in ((X(-0.7), Y(1.2), 90), (X(W + 0.8), Y(1.2), 90)):
        p.L(px, py, px, py + 8, INK); p.P([(px - 2, py + 5), (px, py + 8), (px + 2, py + 5)], False, INK)
    p.T(X(-0.7) - 2, Y(1.2) - 7, 3.4, "A", INK); p.T(X(W + 0.8) - 2, Y(1.2) - 7, 3.4, "A", INK)
    for (px, py) in ((X(3.4), Y(-0.9)), (X(3.4), Y(H + 0.7))):
        p.L(px, py, px - 8, py, INK); p.P([(px - 5, py - 2), (px - 8, py), (px - 5, py + 2)], False, INK)
    p.T(X(3.4) + 3, Y(-0.9) - 1, 3.4, "B", INK); p.T(X(3.4) + 3, Y(H + 0.7) - 1, 3.4, "B", INK)
    # ---- cotas
    p.camada("COTAS", DIM)
    p.cota(X(0), Y(H + E), X(W), Y(H + E), "6,60 m (interno)", 30)
    p.cota(X(W + E), Y(0), X(W + E), Y(H), "5,00 m (interno)", 46)
    for (a, b, t) in ((0, 2.0, "2,00"), (2.15, 4.7, "2,55"), (4.85, 6.6, "1,75")):
        p.cota(X(a), Y(-E), X(b), Y(-E), t, -22 if False else -14, DIM, 2.2)
    p.cota(X(-E), Y(0), X(-E), Y(2.5), "2,50", -26, DIM, 2.2)
    p.cota(X(-E), Y(2.65), X(-E), Y(5.0), "2,35", -26, DIM, 2.2)
    p.cota(X(2.15), Y(3.3), X(2.85), Y(3.3), "0,53", 4, DIM, 2.0)
    p.cota(X(3.95) + 0, Y(3.3), X(4.7), Y(3.3), "0,53", 4, DIM, 2.0)
    p.cota(X(2.85), Y(2.0), X(2.85), Y(0), "2,00", -8, DIM, 2.0)
    p.cota(X(2.85), Y(3.0), X(2.85), Y(5.0), "2,00", -8, DIM, 2.0)
    # ---- coluna direita: legenda e tabelas
    dx = 545.0
    p.camada("TABELA", INK)
    itens = [("1", "Caixa de medição (concessionária) 0,70 x 0,50"), ("2", "Cubículo TC/TP de medição 0,80 x 1,30"), ("3", "Cubículo seccionadora c/ terra 0,90 x 1,00"),
             ("4", "Cubículo disjuntor MT a vácuo 0,90 x 1,10"), ("5", "Armário da fonte 125 Vcc 0,40 x 0,60"), ("6", "Transformador 300 kVA 1,50 x 1,00 s/ trilhos"),
             ("7", "Bacia de contenção de óleo 2,20 x 1,70 x 0,40"), ("8", "QGBT 1,60 x 0,80 (corredor frontal >= 1,00 m)"), ("9", "Banco de capacitores 45 kvar 0,80 x 0,60"),
             ("10", "Canaleta 0,40 x 0,40 c/ tampas"), ("11", "Eletrocalha MT aérea (h = 2,50 m)"), ("12", "Luminárias e pontos de luz (círculo com cruz)"),
             ("13", "Extintor CO2 6 kg (classe C)"), ("14", "Placa PERIGO DE MORTE / ALTA TENSÃO"), ("15", "Luminária de emergência"), ("16", "Barras de terra BT-1 e BT-2 (Cu 50 x 5)")]
    p.tabela(dx, 490, [9, 132], [["Nº", "LEGENDA DA PLANTA"]] + [list(t) for t in itens], h=2.3, alt=5.6)
    p.tabela(dx, 385, [11, 96, 34], [
        ["ITEM", "PORTAS E VENTILAÇÃO", "DIMENSÃO (m)"],
        ["P1", "Medição - acesso externo, abre p/ fora", "0,90 x 2,10"],
        ["P2", "Trafo - 2 folhas c/ veneziana, p/ fora", "1,70 x 2,40"],
        ["P3", "Proteção MT - intertravada c/ secc.", "0,90 x 2,10"],
        ["P4", "QGBT - 2 folhas, ferragem antipânico", "1,20 x 2,10"],
        ["V1", "Entrada de ar (folhas da P2)", "0,80 x 0,50"],
        ["V2", "Saída de ar - parede do fundo", "0,60 x 0,40"],
    ], h=2.3, alt=5.8)
    p.tabela(dx, 336, [80, 61], [
        ["AFASTAMENTOS (m)", "PROJETO / EXIGIDO*"],
        ["Trafo - parede lateral (0,53)", "0,53 / >= 0,60  REVER"],
        ["Trafo - porta P2 (frente)", "2,00 / >= 0,80  OK"],
        ["Frente dos cubículos MT", "1,50 / >= 1,50  OK"],
        ["Frente do QGBT", "1,00 / >= 1,00  OK"],
    ], h=2.3, alt=5.8)
    p.T(dx, 296, 2.2, "* exigências indicativas: conferir NBR 14039 e a norma da concessionária.", DIM)
    notas(p, ["Cabine em alvenaria, laje impermeabilizada, pé-direito 3,50 m.", "Piso desempenado com canaleta e bacia; portas abrem para fora.", "Cortes AA e BB: prancha 3.",
              "Iluminação de emergência e extintor CO2 (classe C)."],
          ["Vermelho = equipamentos MT", "Azul = BT", "Verde = canaleta / terra", "Hachura = alvenaria em planta"])


# ====================================================================== 3. CORTES
def hachura(p, x, y, w, h, passo=2.6, cor=DIM):
    """Hachura a 45° recortada no retângulo (x, y, w, h) — alvenaria e concreto em corte."""
    c = -h
    while c < w:
        x1, y1, x2, y2 = c, 0.0, c + h, h
        if x1 < 0:
            y1, x1 = -x1, 0.0
        if x2 > w:
            y2, x2 = h - (x2 - w), w
        if x2 > x1 + 0.05:
            p.L(x + x1, y + y1, x + x2, y + y2, cor)
        c += passo


def brita(p, x, y, w, h, cor=DIM):
    k = 0
    yy = y + 1.0
    while yy < y + h - 0.6:
        xx = x + 1.2 + (1.3 if (k % 2) else 0.0)
        while xx < x + w - 1.0:
            p.C(xx, yy, 0.55, cor)
            xx += 2.6
        yy += 1.7
        k += 1


def buchas_mt(p, x, ybase, cor=MT):
    """Isolador passante de MT: base, 4 saias e terminal."""
    p.R(x - 0.9, ybase, 1.8, 1.2, cor)
    for k in range(4):
        yy = ybase + 1.2 + k * 1.6
        w = 3.2 - k * 0.3
        p.L(x - w, yy, x + w, yy, cor); p.L(x - w, yy, x - w + 0.5, yy + 1.3, cor); p.L(x + w, yy, x + w - 0.5, yy + 1.3, cor)
    p.R(x - 0.9, ybase + 7.6, 1.8, 2.0, cor)
    p.C(x, ybase + 9.6, 0.9, cor)


def veneziana(p, x, y, w, h, cor=DIM, vertical=True):
    """Aberto com palhetas inclinadas (veneziana com tela)."""
    p.R(x, y, w, h, cor)
    yy = y + 1.4
    while yy < y + h - 0.8:
        p.L(x + 0.6, yy, x + w - 0.6, yy + 1.0, cor)
        yy += 1.6


def corte_aa(p, ox, oy, S):
    X = lambda m: ox + m * S
    Y = lambda m: oy + m * S
    W, PE, LJ, EM = 6.6, 3.5, 0.15, 0.15
    # ---- estrutura em corte
    p.camada("CORTE", INK)
    for x0, w in ((-EM, EM), (W, EM)):                                    # paredes externas + baldrame
        p.R(X(x0), Y(-0.55), w * S, (PE + 0.55 + LJ) * S, INK); hachura(p, X(x0), Y(-0.55), w * S, (PE + 0.55 + LJ) * S)
    p.R(X(-0.35), Y(-0.65), 0.35 * S, 0.1 * S, INK); p.R(X(W + EM), Y(-0.65), 0.35 * S, 0.1 * S, INK)       # sapatas
    for x0 in (2.0, 4.7):                                                  # divisórias (sobre a canaleta)
        p.R(X(x0), Y(0), EM * S, PE * S, INK); hachura(p, X(x0), Y(0), EM * S, PE * S)
    p.R(X(-EM), Y(PE), (W + 2 * EM) * S, LJ * S, INK); hachura(p, X(-EM), Y(PE), (W + 2 * EM) * S, LJ * S, 2.2)   # laje
    p.camada("CORTE-FINO", DIM)
    p.R(X(-EM), Y(PE + LJ), (W + 2 * EM) * S, 0.02 * S, INK)                                                      # manta impermeabilizante
    p.R(X(-EM), Y(PE + LJ + 0.02), EM * S, 0.3 * S, INK); p.R(X(W), Y(PE + LJ + 0.02), EM * S, 0.3 * S, INK)      # platibanda
    p.L(X(1.0), Y(PE + LJ) + 6, X(3.0), Y(PE + LJ) + 2, DIM); p.T(X(1.0), Y(PE + LJ) + 8, 1.8, "caimento 2 %", DIM)
    # piso: contrapiso sobre lastro (só onde não há canaleta/bacia)
    for x0, x1 in ((0, 0.5), (2.3, 2.4), (4.4, 4.5), (6.3, W)):
        if x1 > x0:
            p.R(X(x0), Y(-0.1), (x1 - x0) * S, 0.1 * S, INK); hachura(p, X(x0), Y(-0.1), (x1 - x0) * S, 0.1 * S, 1.6)
    p.L(X(0), Y(-0.1), X(W), Y(-0.1), DIM)
    p.L(X(0), Y(0), X(0.5), Y(0), INK); p.L(X(6.3), Y(0), X(W), Y(0), INK)
    # ---- canaletas com tampas e cabos
    p.camada("CANALETA", TERRA)
    for x0, x1 in ((0.5, 2.3), (4.5, 6.3)):
        p.R(X(x0), Y(-0.4), (x1 - x0) * S, 0.4 * S, TERRA)
        k = x0
        while k < x1 - 1e-6:
            e = min(k + 0.6, x1)
            p.R(X(k), Y(-0.03), (e - k) * S, 0.03 * S, TERRA)
            k = e
        for j, cx in enumerate((0.1, 0.16, 0.22)):
            p.C(X(x0 + cx + 0.02), Y(-0.33), 0.5 * S / 20, MT)
        for j in range(3):
            p.C(X(x1 - 0.35 + j * 0.08), Y(-0.31), 0.6, BT)
    p.L(X(2.3), Y(-0.44), X(4.5), Y(-0.44), TERRA); p.T(X(2.55), Y(-0.6), 1.7, "eletroduto Ø100 sob a bacia", TERRA)
    # ---- bacia de contenção (brita + grade)
    p.camada("BACIA", INK)
    p.R(X(2.3), Y(-0.4), 2.2 * S, 0.4 * S, INK); p.R(X(2.4), Y(-0.36), 2.0 * S, 0.36 * S, DIM)
    brita(p, X(2.4), Y(-0.36), 2.0 * S, 0.36 * S)
    p.L(X(2.4), Y(0), X(4.4), Y(0), INK)
    for k in range(1, 20):
        p.L(X(2.4 + k * 0.1), Y(0), X(2.4 + k * 0.1), Y(0) + 1.0, DIM)
    p.T(X(2.42), Y(-0.36) - 3.4, 1.6, "brita + grade galvanizada + sifão corta-chama", DIM)
    # ---- cubículo MT (corte interno)
    p.camada("CUBICULO", MT)
    p.R(X(0.5), Y(0), 1.0 * S, 2.0 * S, MT)
    p.L(X(0.5), Y(1.5), X(1.5), Y(1.5), MT); p.L(X(0.5), Y(0.75), X(1.5), Y(0.75), MT)
    for j in range(3):
        p.C(X(0.7 + j * 0.3), Y(1.78), 1.1, MT)
    p.T(X(0.55), Y(1.9) - 2, 1.6, "barramentos", MT)
    p.R(X(0.7), Y(0.85), 0.6 * S, 0.55 * S, MT); p.T(X(0.75), Y(1.1), 1.7, "52  DISJ", MT); p.C(X(0.8), Y(0.85), 0.8, MT); p.C(X(1.2), Y(0.85), 0.8, MT)
    for j in range(2):
        p.C(X(0.7 + j * 0.4), Y(0.55), 1.8, MT)
    p.T(X(0.55), Y(0.42) - 3, 1.6, "TC / mufla", MT)
    for j in range(3):
        p.L(X(0.72 + j * 0.16), Y(0), X(0.72 + j * 0.16), Y(-0.28), MT)
    p.R(X(0.6), Y(0.05), 0.8 * S, 0.02 * S, MT)
    p.R(X(0.62), Y(1.55), 0.16 * S, 0.3 * S, MT)
    # ---- trafo (peça do pack de elétrica, tamanho real; 1 mm real = S/1000 mm de papel)
    p.camada("TRAFO", MT)
    transformador_lateral(p, X(2.65), Y(0), S / 1000.0, 300, MT, BT)
    # ---- QGBT (corte interno)
    p.camada("QGBT", BT)
    p.R(X(5.0), Y(0), 1.4 * S, 2.1 * S, BT); p.R(X(5.0), Y(0), 1.4 * S, 0.1 * S, BT)
    for j in range(6):
        p.L(X(5.1 + j * 0.05), Y(2.0), X(5.1 + j * 0.05), Y(2.1), BT)
    for j in range(3):
        p.L(X(5.05), Y(1.88 - j * 0.05), X(6.35), Y(1.88 - j * 0.05), BT)
    p.T(X(5.6), Y(1.93), 1.4, "barram. 630 A", BT)
    p.R(X(5.15), Y(1.15), 0.5 * S, 0.55 * S, BT); p.T(X(5.19), Y(1.4), 1.7, "DG 500 A", BT)
    for j in range(3):
        p.R(X(5.8 + j * 0.2), Y(0.75), 0.15 * S, 0.4 * S, BT)
    p.T(X(5.78), Y(0.55), 1.5, "saídas QD", BT)
    p.R(X(5.15), Y(0.25), 0.5 * S, 0.3 * S, BT); p.T(X(5.19), Y(0.35), 1.5, "multimed.", BT)
    for j in range(3):
        p.L(X(5.3 + j * 0.15), Y(0), X(5.3 + j * 0.15), Y(-0.28), BT)
    # ---- eletrocalhas e cabos aéreos
    p.camada("ELETROCALHA", INK)
    p.R(X(1.3), Y(2.5), 2.3 * S, 0.1 * S, INK); p.T(X(1.4), Y(2.62) + 1, 1.7, "eletrocalha MT (cabos 8,7/15 kV)", MT)
    p.R(X(3.75), Y(2.28), 1.75 * S, 0.1 * S, INK); p.T(X(3.85), Y(2.4) + 1, 1.7, "eletrocalha BT", BT)
    p.tracejada(X(1.35), Y(2.0), X(1.35), Y(2.5), MT); p.tracejada(X(3.28), Y(2.5), X(3.28), Y(1.55) + 9.6, MT)
    p.tracejada(X(3.96), Y(1.55) + 3.4, X(3.96), Y(2.28), BT); p.tracejada(X(5.4), Y(2.28), X(5.4), Y(2.1), BT)
    # ---- terra, iluminação, segurança, veneziana V2 (parede do fundo)
    p.camada("TERRA", TERRA)
    p.L(X(0.02), Y(0.28), X(W - 0.02), Y(0.28), TERRA); p.T(X(0.05), Y(0.28) + 1.2, 1.6, "barra de terra Cu 50 x 5", TERRA)
    for xx in (0.55, 3.0, 5.05):
        p.L(X(xx), Y(0.28), X(xx), Y(0.12), TERRA)
    p.C(X(1.8), Y(-0.62), 0.7, TERRA); p.C(X(3.4), Y(-0.62), 0.7, TERRA); p.C(X(5.0), Y(-0.62), 0.7, TERRA)
    p.L(X(-0.2), Y(-0.62), X(W + 0.2), Y(-0.62), TERRA); p.T(X(0.1), Y(-0.62) - 3.2, 1.6, "malha Cu 50 mm² a 0,60 m (prancha 4)", TERRA)
    p.camada("ACESSORIOS", INK)
    for xx, w in ((3.3, 0.7), (1.0, 0.3), (6.0, 0.3)):
        p.R(X(xx - w / 2), Y(PE - 0.09), w * S, 0.06 * S, INK)
    p.T(X(3.0), Y(PE - 0.09) - 2.6, 1.6, "luminária 2x36 W", DIM)
    p.R(X(6.45), Y(0.2), 0.1 * S, 0.7 * S, MT); p.C(X(6.5), Y(0.9) + 0.9, 0.9, MT)
    p.R(X(2.16), Y(1.35), 0.04 * S, 0.5 * S, TIT)
    veneziana(p, X(2.9), Y(2.9), 1.0 * S, 0.4 * S, DIM)
    p.T(X(2.9), Y(3.3) + 1.2, 1.6, "V2 (parede do fundo)", DIM)
    # ---- cotas
    p.camada("COTAS", DIM)
    y_ch = Y(-0.95)
    pts = [0, 0.5, 1.5, 2.65, 4.15, 5.0, 6.4, 6.6]
    for a, b in zip(pts, pts[1:]):
        p.cota(X(a), y_ch, X(b), y_ch, f"{b - a:.2f}", -4, DIM, 1.7)
    p.cota(X(0), Y(-1.2), X(W), Y(-1.2), "6,60 m (interno)", -6)
    p.cota(X(W + EM), Y(0), X(W + EM), Y(PE), f"{PE:.2f} pé-direito", 12)
    p.cota(X(W + EM), Y(PE), X(W + EM), Y(PE + LJ), "0,15", 30, DIM, 1.7)
    p.cota(X(0.5), Y(0), X(0.5), Y(2.0), "2,00", -10, DIM, 1.8)
    p.cota(X(2.65), Y(0), X(2.65), Y(1.9) + 0.0, "1,90", -8, DIM, 1.8)
    p.cota(X(5.0), Y(0), X(5.0), Y(2.1), "2,10", -8, DIM, 1.8)
    p.cota(X(4.5), Y(0), X(4.5), Y(2.5), "2,50", 20, DIM, 1.7)
    # ---- balões
    def bal(n, x, y, dx, dy, cor=INK):
        p.L(X(x), Y(y), X(x) + dx, Y(y) + dy, cor)
        p.balao(X(x) + dx, Y(y) + dy, n, cor, 2.6)
    p.camada("BALOES", INK)
    bal(1, 5.7, PE + 0.1, 0, 10); bal(2, -0.08, 2.5, -12, 0); bal(3, 5.0, -0.05, 0, -22 - 0)
    bal(4, 1.0, 1.1, 0, 0.001); bal(5, 3.1, 1.0, 0, 0.001); bal(6, 3.28, 1.55, 0, 0.001)
    bal(7, 3.4, -0.2, 0, 0.001); bal(8, 1.4, -0.2, 0, 0.001); bal(9, 5.9, 1.3, 0, 0.001)
    bal(11, 3.4, 3.1, 0, 0.001); bal(12, 2.2, 2.55, 0, 0.001); bal(13, 6.1, 0.28, 0, 0.001)
    bal(14, 0.1, -0.2, 0, 0.001); bal(15, 3.3, PE - 0.09, 0, 0.001); bal(16, 6.5, 0.5, 0, 0.001)
    p.T(X(0), Y(PE + LJ + 0.3) + 6, 3.4, "CORTE AA (longitudinal, olhando para o fundo)", TIT)


def corte_bb(p, ox, oy, S):
    X = lambda m: ox + m * S      # m = profundidade a partir da parede sul (à esquerda)
    Y = lambda m: oy + m * S
    W, PE, LJ, EM = 5.0, 3.5, 0.15, 0.15
    p.camada("CORTE", INK)
    # parede norte (direita), com a abertura V2 (2,90 a 3,30)
    p.R(X(W), Y(-0.55), EM * S, (2.9 + 0.55) * S, INK); hachura(p, X(W), Y(-0.55), EM * S, (2.9 + 0.55) * S)
    p.R(X(W), Y(3.3), EM * S, (PE + LJ - 3.3) * S, INK); hachura(p, X(W), Y(3.3), EM * S, (PE + LJ - 3.3) * S)
    # parede sul (esquerda) com a abertura da porta P2 (0 a 2,40): verga + folha
    p.R(X(-EM), Y(-0.55), EM * S, 0.55 * S, INK); hachura(p, X(-EM), Y(-0.55), EM * S, 0.55 * S)
    p.R(X(-EM), Y(2.4), EM * S, (PE + LJ - 2.4) * S, INK); hachura(p, X(-EM), Y(2.4), EM * S, (PE + LJ - 2.4) * S)
    p.camada("PORTA", TIT)
    p.R(X(-0.06), Y(0), 0.05 * S, 2.35 * S, TIT)
    for k in range(1, 12):
        p.L(X(-0.06), Y(0.18 * k), X(-0.01), Y(0.18 * k) + 0.6, TIT)
    p.R(X(-0.4), Y(-0.05), 0.4 * S, 0.05 * S, INK)                                                             # soleira/degrau
    p.T(X(-0.15) - 32, Y(2.45) + 2, 1.7, "P2 (folhas c/ veneziana)", TIT)
    p.camada("CORTE", INK)
    p.R(X(-EM), Y(PE), (W + 2 * EM) * S, LJ * S, INK); hachura(p, X(-EM), Y(PE), (W + 2 * EM) * S, LJ * S, 2.2)
    p.R(X(-EM), Y(PE + LJ), (W + 2 * EM) * S, 0.02 * S, INK)
    p.R(X(-EM), Y(PE + LJ + 0.02), EM * S, 0.3 * S, INK); p.R(X(W), Y(PE + LJ + 0.02), EM * S, 0.3 * S, INK)
    p.L(X(0), Y(0), X(W), Y(0), INK)
    p.R(X(0), Y(-0.1), W * S, 0.1 * S, INK); hachura(p, X(0), Y(-0.1), 1.6 * S, 0.1 * S, 1.6); hachura(p, X(3.4), Y(-0.1), 1.6 * S, 0.1 * S, 1.6)
    p.R(X(-0.35), Y(-0.65), 0.35 * S, 0.1 * S, INK); p.R(X(W + EM), Y(-0.65), 0.35 * S, 0.1 * S, INK)
    veneziana(p, X(W) - 0.0, Y(2.9), EM * S, 0.4 * S, DIM)
    p.T(X(W) + 6, Y(3.1), 1.7, "V2 0,60 x 0,40 (tela)", DIM)
    # bacia
    p.camada("BACIA", INK)
    p.R(X(1.6), Y(-0.4), 1.8 * S, 0.4 * S, INK); p.R(X(1.7), Y(-0.36), 1.6 * S, 0.36 * S, DIM)
    brita(p, X(1.7), Y(-0.36), 1.6 * S, 0.36 * S)
    for k in range(1, 16):
        p.L(X(1.7 + k * 0.1), Y(0), X(1.7 + k * 0.1), Y(0) + 1.0, DIM)
    p.L(X(3.0), Y(-0.36), X(3.0), Y(-0.55), INK); p.L(X(3.0), Y(-0.55), X(W + EM), Y(-0.55), INK)
    p.C(X(3.0), Y(-0.4), 1.2, INK)
    p.T(X(3.05), Y(-0.55) + 1.2, 1.6, "sifão corta-chama + dreno Ø100", DIM)
    p.R(X(W + 0.3), Y(-0.9), 0.5 * S, 0.5 * S, INK); p.T(X(W + 0.28), Y(-0.3) - 2, 1.5, "caixa separadora", DIM)
    p.L(X(W + EM), Y(-0.55), X(W + 0.3), Y(-0.55), INK)
    # trafo (peça do pack de elétrica, vista frontal)
    p.camada("TRAFO", MT)
    transformador_frontal(p, X(1.75), Y(0), S / 1000.0, 300, MT, BT)
    # aterramento embutido no chão
    p.camada("TERRA", TERRA)
    p.L(X(0.02), Y(0.28), X(W - 0.02), Y(0.28), TERRA)
    p.L(X(2.5), Y(0.28), X(2.5), Y(0.15), TERRA)
    for yy in (0.4, 1.5, 2.6, 3.7, 4.6):
        p.C(X(yy), Y(-0.65), 0.7, TERRA)
    p.L(X(-0.2), Y(-0.65), X(W + 0.2), Y(-0.65), TERRA)
    p.T(X(0.05), Y(-0.65) - 3.2, 1.6, "malha Cu 50 mm² a 0,60 m", TERRA)
    # eletrocalha (seção) e luminária
    p.camada("ACESSORIOS", INK)
    p.R(X(2.1), Y(2.5), 0.8 * S, 0.1 * S, INK)
    for j in range(3):
        p.C(X(2.25 + j * 0.2), Y(2.58), 0.8, MT)
    p.T(X(1.6), Y(2.62) + 1.2, 1.7, "eletrocalha MT (seção)", MT)
    p.R(X(2.0), Y(PE - 0.09), 1.0 * S, 0.06 * S, INK)
    p.R(X(0.3), Y(PE - 0.09), 0.3 * S, 0.06 * S, INK)
    p.R(X(4.3), Y(PE - 0.09), 0.3 * S, 0.06 * S, INK)
    p.T(X(2.0), Y(PE - 0.09) - 2.6, 1.6, "luminária 2x36 W", DIM)
    p.T(X(0.1), Y(2.55), 1.6, "sentido de abertura da porta ->", TIT, 0)
    p.camada("COTAS", DIM)
    p.cota(X(0), Y(-0.95), X(1.6), Y(-0.95), "1,60", -4, DIM, 1.7)
    p.cota(X(1.6), Y(-0.95), X(3.4), Y(-0.95), "1,80 (bacia)", -4, DIM, 1.7)
    p.cota(X(3.4), Y(-0.95), X(W), Y(-0.95), "1,60", -4, DIM, 1.7)
    p.cota(X(0), Y(-1.2), X(W), Y(-1.2), "5,00 m (interno)", -6)
    p.cota(X(W + EM), Y(0), X(W + EM), Y(PE), f"{PE:.2f} pé-direito", 12)
    p.cota(X(0), Y(0), X(0), Y(2.4), "2,40 (porta)", 16 - 30 + 30, DIM, 1.7)
    p.cota(X(3.8), Y(0), X(3.8), Y(1.9), "1,90", 8, DIM, 1.7)
    p.T(X(0), Y(PE + LJ + 0.3) + 6, 3.4, "CORTE BB (transversal, pelo trafo)", TIT)


def p3(dw):
    p = Pg(dw, 2)
    p.titulo("3. CORTES AA (LONGITUDINAL) E BB (TRANSVERSAL) - ESCALA 1:20")
    S = 50.0
    corte_aa(p, 60.0, 295.0, S)
    corte_bb(p, 490.0, 295.0, S)
    # legenda numerada (duas colunas)
    itens = [("1", "Laje impermeabilizada (manta), platibanda, caimento 2 %"), ("2", "Alvenaria 15 cm reboc. + baldrame; cinta/verga sobre vãos"),
             ("3", "Contrapiso desempenado sobre lastro (cotas em m)"), ("4", "Cubículo MT blindado: barramentos, disjuntor extraível, TCs"),
             ("5", "Transformador 300 kVA sobre trilhos, radiadores, placa"), ("6", "Buchas MT 15 kV (3) e BT (4), termômetro 26/49"),
             ("7", "Bacia de contenção: brita, grade, sifão corta-chama, dreno"), ("8", "Canaleta 0,40 x 0,40 c/ tampas (MT e BT separados)"),
             ("9", "QGBT: DG 500 A, barramento 630 A, saídas, multimedidor"), ("10", "Porta P2 (BB): 2 folhas c/ veneziana, abre p/ fora"),
             ("11", "Veneziana V2 (saída de ar) com tela, h = 2,90 a 3,30 m"), ("12", "Eletrocalha MT (2,50 m) e BT (2,28 m)"),
             ("13", "Barra de terra Cu 50 x 5 mm e derivações aos equipamentos"), ("14", "Eletroduto Ø100 do ramal MT entrando pelo lado esquerdo"),
             ("15", "Iluminação 2x36 W e de emergência"), ("16", "Extintor CO2 (classe C) e placa PERIGO DE MORTE")]
    p.camada("LEGENDA-CORTES", INK)
    p.tabela(60, 200, [8, 158], [["Nº", "DESCRIÇÃO"]] + [list(t) for t in itens[:8]], alt=5.0)
    p.tabela(240, 200, [8, 158], [["Nº", "DESCRIÇÃO"]] + [list(t) for t in itens[8:]], alt=5.0)
    p.tabela(430, 200, [42, 150, 30], [["ELEMENTO", "DESCRIÇÃO", "COTA (m)"],
             ["Bacia / canaleta", "Concreto impermeável, 0,40 m abaixo do piso", "-0,40"],
             ["Ventilação V1", "Nas folhas da porta P2 (entrada de ar)", "0,15 a 0,55"],
             ["Ventilação V2", "Parede do fundo, saída de ar", "2,90 a 3,30"],
             ["Eletrocalhas", "MT a 2,50 e BT a 2,28 (topo)", "2,28 / 2,50"],
             ["Pé-direito", "Altura livre interna", "3,50"]], alt=5.0)
    p.T(60, 146, 2.0, "Alturas do trafo, cubículo e QGBT: conferir com o fabricante. Detalhes indicativos; sem fundações dimensionadas.", DIM)
    notas(p, ["Cotas em m; escala 1:20.", "Cortes esquemáticos: fundações não dimensionadas.", "Ventilação natural por V1 (folhas da P2) e V2 (verificar carga térmica).",
              "Cabos MT/BT em eletrocalha aérea e canaletas sob o piso."],
          ["Vermelho = MT", "Azul = BT", "Verde = terra / canaleta", "Hachura = alvenaria / concreto em corte"])


# ====================================================================== 4. ATERRAMENTO
def p4(dw):
    p = Pg(dw, 3)
    p.titulo("4. MALHA DE ATERRAMENTO E SPDA - PLANTA 1:50 E DETALHES")
    S = 18.0                                   # mm por metro (1:50 seria 20; 18 para caber com o terreno)
    ox, oy = 92.0, 178.0
    X = lambda m: ox + m * S
    Y = lambda m: oy + m * S
    MW, MH = 12.0, 9.0                         # malha
    CX0, CY0, CW, CH = 2.7, 1.9, 6.9, 5.3      # cabine (externo)
    # ---- terreno, cerca, portão, acesso
    p.camada("TERRENO", DIM)
    p.retang_tracejado(X(-3.5), Y(-3.0), 19.0 * S, 15.0 * S, DIM)
    p.T(X(-3.4), Y(12.0) + 2, 2.0, "LIMITE DO TERRENO / CERCA (alambrado h = 2,00 m)", DIM)
    p.L(X(-3.5), Y(3.0), X(-3.5), Y(5.5), INK)
    p.T(X(-3.5) + 2, Y(5.5) + 2, 1.9, "PORTÃO 2,50 m", INK)
    p.L(X(-3.5) - 6, Y(4.25), X(-3.5) + 8, Y(4.25), DIM)
    # ---- cabine em planta
    p.camada("CABINE", INK)
    p.R(X(CX0), Y(CY0), CW * S, CH * S, INK); p.R(X(CX0 + 0.15), Y(CY0 + 0.15), (CW - 0.3) * S, (CH - 0.3) * S, INK)
    for xx in (2.0, 4.7):
        p.L(X(CX0 + 0.15 + xx), Y(CY0 + 0.15), X(CX0 + 0.15 + xx), Y(CY0 + CH - 0.15), INK)
    p.L(X(CX0 + 0.15), Y(CY0 + 0.15 + 2.5), X(CX0 + 0.15 + 2.0), Y(CY0 + 0.15 + 2.5), INK)
    p.T(X(CX0) + 4, Y(CY0 + CH) - 6, 2.0, "MED.", TIT); p.T(X(CX0) + 4, Y(CY0) + 5, 2.0, "PROT. MT", TIT)
    p.T(X(CX0 + 2.6), Y(CY0 + 2.4), 2.4, "TRAFO", MT); p.T(X(CX0 + 5.2), Y(CY0 + 2.4), 2.4, "QGBT", BT)
    p.T(X(CX0) + 2, Y(CY0 + CH) + 3, 2.2, "CABINE 6,90 x 5,30 m (externo)", INK)
    # ---- malha
    p.camada("MALHA", TERRA)
    for k in range(int(MW / 2) + 1):
        p.L(X(k * 2), Y(0), X(k * 2), Y(MH), TERRA)
    for k in range(int(MH / 2) + 1):
        p.L(X(0), Y(k * 2), X(MW), Y(k * 2), TERRA)
    p.L(X(0), Y(MH), X(MW), Y(MH), TERRA); p.L(X(MW), Y(0), X(MW), Y(MH), TERRA)
    for i in range(int(MW / 2) + 1):
        for j in range(int(MH / 2) + 1):
            p.C(X(i * 2), Y(j * 2), 0.45, TERRA)
    # hastes (círculo com cruz)
    hast = [(0, 0), (MW, 0), (0, MH), (MW, MH), (MW / 2, 0), (MW / 2, MH), (0, MH / 2), (MW, MH / 2)]
    for (hx, hy) in hast:
        p.C(X(hx), Y(hy), 2.4, TERRA); p.L(X(hx) - 2.4, Y(hy), X(hx) + 2.4, Y(hy), TERRA); p.L(X(hx), Y(hy) - 2.4, X(hx), Y(hy) + 2.4, TERRA)
    # caixas de inspeção
    for k, (cx, cy) in enumerate(((0, 0), (MW, MH))):
        p.R(X(cx) - 5, Y(cy) - 5, 10, 10, INK); p.T(X(cx) + 6, Y(cy) - 7, 1.9, f"CI-{k + 1}", INK)
    # ligações à barra de terra (cordoalha 50 mm² até a cabine)
    p.camada("LIGACOES", TERRA)
    for xx in (CX0 + 1.2, CX0 + 5.7):
        p.L(X(xx), Y(CY0), X(xx), Y(2.0), TERRA); p.R(X(xx) - 1.6, Y(CY0) - 1.6, 3.2, 3.2, TERRA)
    p.T(X(CX0 + 1.2) + 2, Y(CY0) - 6.5, 1.8, "BT-1", TERRA); p.T(X(CX0 + 5.7) + 2, Y(CY0) - 6.5, 1.8, "BT-2", TERRA)
    # poste e pára-raios
    p.camada("POSTE", MT)
    p.C(X(-1.8), Y(4.25), 3.0, MT); p.L(X(-1.8) - 3, Y(4.25), X(-1.8) + 3, Y(4.25), MT); p.L(X(-1.8), Y(4.25) - 3, X(-1.8), Y(4.25) + 3, MT)
    p.L(X(-1.8) + 3, Y(4.25), X(0), Y(4.25), TERRA)
    p.T(X(-3.3), Y(4.25) - 8, 1.8, "POSTE C/ PÁRA-RAIOS", MT)
    # ramal MT subterrâneo até a cabine (referência)
    p.camada("RAMAL-MT", MT)
    p.tracejada(X(-1.8) + 3, Y(3.0), X(0.6), Y(3.0), MT, 4, 2); p.R(X(0.6) - 2.5, Y(3.0) - 2.5, 5, 5, MT)
    p.tracejada(X(0.6) + 2.5, Y(3.0), X(CX0), Y(3.0), MT, 4, 2)
    p.T(X(-1.4), Y(3.0) - 7, 1.8, "ramal MT subterrâneo (ref.)", MT); p.T(X(0.6) - 3, Y(3.0) + 4, 1.7, "CP", MT)
    # ligação da cerca
    p.camada("CERCA", TERRA)
    for (yy) in (0.0, MH):
        p.L(X(-3.5), Y(yy), X(0), Y(yy), TERRA)
    p.T(X(-3.4), Y(0.0) + 2, 1.7, "ligação da cerca à malha", TERRA)
    # cotas
    p.camada("COTAS", DIM)
    p.cota(X(0), Y(MH), X(MW), Y(MH), "12,00 m", 20)
    p.cota(X(MW), Y(0), X(MW), Y(MH), "9,00 m", 18)
    p.cota(X(0), Y(0), X(2), Y(0), "2,00", -12, DIM, 1.8)
    p.cota(X(0), Y(CY0), X(CX0), Y(CY0), "2,70", -32, DIM, 1.8)
    p.cota(X(CX0 + CW), Y(0), X(CX0 + CW), Y(CY0), "1,90", 8, DIM, 1.8)
    # ---- balões da planta
    def bal(n, x, y, dx, dy, cor=INK):
        p.L(X(x), Y(y), X(x) + dx, Y(y) + dy, cor); p.balao(X(x) + dx, Y(y) + dy, n, cor, 2.7)
    p.camada("BALOES", INK)
    bal(1, 8.0, 0.0, 0, -14); bal(2, 6.0, MH, 0, 14); bal(3, MW, MH, 10, 10); bal(4, 4.0, 4.0, 0, 0.001)
    bal(5, CX0 + 1.2, 2.0, -14, -14); bal(6, -1.8, 4.25, 0, 14); bal(7, -3.5, 0, 0, -13); bal(8, 0.6, 3.0, 0, 12)
    bal(10, 15.5, 6.0, 0, 0.001)
    # ---- detalhes (coluna direita)
    dx = 440.0
    p.camada("DETALHES", INK)
    # A - haste e caixa de inspeção (1:10)
    ax, ay = dx + 10, 505.0
    p.T(ax - 6, 552, 2.6, "DETALHE A - HASTE E CAIXA DE INSPEÇÃO (esc. 1:10)", TIT)
    p.L(ax - 6, ay, ax + 90, ay, INK)
    for k in range(0, 96, 6):
        p.L(ax - 6 + k, ay, ax - 10 + k, ay - 3, DIM)
    p.R(ax + 30, ay - 30, 30, 30, INK)                      # caixa Ø300 x 300 de profundidade
    p.R(ax + 28, ay, 34, 3, INK); p.T(ax + 64, ay + 4, 1.7, "tampa ferro fundido Ø300 (carga leve)", INK)
    brita(p, ax + 31, ay - 30, 28, 8)
    p.L(ax + 45, ay - 4, ax + 45, ay - 58, TERRA); p.R(ax + 43.5, ay - 58, 3, 54, TERRA)
    p.L(ax + 45, ay - 60, ax + 45, ay - 66, TERRA)
    p.L(ax + 42, ay - 62, ax + 48, ay - 64, TERRA)
    p.R(ax + 42, ay - 10, 6, 4, TERRA); p.T(ax + 64, ay - 12, 1.7, "grampo / solda exotérmica cabo-haste", TERRA)
    p.L(ax - 6, ay - 60, ax + 20, ay - 60, TERRA); p.L(ax + 20, ay - 60, ax + 45, ay - 8, TERRA)
    p.T(ax - 6, ay - 56, 1.7, "cabo Cu 50 mm² a 0,60 m", TERRA)
    p.T(ax + 50, ay - 40, 1.7, "haste cobreada 5/8 pol x 2,40 m", TERRA)
    p.T(ax + 50, ay - 44, 1.7, "(interrompida: 2,40 m)", DIM)
    p.T(ax + 64, ay - 24, 1.7, "brita e dreno", DIM)
    p.cota(ax + 100, ay, ax + 100, ay - 60, "0,60", 4, DIM, 1.7)
    # B - conexões exotérmicas
    bx, by = dx + 158, 498.0
    p.T(bx - 6, 552, 2.6, "DETALHE B - SOLDA EXOTÉRMICA", TIT)
    for k, nome in enumerate(("cabo-cabo em X", "cabo-cabo em T", "cabo-haste")):
        cx = bx + 6 + k * 36
        if k == 0:
            p.L(cx - 10, by, cx + 10, by, TERRA); p.L(cx, by - 10, cx, by + 10, TERRA)
        elif k == 1:
            p.L(cx - 10, by, cx + 10, by, TERRA); p.L(cx, by, cx, by - 10, TERRA)
        else:
            p.L(cx - 10, by, cx, by, TERRA); p.C(cx, by - 4, 3.0, TERRA); p.L(cx, by - 4, cx, by - 22, TERRA)
        p.C(cx, by, 5.5, INK); p.C(cx, by, 0.8, INK)
        p.T(cx - 12, by - 28, 1.6, nome, DIM)
    p.T(bx - 4, by - 38, 1.7, "molde de grafite + carga (pó), cabo 50 mm²", DIM)
    p.T(bx - 4, by - 42, 1.7, "conexão em todos os nós e hastes", DIM)
    p.T(bx - 4, by - 46, 1.7, "enterrada: proteção com mástique", DIM)
    # C - barra de terra da cabine
    cx0, cy0 = dx + 292, 452.0
    p.T(cx0 - 20, 552, 2.6, "DETALHE C - BARRA DE TERRA (BT-1/2)", TIT)
    p.R(cx0, cy0, 70, 90, INK); hachura(p, cx0, cy0, 70, 90, 3.6)
    p.R(cx0 + 8, cy0 + 40, 54, 6, TERRA)
    for k in range(6):
        p.C(cx0 + 13 + k * 8.6, cy0 + 43, 1.3, TERRA)
    for xx in (cx0 + 10, cx0 + 58):
        p.R(xx - 1.5, cy0 + 36, 3, 4, INK)
    p.L(cx0 + 20, cy0 + 40, cx0 + 20, cy0 + 10, TERRA); p.L(cx0 + 22, cy0 + 40, cx0 + 22, cy0 + 10, TERRA)
    p.L(cx0 + 21, cy0 + 10, cx0 + 21, cy0 - 8, TERRA)
    for k, (t, c) in enumerate((("barra Cu 50 x 5 x 500 mm, h = 0,30 m do piso", TERRA), ("2 isoladores; terminais de compressão e etiqueta", DIM),
                                ("cordoalha 50 mm² em eletroduto Ø32 até a malha", TERRA))):
        p.T(cx0 - 24, cy0 - 14 - k * 4, 1.7, t, c)
    # SPDA da cobertura
    sx, sy, sk = dx + 6, 260.0, 12.0
    p.T(sx - 2, 372, 2.6, "DETALHE D - SPDA DA COBERTURA (esc. ~1:80)", TIT)
    p.R(sx, sy, 6.9 * sk, 5.3 * sk, INK); p.R(sx + 0.15 * sk, sy + 0.15 * sk, 6.6 * sk, 5.0 * sk, DIM)
    p.camada("SPDA", MT)
    p.R(sx + 0.3 * sk, sy + 0.3 * sk, 6.3 * sk, 4.7 * sk, MT)
    p.L(sx + 3.45 * sk, sy + 0.3 * sk, sx + 3.45 * sk, sy + 5.0 * sk, MT)
    p.L(sx + 0.3 * sk, sy + 2.65 * sk, sx + 6.6 * sk, sy + 2.65 * sk, MT)
    for (tx, ty) in ((0.3, 0.3), (6.6, 0.3), (0.3, 5.0), (6.6, 5.0), (3.45, 0.3), (3.45, 5.0), (0.3, 2.65), (6.6, 2.65)):
        p.C(sx + tx * sk, sy + ty * sk, 1.1, MT)
    for (tx, ty) in ((0.3, 0.3), (6.6, 5.0)):
        p.C(sx + tx * sk, sy + ty * sk, 2.6, TERRA); p.L(sx + tx * sk - 2.6, sy + ty * sk, sx + tx * sk + 2.6, sy + ty * sk, TERRA); p.L(sx + tx * sk, sy + ty * sk - 2.6, sx + tx * sk, sy + ty * sk + 2.6, TERRA)
    p.T(sx, sy - 6, 1.7, "captor: cabo Cu 35 mm² (ou Al 8 mm) na platibanda; malha 5 x 5 m", MT)
    p.T(sx, sy - 10, 1.7, "terminais aéreos Franklin h = 0,30 m nos vértices e meios", MT)
    p.T(sx, sy - 14, 1.7, "2 descidas (círculo verde) em cantos opostos, em eletroduto", TERRA)
    p.T(sx, sy - 18, 1.7, "com caixa de ensaio a 1,5 m e ligação à malha (BT-1/BT-2)", TERRA)
    # tabela de materiais da malha
    p.camada("MATERIAIS-MALHA", INK)
    p.tabela(dx + 140, 370, [8, 86, 16, 12, 38], [
        ["ITEM", "MATERIAL DA MALHA E SPDA", "QTD", "UN.", "OBS."],
        ["1", "Cabo de cobre nu 50 mm² (malha 2 x 2 m + ligações)", "~145", "m", "12 x 9 m"],
        ["2", "Haste cobreada 5/8 pol x 2,40 m", "8", "un", "254 µm Cu"],
        ["3", "Caixa de inspeção Ø300 c/ tampa", "2", "un", "CI-1, CI-2"],
        ["4", "Solda exotérmica (molde + cargas)", "~55", "un", "nós e hastes"],
        ["5", "Barra de terra Cu 50 x 5 x 500 mm", "1", "un", "na cabine"],
        ["6", "Cordoalha Cu 50 mm² até a barra", "~20", "m", "2 pontos"],
        ["9", "Cabo captor Cu 35 mm² + terminais", "a def.", "m", "SPDA NBR 5419"],
    ], alt=5.6, h=2.0)
    # legenda da planta
    p.tabela(dx + 140, 320, [8, 130, 22], [
        ["Nº", "LEGENDA DA PLANTA", "REF."],
        ["1", "Malha Cu nu 50 mm² a 0,60 m, reticulado 2 x 2 m", "DET. B"],
        ["2", "Haste cobreada 5/8 pol x 2,40 m (círculo com cruz)", "DET. A"],
        ["3", "Caixa de inspeção Ø300 (CI-1, CI-2)", "DET. A"],
        ["4", "Nó com solda exotérmica (ponto)", "DET. B"],
        ["5", "Ligação à barra de terra da cabine (BT-1, BT-2)", "DET. C"],
        ["6", "Poste com pára-raios: descida ligada à malha", "-"],
        ["7", "Cerca e portão ligados à malha (2 pontos)", "-"],
        ["8", "Ramal MT subterrâneo e caixa de passagem (ref.)", "SE-01"],
        ["10", "Limite do terreno; Rg-alvo <= 10 ohm (a confirmar)", "-"],
    ], alt=5.2, h=1.9)
    p.T(dx + 140, 264, 2.0, "SPDA da cobertura: risco e classe conforme NBR 5419 (avaliação pelo especialista).", DIM)
    notas(p, ["Malha única para MT, BT, carcaça do trafo, cerca e SPDA.", "Rg a medir em campo (queda de potencial) antes de energizar.", "Resistividade do solo: 4 pontos (Wenner) para o projeto final.",
              "Conexões enterradas só por solda exotérmica; acessos só nas CIs."],
          ["Verde = malha, hastes, ligações", "Vermelho = poste / SPDA / ramal MT", "Preto = cabine, CI e cerca", "Ponto = solda exotérmica"])


# ====================================================================== 5. PROTEÇÃO E COMANDO
RTC = 30.0 / 5.0
AJ = {"I51": 15.0, "TMS": 0.10, "I50": 175.0, "T50": 0.10, "I51N": 6.0, "I50N": 40.0}


def t_ni(i, ip=AJ["I51"], tms=AJ["TMS"]):
    """Curva normalmente inversa (IEC 60255-151): t = TMS * 0,14 / ((I/Is)^0,02 - 1)."""
    return tms * 0.14 / ((i / ip) ** 0.02 - 1.0)


def p5(dw):
    p = Pg(dw, 4)
    p.titulo("5. PROTEÇÃO E COMANDO - TCs, RELÉ, CIRCUITO DE ABERTURA E COORDENOGRAMA")
    K = 1.5
    # ---------------- A. circuito de corrente
    p.camada("CIRCUITO-CA", INK)
    p.T(40, 545, 3.2, "A. CIRCUITO DE CORRENTE (secundário dos TCs de proteção 30-5 A, 10B)", TIT)
    ys = {"A": 505.0, "B": 480.0, "C": 455.0}
    for f, y in ys.items():
        p.T(38, y + 4, 3.0, f"FASE {f}", MT)
        p.L(40, y, 78, y, MT); v = p.sub(90, y, K); v.tc(0, 0, MT)
        p.L(102, y, 118, y, MT); p.T(42, y - 6, 2.3, "primário MT 13,8 kV", DIM)
        p.L(90, y - 4.5, 90, y - 14, INK)
        p.T(96, y - 9, 2.4, f"S1 - S2", DIM)
    p.R(160, 428, 110, 96, INK)
    p.T(164, 516, 3.0, "RELÉ DIGITAL 50/51 - 50N/51N", INK)
    for i, (f, y) in enumerate(ys.items()):
        p.L(90, y - 14, 90, 445 - i * 0, INK) if False else None
        p.L(90 + 4.5 * K, y, 128, y, INK)
        p.L(128, y, 160, y, INK)
        p.T(164, y - 1.2, 2.6, f"{i + 1}  I{f}", INK)
    # retorno comum (residual) e terra único
    p.L(90, ys["A"] - 14, 90, 442, INK); p.L(90, 442, 150, 442, INK); p.L(150, 442, 150, 436, INK); p.L(150, 436, 160, 436, INK)
    p.L(90, ys["B"] - 14, 90, 442, INK); p.L(90, ys["C"] - 14, 90, 442, INK)
    p.T(164, 435, 2.6, "4  IN (residual)", INK)
    p.terra(120, 442); p.T(126, 430, 2.3, "terra único do secundário", TERRA)
    p.L(120, 442, 120, 442, TERRA)
    for i, t in enumerate(("Relação 30-5 A (RTC = 6)", "Classe 10B5 - 5 VA", "Saída: TRIP (NA) + alarme", "Alimentação: 125 Vcc")):
        p.T(200, 494 - i * 8, 2.6, t, DIM)
    # ---------------- B. circuito de comando CC
    p.camada("COMANDO", INK)
    p.T(300, 545, 3.2, "B. CIRCUITO DE COMANDO (125 Vcc)", TIT)
    xp, xn = 312, 460
    p.L(xp, 535, xp, 400, MT); p.L(xn, 535, xn, 400, BT)
    p.T(xp - 3, 538, 3.0, "+", MT); p.T(xn - 1, 538, 3.0, "-", BT)

    def comp(x, y, w, h, nome, sub_=None):
        p.R(x, y, w, h, INK); p.T(x + 1.4, y + h / 2 - 0.4 + (1.6 if sub_ else 0), 2.5, nome, INK)
        if sub_:
            p.T(x + 1.4, y + h / 2 - 3.2, 2.1, sub_, DIM)

    # rung 1..3: disparo (contatos em paralelo) -> 52a -> Y1
    yy = 520
    p.L(xp, yy, 326, yy, MT); comp(326, yy - 5, 14, 10, "Q1", "6 A")
    p.L(340, yy, 350, yy, MT)
    for k, (nome, sub_) in enumerate((("K50/51", "trip NA"), ("26/49", "temp. trip"), ("PB1", "emerg."))):
        yk = yy - k * 16
        p.L(350, yk, 358, yk, INK); comp(358, yk - 5, 34, 10, nome, sub_); p.L(392, yk, 400, yk, INK)
    p.L(350, yy, 350, yy - 32, INK); p.L(400, yy, 400, yy - 32, INK)
    p.L(400, yy, 410, yy, INK); comp(410, yy - 5, 18, 10, "52a", "aux."); p.L(428, yy, 434, yy, INK)
    p.C(442, yy, 8, INK); p.T(438.4, yy - 1, 2.6, "Y1", INK); p.L(450, yy, xn, yy, INK)
    p.T(410, yy + 8, 2.3, "abertura (bobina Y1)", DIM)
    # rung: fechamento
    yf = 470
    p.L(xp, yf, 326, yf, MT); comp(326, yf - 5, 14, 10, "Q2", "6 A")
    p.L(340, yf, 350, yf, INK); comp(350, yf - 5, 24, 10, "S1", "liga"); p.L(374, yf, 380, yf, INK)
    comp(380, yf - 5, 24, 10, "89", "secc. aberta?"); p.L(404, yf, 410, yf, INK); comp(410, yf - 5, 18, 10, "52b", "aux."); p.L(428, yf, 434, yf, INK)
    p.C(442, yf, 8, INK); p.T(438.4, yf - 1, 2.6, "Y2", INK); p.L(450, yf, xn, yf, INK)
    p.T(410, yf + 8, 2.3, "fechamento (bobina Y2)", DIM)
    # sinalização
    ysg = 435
    for k, (nome, aux) in enumerate((("H1", "52a - DISJ. FECHADO"), ("H2", "52b - DISJ. ABERTO"), ("H3", "86 - BLOQUEADO"))):
        xx = 326 + k * 46
        p.L(xp, ysg, xx, ysg, MT) if k == 0 else None
        p.L(xx, ysg + 5, xx, 420, INK) if False else None
        p.C(xx + 9, ysg - 12, 6, INK); p.L(xx + 5, ysg - 16, xx + 13, ysg - 8, INK); p.L(xx + 5, ysg - 8, xx + 13, ysg - 16, INK)
        p.T(xx, ysg - 24, 2.2, nome, INK); p.T(xx - 2, ysg - 27.5, 1.9, aux, DIM)
    p.L(xp, ysg, 326 + 2 * 46 + 9, ysg, MT); p.L(326 + 9, ysg, 326 + 9, ysg - 6, MT); p.L(326 + 55, ysg, 326 + 55, ysg - 6, MT); p.L(326 + 101, ysg, 326 + 101, ysg - 6, MT)
    p.L(326 + 9, ysg - 18, 326 + 9, 405, BT); p.L(326 + 55, ysg - 18, 326 + 55, 405, BT); p.L(326 + 101, ysg - 18, 326 + 101, 405, BT)
    p.L(326 + 9, 405, xn, 405, BT)
    p.T(316, 396, 2.3, "Q1/Q2: disjuntores CC 2P 6 A; polaridade e fusíveis conforme fabricante.", DIM)
    # ---------------- C. ajustes
    p.camada("AJUSTES", INK)
    p.T(40, 390, 3.2, "C. AJUSTES INICIAIS SUGERIDOS (validar no estudo de coordenação)", TIT)
    I_ = I_MT
    p.tabela(40, 384, [46, 16, 38, 34, 44, 34, 58], [
        ["FUNÇÃO", "ANSI", "PARTIDA (prim.)", "PARTIDA (sec.)", "CURVA / TMS", "TEMPO", "OBSERVAÇÃO"],
        ["Sobrecorrente de fase", "51", f"{AJ['I51']:.0f} A (1,2 In)", f"{AJ['I51'] / RTC:.1f} A", "NI, TMS 0,10", "curva", "coord. com elo/religador"],
        ["Instantâneo de fase", "50", f"{AJ['I50']:.0f} A (14 In)", f"{AJ['I50'] / RTC:.1f} A", "tempo definido", "0,10 s", "acima do inrush (12 In)"],
        ["Sobrecorrente de neutro", "51N", f"{AJ['I51N']:.0f} A", f"{AJ['I51N'] / RTC:.1f} A", "NI, TMS 0,10", "curva", "sensib. a falta fase-terra"],
        ["Instantâneo de neutro", "50N", f"{AJ['I50N']:.0f} A", f"{AJ['I50N'] / RTC:.1f} A", "tempo definido", "0,10 s", "abaixo da Icc FT mínima"],
        ["Temperatura do óleo", "26/49", "conf. fabricante", "-", "-", "alarme/trip", "2 contatos secos"],
    ], h=2.4, alt=7.4)
    # ---------------- D. dados base
    p.camada("DADOS", INK)
    p.T(40, 328, 3.2, "D. DADOS BASE E VERIFICAÇÕES", TIT)
    p.linhas(40, 320, [
        f"In MT = 300 / (raiz3 x 13,8) = {I_:.1f} A",
        f"Partida 51: 1,2 x In = {1.2 * I_:.1f} A -> ajuste {AJ['I51']:.0f} A ; secundário {AJ['I51'] / RTC:.2f} A (RTC 6)",
        f"Inrush do trafo: 8 a 12 x In = {8 * I_:.0f} a {12 * I_:.0f} A por ~0,1 s -> instantâneo em 14 In = {AJ['I50']:.0f} A",
        f"Corrente de dano (ANSI): In / Z = {I_ / 0.045:.0f} A por 2 s ; t = 2 x ({I_ / 0.045:.0f}/I)² acima disso",
        f"A 279 A o instantâneo (50) atua em {AJ['T50']:.2f} s (< 2 s): protege o trafo (ver F)",
        "Icc MT (3F e FT): carta de informações da concessionária - A OBTER",
    ], 2.6, 6.0)
    p.T(40, 278, 3.2, "E. INTERTRAVAMENTOS E ENSAIOS", TIT)
    p.linhas(40, 270, [
        "1. Seccionadora só opera com o disjuntor MT aberto (intertravamento mecânico).",
        "2. Porta do cubículo só abre com a seccionadora aberta e a lâmina de terra fechada.",
        "3. Lâmina de terra só fecha com a seccionadora aberta (e ausência de tensão).",
        "4. Chave de intertravamento entre porta da cabine e seccionadora (tipo Kirk).",
        "Ensaios: injeção secundária no relé, relação e polaridade dos TCs, isolação,",
        "tempo de abertura do disjuntor e teste do circuito de trip antes de energizar.",
    ], 2.6, 6.0)
    p.camada("LOGICA", INK)
    p.T(40, 226, 3.2, "G. LÓGICA DE ATUAÇÃO", TIT)
    p.tabela(40, 220, [70, 60, 84, 70], [
        ["EVENTO", "DETECÇÃO", "AÇÃO", "SINALIZAÇÃO"],
        ["Sobrecorrente de fase / neutro", "relé 50/51 - 50N/51N", "trip do disjuntor MT + bloqueio 86", "H3 bloqueado + relé"],
        ["Temperatura do óleo (alarme)", "26/49 - 90 °C (conf. fab.)", "apenas alarme", "buzina / sinaleiro"],
        ["Temperatura do óleo (trip)", "26/49 - 100 °C (conf. fab.)", "trip do disjuntor MT", "H3 + alarme"],
        ["Botão de emergência", "PB1 (porta da cabine)", "trip imediato", "H3 bloqueado"],
        ["Falta da fonte 125 Vcc", "supervisão da fonte", "alarme (sem trip)", "relé de falha"],
    ], h=2.4, alt=7.0)
    p.T(40, 168, 2.4, "Reset do bloqueio 86 somente localmente, após verificar a causa e liberar a intertravagem.", DIM)
    # ---------------- F. coordenograma
    p.camada("COORDENOGRAMA", INK)
    x0, y0_, dw_, dh = 570.0, 200.0, 72.0, 76.0                 # origem (10 A, 0,01 s), largura de década e altura de década
    X = lambda i: x0 + (math.log10(i) - 1) * dw_
    Y = lambda t: y0_ + (math.log10(t) + 2) * dh
    p.T(x0 - 32, 545, 3.2, "F. COORDENOGRAMA (esquemático, lado MT 13,8 kV)", TIT)
    for e in range(1, 5):
        p.L(X(10 ** e), Y(0.01), X(10 ** e), Y(100), DIM)
        p.T(X(10 ** e) - 3, Y(0.01) - 5, 2.3, f"{10 ** e:g}", DIM)
        for m in range(2, 10):
            if e < 4:
                p.L(X(m * 10 ** e), Y(0.01), X(m * 10 ** e), Y(100), "#D8D8D8")
    for e in range(-2, 3):
        p.L(X(10), Y(10.0 ** e), X(10000), Y(10.0 ** e), DIM)
        p.T(x0 - 14, Y(10.0 ** e) - 1, 2.3, f"{10.0 ** e:g}", DIM)
        for m in range(2, 10):
            if e < 2:
                p.L(X(10), Y(m * 10.0 ** e), X(10000), Y(m * 10.0 ** e), "#D8D8D8")
    p.T(X(10) + 60, Y(0.01) - 11, 2.5, "CORRENTE (A) no lado 13,8 kV", INK)
    p.T(x0 - 22, Y(100) + 4, 2.5, "TEMPO (s)", INK)
    # curva do relé: 51 (NI) até o instantâneo, depois t definido
    pts = []
    i = AJ["I51"] * 1.02
    while i < AJ["I50"]:
        t = t_ni(i)
        if 0.01 <= t <= 100:
            pts.append((X(i), Y(t)))
        i *= 1.06
    pts += [(X(AJ["I50"]), Y(t_ni(AJ["I50"] * 0.999))), (X(AJ["I50"]), Y(AJ["T50"])), (X(10000), Y(AJ["T50"]))]
    p.P(pts, False, MT)
    p.T(X(20), Y(t_ni(20)) + 3, 2.4, "RELÉ 51/50 (TMS 0,10)", MT)
    # curva de dano do trafo (ANSI): 2 s em In/Z, t = 2 (Id/I)^2
    Id = I_ / 0.045
    dano = []
    i = Id
    while True:
        t = 2.0 * (Id / i) ** 2
        if t < 0.01:
            break
        dano.append((X(i), Y(t)))
        i *= 1.06
    dano = [(X(Id * 0.72), Y(2.0)), (X(Id), Y(2.0))] + dano
    p.P(dano, False, BT)
    p.T(X(Id) + 2, Y(2.0) + 3, 2.4, "DANO DO TRAFO (ANSI)", BT)
    # inrush e In
    p.C(X(12 * I_), Y(0.1), 2.0, TIT); p.T(X(12 * I_) - 30, Y(0.1) - 9, 2.3, "inrush 12 In / 0,1 s", TIT)
    p.tracejada(X(I_), Y(0.01), X(I_), Y(100), TERRA, 3, 2); p.T(X(I_) - 6, Y(100) + 3, 2.3, "In = 12,6 A", TERRA)
    p.tracejada(X(4000), Y(0.01), X(4000), Y(100), MT, 2, 3); p.T(X(4000) - 22, Y(30), 2.2, "Icc MT (a obter)", MT, 90)
    p.T(x0 - 32, y0_ - 26, 2.3, "Curva do elo fusível da concessionária a sobrepor (não disponível).", DIM)
    p.T(x0 - 32, y0_ - 30, 2.3, "Curva NI: t = TMS x 0,14 / ((I/Is)^0,02 - 1), Is = 15 A.", DIM)
    notas(p, ["Ajustes são iniciais, para o especialista revisar.", "Coordenar com a proteção a montante da concessionária.", "Relé com autoalimentação ou fonte CC com baterias.",
              "Testar o circuito de trip antes de energizar."],
          ["Vermelho = MT / polo +", "Azul = BT / polo - / dano do trafo", "Verde = terra / In", "52a/52b = contatos auxiliares do disjuntor"])


# ====================================================================== 6. MEMORIAL
def p6(dw):
    p = Pg(dw, 5)
    p.titulo("6. MEMORIAL, VERIFICAÇÕES, LISTA DE MATERIAIS E PENDÊNCIAS PARA O ESPECIALISTA")
    p.camada("MEMORIAL", INK)
    p.T(40, 545, 3.2, "MEMORIAL DE CÁLCULO (anteprojeto)", TIT)
    perdas = 4.6
    vaz = perdas * 1000 / (1.2 * 1005 * 15) * 3600
    p.linhas(40, 536, [
        "Potência do transformador: 300 kVA (dado do cliente).",
        f"In MT = 300 / (raiz3 x 13,8) = {I_MT:.1f} A ; In BT = 300 / (raiz3 x 0,38) = {I_BT:.0f} A.",
        f"Icc BT no secundário (barramento infinito) = In / Z = {I_BT:.0f} / 0,045 = {ICC_TRAFO / 1000:.1f} kA.",
        "Disjuntor geral BT: 500 A regulável (In BT = 456 A), Icu >= 25 kA.",
        "Barramento QGBT: Cu 50 x 5 mm (250 mm²), 630 A.",
        "Cabo MT do ramal: 3 x 1 x 50 mm² EPR 8,7/15 kV (mínimo da concessionária).",
        "Curto-circuito no cabo: S >= Icc x raiz(t) / k ; k = 143 (Cu/EPR):",
        "  50 mm² suporta 143 x 50 / raiz(0,5) = 10,1 kA por 0,5 s.",
        "Ventilação (estimativa): perdas do trafo ~ 4,6 kW ; ΔT = 15 K:",
        f"  vazão = 4600 / (1,2 x 1005 x 15) = 0,25 m³/s = {vaz:.0f} m³/h.",
        "  área livre de entrada ≈ 0,188 x P / raiz(H) = 0,188 x 4,6 / raiz(2,5) = 0,55 m².",
        "Óleo do trafo (300 kVA) ≈ 370 L: bacia 2,2 x 1,7 x 0,40 m = 1,5 m³.",
        "  volume livre com brita (~40 %) ≈ 0,6 m³ >= 0,37 m³ (OK, a confirmar).",
        "Demanda da instalação: NÃO informada - 300 kVA adotado por decisão do cliente.",
    ], 2.7, 6.3)
    p.camada("VERIFICACOES", INK)
    p.T(40, 442, 3.2, "RESUMO DAS VERIFICAÇÕES", TIT)
    ver = [["Nº", "VERIFICAÇÃO", "CÁLCULO", "CRITÉRIO", "STATUS"],
           ["1", "Corrente nominal MT x seccionadora", "12,6 A", "<= 400 A", "OK"],
           ["2", "Disjuntor geral BT x In BT", "456 A", "<= 500 A", "OK"],
           ["3", "Poder de interrupção BT x Icc", "10,1 kA", "<= Icu 25 kA", "OK"],
           ["4", "Barramento QGBT x In BT", "456 A", "<= 630 A", "OK"],
           ["5", "Cabo MT: capacidade térmica", "12,6 A", "<< ~200 A", "OK"],
           ["6", "Cabo MT: curto-circuito", "Icc MT ?", "<= 10 kA / 0,5 s", "A OBTER"],
           ["7", "Ventilação V1 (entrada de ar)", "0,40 m² brutos", ">= 0,55 m² livres", "REVER"],
           ["8", "Bacia de contenção x óleo", "0,6 m³ livres", ">= 0,37 m³", "OK"],
           ["9", "Resistência de aterramento Rg", "medir", "<= 10 ohm", "A MEDIR"],
           ["10", "Coordenação e seletividade", "estudo", "com a concession.", "A FAZER"],
           ["11", "Afastamento lateral trafo-parede", "0,53 m", ">= 0,60 m (a conf.)", "REVER"]]
    p.tabela(40, 434, [9, 82, 32, 36, 24], ver, h=2.5, alt=6.4)
    for i, r in enumerate(ver[1:], start=1):
        cor = TERRA if r[4] == "OK" else MT
        p.T(40 + 9 + 82 + 32 + 36 + 1.2, 434 - (i + 1) * 6.4 + 2.2, 2.5, r[4], cor)
    p.camada("MATERIAIS", INK)
    p.T(300, 545, 3.2, "LISTA DE MATERIAIS (principais)", TIT)
    mat = [["ITEM", "DESCRIÇÃO", "QTD", "UN."],
           ["1", "Transformador 300 kVA, 13,8 kV / 380-220 V, Dyn1, óleo", "1", "un"],
           ["2", "Disjuntor MT a vácuo 15 kV 400 A 12,5 kA c/ bobina", "1", "un"],
           ["3", "Seccionadora tripolar 15 kV 400 A c/ terra", "1", "un"],
           ["4", "TC medição 15-5 A e TC proteção 30-5 A", "6", "un"],
           ["5", "TP medição 13800/115 V", "2", "un"],
           ["6", "Relé de proteção 50/51 50N/51N", "1", "un"],
           ["7", "Pára-raios ZnO 12 kV / 10 kA", "3", "un"],
           ["8", "Mufla terminal interna 15 kV", "3", "un"],
           ["9", "Cabo EPR 8,7/15 kV 50 mm²", "a def.", "m"],
           ["10", "Disjuntor geral BT 500 A", "1", "un"],
           ["11", "QGBT com barramento 630 A, DPS e multimedidor", "1", "un"],
           ["12", "Banco de capacitores automático 45 kvar", "1", "un"],
           ["13", "Fonte auxiliar 125 Vcc (retificador + baterias)", "1", "un"],
           ["14", "Cabo de cobre nu 50 mm² (malha)", "~145", "m"],
           ["15", "Haste cobreada 5/8 pol x 2,40 m", "8", "un"],
           ["16", "Caixa de inspeção Ø300", "2", "un"],
           ["17", "Bacia de contenção + brita + grade", "1", "cj"],
           ["18", "Venezianas com tela (V1, V2)", "2", "cj"]]
    p.tabela(300, 538, [10, 110, 16, 12], mat, h=2.5, alt=6.2)
    p.camada("PENDENCIAS", MT)
    p.T(560, 545, 3.2, "PENDÊNCIAS / ITENS PARA O ESPECIALISTA VALIDAR", MT)
    p.linhas(560, 536, [
        "1. Norma técnica e ligação da concessionária local.",
        "2. Icc trifásica e fase-terra no ponto de entrega.",
        "3. Estudo de coordenação e seletividade.",
        "4. Elo fusível, TCs e ajustes do relé (item 2).",
        "5. Demanda contratada e curva de carga (trafo).",
        "6. Resistividade do solo e projeto da malha.",
        "7. SPDA: análise de risco NBR 5419.",
        "8. V1 subdimensionada: refazer o cálculo de",
        "    ventilação (carga térmica e áreas de V1/V2).",
        "9. Bacia de óleo: volume e afastamentos.",
        "10. Ramal: entrada aérea ou subterrânea.",
        "11. ART do responsável técnico e aprovação.",
        "12. Usina solar (QD-4): parecer de acesso.",
        "13. Correção do fator de potência (banco).",
        "14. Tensões de toque e de passo na malha.",
    ], 2.7, 6.3)
    p.T(560, 428, 3.2, "LISTA DE DESENHOS", TIT)
    p.linhas(560, 420, ["SE-01 Unifilar geral", "SE-02 Planta baixa da cabine", "SE-03 Cortes AA e BB", "SE-04 Malha de aterramento e SPDA",
                        "SE-05 Proteção, comando e coordenograma", "SE-06 Memorial e pendências"], 2.7, 6.0)
    p.camada("DESCRICAO", INK)
    p.T(40, 292, 3.2, "DESCRIÇÃO DO SISTEMA", TIT)
    p.linhas(40, 284, [
        "Cabine de medição e transformação em alvenaria, ligada à rede de 13,8 kV por ramal subterrâneo.",
        "A entrada tem chave fusível e pára-raios no poste, cabos EPR 8,7/15 kV e mufla dentro da cabine.",
        "A medição é indireta (TCs e TPs), com medidor da concessionária. Depois vem a seccionadora",
        "com lâmina de terra, o disjuntor a vácuo com relé secundário (50/51, 50N/51N) e o transformador",
        "de 300 kVA (Dyn1), que alimenta o QGBT a 380/220 V. O QGBT tem disjuntor geral de 500 A,",
        "barramento de 630 A, alimentadores, banco de capacitores, DPS e multimedidor.",
        "A malha de aterramento única serve MT, BT, cerca, poste e o SPDA da cobertura.",
    ], 2.7, 6.3)
    p.T(300, 400, 3.2, "NORMAS E DOCUMENTOS DE REFERÊNCIA", TIT)
    p.tabela(300, 393, [40, 148], [
        ["NORMA", "ASSUNTO"],
        ["NBR 14039", "Instalações elétricas de média tensão (1 a 36,2 kV)"],
        ["NBR 5410", "Instalações elétricas de baixa tensão"],
        ["NBR 5419", "Proteção contra descargas atmosféricas (SPDA)"],
        ["NBR 5356", "Transformadores de potência"],
        ["IEC 62271-200", "Cubículos de manobra e controle MT (blindados)"],
        ["NR-10", "Segurança em instalações e serviços em eletricidade"],
        ["Norma da concession.", "Ligação de consumidores em MT: A OBTER"],
    ], h=2.5, alt=6.2)
    p.T(560, 320, 3.2, "PERGUNTAS AO ESPECIALISTA", MT)
    p.linhas(560, 312, [
        "1. O instantâneo em 14 In (175 A, 0,10 s) coordena com",
        "    o elo da concessionária e com o inrush do trafo?",
        "2. Basta ventilação natural, ou é preciso exaustor?",
        "3. TC de proteção 30-5 A atende (saturação e carga)?",
        "4. Malha 12 x 9 m com Rg <= 10 ohm: solo permite?",
        "5. A usina FV em QD-4 exige proteção de interligação",
        "    própria (27/59/81 e anti-ilhamento) no relé MT?",
        "6. Faltam elementos do projeto (Buchholz, tela, etc.)?",
    ], 2.7, 6.0)
    p.T(40, 150, 2.5, "TODOS OS VALORES DE ANTEPROJETO ESTÃO SUJEITOS À REVISÃO POR PROFISSIONAL HABILITADO (CREA).", DIM)
    notas(p, ["Documento de anteprojeto para avaliação técnica.", "Nenhum valor substitui estudo, ART ou aprovação.", "Fonte dos cálculos: dados de placa/nominais.", "Sem telemetria: arquivos locais."],
          ["Vermelho = pendência / REVER", "Verde = OK", "Dourado = títulos", "SE-xx = número do desenho"])


def carimbo(dw, i, num, tit, sub):
    """Moldura e carimbo da prancha anterior (`solar_base.fct`, camadas FOLHA e CARIMBO), deslocados para a posição i."""
    dx = i * OFF
    trocas = {'"A DEFINIR"': '"CABINE DE MEDIÇÃO E TRANSFORMAÇÃO - CONSUMIDOR"', '"ESTUDO PRELIMINAR"': '"ANTEPROJETO PARA AVALIAÇÃO"',
              '"ELÉTRICA - ENERGIA SOLAR"': '"ELÉTRICA - SUBESTAÇÃO 13,8 kV / 300 kVA"', '"SISTEMA FOTOVOLTAICO ON-GRID 4,40 kWp"': f'"{tit}"',
              '"UNIFILAR, PERSPECTIVA E CÁLCULOS"': f'"{sub}"', '"FV-01"': f'"{num}"', '"01"': f'"{i + 1:02d}/06"', '"1:1"': '"INDICADA"',
              '"EMISSÃO INICIAL - ESTUDO PRELIMINAR"': '"EMISSÃO INICIAL - ANTEPROJETO"', '"30/09/2026"': '"29/09/2026"'}
    camada = None
    for lin in open("solar_base.fct", encoding="utf-8").read().splitlines()[1:]:
        if lin.startswith("camada "):
            camada = lin.split()[1]
            if camada in ("FOLHA", "CARIMBO"):
                dw.out.append(f"camada {camada} #FFFFFF")
            continue
        if camada not in ("FOLHA", "CARIMBO"):
            continue
        for a, b in trocas.items():
            if a in lin:
                lin = lin.replace(a, b)
        t = lin.split(" ", 3)
        tipo = t[0]
        if tipo in ("L", "R", "C", "T"):
            x = float(t[1]) + dx
            resto = lin.split(" ", 2)[2]
            if tipo == "L":
                q = resto.split(" ")
                q[1] = f"{float(q[1]) + dx:.3f}"
                resto = " ".join(q)
            lin = f"{tipo} {x:.3f} {resto}"
        dw.out.append(lin)


def montar():
    dw = Desenho()
    for i, f in enumerate((p1, p2, p3, p4, p5, p6)):
        f(dw)
        carimbo(dw, i, *PAGINAS[i])
    return dw.texto()


PAGINAS = [
    ("SE-01", "UNIFILAR GERAL", "MT 13,8 kV / BT 380-220 V - 300 kVA"),
    ("SE-02", "PLANTA BAIXA DA CABINE", "ESCALA 1:20"),
    ("SE-03", "CORTES AA E BB", "ESCALA 1:20"),
    ("SE-04", "ATERRAMENTO E SPDA", "MALHA - ESCALA 1:50"),
    ("SE-05", "PROTEÇÃO E COMANDO", "COORDENOGRAMA E AJUSTES"),
    ("SE-06", "MEMORIAL E PENDÊNCIAS", "PARA AVALIAÇÃO DO ESPECIALISTA"),
]

if __name__ == "__main__":
    pasta = sys.argv[1] if len(sys.argv) > 1 else "/home/maragon/Documentos/FeatherCAD"
    texto = montar()
    open(f"{pasta}/subestacao.fct", "w", encoding="utf-8").write(texto)
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": texto})
    for ext in ("fcdz", "dxf"):
        m.call("cad_salvar", {"caminho": f"{pasta}/subestacao.{ext}", "sobrescrever": True})
    m.call("cad_pdf", {"caminho": f"{pasta}/subestacao.pdf", "papel": "FOLHA", "cores": True, "sobrescrever": True})
    r = m.call("cad_resumo")
    print(r["total_de_entidades"], r["por_tipo"])
    m.fechar()

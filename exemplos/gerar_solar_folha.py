#!/usr/bin/env python3
"""Prancha A1 completa do sistema fotovoltaico da casa do print: (1) perspectiva 3D, (2) planta com o norte para cima — feitas por
gerar_solar.py — e, aproveitados de `solar_base.fct` (a prancha anterior): (3) dados e cálculos, (4) unifilar geral, (5) diagrama por MPPT,
(6) inversor + carga até o medidor, notas, legenda e carimbo. Papel em mm, escala 1:1 (o 3D e a planta usam 1 m = 3,6 mm, esquemático).

Sistema: 8 módulos de 550 Wp (2 fileiras x 4, uma string por MPPT) = 4,40 kWp, inversor 5 kW / 220 V."""
import math, os, sys
import gerar_solar as g
from mcp_cliente import Mcp

AQUI = os.path.dirname(os.path.abspath(__file__))
MANTER = {"FOLHA", "QUADROS", "TITULOS", "CARIMBO", "CALCULOS", "MPPT-TEXTO", "INV-CARGA", "NOTAS", "LEGENDA"}
TROCAS = {
    '"1. PERSPECTIVA DO TELHADO COM OS MÓDULOS"': '"1. PERSPECTIVA 3D DA CASA COM OS MÓDULOS"',
    '"2. CORTE DO TELHADO - INCLINAÇÃO"': '"2. PLANTA DE COBERTURA (NORTE PARA CIMA)"',
    '"4,40 x 5,0 x 0,80 = 17,6 kWh/dia = 528 kWh/mês (>= 500 kWh/mês)"': '"4,40 x 5,0 x 0,80 x 0,95 = 16,7 kWh/dia = 502 kWh/mês (>= 500 kWh/mês)"',
    '"35 % -> θ = arctg 0,35 = 19,3° | face Norte | lat. 23°S"': '"30 % -> θ = arctg 0,30 = 16,7° | face NO (azimute ≈ 319°) | latitude a confirmar"',
    '"Perda por inclinação em relação à ótima (23°) < 1 %"': '"Desvio azimutal de ≈ 41° do norte: perda estimada 5 % (fator 0,95 acima)"',
    '"θ = inclinação do telhado (35 % = 19,3°)"': '"θ = inclinação do telhado (30 % = 16,7°)"',
    '"UNIFILAR, PERSPECTIVA E CÁLCULOS"': '"PERSPECTIVA, PLANTA, UNIFILAR E CÁLCULOS"',
    '"1:1"': '"S/ ESC."',
}
# cores pensadas para a tela escura viram tons legíveis no papel branco (o PDF só troca o branco por preto)
PAPEL = {"#E0E0E0": "#FFFFFF", "#FFE04A": "#A87400", "#FFD84A": "#A87400", "#F5C400": "#C79A00", "#FFB84A": "#C77700", "#7AE07A": "#2E8B57",
         "#7AB8FF": "#1F6FD0", "#4AA3FF": "#1F6FD0", "#FF7A7A": "#D02020", "#FF6A6A": "#D02020", "#9AA0A6": "#606060", "#B0B0B0": "#606060"}
K = 3.6                                   # mm de papel por metro na perspectiva
K2 = 3.6                                  # e na planta
QUADRO1 = (98.0, 367.4, 229.6, 208.28)    # x, y, largura, altura (de solar_base.fct)
QUADRO2 = (327.6, 367.4, 213.2, 208.28)


def base():
    saida, camada = [], None
    for lin in open(f"{AQUI}/solar_base.fct", encoding="utf-8").read().splitlines():
        if lin.startswith("unidade"):
            continue
        if lin.startswith("camada "):
            camada = lin.split()[1]
            if camada in MANTER or camada.startswith("UF-"):
                saida.append(lin)
            continue
        if camada in MANTER or (camada or "").startswith("UF-"):
            for a, b in TROCAS.items():
                if a in lin:
                    lin = lin.replace(a, b)
            saida.append(lin)
    return saida


def legenda_3d(x, y):
    lg = ["1  Arranjo FV: 8 módulos de 550 Wp = 4,40 kWp (2 x 4)",
          "2  Strings CC: fileira de cima = string 1 (MPPT 1), de baixo = string 2 (MPPT 2)",
          "3  Caixa de proteção CC (seccionadora + DPS)",
          "4  Inversor on-grid 5 kW, 2 MPPT, 220 V",
          "5  QD da casa (disjuntor 32 A + DR + DPS)",
          "6  Medidor bidirecional",
          "7  Poste / rede da concessionária (Av. João Motta)",
          "8  Aterramento: haste + cabo verde/amarelo"]
    g.I.camada("LEGENDA-3D", g.BR)
    for i, t in enumerate(lg):
        g.I.texto(x, y - i * 3.2, 2.0, t, 0, g.CI)


def montar():
    # ---- 1. perspectiva (centro do isométrico em m: x 11,6, y -1)
    g.reiniciar(K, QUADRO1[0] + QUADRO1[2] / 2 - 11.6 * K, 487.0 + 1.0 * K)
    g.perspectiva()
    legenda_3d(QUADRO1[0] + 3, QUADRO1[1] + 33)
    p3d = list(g.I.out[1:])
    # ---- 2. planta: centro pelo retângulo que envolve casas e rua
    th = math.radians(g.ANG)
    nw, sw = (-math.sin(th), math.cos(th)), (-math.cos(th), -math.sin(th))
    q0 = lambda e1, e2: ((e1 * nw[0] + e2 * sw[0]) * K2, (e1 * nw[1] + e2 * sw[1]) * K2)
    pts = [q0(g.LARG + g.DIST_RUA, -1), q0(g.LARG + g.DIST_RUA, g.COMP + 1)] + [q0(a, b) for a in (0, g.LARG) for b in (0, g.COMP)] + [q0(a, b) for a in g.C2_E1 for b in g.C2_E2]
    cx = (min(p[0] for p in pts) + max(p[0] for p in pts)) / 2
    cy = (min(p[1] for p in pts) + max(p[1] for p in pts)) / 2
    g.reiniciar(K2, 0, 0)
    g.NORTE_PLANTA = (QUADRO2[0] + QUADRO2[2] - 14, QUADRO2[1] + 40)
    g.planta(QUADRO2[0] + QUADRO2[2] / 2 - cx, 470.0 - cy)
    pplanta = list(g.I.out[1:])
    texto = "\n".join(["unidade mm"] + base() + p3d + pplanta) + "\n"
    for a, b in PAPEL.items():
        texto = texto.replace(a, b)
    return texto


if __name__ == "__main__":
    pasta = sys.argv[1] if len(sys.argv) > 1 else "/home/maragon/Documentos/FeatherCAD"
    texto = montar()
    open(f"{pasta}/solar_prancha.fct", "w", encoding="utf-8").write(texto)
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": texto})
    for ext in ("fcdz", "dxf"):
        m.call("cad_salvar", {"caminho": f"{pasta}/solar_prancha.{ext}", "sobrescrever": True})
    m.call("cad_pdf", {"caminho": f"{pasta}/solar_prancha.pdf", "papel": "A1", "cores": True, "sobrescrever": True})
    r = m.call("cad_resumo")
    print(r["total_de_entidades"], r["por_tipo"])
    m.fechar()

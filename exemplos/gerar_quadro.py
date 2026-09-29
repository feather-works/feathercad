#!/usr/bin/env python3
"""Quadro de distribuição em vista frontal (tamanho real, mm) e a folha 'DETALHE DISJUNTORES', feitos com o pack_eletrica."""
import sys
from mcp_cliente import Mcp
import pack_eletrica as pk
from pack_eletrica import Desenho, MODULO, ALTURA, FASE, NEUTRO, TERRA, PRETO

def quadro():
    d = Desenho()
    W, H = 340.0, 340.0
    pk.caixa(d, 0, 0, W, H, glandulas=(105, 155, 205, 255))
    # trilhos DIN (duas fileiras)
    yA, yB = 200.0, 75.0
    pk.trilho_din(d, 30, yA, 280)
    pk.trilho_din(d, 30, yB, 280)
    topo_y = yA + 17.5 - ALTURA / 2       # dispositivos centrados no trilho
    base_y = yB + 17.5 - ALTURA / 2
    # fileira de cima: barra de neutro, disjuntor bipolar, DR de 4 polos, 3 DPS, barra de terra
    pk.barra_de_bornes(d, 28, topo_y - 6, 102, NEUTRO)
    pk.disjuntor(d, 62, topo_y, 2, "C25", "6kA")
    pk.dr(d, 125, topo_y, 4)
    for i in range(3):
        pk.dps(d, 205 + i * MODULO, topo_y)
    pk.barra_de_bornes(d, 296, topo_y - 6, 102, TERRA)
    # fileira de baixo: quatro bipolares e o pente
    for i in range(4):
        pk.disjuntor(d, 62 + i * 2 * MODULO, base_y, 2, ("C10", "C16", "C20", "C25")[i], "6kA")
    pk.pente(d, 62, base_y + ALTURA + 2, 8)
    # cabos
    d.camada("CABO_FASE", FASE)
    for x in (70, 87.5):
        d.cabo([(x, topo_y + ALTURA - 11), (x, 300)], FASE, 2.6)
    d.cabo([(80, base_y - 11 + 11), (80, 28)], FASE, 2.6) if False else None
    for i in range(4):
        cx = 62 + i * 2 * MODULO + MODULO / 2
        d.cabo([(cx, base_y + 11), (cx, 24)], FASE, 2.6)
        d.cabo([(cx + MODULO, base_y + 11), (cx + MODULO, 24)], NEUTRO, 2.6)
        d.cabo([(cx, base_y + ALTURA - 11), (cx, base_y + ALTURA + 8)], FASE, 2.6)
    d.camada("CABO_NEUTRO", NEUTRO)
    d.cabo([(36, topo_y + 96), (36, 300)], NEUTRO, 2.6)
    d.cabo([(36, topo_y + 10), (36, 150), (130, 150), (130, topo_y + 11)], NEUTRO, 2.6)
    d.camada("CABO_RETORNO", PRETO)
    d.cabo([(96, topo_y + 11), (96, 160), (142, 160), (142, topo_y + ALTURA - 11)], PRETO, 2.6)
    d.cabo([(147, topo_y + ALTURA - 11), (147, 290), (222, 290), (222, topo_y + ALTURA - 11)], PRETO, 2.6)
    d.camada("CABO_TERRA", TERRA)
    d.cabo([(304, topo_y + 10), (304, 168), (232, 168), (232, topo_y + 11)], TERRA, 2.6)
    d.cabo([(304, topo_y + 60), (304, 168)], TERRA, 2.6)
    return d.texto()

def montar(fcdz, dxf, texto):
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": texto})
    m.call("cad_salvar", {"caminho": fcdz, "sobrescrever": True})
    m.call("cad_salvar", {"caminho": dxf, "sobrescrever": True})
    r = m.call("cad_resumo"); m.fechar(); return r

if __name__ == "__main__":
    base = sys.argv[1] if len(sys.argv) > 1 else "/home/maragon/Documentos/FeatherCAD"
    det = Desenho(); pk.detalhe_disjuntores(det, 0, 0)
    r = montar(f"{base}/disjuntores_detalhe.fcdz", f"{base}/disjuntores_detalhe.dxf", det.texto())
    print("detalhe:", r["total_de_entidades"], r["por_tipo"])
    r = montar(f"{base}/quadro_distribuicao.fcdz", f"{base}/quadro_distribuicao.dxf", quadro())
    print("quadro:", r["total_de_entidades"], r["por_tipo"])

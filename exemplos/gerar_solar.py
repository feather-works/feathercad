#!/usr/bin/env python3
"""Sistema fotovoltaico residencial (on-grid) da casa do print do Google: perspectiva isométrica 3D, planta com o norte para
cima, diagrama unifilar e quadro de dados. Medidas tiradas do print (escala pela régua de 5,37 m); tudo é premissa editável.

Vista 3D: o observador está a noroeste (a água NO do telhado, a que recebe o sol do norte, aparece de frente).
Eixos do isométrico: E1 = noroeste (perpendicular ao cume), E2 = sudoeste (ao longo do cume), Z = altura. O norte, portanto,
aponta PARA BAIXO na perspectiva; a planta ao lado tem o norte para cima, como o print."""
import math, sys
from iso import Iso, n
from mcp_cliente import Mcp

M = 1000.0                       # mm de desenho por metro (a folha usa 3,6: 1 m = 3,6 mm de papel)
OX, OY = 0.0, 0.0                # origem do isométrico (mm)
NORTE_PLANTA = (0.0, 0.0)        # onde a seta do norte da planta fica (mm)
LARG, COMP = 8.0, 23.0           # casa: E1 (perpendicular ao cume) x E2 (ao longo do cume)
H_BEIRAL, H_CUME = 3.0, 4.2
DIST_RUA = 11.6                  # parede NO -> eixo da Av. João Motta, estimado do print (~ 0,06 m/pixel)
MEDIDA = 5.37                    # régua do print: largura da parte NE da casa 1 (e1 de 3,0 a 8,4, junto a e2 = -0,6)
# casa 2 (telhado laranja do print, a nordeste): retângulo em (e1, e2), cumeeira ao longo de E1
C2_E1, C2_E2 = (-3.7, 7.7), (-15.3, -6.0)
ANG = 40.7                       # ângulo do cume em relação ao leste (do print), graus
MOD_L, MOD_A = 2.278, 1.134      # módulo 550 W: comprimento (ao longo de E2) e largura (ao longo da água)
GAP, COLS, LINHAS = 0.03, 4, 2
E2_INI = 3.0
S0 = 0.9                         # distância do cume à 1ª fileira, medida na água
LW = math.hypot(LARG / 2, H_CUME - H_BEIRAL)

I = Iso(k=M, ox=OX, oy=OY)


def reiniciar(m, ox, oy):
    """Zera o desenho com outra escala/origem (a folha usa 3,6 mm/m)."""
    global M, OX, OY, I
    M, OX, OY = m, ox, oy
    I = Iso(k=M, ox=OX, oy=OY)
    I.out.append("unidade mm")
BR, AZ, VM, VD, AM, CI = "#FFFFFF", "#2F6BFF", "#FF2A2A", "#22B14C", "#F5C400", "#9AA0A6"


def agua(s, e2):
    """Ponto (e1,e2,z) na água NO do telhado, a `s` metros do cume medidos ao longo da água."""
    t = s / LW
    return (LARG / 2 + (LARG / 2) * t, e2, H_CUME - (H_CUME - H_BEIRAL) * t)


def P(*a):
    return I.p(*a)


def cabo(pts3, cor, camada):
    I.camada(camada, cor)
    I.poli([P(*q) for q in pts3], False, cor)


def rotulo(x, y, h, txt, cor=BR, giro=0):
    I.camada("TEXTO", BR)
    I.texto(x * M, y * M, h * 1.35 * M, txt, giro, cor)


def parede(e2a, e2b, za, zb, cor, contorno=True):
    I.face([(LARG, e2a, za), (LARG, e2b, za), (LARG, e2b, zb), (LARG, e2a, zb)], cor)


# ------------------------------------------------------------------ 3D
def perspectiva():
    # terreno (rua a NO: E1 cresce para a rua) — chão como um losango tênue
    ch = [(-5, -17, 0), (LARG + DIST_RUA + 1.5, -17, 0), (LARG + DIST_RUA + 1.5, COMP + 2, 0), (-5, COMP + 2, 0)]
    I.camada("TERRENO", "#3A4636")
    I.preencher([[P(*q) for q in ch]], "#2A3327")
    I.poli([P(*q) for q in ch], True, "#5A6B52")
    I.camada("RUA", "#6B6B6B")
    r0, r1 = LARG + DIST_RUA - 1.5, LARG + DIST_RUA + 1.5
    I.preencher([[P(r0, -17, 0), P(r1, -17, 0), P(r1, COMP + 2, 0), P(r0, COMP + 2, 0)]], "#4A4A4A")
    # casa: perfil (E1,z) anti-horário, extrudado em E2
    perfil = [(0, 0), (LARG, 0), (LARG, H_BEIRAL), (LARG / 2, H_CUME), (0, H_BEIRAL)]
    I.prisma(perfil, 0, COMP, "e1", topo="#C9683F", frente="#D9CDB4", esquerda="#B8AC94", tampa="#D9CDB4")
    # janelas e porta na parede NO
    for a, b in ((2.0, 3.6), (7.0, 8.6), (16.0, 17.6), (20.0, 21.6)):
        parede(a, b, 1.0, 2.1, "#7FB2D6")
    parede(11.0, 12.0, 0, 2.15, "#8A5A34")
    # casa 2 (nordeste): mesma altura de beiral, cumeeira ao longo de E1
    a2, b2 = C2_E2
    perfil2 = [(a2, 0), (b2, 0), (b2, H_BEIRAL), ((a2 + b2) / 2, H_CUME - 0.2), (a2, H_BEIRAL)]
    I.prisma(perfil2, C2_E1[0], C2_E1[1], "e2", topo="#C9683F", frente="#D9CDB4", esquerda="#B8AC94", tampa="#D9CDB4")
    for a, b in ((-15.0 + 0.0, -13.4), (-11.0, -9.4)):
        I.face([(C2_E1[1], a, 1.0), (C2_E1[1], b, 1.0), (C2_E1[1], b, 2.1), (C2_E1[1], a, 2.1)], "#7FB2D6")
    I.face([(C2_E1[1], -8.6, 0), (C2_E1[1], -7.6, 0), (C2_E1[1], -7.6, 2.15), (C2_E1[1], -8.6, 2.15)], "#8A5A34")
    # painéis
    passo_s = MOD_A + GAP
    for c in range(COLS):
        a = E2_INI + c * (MOD_L + GAP)
        b = a + MOD_L
        for r in range(LINHAS):
            s0 = S0 + r * passo_s
            s1 = s0 + MOD_A
            quad = [agua(s0, a), agua(s0, b), agua(s1, b), agua(s1, a)]
            I.face(quad, "#16265E" if r == 0 else "#1B3170")
            I.camada("CELULAS", CI)
            for j in range(1, 6):    # divisórias das células ao longo de E2
                e = a + MOD_L * j / 6
                I.linha(P(*agua(s0, e)), P(*agua(s1, e)), "#5A6FA8")
            I.linha(P(*agua((s0 + s1) / 2, a)), P(*agua((s0 + s1) / 2, b)), "#5A6FA8")
    # equipamentos na parede NO (E1 = LARG)
    E_STR, E_INV, E_QCA, E_MED = 14.6, 13.2, 9.4, 5.0
    parede(E_STR - 0.2, E_STR + 0.2, 2.15, 2.6, "#E0E0E0")       # string box
    parede(E_INV - 0.3, E_INV + 0.3, 1.05, 1.75, "#F0F0F0")      # inversor
    parede(E_QCA - 0.2, E_QCA + 0.2, 1.2, 1.7, "#B9C4CC")        # quadro CA
    parede(E_MED - 0.2, E_MED + 0.2, 1.3, 1.9, "#CFCFCF")        # medição
    # cabos CC (vermelho +, azul −) do fim de cada fileira até a string box
    for r, cor in ((0, VM), (1, AZ)):
        s_meio = S0 + r * passo_s + MOD_A / 2
        fim = E2_INI + COLS * (MOD_L + GAP)
        beiral = (LARG, fim + 0.5 + 0.1 * r, H_BEIRAL)
        cabo([agua(s_meio, fim - 0.3), agua(s_meio, fim + 0.5 + 0.1 * r), beiral,
              (LARG, fim + 0.5 + 0.1 * r, 2.6 - 0.1 * r), (LARG, E_STR - 0.1 + 0.2 * r, 2.6 - 0.1 * r)], cor, "CABO_CC")
    cabo([(LARG, E_STR, 2.15), (LARG, E_STR, 1.75), (LARG, E_INV + 0.3, 1.75)][:2] + [(LARG, E_INV + 0.3, 1.75)], AM, "CABO_CC")
    cabo([(LARG, E_INV - 0.3, 1.4), (LARG, E_QCA + 0.2, 1.4)], VM, "CABO_CA")
    cabo([(LARG, E_QCA - 0.2, 1.4), (LARG, E_MED + 0.2, 1.4)], VM, "CABO_CA")
    # aterramento: haste no chão junto ao inversor
    cabo([(LARG, E_INV, 1.05), (LARG, E_INV, 0.0), (LARG + 0.4, E_INV, 0.0), (LARG + 0.4, E_INV, -0.0)], VD, "TERRA")
    I.camada("TERRA", VD)
    I.poli([P(LARG + 0.4, E_INV, 0), P(LARG + 0.4, E_INV, -0.5)], False, VD)
    # ramal de entrada: medição -> poste na rua (aéreo)
    poste = (r0 - 0.6, E_MED, 0)
    I.camada("POSTE", "#B0906A")
    I.poli([P(*poste), P(poste[0], poste[1], 7.0)], False, "#B0906A")
    I.poli([P(poste[0], poste[1] - 0.6, 6.8), P(poste[0], poste[1] + 0.6, 6.8)], False, "#B0906A")
    seg = [(LARG, E_MED, 1.9), (LARG, E_MED, 3.5), (LARG + 2.5, E_MED, 4.9), (poste[0], poste[1], 6.8)]
    cabo(seg, VM, "RAMAL")
    # cota da rua: 5,37 m
    I.cota((3.0, -0.6, 0), (8.4, -0.6, 0), (0, -1.6, 0), "5,37 m", altura=0.35)
    I.cota((0, -0.3, 0), (LARG, -0.3, 0), (0, -1.3, 0), "8,0 m", altura=0.35)
    I.cota((0, -0.3, 0), (0, COMP, 0), (-1.5, 0, 0), "23,0 m", altura=0.35)
    # sol: acima e à direita, raios em direção ao arranjo
    sx, sy = 27.5 * M + OX, 14.5 * M + OY
    I.camada("SOL", AM)
    I.poli([(sx + 1.0 * M * math.cos(math.radians(a)), sy + 1.0 * M * math.sin(math.radians(a))) for a in range(0, 361, 10)], True, AM)
    for a in range(0, 360, 30):
        c, s = math.cos(math.radians(a)), math.sin(math.radians(a))
        I.linha((sx + 1.3 * M * c, sy + 1.3 * M * s), (sx + 1.9 * M * c, sy + 1.9 * M * s), AM)
    alvo = P(*agua(1.6, 9.0))
    for d in (-1.0, 0.0, 1.0):
        I.linha((sx - 1.2 * M, sy - 1.0 * M + d * 0.4 * M), (alvo[0] + d * 1.5 * M, alvo[1] + d * 0.25 * M), AM)
    rotulo(sx / M - 3.5, sy / M + 2.4, 0.4, "SOL (NORTE / NOROESTE)", AM)
    # balões numerados (legenda ao lado)
    def balao(num, p3, dx, dy, cor=BR):
        x, y = P(*p3)
        I.camada("BALOES", CI)
        I.linha((x, y), (x + dx * M, y + dy * M), CI)
        cx, cy = x + dx * M, y + dy * M
        I.poli([(cx + 0.38 * M * math.cos(math.radians(a)), cy + 0.38 * M * math.sin(math.radians(a))) for a in range(0, 361, 15)], True, cor)
        rotulo(cx / M - 0.13 * len(str(num)), cy / M - 0.17, 0.38, str(num), cor)
    balao(1, agua(S0 + 0.5, E2_INI + 1), -2.0, 2.2)
    balao(2, agua(S0 + 0.5 + passo_s, E2_INI + 2.6), 0.0, 2.6, VM)
    balao(3, (LARG, E_STR, 2.6), -0.6, 1.8, AM)
    balao(4, (LARG, E_INV, 1.05), -0.3, -1.3)
    balao(5, (LARG, E_QCA, 1.2), -0.3, -1.3)
    balao(6, (LARG, E_MED, 1.3), -0.3, -1.3)
    balao(7, (poste[0], poste[1], 6.8), 1.2, 0.8)
    balao(8, (LARG + 0.4, E_INV, -0.5), 1.0, -0.8, VD)
    # norte (aponta para baixo na tela)
    nx, ny = -5.5 * M + OX, 10.0 * M + OY
    I.camada("NORTE", BR)
    I.linha((nx, ny), (nx, ny - 2.5 * M), BR)
    I.poli([(nx - 0.3 * M, ny - 1.8 * M), (nx, ny - 2.5 * M), (nx + 0.3 * M, ny - 1.8 * M)], False, BR)
    rotulo(nx / M - 0.2, ny / M - 3.5, 0.6, "N")
    rotulo(nx / M - 1.5, ny / M + 0.4, 0.3, "NORTE (nesta vista)")


# ------------------------------------------------------------------ planta (norte para cima)
def planta(ox, oy):
    th = math.radians(ANG)
    nw = (-math.sin(th), math.cos(th))      # E1: noroeste
    sw = (-math.cos(th), -math.sin(th))     # E2: sudoeste
    def q(e1, e2, dx=0, dy=0):
        return (ox + (e1 * nw[0] + e2 * sw[0]) * M, oy + (e1 * nw[1] + e2 * sw[1]) * M)
    I.camada("PLANTA", BR)
    # rua: reta paralela ao cume a DIST_RUA da parede NO
    ra, rb = q(LARG + DIST_RUA, -1), q(LARG + DIST_RUA, COMP + 1)
    I.camada("RUA", "#6B6B6B")
    I.linha(ra, rb, "#8A8A8A")
    rm = q(LARG + DIST_RUA + 1.4, COMP * 0.12)
    rotulo(rm[0] / M, rm[1] / M, 0.55, "Av. João Motta", "#B0B0B0", ANG)
    # casa 2
    I.camada("CASA2", BR)
    c2 = [q(C2_E1[0], C2_E2[0]), q(C2_E1[1], C2_E2[0]), q(C2_E1[1], C2_E2[1]), q(C2_E1[0], C2_E2[1])]
    I.preencher([c2], "#8A4A2E")
    I.poli(c2, True, BR)
    I.linha(q(C2_E1[0], sum(C2_E2) / 2), q(C2_E1[1], sum(C2_E2) / 2), "#E0B090")
    rotulo(q(0, -11)[0] / M - 1.5, q(0, -11)[1] / M - 1.0, 0.5, "CASA 2 (sem módulos)", BR)
    # casa, cume e águas
    I.camada("CASA", BR)
    cont = [q(0, 0), q(LARG, 0), q(LARG, COMP), q(0, COMP)]
    I.preencher([cont], "#8A4A2E")
    I.poli(cont, True, BR)
    I.linha(q(LARG / 2, 0), q(LARG / 2, COMP), "#E0B090")
    I.camada("PAINEIS", AZ)
    for c in range(COLS):
        a = E2_INI + c * (MOD_L + GAP)
        for r in range(LINHAS):
            s0 = S0 + r * (MOD_A + GAP)
            b0, b1 = LARG / 2 + s0 * (LARG / 2) / LW, LARG / 2 + (s0 + MOD_A) * (LARG / 2) / LW
            pts = [q(b0, a), q(b0, a + MOD_L), q(b1, a + MOD_L), q(b1, a)]
            I.preencher([pts], "#16265E")
            I.poli(pts, True, "#5A6FA8")
    # cotas 5,37
    p1, p2 = q(3.0, -0.6), q(8.4, -0.6)
    I.camada("COTAS", "#3B82F6")
    I.linha(p1, p2, "#3B82F6")
    rotulo((p1[0] + p2[0]) / 2 / M - 1.2, (p1[1] + p2[1]) / 2 / M + 0.5, 0.45, "5,37 m", "#3B82F6", ANG - 90 + 180)
    # azimute do plano dos módulos
    az = (90 - (ANG + 90)) % 360               # direção NO em graus a partir do norte, sentido horário
    az = (360 - ANG) % 360
    ax, ay = q(LARG / 2 + 2, COMP / 2)
    I.camada("AZIMUTE", AM)
    tip = (ax + nw[0] * 6 * M, ay + nw[1] * 6 * M)
    I.linha((ax, ay), tip, AM)
    sul = q(0, COMP)
    rotulo(sul[0] / M - 3, sul[1] / M - 3.5, 0.45, f"Azimute dos módulos ≈ {az:.0f}° (NO), inclinação ≈ 17°", AM)
    # norte para cima
    nx, ny = NORTE_PLANTA
    I.camada("NORTE", BR)
    I.linha((nx, ny - 2 * M), (nx, ny + 2 * M), BR)
    I.poli([(nx - 0.6 * M, ny + 1.2 * M), (nx, ny + 2 * M), (nx + 0.6 * M, ny + 1.2 * M)], False, BR)
    rotulo(nx / M - 0.3, ny / M + 2.4, 0.6, "N")


# ------------------------------------------------------------------ unifilar
def caixa(x, y, w, h, titulo, linhas, cor=BR):
    I.camada("UNIFILAR", cor)
    pts = [(x * M, y * M), ((x + w) * M, y * M), ((x + w) * M, (y + h) * M), (x * M, (y + h) * M)]
    I.poli(pts, True, cor)
    rotulo(x + 0.3, y + h - 0.9, 0.5, titulo, cor)
    for i, t in enumerate(linhas):
        rotulo(x + 0.3, y + h - 1.9 - i * 0.75, 0.4, t, CI)


def fio(a, b, cor, camada="UNIFILAR_FIO"):
    I.camada(camada, cor)
    I.poli([(p[0] * M, p[1] * M) for p in (a if isinstance(a, list) else [a, b])], False, cor)


def unifilar(ox, oy):
    x, y = ox, oy
    rotulo(x, y + 9.5, 0.6, "DIAGRAMA UNIFILAR — SISTEMA FOTOVOLTAICO ON-GRID 6,6 kWp / 220 V")
    caixa(x, y, 9, 7.5, "ARRANJO FV", ["String 1: 6 x 550 Wp", "String 2: 6 x 550 Wp", "Voc/string ≈ 297 V", "Isc = 14 A"], AZ)
    caixa(x + 12, y, 9, 7.5, "STRING BOX CC", ["Seccionadora 2 pólos", "DPS classe II 1000 V", "Cabo solar 6 mm²"], AM)
    caixa(x + 24, y, 9, 7.5, "INVERSOR 6 kW", ["On-grid, 2 MPPT", "Saída 220 V / 27 A", "Anti-ilhamento"], BR)
    caixa(x + 36, y, 9, 7.5, "QUADRO CA", ["Disjuntor 2P 32 A (C)", "DR 40 A / 30 mA", "DPS classe II"], BR)
    caixa(x + 48, y, 9, 7.5, "MEDIDOR", ["Bidirecional", "(troca pela", "concessionária)"], BR)
    caixa(x + 60, y, 9, 7.5, "REDE 220 V", ["Ramal aéreo", "Poste na Av.", "João Motta"], BR)
    # condutores
    fio((x + 9, y + 5.2), (x + 12, y + 5.2), VM, "FIO_CC")
    fio((x + 9, y + 2.4), (x + 12, y + 2.4), AZ, "FIO_CC")
    rotulo(x + 9.1, y + 5.6, 0.3, "+", VM); rotulo(x + 9.1, y + 1.5, 0.3, "-", AZ)
    for xa in (21, 33, 45, 57):
        fio((xa + x, y + 4.2), (xa + x + 3, y + 4.2), VM, "FIO_CA" if xa > 30 else "FIO_CC")
    rotulo(x + 21.2, y + 4.6, 0.3, "2 x 6 mm² CC", CI)
    rotulo(x + 33.2, y + 4.6, 0.3, "6 mm² CA", CI)
    rotulo(x + 45.2, y + 4.6, 0.3, "6 mm² CA", CI)
    rotulo(x + 57.2, y + 4.6, 0.3, "ramal", CI)
    # terra
    fio([(x + 4.5, y), (x + 4.5, y - 2.5), (x + 64.5, y - 2.5), (x + 64.5, y)], None, VD, "FIO_TERRA")
    fio([(x + 28.5, y - 2.5), (x + 28.5, y - 3.5)], None, VD, "FIO_TERRA")
    fio([(x + 27.5, y - 3.5), (x + 29.5, y - 3.5)], None, VD, "FIO_TERRA")
    fio([(x + 27.9, y - 3.9), (x + 29.1, y - 3.9)], None, VD, "FIO_TERRA")
    rotulo(x + 31, y - 3.4, 0.35, "Aterramento: haste 2,4 m, cabo 6 mm² verde/amarelo", VD)
    # dados
    ty = y - 8
    dados = ["DADOS DO PROJETO (premissas — conferir em campo)",
             "Módulos: 12 x 550 Wp = 6,6 kWp | Inversor 6 kW | 2 strings de 6 em série",
             "Telhado: 2 águas, cumeeira SO-NE; módulos na água NO, azimute ≈ 319°, inclinação ≈ 17°",
             "Geração estimada ≈ 750 kWh/mês (5,0 h de sol pleno, perdas 20%, desvio azimutal ≈ 5%)",
             "Medidas do print do Google Maps (régua de 5,37 m na ponta NE da casa 1); sombra das árvores não considerada",
             "Ligação à rede sujeita à norma da concessionária (parecer de acesso)."]
    for i, t in enumerate(dados):
        rotulo(x, ty - i * 0.9, 0.5 if i == 0 else 0.4, t, BR if i == 0 else CI)


def montar():
    I.out.insert(0, "unidade mm")
    perspectiva()
    planta(50 * M / M, 6)      # ox, oy em metros
    unifilar(-14, -38)
    return "\n".join(I.out) + "\n"


if __name__ == "__main__":
    base = sys.argv[1] if len(sys.argv) > 1 else "/home/maragon/Documentos/FeatherCAD"
    I.out.clear()
    I.out.append("unidade mm")
    perspectiva()
    planta(66 * M, 2 * M)
    unifilar(-14, -38)
    texto = "\n".join(I.out) + "\n"
    open(f"{base}/solar.fct", "w").write(texto)
    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": texto})
    m.call("cad_salvar", {"caminho": f"{base}/solar.fcdz", "sobrescrever": True})
    m.call("cad_folha", {"acao": "inserir", "papel": "A1", "campos": {"titulo": "Sistema fotovoltaico on-grid 6,6 kWp", "subtitulo": "Perspectiva 3D, planta de cobertura e unifilar", "disciplina": "Energia solar", "fase": "Anteprojeto"}})
    m.call("cad_salvar", {"caminho": f"{base}/solar.fcdz", "sobrescrever": True})
    m.call("cad_salvar", {"caminho": f"{base}/solar.dxf", "sobrescrever": True})
    m.call("cad_pdf", {"caminho": f"{base}/solar.pdf", "papel": "FOLHA", "cores": True, "sobrescrever": True})
    print(m.call("cad_folha", {"acao": "listar"}))
    m.fechar()

"""A folha de resumo geral (A0): quadro do traçado, quantitativos estimados, criticidade de TODOS os elementos, diagrama de massa
(terraplenagem acumulada), curvas verticais, maiores zonas de aterro/escavação e as premissas do projeto."""
import math

from ferrovia_elementos import COR_ATERRO, COR_ESCAVACAO, COR_NIVEL, conselhos_para, fmt_est, quebrar
from ferrovia_lib import Fct, Folha, tabela, v

# seção de via adotada (a mesma do `cad_via`): dormente 2,00 × 0,22 × 0,20 m a cada 0,60 m, lastro 0,30 m sob o dormente, ombro 0,40 m, talude 1:1,5
DORM_C, DORM_L, DORM_H, ESPACO = 2.0, 0.22, 0.20, 0.60
LASTRO, OMBRO, TALUDE, SUBLASTRO = 0.30, 0.40, 1.5, 0.20
KG_POR_M_TR57 = 57.0


def comprimentos_por_tipo(al):
    """Metros de tangente (T), espiral (E) e circular (C) pelo traçado amostrado."""
    t = {"T": 0.0, "E": 0.0, "C": 0.0}
    for a, b in zip(al, al[1:]):
        t[a[5]] = t.get(a[5], 0.0) + (b[0] - a[0])
    return t


def quantitativos(comp, volumes, largura):
    topo = DORM_C + 2 * OMBRO
    base = topo + 2 * LASTRO * TALUDE
    area_lastro = (topo + base) / 2.0 * LASTRO + DORM_H * topo * (1.0 - DORM_L / ESPACO)
    area_sub = (base + base + 2 * SUBLASTRO * TALUDE) / 2.0 * SUBLASTRO
    n_dorm = math.ceil(comp / ESPACO) + 1
    return [
        ["Extensão de via", f"{v(comp, 0)} m", "eixo de projeto"],
        ["Trilho TR-57 (2 filas)", f"{v(2 * comp, 0)} m  ≈ {v(2 * comp * KG_POR_M_TR57 / 1000.0, 0)} t", "57 kg/m"],
        ["Dormentes (0,60 m)", f"{n_dorm} un", f"2,00 × {str(DORM_L).replace('.', ',')} × {str(DORM_H).replace('.', ',')} m"],
        ["Fixações elásticas", f"{4 * n_dorm} un", "2 por apoio, 4 por dormente"],
        ["Placas de apoio / palmilhas", f"{2 * n_dorm} un cada", "1 por apoio"],
        ["Lastro de brita", f"{v(area_lastro * comp, 0)} m³", f"{v(area_lastro, 2)} m²/m (0,30 m sob o dormente + enchimento)"],
        ["Sublastro", f"{v(area_sub * comp, 0)} m³", f"{v(area_sub, 2)} m²/m (0,20 m, talude 1:1,5)"],
        ["Terraplenagem — aterro", f"{v(volumes['aterro_m3'], 0)} m³", f"plataforma de {v(largura, 1)} m (premissa)"],
        ["Terraplenagem — escavação", f"{v(volumes['escavacao_m3'], 0)} m³", "estimativa a partir do eixo"],
        ["Saldo (aterro − escavação)", f"{v(volumes['saldo_m3'], 0)} m³", "greide compensado: perto de zero"],
    ]


def quadro_geral(d, comp, tipos):
    a = d["analise"]
    curvas = [e for e in a["elementos"] if e["tipo"] == "curva"]
    raios = [e["curva"]["raio"] for e in curvas]
    esq = sum(1 for e in curvas if e["curva"]["lado"] == "esquerda")
    levs = [e["levante_m"] for e in a["elementos"]]
    return [
        ["Extensão total", f"{v(comp, 1)} m  (km {v(comp / 1000.0, 3)})"],
        ["Tangentes", f"{v(tipos['T'], 0)} m  ({tipos['T'] / comp * 100:.0f}%)"],
        ["Espirais (clotoides)", f"{v(tipos['E'], 0)} m  ({tipos['E'] / comp * 100:.0f}%)"],
        ["Curvas circulares", f"{v(tipos['C'], 0)} m  ({tipos['C'] / comp * 100:.0f}%)"],
        ["Número de curvas", f"{len(curvas)}  ({esq} à esquerda, {len(curvas) - esq} à direita)"],
        ["Raio mínimo / máximo", f"{v(min(raios), 0)} / {v(max(raios), 0)} m"],
        ["Superelevação máxima adotada", f"{v(max(e['curva']['h_mm'] for e in curvas), 0)} mm  (limite {v(a['regras']['h_max_mm'], 0)} mm)"],
        ["Velocidade diretriz / mín. das curvas", f"{v(a['regras']['V'], 0)} / {v(min(e['curva']['v_max'] for e in curvas), 1)} km/h"],
        ["Levante mín. / máx. sobre o terreno", f"{v(min(l['min'] for l in levs), 2)} / {v(max(l['max'] for l in levs), 2)} m"],
        ["Rampa máxima do greide", f"{v(max(max(abs(e['rampa_pct']['min']), abs(e['rampa_pct']['max'])) for e in a['elementos']), 2)} %"],
        ["Curvas verticais (PIVs)", f"{len(a['pivs'])}"],
        ["Elementos: críticos / atenção / normais", f"{a['contagem']['criticos']} / {a['contagem']['atencao']} / {a['contagem']['normais']}"],
    ]


def tabela_de_criticidade(d):
    a = d["analise"]
    ordem = {"CRÍTICO": 0, "ATENÇÃO": 1, "NORMAL": 2}
    linhas, cores = [], []
    for e in sorted(a["elementos"], key=lambda x: (ordem[x["nivel"]], x["de"])):
        nome = ("C" if e["tipo"] == "curva" else "T") + str(e["n"])
        motivo = e["motivos"][0]["texto"] if e["motivos"] else "dentro dos critérios"
        if len(motivo) > 125:
            motivo = motivo[:123] + "…"
        extra = f" (+{len(e['motivos']) - 1})" if len(e["motivos"]) > 1 else ""
        linhas.append([nome, f"{fmt_est(e['de'])} a {fmt_est(e['ate'])}", v(e["comprimento"], 0), e["nivel"], motivo + extra])
        cor = COR_NIVEL[e["nivel"]]
        cores.append([None, None, None, cor, cor if e["nivel"] != "NORMAL" else None])
    return linhas, cores


def diagrama_de_massa(c, d, a, b):
    c.camada("RESUMO_MOLDURA", "#FFFFFF")
    c.ret(a, b)
    c.texto((a[0] + 2.0, b[1] + 2.5), 3.4, "DIAGRAMA DE MASSA (volume acumulado: aterro +, escavação −)")
    cm = d["analise"]["volumes"]["curva_de_massa"]
    comp = d["analise"]["alinhamento_geral"]["comprimento"]
    vs = [p[1] for p in cm]
    vmax = max(abs(min(vs)), abs(max(vs)), 1.0)
    cx0, cx1, cy0, cy1 = a[0] + 16.0, b[0] - 6.0, a[1] + 12.0, b[1] - 8.0
    ym = (cy0 + cy1) / 2.0
    X = lambda s: cx0 + s / comp * (cx1 - cx0)
    Y = lambda q: ym + q / vmax * (cy1 - cy0) / 2.0
    c.camada("RESUMO_MASSA_GRADE", "#909090")
    c.linha((cx0, ym), (cx1, ym))
    for q in (-vmax, -vmax / 2, vmax / 2, vmax):
        c.linha((cx0, Y(q)), (cx1, Y(q)))
        c.texto((cx0 - 1.5, Y(q)), 2.0, v(q, 0), "dm")
    s = 0.0
    while s <= comp:
        c.linha((X(s), cy0), (X(s), cy1))
        c.texto((X(s), cy0 - 3.0), 2.0, f"km {s / 1000.0:.0f}", "cm")
        s += 1000.0
    c.camada("RESUMO_MASSA", "#FF00FF")
    c.poli([(X(p[0]), Y(p[1])) for p in cm])
    kmax = max(range(len(cm)), key=lambda i: cm[i][1])
    kmin = min(range(len(cm)), key=lambda i: cm[i][1])
    c.camada("RESUMO_MASSA_ROTULO", "#FFFFFF")
    c.texto((X(cm[kmax][0]), Y(cm[kmax][1]) + 3.0), 2.2, f"máx. {v(cm[kmax][1], 0)} m³ (km {cm[kmax][0] / 1000.0:.2f})".replace(".", ",", 1), "cm")
    c.texto((X(cm[kmin][0]), Y(cm[kmin][1]) - 3.0), 2.2, f"mín. {v(cm[kmin][1], 0)} m³ (km {cm[kmin][0] / 1000.0:.2f})".replace(".", ",", 1), "cm")


def premissas(c, d, x, y_topo, largura, nuvem_info):
    r = d["analise"]["regras"]
    itens = [
        ("VIA", f"Bitola métrica (1,000 m entre faces; B = {v(r['B'], 3)} m entre eixos). Trilho TR-57. Dormentes de 2,00 m a cada 0,60 m, lastro de brita com 0,30 m sob o dormente e plataforma de 4,00 m (premissas editáveis)."),
        ("CRITÉRIOS", f"Tabela de regras «{r['nome']}»: V = {v(r['V'], 0)} km/h; h_max = {v(r['h_max_mm'], 0)} mm; I_max = {v(r['i_max_mm'], 0)} mm; dh/dt = {v(r['dh_dt_mm_s'], 0)} mm/s; rampa de superelevação máx. 1:{1.0 / r['rampa']:.0f}; ψ = {r['psi']} m/s³; Ls_min = {v(r['ls_min'], 0)} m; Lc_min = {v(r['lc_min'], 0)} m."),
        ("CRITICIDADE", "CRÍTICO = limite violado (raio abaixo do mínimo, I acima do máximo, Ls abaixo de L_min, ripagem ≥ 0,75 m, rampa ≥ 1,5%, aterro/escavação ≥ 0,20 m). ATENÇÃO = perto do limite (h ≥ 95% de h_max, I ≥ 80%, Ls < 1,03 L_min, R < 1,25 R_min, ripagem ≥ 0,50 m, tangente < 30 m, aterro/escavação ≥ 0,10 m, poucos pontos)."),
        ("ATERRO E ESCAVAÇÃO", "Levante = cota do greide − cota existente do trilho: positivo é aterro (ou lastro adicional), negativo é escavação (ou rebaixo). Os volumes são ordem de grandeza: usam o eixo dos trilhos e uma plataforma de largura fixa, sem seção do terreno."),
        ("NUVEM DE PONTOS", nuvem_info),
        ("PRÓXIMOS PASSOS", "Levantar seções transversais e sondagens; conferir a cota dos trilhos (em muitos pares os dois trilhos têm a mesma cota); avaliar drenagem, passagens em nível e obras de arte; validar os limites com a norma e o regulamento da ferrovia."),
    ]
    c.camada("RESUMO_PREMISSAS", "#FFFFFF")
    y = y_topo
    for titulo, texto in itens:
        c.texto((x, y), 3.4, titulo, "eb", cor=None)
        y -= 4.6
        for ln in quebrar(texto, largura, 3.0):
            c.texto((x, y), 3.0, ln, "eb")
            y -= 4.1
        y -= 2.2
    return y


def acoes_recomendadas(c, d, x, y_topo, largura):
    """Críticos primeiro, depois os de atenção com mais motivos: o motivo e o que fazer."""
    a = d["analise"]
    ordem = {"CRÍTICO": 0, "ATENÇÃO": 1, "NORMAL": 2}
    lista = sorted((e for e in a["elementos"] if e["nivel"] != "NORMAL"), key=lambda e: (ordem[e["nivel"]], -len(e["motivos"]), e["de"]))
    c.camada("RESUMO_ACOES", "#FFFFFF")
    c.texto((x, y_topo), 4.0, "AÇÕES RECOMENDADAS POR PRIORIDADE (críticos primeiro)", "eb")
    y = y_topo - 7.0
    for e in lista[:9]:
        nome = ("Curva " if e["tipo"] == "curva" else "Tangente ") + str(e["n"])
        cor = COR_NIVEL[e["nivel"]]
        c.texto((x, y), 3.1, f"{nome} — {e['nivel']} — est. {fmt_est(e['de'])} a {fmt_est(e['ate'])}", "eb", cor=cor)
        y -= 4.4
        dadas = set()
        for m in e["motivos"][:3]:
            for k, ln in enumerate(quebrar("• " + m["texto"], largura - 4, 2.8)):
                c.texto((x + 3 + (0 if k == 0 else 2), y), 2.8, ln, "eb")
                y -= 3.7
            conselho = conselhos_para(m["texto"])
            if conselho and conselho not in dadas:
                dadas.add(conselho)
                for ln in quebrar("→ " + conselho, largura - 8, 2.6):
                    c.texto((x + 6, y), 2.6, ln, "eb", cor="#808080")
                    y -= 3.4
        y -= 3.0
        if y < 185:
            break
    return y


def folha_do_resumo(mcp, d, carimbo, nuvem_info):
    a = d["analise"]
    comp = a["alinhamento_geral"]["comprimento"]
    campos = dict(carimbo)
    campos["titulo"] = "Resumo geral, quantitativos e criticidade"
    r = mcp.call("cad_folha", {"acao": "inserir", "papel": "A0", "escala": 10000, "campos": campos})
    f = Folha(r["folha"])
    c = Fct(f)
    c.camada("RESUMO_TITULO", "#FFFFFF")
    c.texto((60.0, 790.0), 9.0, "RESUMO GERAL DO PROJETO FERROVIÁRIO — QUANTITATIVOS, CRITICIDADE E PONTOS DE ATENÇÃO")
    tipos = comprimentos_por_tipo(d["al"])
    c.camada("RESUMO_TABELA", "#FFFFFF")
    y = tabela(c, 60.0, 765.0, [("Grandeza", 130.0, "e"), ("Valor", 190.0, "e")], quadro_geral(d, comp, tipos), 7.6, 3.3, "QUADRO GERAL DO TRAÇADO")
    y4 = tabela(c, 60.0, y - 12.0, [("Item", 100.0, "e"), ("Quantidade", 84.0, "e"), ("Observação", 136.0, "e")], quantitativos(comp, a["volumes"], a["volumes"]["largura_plataforma_m"]), 7.6, 3.1, "QUANTITATIVOS ESTIMADOS")
    linhas, cores = tabela_de_criticidade(d)
    y3 = tabela(c, 410.0, 765.0, [("Elem.", 16.0, "c"), ("Estacas", 56.0, "c"), ("m", 14.0, "d"), ("Nível", 22.0, "c"), ("Principal motivo (e quantos outros)", 262.0, "e")], linhas, 8.0, 3.0, "QUADRO DE CRITICIDADE — TODAS AS CURVAS E TANGENTES", cores)
    diagrama_de_massa(c, d, (800.0, 575.0), (1150.0, 770.0))
    acoes_recomendadas(c, d, 410.0, y3 - 14.0, 350.0)
    pivs = [[i + 1, fmt_est(p["progressiva"]), v(p["cota"], 2), v(p["rampa_chegada_pct"], 3), v(p["rampa_saida_pct"], 3), v(p["lv"], 0), "convexa" if p["convexa"] else "côncava"] for i, p in enumerate(a["pivs"])]
    y2 = tabela(c, 800.0, 560.0, [("PIV", 14.0, "c"), ("Estaca", 38.0, "c"), ("Cota (m)", 38.0, "c"), ("i₁ (%)", 40.0, "c"), ("i₂ (%)", 40.0, "c"), ("Lv (m)", 34.0, "c"), ("Tipo", 46.0, "c")], pivs, 6.2, 2.9, "CURVAS VERTICAIS DO GREIDE")
    zonas = sorted(a["zonas"], key=lambda z: -z["maximo_m"])[:8]
    tabela(c, 800.0, y2 - 10.0, [("Natureza", 62.0, "e"), ("Estacas", 94.0, "c"), ("Máx. (m)", 40.0, "c"), ("Nível", 54.0, "c")],
           [[z["natureza"], f"{fmt_est(z['de'])} a {fmt_est(z['ate'])}", v(z["maximo_m"], 2), z["nivel"]] for z in zonas], 6.2, 2.9, "MAIORES ZONAS DE ATERRO E ESCAVAÇÃO",
           [[COR_ATERRO if z["natureza"] == "aterro" else COR_ESCAVACAO, None, None, COR_NIVEL[z["nivel"]]] for z in zonas])
    premissas(c, d, 60.0, y4 - 14.0, 320.0, nuvem_info)
    c.enviar(mcp, "resumo")
    return f.indice

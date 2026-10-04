"""A folha da nuvem de pontos (A0): só o levantamento, com o detalhamento geográfico — mapa dos pontos por cota, pontos suspeitos e
lacunas, perfil das cotas, histogramas de bitola e de nivelamento transversal, estatísticas e as coordenadas geográficas."""
import csv
import math
import statistics

from ferrovia_lib import Fct, Folha, interpolar, tabela, v

PALETA = ["#3060D0", "#20A0B0", "#30A040", "#B09000", "#E07000", "#D03030"]
COR_SUSPEITO = "#E03030"


def ler_nuvem(caminho):
    with open(caminho) as f:
        linhas = [r for r in csv.reader(f)]
    return [(float(r[0]), float(r[1]), float(r[2])) for r in linhas[1:] if len(r) >= 3]


def classe(z, limites):
    return next((i for i, l in enumerate(limites) if z < l), len(limites))


def painel(c, a, b, titulo):
    c.camada("NUVEM_MOLDURA", "#FFFFFF")
    c.ret(a, b)
    c.texto((a[0] + 2.0, b[1] + 2.5), 4.0, titulo)


def mapa(c, pts, dados, limites):
    x0, y0, x1, y1 = 60.0, 170.0, 360.0, 760.0
    painel(c, (x0, y0), (x1, y1), "MAPA DOS PONTOS (escala 1:10000, norte da quadrícula para cima)")
    emin, emax = min(p[0] for p in pts), max(p[0] for p in pts)
    nmin, nmax = min(p[1] for p in pts), max(p[1] for p in pts)
    esc = 10000.0
    k = 1000.0 / esc
    ox = (x0 + x1) / 2.0 - (emin + emax) / 2.0 * k
    oy = y0 + 18.0 - nmin * k
    X = lambda e: ox + e * k
    Y = lambda n: oy + n * k
    # malha de 500 m com coordenadas
    c.camada("NUVEM_MALHA", "#909090")
    passo = 500.0
    ge = math.floor((emin - 150) / passo) * passo
    while ge <= emax + 150:
        gn = math.ceil((nmin - 100) / passo) * passo
        while gn <= nmax + 100:
            x, y = X(ge), Y(gn)
            if x0 + 3 < x < x1 - 3 and y0 + 12 < y < y1 - 3:
                c.linha((x - 2.5, y), (x + 2.5, y))
                c.linha((x, y - 2.5), (x, y + 2.5))
                c.texto((x1 - 1.5, y), 2.0, f"N {gn:.0f}", "dm")
            gn += passo
        if x0 + 3 < X(ge) < x1 - 3:
            c.texto((X(ge), y0 + 4.0), 2.0, f"E {ge:.0f}", "cm")
        ge += passo
    # eixo de projeto e quilômetros
    al = dados["al"]
    c.camada("NUVEM_EIXO", "#FF00FF")
    c.poli([(X(r[1]), Y(r[2])) for r in al[::5]])
    c.camada("NUVEM_KM", "#FF7FDF")
    ss = [r[0] for r in al]
    km = 0.0
    while km <= al[-1][0]:
        E, N = interpolar(ss, [r[1] for r in al], km), interpolar(ss, [r[2] for r in al], km)
        if E is not None:
            c.circulo((X(E), Y(N)), 1.0)
            c.texto((X(E) - 3.0, Y(N)), 2.4, f"KM {km / 1000.0:.0f}", "dm")
        km += 1000.0
    # pontos por classe de cota
    for i, cor in enumerate(PALETA):
        c.camada(f"NUVEM_COTA_{i + 1}", cor)
        for p in pts:
            if classe(p[2], limites) == i:
                c.circulo((X(p[0]), Y(p[1])), 0.45)
    # suspeitos
    c.camada("NUVEM_SUSPEITOS", COR_SUSPEITO, espessura=0.4)
    for k2, s in enumerate(dados["suspeitos"]):
        c.circulo((X(s["E"]), Y(s["N"])), 2.2)
        c.texto((X(s["E"]) + 3.0, Y(s["N"])), 2.2, f"S{k2 + 1}: cota {v(s['cota'], 2)} (mediana local {v(s['mediana_local'], 2)})", "em", cor=COR_SUSPEITO)
    # cotas pontuais a cada ~150 m
    c.camada("NUVEM_COTAS_PONTUAIS", "#808080")
    ult = None
    for p in sorted(pts, key=lambda q: q[1]):
        if ult is None or math.hypot(p[0] - ult[0], p[1] - ult[1]) > 150.0:
            c.texto((X(p[0]) + 1.2, Y(p[1]) + 1.0), 1.8, v(p[2], 1), "eb")
            ult = p
    # lacunas
    c.camada("NUVEM_LACUNAS", "#E08000", espessura=0.6)
    for l in dados["lacunas"]:
        if l["m"] >= 40.0:
            ea = interpolar(ss, [r[1] for r in al], l["de"])
            na = interpolar(ss, [r[2] for r in al], l["de"])
            eb = interpolar(ss, [r[1] for r in al], l["ate"])
            nb = interpolar(ss, [r[2] for r in al], l["ate"])
            if None not in (ea, na, eb, nb):
                c.linha((X(ea) + 4.0, Y(na)), (X(eb) + 4.0, Y(nb)))
    c.camada("NUVEM_NORTE", "#FFFFFF")
    nx, ny = x1 - 22.0, y1 - 20.0
    c.linha((nx, ny - 8), (nx, ny + 8))
    c.poli([(nx, ny + 8), (nx - 2.5, ny + 2), (nx + 2.5, ny + 2)], True)
    c.texto((nx, ny + 11), 3.0, "N", "cm")
    c.linha((x0 + 8.0, y0 + 9.0), (x0 + 8.0 + 50.0, y0 + 9.0))
    for t in (0, 25, 50):
        c.linha((x0 + 8.0 + t, y0 + 7.8), (x0 + 8.0 + t, y0 + 10.2))
    c.texto((x0 + 33.0, y0 + 12.0), 2.2, "500 m", "cm")
    c.texto((x0 + 62.0, y0 + 9.0), 2.0, "laranja: lacuna ≥ 40 m  ·  vermelho: ponto suspeito", "em")


def perfil_das_cotas(c, pares, limites, comp):
    a, b = (400.0, 560.0), (1150.0, 760.0)
    painel(c, a, b, "PERFIL DAS COTAS DA NUVEM (cota do trilho × distância ao longo do eixo rastreado)")
    cx0, cx1, cy0, cy1 = a[0] + 16.0, b[0] - 6.0, a[1] + 14.0, b[1] - 8.0
    zs = [p[1] for p in pares]
    z0, z1 = math.floor(min(zs)), math.ceil(max(zs))
    X = lambda s: cx0 + s / comp * (cx1 - cx0)
    Y = lambda z: cy0 + (z - z0) / max(z1 - z0, 1) * (cy1 - cy0)
    c.camada("NUVEM_PERFIL_GRADE", "#909090")
    z = z0
    while z <= z1:
        c.linha((cx0, Y(z)), (cx1, Y(z)))
        c.texto((cx0 - 1.5, Y(z)), 2.2, f"{z}", "dm")
        z += 2
    s = 0.0
    while s <= comp:
        c.linha((X(s), cy0), (X(s), cy1))
        c.texto((X(s), cy0 - 3.0), 2.2, f"km {s / 1000.0:.0f}", "cm")
        s += 1000.0
    c.camada("NUVEM_PERFIL_LINHA", "#808080")
    c.poli([(X(p[0]), Y(p[1])) for p in pares])
    for i, cor in enumerate(PALETA):
        c.camada(f"NUVEM_PERFIL_COTA_{i + 1}", cor)
        for p in pares:
            if classe(p[1], limites) == i:
                c.circulo((X(p[0]), Y(p[1])), 0.7)
    c.camada("NUVEM_PERFIL_LEGENDA", "#FFFFFF")
    x = a[0] + 6.0
    for i, cor in enumerate(PALETA):
        lo = f"{limites[i - 1]:.1f}" if i > 0 else "—"
        hi = f"{limites[i]:.1f}" if i < len(limites) else "—"
        c.camada(f"NUVEM_PERFIL_COTA_{i + 1}", cor)
        c.circulo((x, b[1] - 6.0), 1.2)
        c.camada("NUVEM_PERFIL_LEGENDA", "#FFFFFF")
        c.texto((x + 3.0, b[1] - 6.0), 2.4, f"{lo.replace('.', ',')} a {hi.replace('.', ',')} m", "em")
        x += 62.0


def histograma(c, a, b, titulo, valores, de, ate, n, rotulo_x, casas=0):
    painel(c, a, b, titulo)
    cx0, cx1, cy0, cy1 = a[0] + 14.0, b[0] - 6.0, a[1] + 16.0, b[1] - 10.0
    cont = [0] * n
    for x in valores:
        if de <= x < ate:
            cont[int((x - de) / (ate - de) * n)] += 1
    fora = len(valores) - sum(cont)
    ordenados = sorted(cont, reverse=True)
    teto = max(ordenados[1] * 1.15, 1.0) if len(ordenados) > 1 and ordenados[0] > 4 * ordenados[1] else (ordenados[0] or 1)
    m = teto
    c.camada("NUVEM_HISTOGRAMA", "#3080D0")
    w = (cx1 - cx0) / n
    for i, q in enumerate(cont):
        if q:
            h = min(q / m, 1.0) * (cy1 - cy0)
            c.ret((cx0 + i * w + 0.3, cy0), (cx0 + (i + 1) * w - 0.3, cy0 + h))
            quebrada = q > m
            if quebrada:  # barra cortada: o traço duplo marca a quebra e o rótulo diz o valor real
                c.linha((cx0 + i * w, cy0 + h - 4.0), (cx0 + (i + 1) * w, cy0 + h - 2.0))
            c.texto((cx0 + (i + 0.5) * w, cy0 + h + 2.0), 2.4 if quebrada else 2.0, f"{q} (barra cortada)" if quebrada else str(q), "cm", cor="#808080")
    c.camada("NUVEM_HISTOGRAMA_EIXO", "#909090")
    c.linha((cx0, cy0), (cx1, cy0))
    for t in range(0, n + 1, max(n // 5, 1)):
        x = de + (ate - de) * t / n
        c.texto((cx0 + t * w, cy0 - 3.0), 2.2, v(x, casas), "cm")
    c.texto(((cx0 + cx1) / 2.0, a[1] + 3.0), 2.4, f"{rotulo_x}  ({fora} fora do intervalo)", "cm")


def estatisticas(c, pts, pares, dados, comp):
    a, b = (400.0, 170.0), (770.0, 320.0)
    painel(c, a, b, "ESTATÍSTICAS DA NUVEM")
    zs = [p[2] for p in pts]
    esp = [pares[i + 1][0] - pares[i][0] for i in range(len(pares) - 1)]
    zero = dados["zero"]
    linhas = [
        ["Pontos do levantamento", f"{len(pts)}"],
        ["Pares de trilho identificados", f"{len(pares)}  (bitola mediana {v(dados['bitola_mediana'] * 1000, 0)} mm)"],
        ["Extensão rastreada", f"{v(comp / 1000.0, 3)} km"],
        ["Cotas mín / média / máx", f"{v(min(zs), 2)} / {v(statistics.mean(zs), 2)} / {v(max(zs), 2)} m"],
        ["Desnível total", f"{v(max(zs) - min(zs), 2)} m"],
        ["Espaçamento dos pares (mediana / máx.)", f"{v(statistics.median(esp), 1)} / {v(max(esp), 0)} m"],
        ["Lacunas ≥ 40 m", f"{sum(1 for l in dados['lacunas'] if l['m'] >= 40)}  (maior {v(max(l['m'] for l in dados['lacunas']), 0)} m)"],
        ["Pontos suspeitos (cota fora da mediana)", f"{len(dados['suspeitos'])}"],
        ["Pares com os 2 trilhos na mesma cota", f"{zero} de {len(pares)} ({zero / len(pares) * 100:.0f}%)"],
    ]
    tabela(c, a[0] + 3.0, b[1] - 8.0, [("Grandeza", 150.0, "e"), ("Valor", 214.0, "e")], linhas, 11.0, 2.9)
    c.camada("NUVEM_ALERTA", COR_SUSPEITO)
    c.texto((a[0] + 3.0, a[1] + 4.0), 2.3, "ALERTA: cotas iguais nos dois trilhos em série indicam cota copiada; a superelevação medida na nuvem não é confiável.", "em", cor=COR_SUSPEITO)


def geografico(c, mcp, pts, dados):
    a, b = (790.0, 170.0), (1150.0, 320.0)
    painel(c, a, b, "DETALHAMENTO GEOGRÁFICO (UTM SIRGAS 2000, zona 23 S — premissa a confirmar)")
    emin, emax = min(p[0] for p in pts), max(p[0] for p in pts)
    nmin, nmax = min(p[1] for p in pts), max(p[1] for p in pts)
    alvos = [("Início do traçado", dados["al"][0][1], dados["al"][0][2]), ("Fim do traçado", dados["al"][-1][1], dados["al"][-1][2]), ("Centro da nuvem", (emin + emax) / 2.0, (nmin + nmax) / 2.0), ("Canto SO da caixa", emin, nmin), ("Canto NE da caixa", emax, nmax)]
    g = mcp.call("cad_geografico", {"pontos": [[e, n] for _, e, n in alvos], "zona": 23, "hemisferio": "sul"})
    linhas = [[nome, f"{e:.0f} / {n:.0f}", p["latitude_dms"], p["longitude_dms"]] for (nome, e, n), p in zip(alvos, g["pontos"])]
    tabela(c, a[0] + 3.0, b[1] - 8.0, [("Ponto", 62.0, "e"), ("E / N (m)", 66.0, "c"), ("Latitude", 100.0, "c"), ("Longitude", 100.0, "c")], linhas, 10.0, 2.6)
    p0 = g["pontos"][2]
    c.camada("NUVEM_GEO", "#808080")
    c.texto((a[0] + 3.0, a[1] + 12.0), 2.4, f"Meridiano central {g['meridiano_central']:.0f}° O · convergência de meridianos γ = {v(p0['convergencia_graus'], 3)}° · fator de escala k = {v(p0['escala'], 6)}", "em", cor="#808080")
    c.texto((a[0] + 3.0, a[1] + 7.0), 2.4, f"Datum {g['datum']}. Cotas: as do levantamento (altimetria de referência não informada).", "em", cor="#808080")
    c.texto((a[0] + 3.0, a[1] + 2.5), 2.2, "A zona UTM foi suposta (23 S); confirme no relatório do levantamento antes de usar as coordenadas geográficas.", "em", cor="#808080")


def folha_da_nuvem(mcp, dados, caminho, carimbo):
    pts = ler_nuvem(caminho)
    pares = dados["pares_sz"]
    comp = dados["comp"]
    zs = sorted(p[2] for p in pts)
    limites = [zs[int(len(zs) * q / 6)] for q in range(1, 6)]
    campos = dict(carimbo)
    campos["titulo"] = "Nuvem de pontos do levantamento — detalhamento geográfico"
    r = mcp.call("cad_folha", {"acao": "inserir", "papel": "A0", "escala": 10000, "campos": campos})
    f = Folha(r["folha"])
    c = Fct(f)
    c.camada("NUVEM_TITULO", "#FFFFFF")
    c.texto((60.0, 790.0), 9.0, "NUVEM DE PONTOS DO LEVANTAMENTO — DETALHAMENTO GEOGRÁFICO")
    c.texto((60.0, 779.0), 4.0, f"{caminho.split('/')[-1]} — {len(pts)} pontos — somente o levantamento, sem o projeto (o eixo magenta é a referência do rastreio)")
    mapa(c, pts, dados, limites)
    perfil_das_cotas(c, pares, limites, comp)
    histograma(c, (400.0, 340.0), (770.0, 540.0), "BITOLA MEDIDA ENTRE OS TRILHOS (eixo a eixo)", [p[2] * 1000.0 for p in dados["pares_bn"]], 1060.0, 1090.0, 30, "bitola (mm)")
    histograma(c, (790.0, 340.0), (1150.0, 540.0), "NIVELAMENTO TRANSVERSAL MEDIDO (trilho esq. − dir.)", [p[1] for p in dados["pares_bn"]], -100.0, 100.0, 20, "nivelamento (mm)")
    estatisticas(c, pts, pares, dados, comp)
    geografico(c, mcp, pts, dados)
    c.enviar(mcp, "nuvem")
    return f.indice

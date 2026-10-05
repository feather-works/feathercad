#!/usr/bin/env python3
"""Projeto ferroviário completo a partir de uma nuvem de pontos, dirigindo o servidor MCP do FeatherCAD.

O MCP faz o cálculo pesado (rastreio do eixo, traçado ajustado com transições, superelevação, greide, criticidade) e as
folhas-base (`cad_modelo ferrovia`); este script acrescenta as folhas que dependem da análise: uma por curva e por tangente,
a da nuvem de pontos e o resumo geral, renumera o carimbo e grava `.fcdz`, `.dxf` e `.pdf`.

    python3 ferrovia_completa.py nuvem.csv saida_base [--vivo] [--so 6,13]

Sem `--vivo` o servidor trabalha isolado (`FEATHERCAD_VIVO=0`); com `--vivo`, se o aplicativo estiver aberto, tudo aparece na tela.
`--so` (teste) limita as folhas de elemento (ex.: `C6,T7`).
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from mcp_cliente import Mcp  # noqa: E402
from ferrovia_elementos import Elemento, folha_do_elemento  # noqa: E402
from ferrovia_nuvem import folha_da_nuvem  # noqa: E402
from ferrovia_resumo import folha_do_resumo  # noqa: E402


def lista(m, resposta):
    if "linhas" in resposta:
        return resposta["linhas"]
    with open(resposta["arquivo"]) as f:
        return json.load(f)


def carregar(m, comprimento):
    """Os dados do projeto já calculado na sessão do MCP, no formato que as folhas leem."""
    analise = m.call("cad_analise", {})
    al = lista(m, m.call("cad_alinhamento_ponto", {"de": 0, "ate": comprimento, "passo": 2, "salvar": True}))
    tab = lista(m, m.call("cad_tabela_estacas", {"passo": 5, "salvar": True, "colunas": ["progressiva", "cota_existente", "cota_projeto"]}))
    fe = m.call("cad_nuvem_feicoes", {"bitola": 1.0, "salvar": True, "nome": "FP"})
    pares = lista(m, fe)
    pares.sort(key=lambda r: r[1])
    q = m.call("cad_nuvem_qualidade", {"feicoes": "FP", "marcar": False})
    tin_r = m.call("cad_tin", {})
    with open(tin_r["arquivo"]) as f:
        tin = json.load(f)
    return {
        "tin": tin,
        "suspeitos": [{"E": s["E"], "N": s["N"], "cota": s["cota"], "mediana_local": s["mediana_local"]} for s in q["cota_fora_da_mediana"]["lista"]],
        "lacunas": q["lacunas"]["lista"],
        "zero": q["cotas_identicas_em_serie"]["pares_com_nivelamento_zero"],
        "bitola_mediana": fe["bitola_medida"]["mediana"],
        "pares_sz": [(r[1], r[4]) for r in pares],
        "pares_bn": [(r[1], r[5], r[6]) for r in pares],
        "comp": max(r[1] for r in pares),
        "analise": analise,
        "al": al,
        "tab": tab,
        "existente": [(r[2], r[3]) for r in pares],
        "pivs": analise["pivs"],
        "regras": analise["regras"],
        "n_curvas": analise["alinhamento_geral"]["curvas"],
        "criterios_tangente": 30.0,
    }


ORDEM_POR_TITULO = [
    ("planta: eixo", 0), ("superelevação, curvatura", 1), ("seção tipo", 2), ("memorial", 3), ("resumo geral", 4), ("nuvem de pontos", 5),
    ("perfil longitudinal", 6), ("planta e curvas", 7), ("curva ", 8), ("tangente ", 8), ("tabela por estaca", 9),
]


def classe_da_folha(titulo):
    t = titulo.lower()
    return next((c for prefixo, c in ORDEM_POR_TITULO if t.startswith(prefixo)), 99)


def renumerar(m):
    """O carimbo `NN/MM` na ordem final: geral, gráficos, seção, memorial, resumo, nuvem, perfis, plantas, elementos, tabelas."""
    folhas = m.call("cad_folha", {"acao": "listar"})["folhas"]
    dados = []
    for f in folhas:
        campos = m.call("cad_folha", {"acao": "campos", "indice": f["indice"]})["campos"]
        dados.append((classe_da_folha(campos["titulo"]), f["indice"], campos))
    dados.sort(key=lambda x: (x[0], x[1]))
    total = len(dados)
    for n, (_, indice, campos) in enumerate(dados, 1):
        campos["folha"] = f"{n:02d}/{total:02d}"
        m.call("cad_folha", {"acao": "campos_definir", "indice": indice, "campos": campos})
    return total


def principal():
    ap = argparse.ArgumentParser()
    ap.add_argument("nuvem")
    ap.add_argument("saida")
    ap.add_argument("--vivo", action="store_true")
    ap.add_argument("--so", default="")
    ap.add_argument("--V", type=float, default=65.0)
    args = ap.parse_args()
    if not args.vivo:
        os.environ["FEATHERCAD_VIVO"] = "0"
    m = Mcp()
    carimbo = {"cliente": "Teste", "fase": "Anteprojeto", "disciplina": "Via permanente"}
    r = m.call("cad_modelo", {"nome": "ferrovia", "dados": {"nuvem": args.nuvem, "V": args.V, "carimbo": carimbo}})
    print("modelo:", r["folhas"], "folhas;", r["criticidade"])
    comprimento = r["alinhamento"]["comprimento"]
    d = carregar(m, comprimento)
    elementos = [Elemento(e, d) for e in d["analise"]["elementos"]]
    if args.so:
        quer = set(args.so.upper().split(","))
        elementos = [e for e in elementos if e.curto in quer]
    nuvem_idx = folha_da_nuvem(m, d, args.nuvem, carimbo)
    print("  folha da nuvem", nuvem_idx)
    nuvem_info = f"{len(d['pares_sz'])} pares de trilho identificados; {d['zero']} deles com os dois trilhos na mesma cota; {len(d['suspeitos'])} pontos suspeitos de cota; {sum(1 for l in d['lacunas'] if l['m'] >= 40)} lacunas de 40 m ou mais."
    resumo_idx = folha_do_resumo(m, d, carimbo, nuvem_info)
    print("  folha do resumo", resumo_idx)
    indices = {}
    for e in elementos:
        indices[e.curto] = folha_do_elemento(m, e, len(elementos), carimbo)
        print("  folha", e.curto, indices[e.curto])
    if args.so:
        fo = m.call("cad_folha", {"acao": "listar"})["folhas"]
        for nome, idx in [(e.curto, indices[e.curto]) for e in elementos] + [("nuvem", nuvem_idx), ("resumo", resumo_idx)]:
            f = fo[idx]
            m.call("cad_ver", {"janela": {"x1": f["min"]["x"], "y1": f["min"]["y"], "x2": f["max"]["x"], "y2": f["max"]["y"]}, "arquivo": f"{args.saida}_{nome}.png", "largura": 1800, "rotulos": False, "grade": False})
    if not args.so:
        total = renumerar(m)
        print("carimbo renumerado:", total, "folhas")
        for ext in ("fcdz", "dxf"):
            r = m.call("cad_salvar", {"caminho": f"{args.saida}.{ext}", "sobrescrever": True})
            print(" ", ext, r.get("bytes"), "bytes", r.get("avisos") or "")
        r = m.call("cad_pdf", {"caminho": f"{args.saida}.pdf", "papel": "auto", "sobrescrever": True})
        print("  pdf", r.get("bytes"), "bytes,", r.get("paginas"), "páginas")
        v = m.call("cad_verificar", {"leiaute": True})
        print("leiaute:", json.dumps(v.get("leiaute", {}).get("contagem", v.get("contagem"))))
    m.fechar()


if __name__ == "__main__":
    principal()

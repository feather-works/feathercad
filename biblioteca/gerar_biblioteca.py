#!/usr/bin/env python3
"""Gera os componentes .fct de fábrica (pasta eletrica/) a partir do pack_eletrica.py — tamanho REAL em mm, origem no canto
inferior esquerdo, com `{{parâmetros}}` que o MCP troca ao colocar (`cad_componente`). Rode de novo para regerar; depois
`npm run mcp:build`. Componente novo também pode ser só um .fct escrito à mão (ou salvo pelo próprio FeatherCAD) largado aqui."""
import os, sys

AQUI = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(AQUI, "..", "exemplos"))
import pack_eletrica as pk
from pack_eletrica import Desenho

SAIDA = os.path.join(AQUI, "eletrica")


def gravar(nome, descricao, categoria, tamanho, parametros, corpo):
    cab = [f"// nome: {nome}", f"// descricao: {descricao}", f"// categoria: {categoria}", f"// tamanho: {tamanho} mm"]
    if parametros:
        cab.append("// parametros: " + " ".join(f"{k}={v}" for k, v in parametros.items()))
    os.makedirs(SAIDA, exist_ok=True)
    with open(os.path.join(SAIDA, nome + ".fct"), "w", encoding="utf-8") as f:
        f.write("\n".join(cab) + "\n" + corpo)


def desenhar(fn):
    d = Desenho()
    r = fn(d)
    return d.texto(), r


def main():
    for p in (1, 2, 3, 4):
        t, _ = desenhar(lambda d, p=p: pk.disjuntor(d, 0, 0, p, "{{corrente}}", "{{icc}}"))
        gravar(f"disjuntor_modular_{p}p", f"Disjuntor modular DIN de {p} polo(s), vista frontal, com corrente e capacidade de interrupção.", "quadro",
               f"{p * pk.MODULO:g} x {pk.ALTURA:g}", {"corrente": "C16", "icc": "6kA"}, t)
    for p in (2, 4):
        t, _ = desenhar(lambda d, p=p: pk.dr(d, 0, 0, p))
        gravar(f"dr_{p}p", f"Interruptor diferencial residual (DR) de {p} polos, 30 mA, com botão de teste.", "quadro", f"{p * pk.MODULO:g} x {pk.ALTURA:g}", {}, t)
    t, _ = desenhar(lambda d: pk.dps(d, 0, 0))
    gravar("dps", "Protetor contra surtos (DPS) de 1 módulo, com janela de estado e etiqueta.", "quadro", f"{pk.MODULO:g} x {pk.ALTURA:g}", {}, t)
    for c in (100, 200, 300):
        t, _ = desenhar(lambda d, c=c: pk.trilho_din(d, 0, 0, c))
        gravar(f"trilho_din_{c}", f"Trilho DIN de 35 mm com {c} mm de comprimento.", "quadro", f"{c} x {pk.TRILHO:g}", {}, t)
    for h in (100, 200):
        t, _ = desenhar(lambda d, h=h: pk.barra_de_bornes(d, 0, 0, h, pk.NEUTRO))
        gravar(f"barra_neutro_{h}", f"Barra de neutro (azul) com bornes, {h} mm, em pé.", "quadro", f"16 x {h}", {}, t)
        t, _ = desenhar(lambda d, h=h: pk.barra_de_bornes(d, 0, 0, h, pk.TERRA))
        gravar(f"barra_terra_{h}", f"Barra de terra (verde) com bornes, {h} mm, em pé.", "quadro", f"16 x {h}", {}, t)
    for n in (4, 8, 12):
        t, _ = desenhar(lambda d, n=n: pk.pente(d, 0, 0, n))
        gravar(f"pente_{n}", f"Barramento tipo pente para {n} polos.", "quadro", f"{n * pk.MODULO:g} x 18", {}, t)
    for larg, alt, gl in ((340, 340, (105, 155, 205, 255)), (460, 400, (120, 190, 260, 330))):
        t, _ = desenhar(lambda d, larg=larg, alt=alt, gl=gl: pk.caixa(d, 0, 0, larg, alt, gl))
        gravar(f"quadro_caixa_{larg}x{alt}", f"Caixa de quadro {larg} x {alt} mm com placa de montagem, parafusos e prensa-cabos.", "quadro", f"{larg} x {alt + 12}", {}, t)
    for vista in ("lateral", "frontal"):
        fn = pk.transformador_lateral if vista == "lateral" else pk.transformador_frontal
        t, _ = desenhar(lambda d, fn=fn: fn(d, 0, 0, 1.0, "{{kva}}"))
        gravar(f"transformador_{vista}", f"Transformador de distribuição a óleo (13,8 kV / 380-220 V) em elevação {vista}; tamanho de um 300 kVA (ajuste `ampliacao` para outras potências).",
               "subestacao", "1500 x 1750", {"kva": "300"}, t)
    print("ok:", len(os.listdir(SAIDA)), "componentes em", SAIDA)


if __name__ == "__main__":
    main()

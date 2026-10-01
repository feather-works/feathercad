#!/usr/bin/env python3
"""
Gera o Projeto Completo de Energia Solar Fotovoltaica no Padrão EDP (Modelo 1 - BT)
contendo:
1. Moldura A1 com Carimbo Oficial EDP preenchido
2. Diagrama Unifilar Informativo Modelo 1 EDP (BT)
3. Placa de Advertência / Sinalização Amarela EDP (25 x 15 cm)
4. Quadro de Dados da Unidade Consumidora / Acessante e Responsável Técnico
5. Memorial de Cálculo e Normas Aplicáveis (EDP / NDU 013 / ANEEL REN 1000 / NBR 5410)
"""

import os, sys, json
from mcp_cliente import Mcp

def gerar_fct_edp():
    fct = []
    fct.append("unidade mm")

    # ------------------------------------------------------------- 1. MOLDURA E CARIMBO (FOLHA A1 841x594)
    fct.append("camada FOLHA #FFFFFF")
    fct.append("R 0 0 841 594")
    fct.append("R 25 10 806 574") # Margem interna

    fct.append("camada CARIMBO #FFFFFF")
    # Divisórias verticais de dobramento A1
    for x in [210.6, 341.0, 471.4, 656.4]:
        fct.append(f"L {x} 584 {x} 594")
        fct.append(f"L {x} 0 {x} 10")
    fct.append("L 0 300 25 300")
    fct.append("L 831 300 841 300")

    # Faixa inferior de notas, legenda, revisões e carimbo
    fct.append("L 25 99 831 99")
    fct.append("L 165.3 10 165.3 99")
    fct.append("L 505.7 10 505.7 99")
    fct.append("L 656.4 10 656.4 99")
    fct.append("L 25 93 656.4 93")

    # Seção de Notas e Legenda
    fct.append('T 95.15 94.5 2.2 0 "NOTAS TÉCNICAS"')
    fct.append('T 335.5 94.5 2.2 0 "LEGENDA DE SIMBOLOGIA"')

    # Seção de Revisões
    for y in [92.9, 86.8, 80.7, 74.6, 68.5, 62.4, 56.3, 50.2, 44.1]:
        fct.append(f"L 505.7 {y} 656.4 {y}")
    for x in [515.3, 603.7, 615.7, 627.7, 640.7]:
        fct.append(f"L {x} 50.2 {x} 99")

    fct.append('T 507.7 52 1.7 0 "REV"')
    fct.append('T 545.3 52 1.7 0 "DESCRIÇÃO DA REVISÃO"')
    fct.append('T 605.2 52 1.7 0 "FINAL"')
    fct.append('T 618.2 52 1.7 0 "INI."')
    fct.append('T 630.2 52 1.7 0 "VIS."')
    fct.append('T 644.7 52 1.7 0 "DATA"')
    fct.append('T 581.05 45.6 2.2 0 "REVISÕES"')

    # Responsável Técnico / Empresa
    fct.append('T 507.7 39 1.9 0 "RESPONSÁVEL TÉCNICO / EMPRESA:"')
    fct.append('T 507.7 31 2.2 0 "ENG. ELETRICISTA - CREA-SP 123456/D"')
    fct.append('T 507.7 24 1.8 0 "SOLAR ENGENHARIA E CONEXÕES LTDA - CNPJ 00.000.000/0001-00"')

    # Divisórias do Carimbo Principal
    for y in [80.0, 72.5, 64.8, 57.1, 49.4, 41.7, 33.0]:
        fct.append(f"L 656.4 {y} 831 {y}")
    fct.append("L 744.9 80 744.9 99")
    fct.append("R 689.1 82 31.9 15") # Logos/Insígnias
    fct.append("R 776.8 82 22.3 15")
    for x in [685.1, 751.3, 816.6]:
        fct.append(f"L {x} 10 {x} 41.7")

    # Textos do Carimbo
    fct.append('T 657.9 75 1.6 0 "PROJETO"')
    fct.append('T 657.9 67.2 2.8 0 "CONCESSIONÁRIA EDP S.A."')
    fct.append('T 657.9 59.5 2.8 0 "PROJETO DE ACESSO - MICROGERAÇÃO (BT)"')
    fct.append('T 657.9 51.8 2.8 0 "ENERGIA SOLAR FOTOVOLTAICA - EDP"')
    fct.append('T 657.9 46 2.6 0 "DIAGRAMA UNIFILAR E PLACA DE ADVERTÊNCIA"')
    fct.append('T 657.9 42.6 1.9 0 "SISTEMA FOTOVOLTAICO CONECTADO À REDE (ON-GRID)"')

    fct.append('T 657.9 35 1.5 0 "ESCALA"')
    fct.append('T 686.4 35 1.5 0 "Nº DO DESENHO"')
    fct.append('T 752.9 35 1.5 0 "FOLHA"')
    fct.append('T 817.9 35 1.5 0 "REV."')

    fct.append('T 657.9 20 3.4 0 "1:1"')
    fct.append('T 686.4 20 3.4 0 "SOL-EDP-01"')
    fct.append('T 752.9 20 3.4 0 "01/01"')
    fct.append('T 821.4 20 3.4 0 "00"')

    # ------------------------------------------------------------- 2. QUADROS PRINCIPAIS DO DESENHO
    fct.append("camada QUADROS #7F7F7F")
    fct.append("R 35 105 460 460 #3A80D0") # Quadro do Diagrama Unifilar EDP
    fct.append("R 510 325 315 240 #3A80D0") # Quadro da Placa de Advertência EDP
    fct.append("R 510 105 315 210 #3A80D0") # Quadro de Dados da UC e Dimensionamento

    # Títulos das Pranchas
    fct.append("camada TITULOS #0088FF")
    fct.append('T 40 550 3.5 0 "1. DIAGRAMA UNIFILAR / INFORMATIVO - MODELO 1 (EDP)" #0088FF')
    fct.append('T 515 550 3.5 0 "2. PLACA DE SINALIZAÇÃO / ADVERTÊNCIA (NORMA EDP)" #0088FF')
    fct.append('T 515 300 3.5 0 "3. DADOS DA UNIDADE CONSUMIDORA E DENSIONAMENTO" #0088FF')

    # ------------------------------------------------------------- 3. DIAGRAMA UNIFILAR EDP (MODELO 1)
    fct.append("camada REDE_EDP #00AAFF")
    # Linha da Rede BT EDP
    fct.append("L 50 510 470 510 #00AAFF")
    fct.append("L 50 511.5 470 511.5 #00AAFF")
    fct.append('T 170 515 3.0 0 "Rede Secundária (BT) da Concessionária EDP (220/127V ou 380/220V - 60 Hz)" #00AAFF')

    # Fronteira Acessada / Acessante (Linha Tracejada em Y=485)
    fct.append("camada FRONTEIRA #4AA3FF")
    fct.append("L 50 485 470 485 #4AA3FF")
    fct.append('T 380 493 2.2 0 "▲ Acessada (Rede EDP)" #4AA3FF')
    fct.append('T 380 474 2.2 0 "▼ Acessante (Unidade Consumidora)" #4AA3FF')

    # Ramal de Entrada Principal
    fct.append("camada DIAGRAMA #FFFFFF")
    fct.append("L 200 510 200 140")

    # Caixas de Medição / Padrão EDP (Linha tracejada azul)
    fct.append("camada PADRAO_EDP #FF9900")
    fct.append("R 130 380 200 95 #FF9900")
    fct.append('T 335 385 2.2 0 "Padrão de Entrada EDP" #FF9900')

    # Transformador de Medição / Medidor Bidirecional EDP
    fct.append("R 240 430 40 25")
    fct.append('T 245 440 2.2 0 "Medição" #FFFFFF')
    fct.append('T 135 445 2.0 0 "Caixa de Medição" #FF9900')
    fct.append('T 135 437 1.8 0 "(Tampa Transparente)" #FF9900')

    # Linhas de conexão da medição
    fct.append("L 200 455 240 455")
    fct.append("L 200 430 240 430")

    # Disjuntor de Entrada Padrão EDP / Seccionamento Visível
    fct.append("C 200 405 2.5")
    fct.append("C 200 395 2.5")
    fct.append("L 200 402.5 194 397.5") # Contato aberto
    fct.append('T 208 403 2.2 0 "Disjuntor de Entrada / Padrão EDP (63A)" #FFFFFF')
    fct.append('T 208 393 2.0 0 "Dispositivo de Seccionamento Visível" #FF9900')

    # Aviso Oficial EDP (Balão / Texto de Atenção)
    fct.append("camada AVISO_EDP #FFCC00")
    fct.append("L 200 398 320 460 #FFCC00")
    fct.append("L 320 460 465 460 #FFCC00")
    fct.append('T 320 467 2.2 0 "ATENÇÃO:" #FFCC00')
    fct.append('T 320 457 1.8 0 "MODELO A SER ADOTADO QUANDO O MEDIDOR" #FFFFFF')
    fct.append('T 320 450 1.8 0 "ESTIVER INSTALADO EM CAIXA COM TAMPA" #FFFFFF')
    fct.append('T 320 443 1.8 0 "TRANSPARENTE (MEDIÇÃO DIRETA ATÉ 100A)" #FFFFFF')

    # Derivação para Cargas
    fct.append("camada DIAGRAMA #FFFFFF")
    fct.append("L 200 340 330 340")
    fct.append("L 330 340 330 310")
    fct.append("L 330 310 327 317")
    fct.append("L 330 310 333 317")
    fct.append('T 320 300 2.2 0 "Cargas (QDG da UC)" #FFFFFF')

    # Disjuntor de Proteção CA do Inversor
    fct.append("C 200 280 2.5")
    fct.append("C 200 270 2.5")
    fct.append("L 200 277.5 194 272.5")
    fct.append('T 208 275 2.2 0 "Dispositivo de Proteção CA (Disjuntor 25A / DPS)" #FFFFFF')

    # Inversor On-Grid (Simbolo CA / CC)
    fct.append("camada INVERSOR #7AE07A")
    fct.append("R 175 200 50 40 #7AE07A")
    fct.append("L 175 200 225 240 #7AE07A")
    fct.append('T 182 225 2.4 0 "CA" #FFFFFF')
    fct.append('T 210 208 2.4 0 "CC" #FFFFFF')
    fct.append('T 232 230 2.4 0 "Inversor Solar On-Grid" #7AE07A')
    fct.append('T 232 221 1.9 0 "Marca: GROWATT / WEG" #FFFFFF')
    fct.append('T 232 213 1.9 0 "Modelo: MIN 5000TL-X" #FFFFFF')
    fct.append('T 232 205 1.9 0 "Potência CA: 5,0 kW | Quantidade: 1 UN" #FFFFFF')

    # Gerador Fotovoltaico (Símbolo G)
    fct.append("camada GERADOR #FFCC00")
    fct.append("L 200 200 200 160")
    fct.append("C 200 145 15 #FFCC00")
    fct.append('T 196 141 3.5 0 "G" #FFCC00')
    fct.append('T 222 155 2.4 0 "Gerador Fotovoltaico (Arranjo CC)" #FFCC00')
    fct.append('T 222 146 1.9 0 "Marca: CANADIAN SOLAR / LONGI" #FFFFFF')
    fct.append('T 222 138 1.9 0 "Modelo: CS6W-550MS (550 Wp)" #FFFFFF')
    fct.append('T 222 130 1.9 0 "Qtd: 10 Módulos (2 Strings x 5 Módulos)" #FFFFFF')
    fct.append('T 222 122 1.9 0 "Potência Total CC: 5,50 kWp" #FFFFFF')

    # ------------------------------------------------------------- 4. PLACA DE ADVERTÊNCIA / SINALIZAÇÃO EDP (25 x 15 cm)
    # Caixa base: X de 540 a 790 (Largura 250), Y de 360 a 510 (Altura 150)
    # Centro X = 665
    
    # 1. Fundo Amarelo Total recortado (Corpo central + abas sem invadir os cantos de R=14)
    fct.append("camada PLACA_FUNDO #FFE600")
    fct.append("H solida 540 374 790 374 790 496 540 496 #FFE600") # Bloco central (toda largura, exceto topos das curvas)
    fct.append("H solida 554 360 776 360 776 510 554 510 #FFE600") # Bloco vertical cruzado (preenche o centro até as bordas retas)
    # Preenchimento dos 4 quadrantes dos cantos arredondados
    for cx, cy in [(554, 496), (776, 496), (554, 374), (776, 374)]:
        fct.append(f"C {cx} {cy} 14 #FFE600")
        fct.append(f"H solida {cx-14} {cy-14} {cx+14} {cy-14} {cx+14} {cy+14} {cx-14} {cy+14} #FFE600")

    # 2. Contorno Externo Preto com Cantos Arredondados (R = 14 mm)
    fct.append("camada PLACA_CONTORNO #000000")
    fct.append("L 554 510 776 510 #000000") # Superior
    fct.append("L 790 496 790 374 #000000") # Direita
    fct.append("L 776 360 554 360 #000000") # Inferior
    fct.append("L 540 374 540 496 #000000") # Esquerda
    fct.append("A 554 496 14 90 180 #000000")  # Canto Sup. Esq.
    fct.append("A 776 496 14 0 90 #000000")   # Canto Sup. Dir.
    fct.append("A 776 374 14 270 360 #000000") # Canto Inf. Dir.
    fct.append("A 554 374 14 180 270 #000000") # Canto Inf. Esq.

    # 3. Tarja Preta Superior do "CUIDADO"
    # Fica com uma margem de ~7mm da borda amarela, X: 547 a 783 (L=236), Y: 452 a 503 (A=51)
    fct.append("camada PLACA_TARJA #000000")
    fct.append("R 547 452 236 51 #000000")
    fct.append("H solida 547 452 783 452 783 503 547 503 #000000")

    # 4. Texto CUIDADO (Grande, amarelo, bem destacado e centrado na tarja)
    fct.append("camada PLACA_TEXTO_CUIDADO #FFE600")
    # Para dar espessura de negrito (bold stroke):
    for dx in [0.0, 0.4, 0.8]:
        fct.append(f'T {558.5 + dx:.1f} 462 31.0 0 "CUIDADO" #FFE600')

    # 5. Parafusos com fenda Phillips nos 4 cantos
    fct.append("camada PLACA_PARAFUSOS #7F7F7F")
    for px, py in [(547.5, 502.5), (782.5, 502.5), (547.5, 367.5), (782.5, 367.5)]:
        fct.append(f"C {px} {py} 4.8 #555555")
        fct.append(f"H solida {px-4.8:.1f} {py-4.8:.1f} {px+4.8:.1f} {py-4.8:.1f} {px+4.8:.1f} {py+4.8:.1f} {px-4.8:.1f} {py+4.8:.1f} #9E9E9E")
        fct.append(f"C {px} {py} 4.8 #1A1A1A")
        # Fenda em cruz (+)
        fct.append(f"L {px-3.2:.1f} {py} {px+3.2:.1f} {py} #2A2A2A")
        fct.append(f"L {px} {py-3.2:.1f} {px} {py+3.2:.1f} #2A2A2A")
        fct.append(f"L {px-2.5:.1f} {py-2.5:.1f} {px+2.5:.1f} {py+2.5:.1f} #3A3A3A")
        fct.append(f"L {px-2.5:.1f} {py+2.5:.1f} {px+2.5:.1f} {py-2.5:.1f} #3A3A3A")

    # 6. Textos Pretos Centrais (Caixa alta, proporção e posicionamento idênticos ao modelo)
    fct.append("camada PLACA_TEXTO_CORPO #000000")
    # Linha 1: RISCO DE CHOQUE (Y = 428)
    for dx in [0.0, 0.3]:
        fct.append(f'T {560.0 + dx:.1f} 428 13.5 0 "RISCO DE CHOQUE" #000000')

    # Linha 2: ELÉTRICO (Y = 405)
    for dx in [0.0, 0.3]:
        fct.append(f'T {607.0 + dx:.1f} 405 13.5 0 "ELÉTRICO" #000000')

    # Linha 3: GERAÇÃO PRÓPRIA (Y = 380)
    for dx in [0.0, 0.3]:
        fct.append(f'T {558.0 + dx:.1f} 380 13.8 0 "GERAÇÃO PRÓPRIA" #000000')

    # Linha 4: (ENDEREÇO) (Y = 364)
    for dx in [0.0, 0.3]:
        fct.append(f'T {596.0 + dx:.1f} 364 12.0 0 "(ENDEREÇO)" #000000')

    # 7. Cotas Azuis Simples (Conforme a imagem: "25" e "15")
    fct.append("camada COTAS_PLACA #0033FF")
    # Cota Horizontal 25 (Superior)
    fct.append("L 540 523 790 523 #0033FF")
    fct.append("L 540 520 540 526 #0033FF")
    fct.append("L 790 520 790 526 #0033FF")
    # Setas da cota superior
    fct.append("L 540 523 547 525 #0033FF")
    fct.append("L 540 523 547 521 #0033FF")
    fct.append("L 790 523 783 525 #0033FF")
    fct.append("L 790 523 783 521 #0033FF")
    fct.append('T 661 526 4.5 0 "25" #0033FF')

    # Cota Vertical 15 (Lateral direita)
    fct.append("L 804 360 804 510 #0033FF")
    fct.append("L 801 360 807 360 #0033FF")
    fct.append("L 801 510 807 510 #0033FF")
    # Setas da cota vertical
    fct.append("L 804 510 802 503 #0033FF")
    fct.append("L 804 510 806 503 #0033FF")
    fct.append("L 804 360 802 367 #0033FF")
    fct.append("L 804 360 806 367 #0033FF")
    # Texto "15" na vertical acompanhando a cota
    fct.append('T 807 432 4.5 90 "15" #0033FF')

    # ------------------------------------------------------------- 5. DADOS DA UC E DENSIONAMENTO
    fct.append("camada MEMORIAL #FFFFFF")
    # Tabela de Dados Técnicos
    fct.append("R 515 112 305 175 #7F7F7F")
    for y in [262, 237, 212, 187, 162, 137]:
        fct.append(f"L 515 {y} 820 {y} #7F7F7F")
    fct.append("L 650 112 650 287 #7F7F7F")

    # Cabeçalhos da Tabela
    fct.append('T 520 272 2.2 0 "PARAMETRO TÉCNICO" #0088FF')
    fct.append('T 655 272 2.2 0 "ESPECIFICAÇÃO / VALOR ADOTADO" #0088FF')

    # Linhas de Dados
    fct.append('T 520 247 1.9 0 "Concessionária:" #FFFFFF')
    fct.append('T 655 247 1.9 0 "EDP (Distribuidora de Energia)" #FFCC00')

    fct.append('T 520 222 1.9 0 "Código da UC / Cliente:" #FFFFFF')
    fct.append('T 655 222 1.9 0 "123456789 / CLIENTE EXEMPLO EDP" #FFFFFF')

    fct.append('T 520 197 1.9 0 "Potência Gerador (CC):" #FFFFFF')
    fct.append('T 655 197 1.9 0 "5,50 kWp (10 Módulos de 550 Wp)" #7AE07A')

    fct.append('T 520 172 1.9 0 "Potência Inversor (CA):" #FFFFFF')
    fct.append('T 655 172 1.9 0 "5,00 kW (Tensão 220V/380V - 60 Hz)" #7AE07A')

    fct.append('T 520 147 1.9 0 "Estimativa de Geração:" #FFFFFF')
    fct.append('T 655 147 1.9 0 "~660 kWh/mês (HSP Média: 4,8 kWh/m².dia)" #FFFFFF')

    fct.append('T 520 122 1.9 0 "Normas de Referência:" #FFFFFF')
    fct.append('T 655 122 1.8 0 "EDP NDU 013 / ANEEL REN 1.000 / NBR 5410" #4AA3FF')

    # ------------------------------------------------------------- 6. NOTAS TÉCNICAS DA EDP (NA FAIXA INFERIOR)
    fct.append("camada NOTAS #E0E0E0")
    fct.append('T 30 85 1.8 0 "1. O sistema de microgeração fotovoltaica opera em paralelo com a rede da EDP sem injeção de harmônicos nocivos."')
    fct.append('T 30 78 1.8 0 "2. O inversor possui proteção contra ilhamento automática em conformidade com as normas NBR IEC 62116 e NDU EDP."')
    fct.append('T 30 71 1.8 0 "3. Os condutores CC são específicos para energia solar fotovoltaica 1kV resistentes a intempéries e raios UV."')
    fct.append('T 30 64 1.8 0 "4. O aterramento das carcaças dos módulos e inversor está interligado à malha de aterramento da entrada de energia."')
    fct.append('T 30 57 1.8 0 "5. A medição bidirecional será instalada pela Concessionária EDP após homologação do parecer de acesso."')
    fct.append('T 30 50 1.8 0 "6. A placa de advertência será fixada na caixa de medição antes da vistoria técnica da EDP."')

    return "\n".join(fct) + "\n"

if __name__ == "__main__":
    pasta_destino = "/home/maragon/Documentos/FeatherCAD"
    os.makedirs(pasta_destino, exist_ok=True)
    
    texto = gerar_fct_edp()
    caminho_fct = os.path.join(pasta_destino, "projeto_solar_edp.fct")
    with open(caminho_fct, "w", encoding="utf-8") as f:
        f.write(texto)
    print(f"FCT salvo em: {caminho_fct}")

    m = Mcp()
    m.call("cad_novo", {"unidade": "mm"})
    m.call("cad_importar", {"texto": texto, "substituir": True})
    
    for ext in ("fcdz", "dxf"):
        caminho_saida = os.path.join(pasta_destino, f"projeto_solar_edp.{ext}")
        m.call("cad_salvar", {"caminho": caminho_saida, "sobrescrever": True})
        print(f"Salvo: {caminho_saida}")
        
    caminho_pdf = os.path.join(pasta_destino, "projeto_solar_edp.pdf")
    m.call("cad_pdf", {"caminho": caminho_pdf, "papel": "A1", "cores": True, "sobrescrever": True})
    print(f"PDF Gerado: {caminho_pdf}")
    
    resumo = m.call("cad_resumo")
    print(f"Entidades criadas: {resumo.get('total_de_entidades')}")
    m.fechar()

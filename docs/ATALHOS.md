# Atalhos do FeatherCAD — a mesma tabela do AutoCAD

Decisão do usuário (29/09/2026): **manter os mesmos atalhos do AutoCAD**. A
tabela abaixo é a referência (versão em português, como colada pelo usuário).
Nada aqui está implementado por inteiro: entra no motor **um comando por vez**,
junto com o comando dele.

## Como isto se reparte

Dois tipos de atalho, dois lugares — e os dois são do **Rust**:

| --- | --- | --- | --- |
| **Tecla** (`Ctrl+G`, `F8`, `Esc`) | alterna grade | `feathercad-core/src/atalhos.rs`; a tecla é o mesmo em toda língua | traduz o evento do teclado em texto normalizado (`Ctrl+G`, `F8`, `Ctrl+Shift+H`) e manda ao Rust (`atalho`); mostra o estado que volta. **Não decide** o que a tecla faz |
| **Apelido digitado** (`L`, `TR`, `CO`) | `L` = `LINHA` | `src/idiomas/comandos.<língua>.json` → `idioma_definir` → `Comandos` no Rust | manda o texto da linha de comando; mostra o prompt que volta |

- Os **apelidos mudam por língua** (a tabela abaixo é a do pt-BR: `L` = `LINHA`;
  no en-US será `L` = `LINE`). A tecla não muda.
- Comando ainda sem motor **não entra** no JSON de língua nem no menu (regra
  "nada morto"): a linha desta tabela espera aqui até o comando existir; aí o
  apelido migra para o `comandos.<língua>.json`.
- Um teste trava: todo id do JSON de língua tem que ser comando que o Rust
  conhece, e toda tecla de `atalhos.rs` tem que aparecer nesta tabela.

## Teclas

### Alternar e tela

| Tecla | Faz |
| --- | --- |
| Ctrl+D | alterna exibição de coordenadas |
| Ctrl+G | alterna grade |
| Ctrl+E | percorre planos isométricos |
| Ctrl+F | alterna snaps ao objeto em execução |
| Ctrl+H | alterna "Selecionar estilo" |
| Ctrl+Shift+H | alterna ocultar paletas |
| Ctrl+I | alterna coordenadas |
| Ctrl+Shift+I | alterna inferir restrições |
| F1 | ajuda |
| F2 | alterna tela de texto |
| F3 | alterna modos de snap ao objeto (OSNAP) |
| F4 | alterna Osnap3D |
| F5 | alterna plano isométrico |
| F6 | alterna UCS dinâmico |
| F7 | alterna modo de grade |
| F8 | alterna modo ortogonal |
| F9 | alterna modo de snap |
| F10 | alterna modo polar |
| F11 | alterna rastreamento de snap ao objeto |
| F12 | **Monitor** (memória, console e métricas) — **desvio do AutoCAD, decidido pelo usuário em 29/09/2026** (copiado do FeatherSheet). No AutoCAD é a entrada dinâmica; aqui ela ganha outra tecla (proposta: `Shift+F12`) quando o botão existir. Também abrem o Monitor: `Alt+C`, `Ctrl+Shift+J`, `Ctrl+Shift+M`. É tratada pela interface (abrir janela não é comando do desenho), não pelo motor |
| Ctrl+0 | limpar tela |
| Ctrl+1 | paleta Propriedades |
| Ctrl+2 | paleta Design Center |
| Ctrl+3 | alterna paleta (de ferramentas) |
| Ctrl+4 | paleta Conjunto de folhas |
| Ctrl+6 | gerenciador de conexões a banco de dados |
| Ctrl+7 | paleta Gerenciador de conjunto de revisões |
| Ctrl+8 | QuickCalc |
| Ctrl+9 | linha de comando |

### Edição, arquivo e janela

| Tecla | Faz |
| --- | --- |
| Ctrl+C / Ctrl+X / Ctrl+V | copiar / cortar / colar objeto |
| Ctrl+Shift+C | copiar para a área de transferência com ponto base |
| Ctrl+Shift+V | colar dados como bloco |
| Ctrl+Z / Ctrl+Y | desfazer / refazer |
| Esc, Ctrl+[ , Ctrl+\ | cancelar o comando atual |
| Delete | apagar os objetos selecionados (o comando `APAGAR`); com o cursor na linha de comando, apaga o texto e não o desenho |
| Ctrl+N / Ctrl+O / Ctrl+S | novo / abrir / salvar desenho |
| Ctrl+Shift+S | salvar como (padrão do AutoCAD) |
| Ctrl+P | caixa de diálogo Plotar |
| Ctrl+Tab / Ctrl+Shift+Tab | próximo / desenho anterior |
| Ctrl+Page Up / Page Down | guia anterior / próxima no desenho atual |
| Ctrl+Q | sair |
| Ctrl+A | selecionar todos os objetos |

**Conflitos a resolver quando chegar a hora** (o AutoCAD tem os mesmos):
`Ctrl+D` e `Ctrl+I` fazem a mesma coisa (coordenadas); `F1` da suíte inteira é
Ajuda (já é assim no FeatherSheet, sem conflito); `Ctrl+Q` sair e `Ctrl+A`
selecionar tudo valem só com o foco na área de desenho.

## Apelidos digitados (pt-BR)

Formato: `apelido` → `COMANDO` — o que faz. Onde o comando tem dois apelidos,
os dois aparecem. **Já no motor:** `L` → `LINHA` (e `DESFAZER`, `REFAZER`).

### A – C
`A` ARC (arco) · `AA` AREA (área e perímetro) · `ADC` ADCENTER (blocos, refexs,
hachuras) · `AL` ALINHAR · `AP` CARRAPLIC (carregar aplicativo) · `AR` MATRIZ ·
`ARR` GRAVARACAO (gravador de ações) · `ARM` MENSAGEMUSUARIOACAO · `ARU`
ENTRADAUSUARIOACAO · `ARS` PARARACAO · `ATI` EDITATRIBNOLOCAL · `ATT`
DEFATRIB · `ATE` EDITARATRIB · `B` BLOCO · `BC` FECHARBLOCO · `BE` EDITARBLOCO
· `BH` HACHURA · `BO` LIMITE · `BR` QUEBRAR · `BS` SALVARBLOCO · `BVS`
ESTADOVISBLOCO · `C` CIRCULO · `CAM` CAMERA · `CBAR` BARRARESTRIC · `CH`
PROPRIEDADES · `CHA` CHANFRO · `CHK` VERIFICANORMAS · `CLI` LINHACOMANDO ·
`COL` COR · `CO` COPIAR · `CT` CTABLESTYLE · `CUBE` CUBONAV · `CYL` CILINDRO.

### D – F
`D` ESTILOCOTA · `DAN` COTAANG · `DAR` COTAARCO · `DBA` COTALINBASE · `DBC`
CONECTARBD · `DCE` COTACENTRO · `DCO` COTASERIE · `DCON` RESTRICAOCOTA · `DDA`
COTADESASSOCIAR · `DDI` COTADIAMETRO · `DED` EDITARCOTA · `DI` DIST · `DIV`
DIVIDIR · `DJL` COTALINDESVIO · `DJO` COTACOMDESVIO · `DL` VINCULODADOS · `DLU`
ATUALIZARVINCULODADOS · `DO` ANEL · `DOR` COTAORDEN · `DOV` SOBRECOTA · `DR`
ORDEMDES · `DRA` COTARAIOS · `DRE` REASSOCIARCOTA · `DRM` RECUPERARDESENHO ·
`DS` CONFIGDES (grade, snap, polar, osnap, entrada dinâmica) · `DT` TEXTO (linha
única) · `DV` VISTAD · `DX` EXTRACAODADOS · `E` APAGAR · `ED` EDITARD · `EL`
ELIPSE · `EPDF` EXPORTARPDF · `ER` REFERENCIASEXTERNAS · `EX` ESTENDER · `EXIT`
SAIR · `EXP` EXPORTAR · `EXT` EXTRUSAO · `F` CONCORD (concordância) · `FI`
FILTRO · `FS` FSMODE · `FSHOT` GEOMPLANA.

### G – I
`G` GRUPO · `GCON` GEOCONSTRAINT · `GD` GRADIENTE · `GEO` LOCALIZGEOGRAFICA ·
`H` HACHURA · `HE` EDITARHACH · `HI` OCULTAR · `I` INSERIR · `IAD` AJUSTARIMAG ·
`IAT` ANEXARIMAG · `ICL` CORTARIMAG · `ID` ID (coordenada de um ponto) · `IM`
IMAGEM · `IMP` IMPORTAR · `IN` INTERSEC · `INF` INTERFERE · `IO` INSERIROBJ.

### J – L
`J` UNIR · `JOG` COTACOMDESVIO · **`L` LINHA** · `LA` CAMADA · `LAS`
ESTADOCAMADA · `LE` LINHACHAMR · `LEN` ALONGAR · `LESS` SUAVIZARMALHAMENOS ·
`LI` LISTA · `LO` LAYOUT · `LT` TIPOLINHA · `LTS` ESCALATL · `LW` ESPLINHA.

### M – O
`M` MOVER · `MA` MATCHPROP · `MAT` MATERIAIS · `ME` MEDIDA · `MEA` MEDIRGEOM ·
`MI` ESPELHAR · `ML` MLINHA · `MLA` ALINHARLINCHMULT · `MLC` CONJLINCHMULT ·
`MLD` LINCHMULT · `MLE` EDITARLINCHMULT · `MLS` ESTILOLINCHMULT · `MO`
PROPRIEDADES · `MORE` SUAVIZARMALHAMAIS · `MS` ESPACOM · `MSM` REVISAO · `MT`
TEXTOM (texto multilinha) · `MV` CVISTA · `NORTH` LOCALIZGEOGRAFICA · `NSHOT`
NOVOINSTANT · `NVIEW` NOVAVISTA · `O` DESLOCAMENTO (offset) · `OP` OPCOES ·
`ORBIT` ORBITA3D · `OS` OSNAP.

### P – R
`P` PAN ⚠ · `PA` COLARESPEC · `PAR` PARAMETROS · `PARAM` PARAMBLOCO · `PATCH`
CORRECAOSUPERF · `PC` POINTCLOUD · `PCATTACH` ANEXARNUVEMPONTOS · `PCINDEX`
INDICENUVEMPONTOS · `PE` EDITARP · `PL` PLINHA (polilinha 2D) · `PO` PONTO ·
`POFF` OCULTARPALETAS · `POL` POLIGONO · `PON` MOSTRARPALETAS · `PR`
PROPRIEDADES · `PRE` VISUALIZAR · `PRINT` PLOTAR · `PS` ESPACOP · `PSOLID`
POLISSÓLIDO · `PU` ELIMINAR (purge) · `PYR` PIRAMIDE · `QC` CALCRAPIDA · `QCUI`
QUICKCUI · `QP` PROPRIEDADESRAPIDAS ⚠ · `QSAVE` SALVARR · `QVD` VRDESENHO ·
`QVDC` FECHARVRDESENHO · `QVL` VRLAYOUT · `QVLC` FECHARVRLAYOUT · `R` REDESEN ·
`RA` REDESTUDO · `RC` RECORTARENDERIZ · `RE` REGEN · `REA` REGENTUDO · `REC`
RETANGULO · `REG` REGIAO · `REN` RENOMEAR · `REV` REVOLUCAO · `RO` ROTAC ·
`RP` PREDEFRENDERIZ · `RPR` PREFACABAM · `RR` RENDER · `RW` JANELARENDERIZ.

### V – Z
`V` VISTA · `VGO` IRPARAVISTA · `VP` DDVPOINT · `VPLAY` REPROVISTA · `VS`
EVATUAL · `VSM` ESTILOSVISUAIS · `W` GBLOCO (gravar bloco) · `WE` CUNHA ·
`WHEEL` CIRCULONAV · `X` EXPLODIR · `XA` ANEXAREFEX · `XB` ASSREFEX · `XC`
CORTAREFEX · `XL` LINHAINF · `XR` REFEX · `ZEBRA` ANALISEZEBRA · `ZIP`
ETRANSMIT.

## ⚠ Observações sobre a fonte

A tabela veio de material do AutoCAD em português com alguns nomes de comando
que **não batem com a descrição** — conferir antes de implementar:

- `P` aparece como "PAN / Adiciona um parâmetro com alças a um bloco
  dinâmico": a descrição é do `PARAM`. `P` = `PAN` (deslocar a vista).
- `QP` aparece com a descrição de `QVD` (visualização prévia); no AutoCAD é
  `PROPRIEDADESRAPIDAS`.
- `IM` aparece como "exibe a paleta Referências externas" (descrição de `ER`);
  é `IMAGEM`.
- `MO`, `CH`, `PR` são três apelidos do mesmo comando (`PROPRIEDADES`), e `H`
  e `BH` do mesmo `HACHURA`, `T` e `MT` do mesmo `TEXTOM`, `DJO` e `JOG` do
  mesmo `COTACOMDESVIO`: **vários apelidos para um comando é normal**, o
  JSON de língua já aceita lista.
- Nomes que ficaram em inglês na tabela (`CTABLESTYLE`, `POINTCLOUD`,
  `MATCHPROP`, `FSMODE`, `GEOCONSTRAINT`, `TILEMODE`, `UNISOLATEOBJECTS`,
  `DEFVAR`…) provavelmente não são o nome em português; decidir o nome na hora
  de implementar cada um.
- O apelido `SO` (SOLIDO 2D) e `SL` (FATIAR) etc. são 3D — o FeatherCAD é 2D
  na primeira fase; os comandos 3D ficam registrados mas sem previsão.

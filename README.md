# FeatherCAD

> **Produto independente**, em repositório próprio desde 16/11/2026 (`feather-works/feathercad`). Versão **alpha**: ainda em desenvolvimento.

## Baixar

| Sistema | Arquivo | Como instalar |
|---|---|---|
| **Windows 10/11** | [FeatherCAD-windows-x64.exe](https://github.com/feather-works/feathercad/releases/latest/download/FeatherCAD-windows-x64.exe) | rode o instalador |
| **Linux** (Ubuntu, Debian, Mint…) | [feathercad-linux-amd64.deb](https://github.com/feather-works/feathercad/releases/latest/download/feathercad-linux-amd64.deb) | `sudo apt install ./feathercad-linux-amd64.deb` |
| **Linux** (qualquer distribuição, sem administrador) | [feathercad-linux-x86_64.tar.gz](https://github.com/feather-works/feathercad/releases/latest/download/feathercad-linux-x86_64.tar.gz) | extraia e rode `sh instalar.sh` (vai para `~/.local`; é o pacote que o FeatherWorks Launcher usa). Gerado do `.deb` por `scripts/tar-linux.sh`. |

**CAD 2D:** formatos `.fcdz` e `.dxf`, com integração MCP planejada/desenvolvida conforme o estado do código; versões próprias.

**Arquitetura compartilhada:** utiliza a base de motores Rust em [`featheroffice/`](https://github.com/M4R4G0N/FeatherOffice/tree/main/featheroffice) e os componentes de interface Rust/Iced de [`feather-ui`](https://github.com/feather-works/feather-ui) quando aplicáveis. As dependências compartilhadas serão versionadas sem acoplar um produto ao código interno de outro.

**Publicação:** primeira versão alpha (`0.1.0`) em 16/11/2026. O produto terá distribuição própria e também poderá ser selecionado isoladamente ou junto aos demais em um **instalador geral planejado**. Isso não significa que a funcionalidade de instalação já esteja pronta.


**Direção atual — 07/10/2026: motor e interface 100% Rust, com Iced.**
Prioridade: **FeatherSlide → FeatherDocs → FeatherSheet**; FeatherCAD
depois, se houver tempo. O inventário abaixo registra funcionalidades e
decisões anteriores; a migração da interface deste aplicativo ainda deve
ser conferida no código, sem presumir que os recursos já existem em Iced.

## Formatos

- **`.fcdz`** — formato próprio, contêiner com Git embutido (contrato em
  `FORMATO.md` da raiz). Projetado para ser extremamente eficiente.
- **`.dxf`** — formato universal de intercâmbio. Abre e salva.
- **Salvamento padrão configurável** pelo usuário nas configurações. Decisão
  de 29/09/2026: o padrão de fábrica é **`.dxf`**, por ser universal; o `.fcdz`
  fica como opção de quem quer o formato eficiente.

## Referência

- `CLAUDE.md` — regras de trabalho e fila.
- [Avaliação de precisão CAD × Drawing](https://github.com/M4R4G0N/FeatherOffice/blob/main/featheroffice/AVALIACAO-DRAWING-FEATHERCAD.md) (APP-013) — decisão de manter geometria, snap e persistência em `f64`; apenas projeção visual pode usar `f32` após `Vista::para_tela`. Contém matriz de compatibilidade, testes e critérios de integração. **Adoção da GUI não concluída.**
- `docs/INTERFACE.md` — composição da tela e Ribbon com sub-abas.
- `docs/DESDE-O-DIA-1.md` — língua em JSON e demais lições do FeatherSheet.
- `docs/DXF-REFERENCIA.md` — estrutura do DXF extraída do ezdxf
  ("DXF Internals"). Partes marcadas ⚠ estão só resumidas; abrir a URL antes
  de implementar.

## Versões de DXF (decidido 29/09/2026)

- **Lê** R12 e R2000+ (o R13/R14 entra como R2000). Ignora código de grupo
  que não conhece e não confia na ordem das tags, como manda a própria doc.
- **(Estado em 29/09/2026: só grava R12/AC1009 — o R2000+ espera poder ser validado; ver `CLAUDE.md`, fila 3.)**
- **Grava na versão que o usuário escolher**, e o padrão de fábrica é o modelo
  mais recente (R2018, `AC1032`). A escolha vale por arquivo (ao salvar como) e
  como padrão nas configurações, junto com a escolha `.dxf` / `.fcdz`.
  - Os dois modelos de gravação são o R12 (simples: só nomes, sem handles) e o
    R2000+ (handles, CLASSES, BLOCK_RECORD, OBJECTS com `DICTIONARY` raiz,
    `LAYOUT` e `VIEWPORT` — ver a referência). Versões intermediárias
    (R2004, R2007, R2010, R2013) mudam pouco em cima do R2000+.
  - Gravar numa versão mais antiga pode perder o que ela não tem (ex.: R12 não
    tem LWPOLYLINE, HATCH, SPLINE nem MTEXT). O motor precisa avisar o que foi
    simplificado ou perdido, e a interface só mostra o aviso.
  - Arquivo com um handle errado ou um objeto obrigatório faltando é recusado
    pelo AutoCAD, então o gravador precisa de teste de ida e volta.

## Em aberto

- Importar, exportar ou os dois já na primeira fase.

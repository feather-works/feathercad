# FeatherCAD

[![Discord](https://img.shields.io/badge/Discord-Maragon.Dev-5865F2?logo=discord&logoColor=white)](https://discord.gg/b4Epb4eSSH)

**2D CAD of the FeatherWorks suite, with the engine written in Rust.** Alpha: still in development.

## Download

| System | File | How to install |
|---|---|---|
| **Windows 10/11** | [FeatherCAD-windows-x64.exe](https://github.com/feather-works/feathercad/releases/latest/download/FeatherCAD-windows-x64.exe) | run the installer |
| **Linux** (Ubuntu, Debian, Mint…) | [feathercad-linux-amd64.deb](https://github.com/feather-works/feathercad/releases/latest/download/feathercad-linux-amd64.deb) | `sudo apt install ./feathercad-linux-amd64.deb` |
| **Linux** (any distribution, no administrator) | [feathercad-linux-x86_64.tar.gz](https://github.com/feather-works/feathercad/releases/latest/download/feathercad-linux-x86_64.tar.gz) | extract it and run `sh instalar.sh` (installs to `~/.local`; this is the package the FeatherWorks Launcher uses) |

## Formats

- **`.dxf`**: the universal exchange format. Opens R12 and R2000+ files (R13/R14 are read as R2000); saves R12 (`AC1009`) for now.
- **`.fcdz`**: FeatherCAD's own format.
- **PDF**: vector export, one page per sheet, drawn as the app shows it.

## Agents (MCP)

`feathercad-mcp` is an MCP server with 62 `cad_*` tools: open and save drawings, layers, entities, blocks, hatches,
title blocks, sheets, counts and analysis, and undo.

## Code

| Folder | What |
|---|---|
| `crates/feathercad-core` | engine: geometry, document, snap, command line, drawing frame |
| `crates/feathercad-io` | `.dxf`, `.fcdz`, PDF |
| `crates/feathercad-mcp` | MCP server |
| `crates/feathercad-ponte` | single command dispatch between the interface and the engine |
| `src/`, `src-tauri/` | the interface of the released alpha (TypeScript + Tauri; the engine runs as WebAssembly) |
| `crates/feathercad` | the new native Rust interface ([pelicano](https://github.com/feather-works/pelicano)), in progress |

## Build

```bash
npm install && npm run app:dev                    # released interface (Tauri)
cd crates/feathercad && cargo run --release       # new native interface
```

Internal notes (in Portuguese) live in `docs/`: interface, shortcuts, DXF reference and feature plan.

License: GPL-3.0.

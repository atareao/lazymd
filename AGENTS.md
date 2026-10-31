# lazymd — AGENTS.md

TUI ratatui para conversión bidireccional entre Markdown, HTML, PDF y Typst.
Los ficheros viven en `src/`; el mapa de módulos y la estrategia de preview están
en `docs/arquitectura.md`.

## Reglas de oro (siempre/nunca)

- **NUNCA shell-out a herramientas del sistema** (`md2html`, `pdftotext`,
  `firefox`, etc.). Toda conversión es un crate Rust; si falta en `Cargo.toml`
  el build falla, sin sorpresas en runtime. Ante fallo de conversión: error claro,
  nunca panic (se muestra en el log panel).
- MD→PDF se hace con **`markdown2pdf`**, nunca con `printpdf`. `printpdf`
  (feature `html`) solo para HTML→PDF directo; requiere `features = ["html"]`
  o `PdfDocument::from_html()` no compila.
- PDF se lee siempre con **`pdf_oxide::api::Pdf`**; nunca `AutoExtractor` ni
  `TextExtractor` internos.
- Conversiones con ficheros intermedios usan **`tempfile::NamedTempFile`** y
  limpian en todas las salidas.
- **Typst** es opcional (feature `typst`). Sin él, `.typ` se reconoce y
  previsualiza, pero convertir devuelve error pidiendo activar el feature.
  La impl del `World` en `converter.rs` solo soporta `.typ` de un fichero;
  dependencias externas fallan con `FileError::NotFound`.
- Detección de tipo: por extensión (`.md`, `.html`/`.htm`, `.pdf`) + magic bytes.

## Toolchain

**Edition 2024** (rustc ≥ 1.85). `cargo fmt` usa estilo 2024. `gen` es keyword
reservada. Ejecuta `cargo fmt` tras cualquier edición significativa.

## Commands

```sh
just build              # debug build
just build-release      # release build
just run                # run the TUI
just check              # fast compilation check
just test               # all tests
just clippy             # lint (must pass before commits)
just fmt                # format (must pass before commits)
just precommit          # fmt + clippy + test
just doc                # generate docs
just codegraph          # update CodeGraph index
```

Build con soporte Typst:

```sh
cargo build --features typst
# or via just:
just build --features typst
```

## CodeGraph

CodeGraph is initialized in this project. Use it for structural code queries:

```sh
just codegraph          # re-index after changes
just codegraph-status   # view index stats
```

See `.codegraph/` for the index database. The file watcher debounces ~500ms
behind writes.

## Git Flow

Este proyecto sigue **Git Flow** estricto. Ver [GIT_FLOW.md](./GIT_FLOW.md):

- Estructura de ramas: `main`, `development`, `feature/*`, `hotfix/*`.
- Conventional commits con **gitmoji**.
- Cómo crear features, hotfixes y releases.

## Dónde está el resto

| Tema | Fichero |
|---|---|
| Qué es, arquitectura de `src/`, estrategia de preview, errores | `docs/arquitectura.md` |
| Matriz de conversión, dependencias `Cargo.toml`, API `pdf_oxide`, flags de `markdown2pdf` | `docs/conversion.md` |
| Feature `typst`, pipeline de compilación, `World` | `docs/typst.md` |
| Keybindings | `docs/keybindings.md` |
| Gotchas completos (edition, tempfiles, testing, ratatui…) | `docs/gotchas.md` |

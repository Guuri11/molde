# molde

Servidor LSP en Rust que sirve snippets deterministas en cualquier lenguaje, con contexto sintáctico (Treesitter), imports automáticos, captura desde el editor, estadísticas de uso y scaffold de features. "molde" es un nombre provisional.

La especificación está en Gherkin en español (`# language: es`) y se ejecuta tal cual con cucumber-rs como tests de aceptación.

## Estado

Los hitos 0 a 7 están terminados: `cargo test` pasa todos los escenarios `@hito0` a `@hito7`. Los de `@flujo`, `@hoja-de-ruta`, `@transversal` y `@futuro` quedan sin pasos y salen como omitidos.

El Hito 0 se ha comprobado en Neovim 0.12 con blink.cmp 1.10.2: aplica la edición adicional del import y ejecuta `molde.recordUse`.

Queda pendiente el criterio del Hito 6 con un proyecto real: los primeros candidatos son útiles, pero la lista completa tiene demasiado ruido. Ver la [ADR 0009](docs/adr/0009-regla-de-tres-por-estructura.md).

## Documentación

- [Empezar a usar molde](docs/guia/empezar-con-molde.md): instalación, editor, primer snippet, captura, revisión y scaffold
- Playbooks por lenguaje: [Rust](docs/playbooks/rust.md), [TypeScript](docs/playbooks/typescript.md) y [Python](docs/playbooks/python.md)
- [Decisiones de arquitectura](docs/adr/README.md)

```sh
cargo install --path crates/molde-cli   # deja el binario `molde` en ~/.cargo/bin
molde check                             # valida snippets, molde.toml y plantillas
molde stats                             # usos y candidatos a borrar, en Markdown
molde candidates [--save N]             # patrones repetidos sin snippet; --save guarda el N
molde scaffold <plantilla> <nombre> [plural=<plural>]
```

## Ficheros

| Fichero | Qué cubre |
|---|---|
| `features/00_flujo_diario.feature` | El día a día: escribir, capturar, scaffold, revisión mensual, equipo |
| `features/01_completado.feature` | Capas, lenguajes, contexto con Treesitter, imports con `requires` |
| `features/02_captura.feature` | Guardar una selección como snippet, generalización, recarga en caliente |
| `features/03_uso_y_poda.feature` | Registro de usos, estadísticas, candidatos a borrar, regla de tres |
| `features/04_scaffold.feature` | Features completas desde una feature dorada |
| `features/05_casos_limite.feature` | Lo que hay que tener en cuenta y las decisiones tomadas |
| `features/06_hoja_de_ruta.feature` | Hitos del 0 al 7 y su definición de terminado |

Cada escenario lleva la etiqueta de su hito (`@hito0` a `@hito7`), `@transversal` o `@futuro`.

## Arquitectura

Workspace de Rust con arquitectura hexagonal. El núcleo no conoce LSP, Treesitter ni SQLite.

```
molde/
├── Cargo.toml                  workspace
├── docs/                       guía, playbooks por lenguaje y ADR
├── features/                   los .feature de esta especificación
├── queries/                    consultas de Treesitter por lenguaje
│   ├── rust/       containers.scm, imports.scm
│   ├── typescript/
│   ├── tsx/
│   └── python/
└── crates/
    ├── molde-core/             dominio y casos de uso, sin IO
    │   ├── domain/             Snippet, Layer, Context, Requirement, UsageEvent, Template
    │   ├── ports/              SnippetRepository, UsageStore, SyntaxAnalyzer, Clock, FileSystem, Shell
    │   └── use_cases/          complete, record_use, capture, stats, candidates, scaffold
    ├── molde-syntax/           adaptador Treesitter: contexto, imports, generalización
    ├── molde-store/            snippets en VSCode JSON y uso en SQLite
    ├── molde-lsp/              adaptador LSP
    ├── molde-cli/              binario: lsp, check, stats, candidates, scaffold
    └── molde-acceptance/       runner de cucumber-rs contra molde-core
```

Añadir un lenguaje es añadir su gramática a `molde-syntax` y sus consultas en `queries/`, sin tocar `molde-core`.

## Dependencias

- `tower-lsp-server`: fork comunitario de tower-lsp, que ya no tiene desarrollo activo. Lo usan Oxc y Biome.
- `tree-sitter` con `tree-sitter-rust`, `tree-sitter-typescript` (TS y TSX) y `tree-sitter-python`.
- `serde_json` y `toml` para snippets, plantillas y configuración.
- `rusqlite` (con SQLite incluido) para el registro de usos.
- `ignore` para respetar `.gitignore` al buscar candidatos y al copiar la feature dorada.
- `tokio` para el servidor.
- `cucumber`, `tempfile` y `tree-sitter-go` para los tests de aceptación.

Se quedaron fuera dos de las previstas:

- `notify`: en cada completion molde compara la fecha y el tamaño de los pocos ficheros de snippets que aplican. No depende del cliente, no necesita hilos y es determinista en los tests.
- `clap`: cinco subcomandos se leen con un `match`.

## Rutas

| Qué | Dónde |
|---|---|
| Snippets de usuario | `~/.config/molde/snippets/<lenguaje>.json` y `all.json` |
| Snippets de proyecto | `.molde/snippets/<lenguaje>.json` y `all.json` |
| Configuración del proyecto | `.molde/molde.toml` |
| Plantillas de scaffold | `.molde/templates/<nombre>/template.toml` |
| Registro de usos | `~/.local/share/molde/usage.sqlite` (fuera del repo) |

## Formato de snippet

VSCode JSON con dos claves extra. La clave del objeto es el id estable: cambiar prefijo o descripción no rompe el historial de uso.

```json
{
  "axum-get-by-id": {
    "prefix": "axum handler get by id",
    "description": "Handler GET por id con servicio y DTO",
    "body": [
      "pub async fn get_${1:invoice}(",
      "    State(svc): State<Arc<${2:Invoice}Service>>,",
      "    Path(id): Path<Uuid>,",
      ") -> Result<Json<${2}Dto>, ApiError> {",
      "    let ${1} = svc.find(id).await?;",
      "    Ok(Json(${1}.into()))",
      "}$0"
    ],
    "requires": [
      "use std::sync::Arc;",
      "use axum::extract::{Path, State};",
      "use axum::Json;",
      "use uuid::Uuid;"
    ],
    "context": ["source_file", "mod_item"]
  }
}
```

- `requires`: imports necesarios. molde los descompone nombre a nombre y solo añade los que faltan.
- `context`: tipos de contenedor de Treesitter donde se ofrece. Sin `context`, se ofrece en todo el lenguaje.

Las claves extra son compatibles con nvim-scissors, que solo toca `prefix` y `body`.

## Configuración en Neovim

```lua
vim.lsp.config("molde", {
  cmd = { "molde", "lsp" },
  filetypes = { "rust", "typescript", "typescriptreact", "python" },
  root_markers = { ".molde", ".git" },
})
vim.lsp.enable("molde")

-- Comandos del servidor desde el editor.
local function molde(command, args)
  local client = vim.lsp.get_clients({ name = "molde" })[1]
  if client then client:exec_cmd({ title = command, command = command, arguments = args }) end
end
vim.api.nvim_create_user_command("MoldeStats", function() molde("molde.stats") end, {})
vim.api.nvim_create_user_command("MoldeCandidates", function() molde("molde.candidates") end, {})
vim.api.nvim_create_user_command("MoldeScaffold", function(o) molde("molde.scaffold", o.fargs) end, { nargs = "+" })
```

La captura es una acción de código: selecciona varias líneas y usa `vim.lsp.buf.code_action()` (`<leader>ca` en LazyVim).

## Ejecutar la especificación

`cargo test` ejecuta los unit tests y todos los `.feature`. Un escenario `@hitoN` con pasos sin implementar cuenta como fallo, así que "pasa" quiere decir que el hito está terminado. Para ejecutar solo algunos escenarios: `cargo test -p molde-acceptance --test acceptance -- -n "<regex del nombre>"`.

Hace falta cucumber-rs 0.23: la 0.21 no sustituye placeholders con espacios, como `<último uso>` en `03_uso_y_poda.feature`.

El runner está en `crates/molde-acceptance/tests/acceptance.rs` y los pasos en `tests/steps/`, un fichero por cada `.feature`. Los escenarios que hablan con el servidor lo hacen por LSP real, con un cliente en memoria (`tests/lsp_client.rs`). Los de la CLI llaman a la misma función que el binario.

## Antes de empezar

El Hito 0 incluye una prueba con un servidor mínimo para confirmar que tu cliente de completion aplica `additionalTextEdits` y ejecuta el `command` de un item de un servidor que no es el del lenguaje. Las estadísticas de uso dependen de eso, así que conviene saberlo antes de construir encima.

## Decisiones de implementación

Lo que la especificación dejaba abierto y cómo ha quedado.

**Configuración** (`.molde/molde.toml`, todo opcional): `sort_priority = 50` (prefijo de `sortText`: más bajo, más arriba), `common_words = []`, `unused_days = 60` y `max_file_size_mb = 2`. Una clave desconocida es un error de `molde check`.

**Completion**
- Se devuelven todos los snippets que aplican y el editor filtra. El texto que se sustituye va desde el inicio de la palabra escrita, contando `-`, porque hay prefijos como `ddd-model`.
- Si los errores del árbol impiden decidir el contexto, los snippets con `context` se ofrecen igualmente con 1000 de prioridad añadida. Un error de una sola línea se considera lo que se está escribiendo y no cuenta.
- Si el cursor está en un comentario con código delante en la misma línea, decide ese código. Treesitter cuelga esos comentarios donde le conviene: en Python, el de la última línea de una función queda en el módulo.
- Un lenguaje sin descomposición de imports (por ejemplo uno añadido solo con gramática y consultas) compara los imports por la sentencia entera. `imports.scm` marca `@import`, `@header` (lo que va por encima de los imports) y `@scope` (ámbitos con imports propios, como `mod tests`).
- El servidor parsea de forma incremental: compara el texto nuevo con el anterior para saber qué ha cambiado. El cliente sigue mandando el documento entero.
- Los ficheros de snippets admiten comentarios y comas finales, como en VSCode.

**Captura**
- Se toman las líneas completas de la selección, sin la sangría común. La clave es `captura-N` y el prefijo provisional `captura N`. No se rellena `context`.
- Vocabulario común incluido: `self`, `get`, `set`, `new`, `id`, `value`, `data`, `result`, `error`... Las `common_words` del proyecto se suman a esa lista.
- La entrada se inserta como texto antes de la llave final, para no tocar el formato ni los comentarios del fichero.
- El servidor declara también `molde.capture`, el comando que ejecuta la acción de código.

**Uso**: cada uso guarda la clave, la raíz del proyecto, el lenguaje del fichero de snippets, la fecha y la capa. Los snippets de usuario suman los usos de todos los proyectos; los de proyecto, solo los del suyo. Un snippet sin usos cuenta desde su fecha de alta.

**Regla de tres**: la huella de un patrón es su estructura sintáctica, sin nombres, literales ni comentarios. Cuenta con 5 líneas o más y 3 apariciones o más. Un patrón que solo aparece dentro de otro mayor no se propone aparte. `molde candidates --save N` lo guarda con la misma generalización que la captura.

**Scaffold**
- `exceptions` son rutas dentro de `source` que no se copian.
- `main`, opcional, es el fichero que se abre tras generar, relativo a `source`. Por defecto, el primero que se llame `mod`, `index`, `__init__`, `main` o `lib`.
- `target` y `line` admiten `{snake}`, `{kebab}`, `{pascal}`, `{camel}`, `{screaming}`, `{title}` y `{lower}`, y sus versiones en plural (`{snake_plural}`...).
- El plural sigue las reglas regulares del inglés (`s`, `es`, `ies`). Los irregulares se pasan con `plural=`.
- La salida es una línea por fichero: `A ruta` si se ha creado y `M ruta` si se ha modificado. La salida de los comandos posteriores va por stderr.

**Tests**
- Las dependencias se compilan optimizadas también en desarrollo (`[profile.dev.package."*"]`). La latencia de completion con 500 snippets y un fichero de 3.000 líneas da unos 40 ms en debug y 12 ms en release.
- En el escenario de scaffold multilenguaje, TypeScript se comprueba ejecutando los módulos con Node, que quita los tipos, y `ruff` es un sustituto que solo compila los `.py`. Así los tests no dependen de tener `tsc` ni `ruff` instalados.

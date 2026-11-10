# Empezar a usar molde

Cómo instalar molde, conectarlo a tu editor y usar snippets que entienden dónde está el cursor y traen sus propios imports.

Esta guía tarda unos 20 minutos.

## Contenido

- [Antes de empezar](#antes-de-empezar)
- [Paso 1: instalar molde](#paso-1-instalar-molde)
- [Paso 2: conectar molde a tu editor](#paso-2-conectar-molde-a-tu-editor)
- [Paso 3: preparar el proyecto](#paso-3-preparar-el-proyecto)
- [Paso 4: escribir tu primer snippet](#paso-4-escribir-tu-primer-snippet)
- [Paso 5: usar el snippet](#paso-5-usar-el-snippet)
- [Paso 6: convertir código en snippet](#paso-6-convertir-código-en-snippet)
- [Paso 7: revisar la colección cada mes](#paso-7-revisar-la-colección-cada-mes)
- [Paso 8: generar una feature completa](#paso-8-generar-una-feature-completa)
- [Ajustar molde a tu proyecto](#ajustar-molde-a-tu-proyecto)
- [Si algo no funciona](#si-algo-no-funciona)
- [Siguientes pasos](#siguientes-pasos)

## Antes de empezar

Necesitas:

- Rust y `cargo`, para compilar molde
- un editor con cliente LSP: esta guía usa Neovim 0.11 o posterior
- un proyecto en Rust, TypeScript o Python dentro de un repositorio git

molde funciona sin conexión. No envía nada a ningún sitio.

> Se ha comprobado con Neovim 0.12 y blink.cmp 1.10. Otros editores con cliente LSP deberían funcionar, pero no se han probado.

## Paso 1: instalar molde

1. Clona el repositorio:

   ```sh
   git clone https://github.com/Guuri11/molde
   cd molde
   ```

2. Instala el binario:

   ```sh
   cargo install --path crates/molde-cli
   ```

3. Comprueba que está instalado:

   ```sh
   molde --help
   ```

Debes ver la lista de comandos: `lsp`, `check`, `stats`, `candidates` y `scaffold`.

## Paso 2: conectar molde a tu editor

molde es un servidor LSP. Tu editor lo arranca con `molde lsp` y habla con él por la entrada y la salida estándar.

### En Neovim con LazyVim

Crea el fichero `~/.config/nvim/lua/plugins/molde.lua` con este contenido:

```lua
vim.lsp.config("molde", {
  cmd = { "molde", "lsp" },
  filetypes = { "rust", "typescript", "typescriptreact", "python" },
  root_markers = { ".molde", ".git" },
})
vim.lsp.enable("molde")

local function molde(command, args)
  local client = vim.lsp.get_clients({ name = "molde" })[1]
  if client then client:exec_cmd({ title = command, command = command, arguments = args }) end
end
vim.api.nvim_create_user_command("MoldeStats", function() molde("molde.stats") end, {})
vim.api.nvim_create_user_command("MoldeCandidates", function() molde("molde.candidates") end, {})
vim.api.nvim_create_user_command("MoldeScaffold", function(o) molde("molde.scaffold", o.fargs) end, { nargs = "+" })

return {}
```

Si no usas LazyVim, pon el mismo código, sin el `return {}`, en tu `init.lua`.

Para comprobar que funciona:

1. Abre un fichero de tu proyecto, por ejemplo un `.rs`.
2. Ejecuta `:checkhealth vim.lsp`.
3. Busca `molde` entre los clientes activos.

### En otros editores

Configura un servidor LSP con:

- comando: `molde lsp`
- lenguajes: Rust, TypeScript, TypeScript React y Python
- raíz del proyecto: la carpeta que contenga `.molde` o, si no hay, `.git`

## Paso 3: preparar el proyecto

Crea la carpeta de molde en la raíz del proyecto:

```sh
mkdir -p .molde/snippets
```

Súbela a git. Así cualquier persona que clone el repositorio tendrá los mismos snippets y plantillas.

Los snippets pueden estar en 2 sitios:

| Capa | Dónde | Para qué |
|---|---|---|
| Usuario | `~/.config/molde/snippets/` | Lo que usas en todos tus proyectos |
| Proyecto | `.molde/snippets/` | Lo propio de este proyecto |

Si un snippet tiene la misma clave en las 2 capas, gana el del proyecto.

El nombre del fichero decide el lenguaje:

- `rust.json`
- `typescript.json`
- `typescriptreact.json`: los ficheros `.tsx` también ven los de `typescript.json`
- `python.json`
- `all.json`: para todos los lenguajes

## Paso 4: escribir tu primer snippet

Crea `.molde/snippets/rust.json`:

```json
{
  "test-tokio": {
    "prefix": "test tokio",
    "description": "Test asíncrono con tokio",
    "body": [
      "#[tokio::test]",
      "async fn ${1:nombre}() {",
      "    let ${2:resultado} = ${3:llamada}.await;",
      "    assert_eq!(${2}, ${4:esperado});$0",
      "}"
    ],
    "context": ["mod_item"]
  }
}
```

Qué significa cada clave:

- la clave del objeto (`test-tokio`) es el identificador del snippet: no la cambies después, porque el historial de uso va asociado a ella
- `prefix` es lo que escribes para encontrarlo
- `body` usa la sintaxis de snippets de VSCode: `${1:texto}` es un hueco, `${1}` repite lo que escribas en él y `$0` es donde acaba el cursor
- `context`, opcional, limita dónde aparece: aquí, solo dentro de un `mod`, como `mod tests`
- `requires`, opcional, lista los imports que necesita: lo ves en el paso 5

Valida el fichero:

```sh
molde check
```

Si hay un error, molde te dice el fichero y la línea.

> No hace falta reiniciar el editor. molde vuelve a leer los ficheros de snippets cuando cambian.

## Paso 5: usar el snippet

1. Abre un fichero Rust del proyecto.
2. Dentro de `mod tests`, escribe `tetok`.
3. En el menú de completion, elige `test tokio`. Lleva la etiqueta `molde`.
4. Escribe el nombre del test y pulsa Tab para pasar al siguiente hueco.

El editor filtra los snippets con su propia búsqueda aproximada, así que no hace falta escribir el prefijo entero.

### Snippets que traen sus imports

Añade `requires` a un snippet para que molde inserte los imports que falten:

```json
"requires": [
  "use std::sync::Arc;",
  "use axum::extract::{Path, State};"
]
```

molde comprueba cada nombre por separado. Si el fichero ya tiene `use axum::extract::Path;`, solo añade `use axum::extract::State;`. Nunca duplica un import.

Cada lenguaje tiene sus particularidades. Consulta el playbook de tu lenguaje en [Siguientes pasos](#siguientes-pasos).

## Paso 6: convertir código en snippet

Cuando escribas algo que vas a repetir, o aceptes una sugerencia de la IA que te sirva, guárdalo como snippet.

1. Selecciona 2 o más líneas.
2. Abre las acciones de código. En LazyVim es `<leader>ca`.
3. Elige **Guardar selección como snippet**.

molde:

- añade el snippet a `.molde/snippets/<lenguaje>.json` con la clave `captura-1` (o el siguiente número libre)
- convierte en huecos los nombres de dominio y los literales
- copia en `requires` los imports que usa la selección, salvo los relativos
- abre el fichero en la entrada nueva

Un nombre es de dominio cuando aparece dentro de al menos 2 identificadores distintos. Por ejemplo, `invoice` en `get_invoice` y en `InvoiceService`. Las palabras comunes como `id`, `new`, `value` o `error` no se convierten.

Después de capturar:

1. Cambia la clave `captura-1` por un nombre que entiendas.
2. Cambia el `prefix` y escribe una `description`.
3. Guarda el fichero.

El snippet ya aparece en la completion.

> Si quieres el snippet en todos tus proyectos, mueve la entrada a `~/.config/molde/snippets/<lenguaje>.json`. La captura siempre guarda en el proyecto.

### Que el snippet lo escriba una IA

Con la variable `MOLDE_CAPTURE_COMMAND`, molde le pasa la selección a una IA y guarda lo que responde. La IA quita los logs y los comentarios, pone los huecos, la clave, el prefijo y la descripción, y deja en `requires` solo los imports que hacen falta.

1. Elige el comando de tu herramienta:

   | Herramienta | Comando |
   |---|---|
   | Claude Code | `claude -p --model haiku` |
   | Copilot CLI | `copilot -s -p "$(cat)"` |
   | OpenCode | `opencode run -m <proveedor/modelo> "$(cat)"` |

   En OpenCode pon siempre `-m`: con el modelo gratuito por defecto no responde a tiempo.

2. Pásaselo al servidor en `~/.config/nvim/lua/plugins/molde.lua`:

   ```lua
   vim.lsp.config("molde", {
     cmd = { "molde", "lsp" },
     cmd_env = { MOLDE_CAPTURE_COMMAND = "claude -p --model haiku" },
     filetypes = { "rust", "typescript", "typescriptreact", "python" },
     root_markers = { ".molde", ".git" },
   })
   ```

3. Captura como siempre, con **Guardar selección como snippet**.

La IA tarda entre 20 y 50 segundos. Mientras tanto el editor muestra el progreso de molde y la completion sigue funcionando. Si la IA falla o tarda más de `MOLDE_CAPTURE_TIMEOUT` segundos (120 por defecto), molde captura con la regla de siempre y te avisa del motivo.

Revisa siempre la entrada: la IA puede cambiar el código para dejarlo limpio, y molde solo comprueba que el snippet es válido, no que sea correcto. La selección y sus imports se envían a la IA que configures.

## Paso 7: revisar la colección cada mes

molde registra cada vez que aceptas un snippet. El registro está en `~/.local/share/molde/usage.sqlite`, fuera del repositorio.

### Ver qué snippets sobran

```sh
molde stats
```

Verás cada snippet con sus usos, su último uso y su fecha de alta. Si lleva 60 días o más sin usarse, aparece como candidato a borrar.

Desde Neovim: `:MoldeStats`. El informe se abre en un buffer.

### Ver qué falta

```sh
molde candidates
```

molde busca bloques de 5 o más líneas que se repiten 3 o más veces con la misma estructura y que ningún snippet cubre. Enseña los 20 que más se repiten y, al final, cuántos hay en total. Recorre todo el proyecto, así que en uno grande tarda unos segundos.

Para guardar el candidato número 2 como snippet:

```sh
molde candidates --save 2
```

Se guarda igual que la captura del paso 6: si tienes `MOLDE_CAPTURE_COMMAND`, lo escribe la IA.

Cuando decidas qué borrar y qué crear, súbelo en un commit.

## Paso 8: generar una feature completa

Un snippet es un trozo de código. Una plantilla de scaffold genera una carpeta entera a partir de una feature que ya existe en tu proyecto, la "feature dorada".

1. Elige una feature que compile y pase sus tests, y llámala con un nombre de 2 palabras, por ejemplo `sample_widget`.
2. Crea `.molde/templates/feature-crud/template.toml`:

   ```toml
   source = "src/features/sample_widget"
   source_name = "sample widget"
   target = "src/features/{snake}"
   post = ["cargo fmt", "cargo check"]

   [[inject]]
   file = "src/features/mod.rs"
   marker = "// molde:mods"
   line = "pub mod {snake};"
   ```

3. Pon el comentario `// molde:mods` en `src/features/mod.rs`.
4. Comprueba la plantilla con `molde check`.
5. Genera la feature:

   ```sh
   molde scaffold feature-crud line-item
   ```

molde copia la carpeta y cambia todas las formas del nombre: `sample_widget` por `line_item`, `SampleWidget` por `LineItem`, `SAMPLE_WIDGET` por `LINE_ITEM`, `sample_widgets` por `line_items`, y así con el resto. Después añade `pub mod line_item;` debajo del marcador y ejecuta los comandos de `post`.

Si el plural no es regular, indícalo:

```sh
molde scaffold feature-crud person plural=people
```

> molde comprueba todo antes de escribir. Si falta un marcador o la feature ya existe, no toca nada. Si falla un comando de `post`, los ficheros generados se quedan para que los revises: puedes deshacerlo con git.

Cada playbook tiene una plantilla completa para su lenguaje.

## Ajustar molde a tu proyecto

Puedes crear `.molde/molde.toml`. Todas las claves son opcionales:

```toml
sort_priority = 50       # posición de los snippets en el menú: más bajo, más arriba
common_words = ["user"]  # palabras que la captura nunca convierte en hueco
unused_days = 60         # días sin uso para proponer borrar un snippet
max_file_size_mb = 2     # por encima, molde no analiza el fichero
```

Una clave desconocida es un error de `molde check`.

En un monorepo, cada subproyecto puede tener su propia carpeta `.molde`. molde usa la más cercana al fichero que editas.

## Si algo no funciona

### No aparece ningún snippet

Comprueba que:

- molde está entre los clientes activos (`:checkhealth vim.lsp`)
- el fichero de snippets se llama como el lenguaje, por ejemplo `python.json`
- `molde check` no da errores

### Aparece en unos sitios y en otros no

El snippet tiene `context`. Solo aparece dentro de esos contenedores. Quita `context` para que aparezca en todo el lenguaje.

### No se añaden los imports

Puede ser porque:

- el lenguaje no tiene soporte de imports: molde inserta el snippet sin imports y lo avisa una vez en el log
- el import ya estaba, quizá con otra forma

### Sale un aviso con un fichero y una línea

El fichero de snippets tiene un error de sintaxis. molde sigue usando la última versión válida hasta que lo corrijas.

### `molde stats` dice que no uso nada

Tu cliente de completion no ejecuta el comando que acompaña a cada snippet. Los snippets funcionan, pero no se registran los usos.

### `:MoldeCandidates` no enseña nada

El análisis recorre todo el proyecto desde la carpeta `.molde` más cercana y, en un repositorio grande, tarda decenas de segundos. Mientras tanto molde no responde a nada más, tampoco a la completion. Espera a que se abra el informe o lanza `molde candidates` desde la terminal.

## Siguientes pasos

- [Playbook de Rust](../playbooks/rust.md)
- [Playbook de TypeScript](../playbooks/typescript.md)
- [Playbook de Python](../playbooks/python.md)
- [Decisiones de arquitectura](../adr/README.md)

Última actualización: 4 de noviembre de 2026

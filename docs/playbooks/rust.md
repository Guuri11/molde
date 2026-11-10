# Usar molde en un proyecto Rust

Cómo escribir snippets de Rust con imports y contexto, capturar código y generar features con scaffold.

## Contenido

- [Antes de empezar](#antes-de-empezar)
- [Dónde van los snippets](#dónde-van-los-snippets)
- [Escribir un snippet con imports](#escribir-un-snippet-con-imports)
- [Limitar dónde aparece un snippet](#limitar-dónde-aparece-un-snippet)
- [Dónde inserta molde los imports](#dónde-inserta-molde-los-imports)
- [Capturar código Rust](#capturar-código-rust)
- [Generar una feature en un solo crate](#generar-una-feature-en-un-solo-crate)
- [Generar una feature con un crate por capa](#generar-una-feature-con-un-crate-por-capa)
- [Problemas frecuentes](#problemas-frecuentes)

## Antes de empezar

Necesitas molde instalado y conectado a tu editor. Si no lo tienes, sigue [Empezar a usar molde](../guia/empezar-con-molde.md).

molde convive con rust-analyzer: los dos ofrecen completion en el mismo buffer y los items de molde llevan la etiqueta `molde`.

## Dónde van los snippets

| Para | Fichero |
|---|---|
| Todos tus proyectos Rust | `~/.config/molde/snippets/rust.json` |
| Solo este proyecto | `.molde/snippets/rust.json` |

## Escribir un snippet con imports

Pon en `requires` los `use` que necesita el snippet, tal como los escribirías:

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

molde separa cada `use` en nombres sueltos y solo añade los que faltan:

- `use axum::extract::{Path, State};` cuenta como `Path` y `State` por separado
- los grupos anidados también se separan: `use axum::{extract::Path, Json};`
- respeta los alias (`use std::io::Result as IoResult;`) y la visibilidad (`pub use`)
- si el fichero ya importa `Path` desde otro `use`, no lo repite

> No pongas los `use` dentro del `body`. Así se insertarían donde está el cursor y se duplicarían si ya existen.

## Limitar dónde aparece un snippet

`context` lista los contenedores donde tiene sentido. El que cuenta es el más cercano al cursor.

| Valor | El cursor está |
|---|---|
| `source_file` | En el nivel superior del fichero |
| `mod_item` | Dentro de un `mod { ... }`, como `mod tests` |
| `function_item` | Dentro de una función o un método |
| `impl_item` | Dentro de un `impl`, fuera de sus métodos |
| `trait_item` | Dentro de un `trait`, fuera de sus métodos |
| `struct_item` | Entre los campos de un `struct` |
| `enum_item` | Entre las variantes de un `enum` |

Dentro de un método, el contenedor más cercano es `function_item`, no `impl_item`. Un snippet de método va con `impl_item`. Uno de código para el cuerpo va con `function_item`.

Ejemplos:

- un test: `["mod_item"]`
- un struct, un enum o un handler: `["source_file", "mod_item"]`
- un método: `["impl_item", "trait_item"]`
- un bloque `match` o un `let ... else`: `["function_item"]`

Sin `context`, el snippet aparece en cualquier parte de un fichero Rust.

## Dónde inserta molde los imports

- Detrás del último `use` del fichero.
- Si no hay ninguno, detrás de los atributos internos (`#![...]`) y de los comentarios de documentación del módulo (`//!`).
- Si el cursor está dentro de un `mod` en línea, como `mod tests { ... }`, dentro de ese módulo y con su sangría.

molde no fusiona líneas ni reordena. Si quieres agruparlos, usa el organizador de imports de rust-analyzer o `cargo fmt` con `imports_granularity`.

## Capturar código Rust

Selecciona 2 o más líneas, abre las acciones de código y elige **Guardar selección como snippet**.

En Rust:

- `requires` recoge los `use` que la selección usa de verdad
- los `use self::...` y `use super::...` se omiten, porque dependen de dónde esté el fichero, y se citan en la descripción para que los revises
- los `use crate::...` sí se copian: revisa que tengan sentido en otro sitio antes de mover el snippet a la capa de usuario

Por ejemplo, al capturar un struct `Invoice` con su `NewInvoiceProps` y su `impl`, `Invoice` pasa a ser `${1:Invoice}` y cada `NewInvoiceProps` pasa a ser `New${1}${2:Props}`. Al usarlo, escribes el nombre una vez y se repite en todo el snippet.

Si un término se convierte en hueco y no quieres, añádelo a `common_words` en `.molde/molde.toml`.

Con `MOLDE_CAPTURE_COMMAND`, el snippet lo escribe una IA: ella elige los huecos, quita lo concreto y pone clave, prefijo y descripción, así que `common_words` y los huecos de literales no se aplican. Los imports que recibe son los de esta lista. Cómo configurarla, en [Que el snippet lo escriba una IA](../guia/empezar-con-molde.md#que-el-snippet-lo-escriba-una-ia).

## Generar una feature en un solo crate

Usa esta plantilla si cada feature es un módulo dentro de `src/features/`.

1. Elige una feature que compile y pase sus tests y llámala `sample_widget`. Usa ese nombre en todas sus formas: `SampleWidget`, `SAMPLE_WIDGET`, `sample-widgets`.
2. Pon los marcadores donde se registra una feature:

   ```rust
   // src/features/mod.rs
   // molde:mods
   pub mod sample_widget;
   ```

   ```rust
   // src/router.rs
   pub fn app() -> Router {
       Router::new()
           // molde:routes
           .merge(sample_widget::routes())
   }
   ```

3. Crea `.molde/templates/feature-crud/template.toml`:

   ```toml
   source = "src/features/sample_widget"
   source_name = "sample widget"
   target = "src/features/{snake}"
   post = ["cargo fmt", "cargo check"]

   [[inject]]
   file = "src/features/mod.rs"
   marker = "// molde:mods"
   line = "pub mod {snake};"

   [[inject]]
   file = "src/router.rs"
   marker = "// molde:routes"
   line = ".merge({snake}::routes())"
   ```

4. Comprueba la plantilla con `molde check`.
5. Genera la feature con `molde scaffold feature-crud line-item`.

Debes ver una línea `A` por cada fichero creado y una `M` por cada fichero modificado:

```
A src/features/line_item/mod.rs
A src/features/line_item/line_item_dto.rs
M src/features/mod.rs
M src/router.rs
```

Debajo, por la salida de error, verás `ok: cargo fmt` y `ok: cargo check`.

## Generar una feature con un crate por capa

Usa esta plantilla si cada feature es una carpeta con un crate por capa:

```
features/sample_widget/
├── domain/          crate sample-widget-domain
├── application/     crate sample-widget-application
└── infrastructure/  crate sample-widget-infrastructure
```

1. Pon el nombre de la feature en el `name` de cada `Cargo.toml`, por ejemplo `name = "sample-widget-domain"`, y en las dependencias entre capas:

   ```toml
   [dependencies]
   sample-widget-domain = { path = "../domain" }
   ```

2. Si el `Cargo.toml` del workspace usa un patrón como `members = ["features/*/*"]`, no hace falta registrar nada. Si lista cada crate, añade un marcador:

   ```toml
   [workspace]
   members = [
       # molde:members
       "features/sample_widget/domain",
       "features/sample_widget/application",
       "features/sample_widget/infrastructure",
   ]
   ```

3. Crea `.molde/templates/feature/template.toml`:

   ```toml
   source = "features/sample_widget"
   source_name = "sample widget"
   target = "features/{snake}"
   post = ["cargo fmt --all", "cargo check --workspace"]

   [[inject]]
   file = "Cargo.toml"
   marker = "# molde:members"
   line = "\"features/{snake}/domain\", \"features/{snake}/application\", \"features/{snake}/infrastructure\","
   ```

4. Genera la feature con `molde scaffold feature line-item`.

molde copia las 3 capas y cambia `sample-widget-domain` por `line-item-domain` en cada `Cargo.toml`.

> `cargo check --workspace` compila todo el workspace. En un workspace grande puede tardar varios minutos. Si te molesta, quita `post` y compila tú el crate nuevo.

Para no copiar algo de la feature dorada, como sus datos de prueba, añádelo a `exceptions`, con rutas relativas a `source`:

```toml
exceptions = ["infrastructure/tests/fixtures"]
```

## Problemas frecuentes

### El snippet no aparece dentro de un método

Tiene `context` con `impl_item`. Dentro de un método el contenedor es `function_item`.

### Aparece al final del menú, detrás de todo

Mientras escribes, el fichero puede tener errores de sintaxis que no dejan decidir el contexto. molde ofrece el snippet igualmente, pero con menos prioridad. Cierra las llaves pendientes y vuelve a pedir la completion.

### `molde scaffold` dice que falta un marcador

El comentario tiene que estar tal cual en el fichero, por ejemplo `// molde:routes`. molde no ha escrito nada: añade el marcador y vuelve a ejecutarlo.

### `cargo check` falla después de generar

Los ficheros generados se quedan para que los revises. Corrige el error o deshazlo con la lista que imprime molde:

- borra la carpeta creada, la de las líneas `A`
- revierte los ficheros de las líneas `M` con `git restore <fichero>`

Última actualización: 4 de noviembre de 2026

# 0004. Los imports viajan en el propio item de completion

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Un snippet con `requires` tiene que insertar los imports que faltan en su sitio, no donde está el cursor. LSP permite mandar ediciones adicionales con el item (`additionalTextEdits`) o calcularlas después, cuando el cliente pide `completionItem/resolve`.

No todos los clientes piden `resolve`, y algunos lo piden al resaltar un item, no al aceptarlo.

En Rust y en TypeScript, importar 2 veces el mismo nombre es un error de compilación.

## Decisión

- La edición de los imports va ya calculada en el item, en `additionalTextEdits`. molde no depende de `resolve`.
- molde descompone cada import en nombres sueltos y solo añade los que faltan. Si el fichero tiene `use axum::Json;` y el snippet pide `use axum::{extract::Path, Json};`, solo añade `use axum::extract::Path;`.
- Un nombre nuevo de un módulo ya importado va en una línea nueva. Fusionar las líneas es cosa del organizador de imports del proyecto.
- Los imports nuevos se insertan detrás del último import. Si no hay ninguno, detrás de la cabecera: shebang, docstring, `"use client"`, atributos internos de Rust.
- En Rust, si el cursor está dentro de un `mod` en línea como `mod tests`, el import va dentro de ese módulo.
- Las posiciones se negocian en UTF-8 si el cliente lo ofrece y, si no, en UTF-16.

## Consecuencias

- Funciona en cualquier cliente que aplique `additionalTextEdits`, que es lo que comprueba el Hito 0.
- Calcular los imports de todos los items en cada petición cuesta. Se mitiga con una caché por petición y con el parseo incremental de la [ADR 0002](0002-treesitter-para-contexto-e-imports.md).
- molde no reordena ni fusiona imports. El resultado puede necesitar pasar por el formateador del proyecto.

## Alternativas descartadas

- Calcular en `resolve`: más barato, pero el import no llega en los clientes que no lo piden.
- Insertar los imports desde el comando que se ejecuta tras aceptar: el cliente puede no ejecutarlo, y el usuario vería el código sin imports durante un instante.

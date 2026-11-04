# 0011. La especificación en Gherkin es la batería de aceptación

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

La especificación de molde está escrita en Gherkin en español (`# language: es`), en `features/`. Cada escenario lleva la etiqueta de su hito. Un hito está terminado cuando pasan todos sus escenarios.

## Decisión

- `cargo test` ejecuta los `.feature` con cucumber-rs, sin modificarlos.
- Un escenario con etiqueta `@hitoN` y pasos sin implementar cuenta como fallo. "Pasa" quiere decir que está terminado.
- Los escenarios sin etiqueta de hito (`@flujo`, `@hoja-de-ruta`, `@transversal`, `@futuro`) salen como omitidos.
- Los escenarios que hablan con el servidor lo hacen por LSP real, con un cliente en memoria. Los de la CLI llaman a la misma función que el binario.
- Se usa cucumber-rs 0.23. La 0.21 no sustituye placeholders con espacios, como `<último uso>`.
- El scaffold multilenguaje usa sustitutos para no depender de tener `tsc` ni `ruff` instalados: Node ejecuta los `.ts` quitando los tipos y un `ruff` falso compila los `.py`. Rust usa `cargo fmt` y `cargo check` reales.

## Consecuencias

- La especificación no puede quedarse desfasada sin que lo note `cargo test`.
- Cambiar el comportamiento empieza por cambiar un `.feature`.
- Para ejecutar solo algunos escenarios: `cargo test -p molde-acceptance --test acceptance -- -n "<regex del nombre>"`.
- El test de latencia mide unos 40 ms en debug frente a un límite de 50. En una máquina muy cargada podría fallar alguna vez.

## Alternativas descartadas

- Traducir los escenarios a tests de Rust: se separarían de la especificación con el primer cambio.

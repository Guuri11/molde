# 0001. Arquitectura hexagonal con un núcleo sin IO

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

molde hace lo mismo desde 2 entradas, el servidor LSP y la CLI, y depende de 3 cosas externas: Treesitter para entender el código, ficheros JSON para los snippets y SQLite para los usos. La especificación se ejecuta como tests de aceptación y tiene que poder fijar la fecha, el sistema de ficheros y los comandos que se ejecutan.

## Decisión

El workspace se divide en crates por responsabilidad:

- `molde-core`: dominio y casos de uso. No hace IO ni conoce LSP, Treesitter ni SQLite.
- `molde-syntax`: adaptador de Treesitter.
- `molde-store`: adaptadores de snippets (JSON), usos (SQLite), sistema de ficheros, shell y reloj.
- `molde-lsp`: adaptador LSP con `tower-lsp-server`.
- `molde-cli`: el binario. Monta el `Engine` y llama a los mismos casos de uso que el servidor.
- `molde-acceptance`: los `.feature` con cucumber-rs.

El núcleo define puertos (`SnippetRepository`, `UsageStore`, `SyntaxAnalyzer`, `Clock`, `FileSystem` y `Shell`) y los adaptadores los implementan.

## Consecuencias

- La CLI y el servidor no pueden comportarse distinto: los dos llaman al mismo `Engine`.
- Los tests fijan la fecha con un reloj falso y ejecutan los comandos de scaffold con un `PATH` controlado.
- Añadir un lenguaje es añadir su gramática y sus consultas a `molde-syntax`. Un test de aceptación lo comprueba con Go y verifica que `molde-core` no cambia.
- Hay más crates de los que pediría un proyecto de este tamaño. Se acepta porque la separación la exige la propia especificación.

## Alternativas descartadas

- Un solo crate con módulos: más simple, pero nada impide que el dominio acabe llamando a SQLite o a Treesitter directamente.

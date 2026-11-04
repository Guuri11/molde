# 0002. Treesitter para el contexto y los imports

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

molde tiene que saber dónde está el cursor (en el nivel superior, dentro de una clase, dentro de una función) para ofrecer solo los snippets que encajan. También tiene que leer los imports del fichero para no duplicarlos y para saber dónde insertar los nuevos. Tiene que funcionar en Rust, TypeScript, TSX y Python, y admitir más lenguajes sin tocar el núcleo.

Mientras escribes, el fichero casi nunca es sintácticamente correcto.

## Decisión

- molde usa Treesitter con una gramática por lenguaje.
- Lo que depende del lenguaje vive en consultas, fuera del código:
  - `queries/<lenguaje>/containers.scm` marca con `@container` los nodos que cuentan como contexto
  - `queries/<lenguaje>/imports.scm` marca `@import` (sentencias de import), `@header` (lo que tiene que quedar por encima, como un shebang o `"use client"`) y `@scope` (ámbitos con imports propios, como `mod tests`)
- El contexto es el contenedor más cercano al cursor.
- Con errores de sintaxis:
  - un error que ocupa una sola línea se trata como lo que se está escribiendo y se ignora
  - si los errores no dejan decidir el contexto, los snippets con `context` se ofrecen igualmente, con 1000 puntos más de prioridad para que salgan detrás
- Un comentario con código delante en la misma línea toma el contexto de ese código. Treesitter cuelga algunos comentarios de un nodo superior: en Python, el de la última línea de una función queda en el módulo.
- Los ficheros de más de `max_file_size_mb` (2 MB por defecto) no se analizan. Sus snippets se ofrecen sin filtrar.
- El árbol se reutiliza entre peticiones y se reparsea de forma incremental. El cliente manda el documento entero y molde calcula qué ha cambiado.
- La descomposición de imports nombre a nombre es código Rust por lenguaje. Un lenguaje sin ella compara las sentencias enteras.

## Consecuencias

- Un lenguaje sin gramática sigue funcionando: se ofrecen todos sus snippets, sin contexto ni imports, y molde lo avisa una vez en el log.
- Ante la duda se ofrece el snippet. Es mejor un snippet de más al final del menú que uno que falta.
- Con 500 snippets y un fichero de 3.000 líneas la completion tarda unos 12 ms en release. El límite de la especificación es 50 ms.
- Los nombres de contexto son los tipos de nodo de cada gramática (`function_item`, `class_body`, `function_definition`). Quien escribe snippets tiene que conocerlos. Los playbooks los listan.

## Alternativas descartadas

- Expresiones regulares por lenguaje: fallan con código a medio escribir y con anidamiento.
- Pedir el contexto al LSP del lenguaje: LSP no tiene una petición para eso y molde tendría que depender de cada servidor.

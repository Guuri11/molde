# 0003. VSCode JSON con `requires` y `context`

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Ya existen colecciones de snippets en el formato de VSCode, y herramientas como nvim-scissors que las editan. Un formato propio obligaría a convertirlas y dejaría fuera esas herramientas.

molde necesita 2 datos que VSCode no tiene: los imports que pide el snippet y dónde tiene sentido.

## Decisión

- Los snippets se escriben en el JSON de VSCode, con 2 claves opcionales:
  - `requires`: lista de imports, escritos como en el lenguaje
  - `context`: lista de tipos de contenedor de Treesitter
- La clave del objeto es el identificador estable del snippet. Cambiar `prefix` o `description` no rompe el historial de uso.
- Un fichero por lenguaje (`rust.json`, `python.json`...) y `all.json` para todos. Los `.tsx` ven también `typescript.json`.
- 2 capas: usuario (`~/.config/molde/snippets/`) y proyecto (`.molde/snippets/`). Si coinciden la clave y el lenguaje, gana el proyecto.
- Se admiten comentarios y comas finales, como en VSCode.

## Consecuencias

- Una colección de VSCode funciona copiando el fichero con el nombre del lenguaje. Los `.code-snippets` se pueden reunir en un solo `<lenguaje>.json`.
- Los snippets de VSCode que llevan los imports dentro del `body` funcionan, pero no aprovechan `requires` hasta que se pasan a esa clave.
- nvim-scissors solo toca `prefix` y `body`, así que respeta las claves extra.
- La captura añade entradas como texto antes de la llave final del fichero, para no perder comentarios ni formato.

## Alternativas descartadas

- TOML o YAML: más cómodos para cuerpos de varias líneas, pero incompatibles con todo lo que ya existe.
- Leer los `.code-snippets` directamente: llevan `scope` en vez de un fichero por lenguaje y molde tendría que mezclar 2 formas de decidir el lenguaje.

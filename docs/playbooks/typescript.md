# Usar molde en un proyecto TypeScript

Cómo escribir snippets de TypeScript y TSX con imports y contexto, capturar código y generar módulos con scaffold.

## Contenido

- [Antes de empezar](#antes-de-empezar)
- [Dónde van los snippets](#dónde-van-los-snippets)
- [Escribir un snippet con imports](#escribir-un-snippet-con-imports)
- [Limitar dónde aparece un snippet](#limitar-dónde-aparece-un-snippet)
- [Dónde inserta molde los imports](#dónde-inserta-molde-los-imports)
- [Capturar código TypeScript](#capturar-código-typescript)
- [Generar un módulo con scaffold](#generar-un-módulo-con-scaffold)
- [Problemas frecuentes](#problemas-frecuentes)

## Antes de empezar

Necesitas molde instalado y conectado a tu editor. Si no lo tienes, sigue [Empezar a usar molde](../guia/empezar-con-molde.md).

En Neovim, el `filetypes` de molde tiene que incluir `typescript` y `typescriptreact`.

## Dónde van los snippets

| Para | Fichero |
|---|---|
| Ficheros `.ts` y `.tsx` | `typescript.json` |
| Solo ficheros `.tsx` | `typescriptreact.json` |

Los 2 pueden estar en `~/.config/molde/snippets/`, para todos tus proyectos, o en `.molde/snippets/`, solo para este.

Un fichero `.tsx` ve los snippets de los 2 ficheros. Pon los componentes y hooks de React en `typescriptreact.json` para que no aparezcan en un `.ts`.

## Escribir un snippet con imports

Pon en `requires` los `import` que necesita el snippet, tal como los escribirías:

```json
{
  "zod-schema": {
    "prefix": "zod schema",
    "description": "Esquema de zod con su tipo inferido",
    "body": [
      "export const ${1:invoice}Schema = z.object({",
      "  ${2:id}: z.string().uuid(),$0",
      "});",
      "",
      "export type ${3:Invoice} = z.infer<typeof ${1}Schema>;"
    ],
    "requires": ["import { z } from \"zod\";"],
    "context": ["program"]
  }
}
```

molde separa cada `import` en nombres sueltos y solo añade los que faltan:

- `import { a, b } from "x";` cuenta como `a` y `b` por separado
- funcionan igual el import por defecto (`import express from "express";`), el de espacio de nombres (`import * as path from "node:path";`) y los alias (`import { a as b } from "x";`)
- `import type { ... }` conserva el `type`
- respeta si escribes el import con punto y coma o sin él

Si el fichero ya importa un nombre, molde no lo repite, aunque venga en otra línea.

> No pongas los `import` dentro del `body`. Así se insertarían donde está el cursor y se duplicarían si ya existen.

## Limitar dónde aparece un snippet

`context` lista los contenedores donde tiene sentido. El que cuenta es el más cercano al cursor.

| Valor | El cursor está |
|---|---|
| `program` | En el nivel superior del fichero |
| `class_body` | Dentro de una clase, fuera de sus métodos |
| `interface_body` | Dentro de una `interface` |
| `enum_body` | Dentro de un `enum` |
| `function_declaration` | Dentro de una función declarada con `function` |
| `generator_function_declaration` | Dentro de una función `function*` |
| `function_expression` | Dentro de una función anónima con `function` |
| `arrow_function` | Dentro de una función flecha |
| `method_definition` | Dentro de un método de clase |
| `jsx_element` | Dentro de un elemento JSX (solo en `.tsx`) |

Un snippet para "dentro de cualquier función" necesita varios valores: `["function_declaration", "function_expression", "arrow_function", "method_definition"]`.

Ejemplos:

- un esquema, un tipo o un componente: `["program"]`
- un método o una propiedad de clase: `["class_body"]`
- un `useEffect`: `["function_declaration", "arrow_function"]`
- un fragmento de JSX: `["jsx_element"]`

Sin `context`, el snippet aparece en cualquier parte del fichero.

## Dónde inserta molde los imports

- Detrás del último `import` del fichero.
- Si no hay ninguno, detrás de las directivas del principio del fichero, como `"use client"` o `"use server"`.

molde no ordena ni agrupa. Si tu proyecto usa un organizador de imports, como el de ESLint o Biome, déjale ese trabajo.

## Capturar código TypeScript

Selecciona 2 o más líneas, abre las acciones de código y elige **Guardar selección como snippet**.

En TypeScript:

- `requires` recoge los `import` de paquetes que la selección usa de verdad
- los imports relativos (`./db`, `../shared`) y los absolutos que empiezan por `/` se omiten, porque dependen de dónde esté el fichero, y se citan en la descripción para que los revises
- los `$` y `}` de los template literals se escapan, así que el snippet reproduce el código tal cual

Los alias de ruta como `@/lib/db` no empiezan por `.` y molde los copia. Si tu proyecto los usa, comprueba que tengan sentido antes de mover el snippet a la capa de usuario.

Con `MOLDE_CAPTURE_COMMAND`, el snippet lo escribe una IA: ella elige los huecos, quita lo concreto y pone clave, prefijo y descripción, así que `common_words` y los huecos de literales no se aplican. Los imports que recibe son los de esta lista. Cómo configurarla, en [Que el snippet lo escriba una IA](../guia/empezar-con-molde.md#que-el-snippet-lo-escriba-una-ia).

## Generar un módulo con scaffold

1. Elige un módulo que funcione y pase sus tests y llámalo `sample-widget`. Usa ese nombre en todas sus formas: `SampleWidget`, `sampleWidget`, `SAMPLE_WIDGET`, `sample-widgets`.

   ```
   src/modules/sample-widget/
   ├── index.ts
   └── sample-widget.service.ts
   ```

2. Pon los marcadores donde se registra un módulo:

   ```ts
   // src/app.ts
   // molde:imports
   import { sampleWidgetRouter } from "./modules/sample-widget/index.ts";

   // molde:routes
   app.use("/sample-widgets", sampleWidgetRouter);
   ```

3. Crea `.molde/templates/module/template.toml`:

   ```toml
   source = "src/modules/sample-widget"
   source_name = "sample widget"
   target = "src/modules/{kebab}"
   post = ["npm run typecheck", "npm run lint"]

   [[inject]]
   file = "src/app.ts"
   marker = "// molde:imports"
   line = "import { {camel}Router } from \"./modules/{kebab}/index.ts\";"

   [[inject]]
   file = "src/app.ts"
   marker = "// molde:routes"
   line = "app.use(\"/{kebab_plural}\", {camel}Router);"
   ```

4. Comprueba la plantilla con `molde check`.
5. Genera el módulo con `molde scaffold module line-item`.

molde crea `src/modules/line-item/` con `line-item.service.ts`, cambia `SampleWidgetService` por `LineItemService` y añade a `src/app.ts`:

```ts
import { lineItemRouter } from "./modules/line-item/index.ts";
app.use("/line-items", lineItemRouter);
```

Los scripts de `post` tienen que existir en tu `package.json`. Puedes poner cualquier comando, por ejemplo `npx tsc --noEmit`.

## Problemas frecuentes

### Un snippet de React aparece en ficheros `.ts`

Está en `typescript.json`. Muévelo a `typescriptreact.json`.

### El snippet no aparece dentro de una función flecha

Su `context` no incluye `arrow_function`. Cada forma de función es un contenedor distinto.

### Se añade un import que ya tenía con otro nombre

molde compara por el nombre local. Si el fichero tiene `import { z as zod } from "zod";` y el snippet pide `import { z } from "zod";`, son nombres distintos y molde lo añade.

### `molde scaffold` dice que falta un marcador

El comentario tiene que estar tal cual en el fichero, por ejemplo `// molde:routes`. molde no ha escrito nada: añade el marcador y vuelve a ejecutarlo.

Última actualización: 4 de noviembre de 2026

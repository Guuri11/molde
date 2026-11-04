# Usar molde en un proyecto Python

Cómo escribir snippets de Python con imports y contexto, capturar código y generar paquetes con scaffold.

## Contenido

- [Antes de empezar](#antes-de-empezar)
- [Dónde van los snippets](#dónde-van-los-snippets)
- [Escribir un snippet con imports](#escribir-un-snippet-con-imports)
- [Limitar dónde aparece un snippet](#limitar-dónde-aparece-un-snippet)
- [Dónde inserta molde los imports](#dónde-inserta-molde-los-imports)
- [Capturar código Python](#capturar-código-python)
- [Generar un paquete con scaffold](#generar-un-paquete-con-scaffold)
- [Problemas frecuentes](#problemas-frecuentes)

## Antes de empezar

Necesitas molde instalado y conectado a tu editor. Si no lo tienes, sigue [Empezar a usar molde](../guia/empezar-con-molde.md).

## Dónde van los snippets

| Para | Fichero |
|---|---|
| Todos tus proyectos Python | `~/.config/molde/snippets/python.json` |
| Solo este proyecto | `.molde/snippets/python.json` |

## Escribir un snippet con imports

Pon en `requires` los `import` que necesita el snippet, tal como los escribirías:

```json
{
  "fastapi-get-by-id": {
    "prefix": "fastapi get by id",
    "description": "Endpoint GET por id que devuelve 404 si no existe",
    "body": [
      "@router.get(\"/${1:invoices}/{${2:invoice}_id}\")",
      "def get_${2}(${2}_id: UUID) -> ${3:Invoice}:",
      "    ${2} = ${4:repository}.find(${2}_id)",
      "    if ${2} is None:",
      "        raise HTTPException(status_code=404)",
      "    return ${2}$0"
    ],
    "requires": [
      "from uuid import UUID",
      "from fastapi import HTTPException"
    ],
    "context": ["module"]
  }
}
```

molde separa cada import en nombres sueltos y solo añade los que faltan:

- `from fastapi import APIRouter, HTTPException` cuenta como `APIRouter` y `HTTPException` por separado
- funcionan igual `import json`, `import numpy as np` y `from x import a as b`
- si el fichero ya tiene `from fastapi import APIRouter` y el snippet pide `HTTPException`, molde añade `from fastapi import HTTPException` en una línea nueva

molde no fusiona las 2 líneas de `fastapi`. Hazlo con `ruff check --fix` o con el organizador de imports que uses.

> No pongas los `import` dentro del `body`. Así se insertarían donde está el cursor y se duplicarían si ya existen.

## Limitar dónde aparece un snippet

`context` lista los contenedores donde tiene sentido. El que cuenta es el más cercano al cursor.

| Valor | El cursor está |
|---|---|
| `module` | En el nivel superior del fichero |
| `class_definition` | Dentro de una clase, fuera de sus métodos |
| `function_definition` | Dentro de una función o un método |

Dentro de un método, el contenedor más cercano es `function_definition`, no `class_definition`. Un snippet para definir un método va con `class_definition`. Uno de código para el cuerpo va con `function_definition`.

Ejemplos:

- un endpoint, una clase o un test: `["module"]`
- un método o un `__init__`: `["class_definition"]`
- un `try`/`except` o un `with`: `["function_definition"]`

Sin `context`, el snippet aparece en cualquier parte del fichero.

## Dónde inserta molde los imports

- Detrás del último `import` del fichero.
- Si no hay ninguno, detrás de la cabecera: el shebang (`#!/usr/bin/env python3`), la línea de codificación (`# -*- coding: utf-8 -*-`) y el docstring del módulo.

`from __future__ import ...` cuenta como un import más, y molde lo pone detrás del último import. Python exige que vaya el primero, así que no lo pongas en `requires`.

## Capturar código Python

Selecciona 2 o más líneas, abre las acciones de código y elige **Guardar selección como snippet**.

En Python:

- `requires` recoge solo los imports que la selección usa de verdad: si el fichero tiene `import json` y `from pathlib import Path` y la selección solo usa `Path`, se copia solo el segundo
- los imports relativos (`from .models import Invoice`) se omiten, porque dependen de dónde esté el fichero, y se citan en la descripción para que los revises
- los literales, como `"/health"`, `5` o `200`, pasan a ser huecos con su valor original

Si un término se convierte en hueco y no quieres, añádelo a `common_words` en `.molde/molde.toml`.

## Generar un paquete con scaffold

1. Elige un paquete que funcione y pase sus tests y llámalo `sample_widget`. Usa ese nombre en todas sus formas: `SampleWidget`, `SAMPLE_WIDGET`, `sample_widgets`.

   ```
   app/sample_widget/
   ├── __init__.py
   └── service.py
   ```

2. Pon los marcadores donde se registra un paquete:

   ```python
   # app/main.py
   # molde:imports
   from app import sample_widget

   # molde:routers
   app.include_router(sample_widget.router)
   ```

3. Crea `.molde/templates/package/template.toml`:

   ```toml
   source = "app/sample_widget"
   source_name = "sample widget"
   target = "app/{snake}"
   post = ["ruff format", "ruff check"]

   [[inject]]
   file = "app/main.py"
   marker = "# molde:imports"
   line = "from app import {snake}"

   [[inject]]
   file = "app/main.py"
   marker = "# molde:routers"
   line = "app.include_router({snake}.router)"
   ```

4. Comprueba la plantilla con `molde check`.
5. Genera el paquete con `molde scaffold package line-item`.

molde crea `app/line_item/`, cambia `SampleWidgetService` por `LineItemService` y añade a `app/main.py`:

```python
from app import line_item
app.include_router(line_item.router)
```

Los comandos de `post` se ejecutan desde la raíz del proyecto. Si usas un entorno virtual, pon la ruta completa (`.venv/bin/ruff check`) o el lanzador de tu proyecto (`uv run ruff check`).

## Problemas frecuentes

### El snippet no aparece dentro de un método

Tiene `context` con `class_definition`. Dentro de un método el contenedor es `function_definition`.

### `molde scaffold` dice que falta un marcador

El comentario tiene que estar tal cual en el fichero, por ejemplo `# molde:routers`. molde no ha escrito nada: añade el marcador y vuelve a ejecutarlo.

Última actualización: 4 de noviembre de 2026

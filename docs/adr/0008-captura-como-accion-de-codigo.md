# 0008. La captura es una acción de código que se revisa en el fichero

Fecha: 4 de noviembre de 2026
Estado: aceptada, ampliada por [0012](0012-captura-con-ia-por-comando.md)

## Contexto

Convertir código en snippet tiene que costar una acción, desde cualquier editor. LSP no tiene una forma estándar de pedir texto libre al usuario, así que el servidor no puede preguntar el prefijo ni la descripción.

## Decisión

- La captura es una acción de código, **Guardar selección como snippet**. Solo aparece con una selección de 2 o más líneas.
- La acción ejecuta el comando `molde.capture`, que el servidor declara al iniciar.
- Se toman las líneas completas de la selección, sin la sangría común.
- La generalización es determinista:
  - un término es de dominio si aparece dentro de al menos 2 identificadores distintos, separando por `snake_case`, `camelCase` y `PascalCase`, y no está en el vocabulario común
  - el vocabulario común es una lista fija (`self`, `new`, `id`, `value`, `error`...) más las `common_words` del proyecto
  - los literales pasan a ser tabstops con su valor
  - los caracteres especiales de la sintaxis de snippets se escapan
- `requires` recoge los imports del fichero que usa la selección. Los relativos (`./db`, `super::`, `from .x`) se omiten y se citan en la descripción para revisarlos.
- La entrada se guarda en `.molde/snippets/<lenguaje>.json` con la clave `captura-N` y el prefijo `captura N`. molde pide al editor que abra el fichero en esa entrada.
- Si no hay carpeta `.molde`, se crea en la raíz del repositorio git.

## Consecuencias

- El prefijo y la descripción se escriben en el fichero. Gracias a la recarga de la [ADR 0006](0006-recarga-por-fecha-y-tamano.md), el snippet está disponible al guardar.
- La heurística a veces generaliza de más: en `props` y `NewInvoiceProps`, `props` pasa a ser un placeholder. Se deja con su valor por defecto y basta pulsar Tab.
- No se rellena `context`. Decidir dónde tiene sentido un snippet es cosa de quien lo revisa.

## Alternativas descartadas

- Pedir el prefijo con `window/showMessageRequest`: solo admite elegir entre botones, no escribir texto.
- Un comando propio de cada editor: obliga a mantener un plugin por editor.

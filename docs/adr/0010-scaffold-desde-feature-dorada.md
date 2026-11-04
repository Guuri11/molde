# 0010. Scaffold desde una feature dorada

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Crear una feature nueva en una arquitectura por capas supone varios ficheros de boilerplate y registrarla en 2 o 3 sitios. Es trabajo repetitivo que hoy se le pide a un agente de IA, con el coste en tokens y la variación que eso supone.

Las plantillas con un lenguaje propio (Handlebars, Tera) se desactualizan porque no se compilan ni se testean.

## Decisión

- La plantilla es una feature real del proyecto, la feature dorada, que compila y pasa sus tests.
- `template.toml` indica la carpeta de origen (`source`), su nombre (`source_name`), dónde crear la nueva (`target`), qué no copiar (`exceptions`), qué comandos ejecutar al terminar (`post`), en qué ficheros registrarla (`inject`) y, opcionalmente, qué fichero abrir (`main`).
- Al generar se sustituyen todas las variantes del nombre en las rutas y en el contenido: snake, kebab, Pascal, camel, SCREAMING, Title y minúsculas con espacios, en singular y en plural. Se sustituye primero la forma más larga, en una sola pasada.
- El plural sigue las reglas regulares del inglés. Los irregulares se pasan con `plural=`.
- `inject` añade una línea justo debajo de un comentario marcador, con su misma sangría.
- Todo se valida antes de escribir. Si falta un marcador, la plantilla está mal o el destino ya existe, no se toca nada.
- Si falla un comando de `post`, los ficheros generados se quedan y molde muestra la salida del comando y termina con error.
- La salida es una línea por fichero (`A ruta` si se crea, `M ruta` si se modifica) y nunca hace preguntas. Así la puede usar un agente.
- `molde check` comprueba que todas las plantillas se pueden aplicar.

## Consecuencias

- La plantilla no se desactualiza: si la feature dorada deja de compilar, lo detectan los tests del proyecto.
- El nombre de la feature dorada debe tener 2 palabras, como `sample widget`, para que cada variante sea inequívoca. Con una sola palabra, `widget` en snake y en minúsculas se confunden.
- La feature dorada se queda en el código como una feature más.
- Borrar lo generado tras un fallo queda en manos de git. Borrarlo de forma automática ocultaría el error.

## Alternativas descartadas

- Un motor de plantillas: es otro lenguaje que mantener, y las plantillas no se compilan.
- Generar con un agente de IA: más flexible, pero no determinista y con coste en cada uso.

# 0006. Recarga por fecha y tamaño en vez de `notify`

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Un snippet editado o capturado tiene que estar disponible en la siguiente completion, sin reiniciar el editor y aunque el fichero se edite desde otro programa. La primera idea era vigilar los ficheros con el crate `notify`.

## Decisión

- En cada completion, molde compara la fecha de modificación y el tamaño de los ficheros de snippets que aplican al lenguaje.
- Si han cambiado, los vuelve a leer.
- Si el fichero nuevo no es válido, sigue usando la última versión válida y muestra un aviso con el fichero y la línea del error.

## Consecuencias

- No depende de que el cliente avise de los cambios ni de eventos del sistema operativo.
- No necesita hilos ni colas, y en los tests es determinista.
- Cada completion hace unas pocas llamadas a `stat`. Con un puñado de ficheros por lenguaje, el coste no se nota.
- Un cambio que no altere ni la fecha ni el tamaño no se detecta. En la práctica, guardar un fichero siempre cambia la fecha.

## Alternativas descartadas

- `notify`: añade una dependencia, un hilo y eventos que llegan cuando quieren. Los tests tendrían que esperar.
- `workspace/didChangeWatchedFiles`: depende de que el cliente lo implemente y de que el fichero esté dentro del workspace. Los snippets de usuario no lo están.

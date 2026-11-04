# language: es
@casos-limite
Característica: Casos límite y decisiones técnicas
  Lo que hay que tener en cuenta para que molde sea fiable en el día a día.
  Cada regla recoge un problema conocido y la decisión tomada.

  Regla: Las posiciones de LSP y de Treesitter no se miden igual

    @hito2
    Escenario: Caracteres fuera de ASCII antes del cursor
      Dada una línea Python con el comentario "# importe en € del año, con 🧾" antes del cursor
      Cuando pido completion al final de esa línea
      Entonces molde localiza el nodo correcto para decidir el contexto
      Y las ediciones que devuelve no cortan ni desplazan caracteres
      # LSP cuenta en UTF-16 por defecto y Treesitter en bytes UTF-8.
      # Al iniciar se negocia UTF-8 si el cliente lo admite; si no, molde convierte.

  Regla: El código a medio escribir es lo normal

    @hito2
    Escenario: Cursor dentro de un nodo de error
      Dado un fichero TypeScript con una llave sin cerrar justo antes del cursor
      Cuando pido completion
      Entonces molde busca el contexto en el primer ancestro sin error
      Y si no puede decidirlo ofrece los snippets igualmente, con menor prioridad

    @hito2
    Escenario: Fichero muy grande
      Dado un fichero de más de 2 MB, umbral configurable con max_file_size_mb
      Cuando pido completion
      Entonces molde no lo analiza con Treesitter y ofrece los snippets sin filtrar por contexto

  Regla: Los imports van donde el lenguaje los espera

    @hito3
    Esquema del escenario: Cabeceras que deben quedar por encima del import
      Dado un fichero <lenguaje> que empieza por <cabecera> y no tiene el import requerido
      Cuando acepto un snippet que lo requiere
      Entonces el import se inserta después de <cabecera>

      Ejemplos:
        | lenguaje   | cabecera                                |
        | Python     | un shebang y un docstring de módulo     |
        | Python     | "from __future__ import annotations"    |
        | TypeScript | la directiva "use client"               |
        | Rust       | comentarios "//!" y atributos "#![...]" |

    @hito3
    Escenario: Módulo en línea de Rust
      Dado el cursor dentro de "mod tests" en un fichero Rust
      Cuando acepto un snippet que requiere "use rstest::rstest;"
      Entonces el import se añade dentro de "mod tests", no al principio del fichero

    @hito3
    Escenario: La edición del import viaja en el propio item
      Cuando molde devuelve un snippet con imports
      Entonces la edición del import va ya en el item de completion
      Y no depende de que el cliente pida "completionItem/resolve"

  Regla: No todos los clientes LSP implementan todo

    @hito0
    Escenario: El servidor declara sus comandos al iniciar
      Cuando el cliente inicializa molde
      Entonces la respuesta declara "molde.recordUse", "molde.stats", "molde.candidates" y "molde.scaffold" como comandos
      # Zed, por ejemplo, solo ejecuta el comando posterior a una completion si el servidor lo ha declarado

    @hito5
    Escenario: Cliente que no ejecuta el comando posterior a la completion
      Dado un editor que inserta los snippets pero no ejecuta su comando
      Cuando acepto snippets en ese editor
      Entonces los snippets funcionan con normalidad
      Pero esos usos no se registran
      Y molde no los estima a partir de "completionItem/resolve", que el cliente pide al resaltar un item y no al aceptarlo

    @hito4
    Escenario: Variantes de mayúsculas como placeholders separados
      Dado que no todos los clientes aplican las transformaciones de placeholders de la sintaxis de VSCode
      Cuando molde generaliza un término que aparece como "invoice" y como "Invoice"
      Entonces usa dos placeholders independientes en lugar de una transformación

  Regla: Repositorios grandes y monorepos

    @hito1
    Escenario: Varias carpetas .molde en un monorepo
      Dado un repositorio con "apps/web/.molde" y "services/api/.molde"
      Cuando abro un fichero de "services/api"
      Entonces molde usa la carpeta ".molde" más cercana subiendo desde ese fichero

    @hito1
    Escenario: Latencia de completion
      Dados 500 snippets cargados entre todas las capas
      Cuando pido completion en un fichero de 3.000 líneas ya abierto
      Entonces molde responde en menos de 50 ms

  Regla: Molde funciona en local

    @transversal
    Escenario: Sin conexión
      Dada una máquina sin red
      Cuando completo, capturo, consulto estadísticas y genero un scaffold
      Entonces todo funciona igual
      Y molde no abre ninguna conexión de red

  Regla: Snippets postfix

    @futuro
    Escenario: Postfix sobre una expresión de una línea
      Dado un snippet postfix "okjson" para Rust con cuerpo "Ok(Json(${receiver}.into()))"
      Y la línea "    build_dto(input).okjson" con el cursor al final
      Cuando acepto el snippet
      Entonces la línea queda "    Ok(Json(build_dto(input).into()))"

    @futuro
    Escenario: Postfix sobre una expresión de varias líneas
      Dada una expresión que empieza en una línea anterior a la del cursor
      Cuando escribo ".okjson" detrás de ella
      Entonces molde no ofrece el postfix
      # LSP exige que la edición principal de una completion ocupe una sola línea y contenga el cursor

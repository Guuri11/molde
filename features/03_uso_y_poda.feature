# language: es
@uso
Característica: Medir el uso y podar la colección
  Como desarrollador
  Quiero saber qué snippets uso, cuáles sobran y qué patrones me faltan
  Para que la colección no se convierta en un cajón desastre

  Regla: Cada expansión queda registrada en local

    @hito5
    Escenario: Registro de un uso
      Cuando acepto el snippet "axum-get-by-id"
      Entonces el editor ejecuta el comando "molde.recordUse" que acompaña al item
      Y molde guarda la clave del snippet, la raíz del proyecto, el lenguaje y la fecha
      Y lo guarda en "~/.local/share/molde/usage.sqlite", fuera del repositorio

    @hito5
    Escenario: El historial sobrevive a un cambio de prefijo
      Dado el snippet "axum-get-by-id" con 12 usos registrados
      Cuando cambio su prefijo y su descripción sin tocar su clave
      Entonces sigue teniendo 12 usos

    @hito5
    Escenario: Cada snippet tiene fecha de alta
      Dado un snippet que molde no había visto nunca
      Cuando lo carga por primera vez
      Entonces guarda esa fecha como fecha de alta del snippet

  Regla: Las estadísticas señalan qué podar

    @hito5
    Esquema del escenario: Candidatos a borrar con el umbral por defecto de 60 días
      Dado un snippet dado de alta hace <alta> días
      Y cuyo último uso fue <último uso>
      Cuando ejecuto "molde stats"
      Entonces el snippet <marca> como candidato a borrar

      Ejemplos:
        | alta | último uso   | marca      |
        | 200  | hace 90 días | aparece    |
        | 200  | hace 10 días | no aparece |
        | 20   | nunca        | no aparece |
        | 90   | nunca        | aparece    |

    @hito5
    Escenario: El umbral se configura por proyecto
      Dado ".molde/molde.toml" con unused_days = 30
      Y un snippet dado de alta hace 100 días y usado por última vez hace 40
      Cuando ejecuto "molde stats"
      Entonces el snippet aparece como candidato a borrar

    @hito5
    Escenario: Estadísticas desde el editor
      Cuando ejecuto el comando "molde.stats" desde el editor
      Entonces molde escribe el informe en Markdown en un fichero temporal
      Y pide al editor que lo abra

  Regla: La regla de tres propone los snippets que faltan

    @hito6
    Escenario: Patrón repetido sin snippet
      Dadas tres funciones Python en ficheros distintos con la misma estructura y distintos nombres y literales
      Y ningún snippet del proyecto con esa estructura
      Cuando ejecuto "molde candidates"
      Entonces aparece un candidato con las tres ubicaciones
      Y puedo crearlo con la misma generalización que usa la captura

    @hito6
    Escenario: El informe enseña los 20 candidatos con más apariciones
      Dados 25 patrones repetidos sin snippet, cada uno un número de veces distinto
      Cuando ejecuto "molde candidates"
      Entonces el informe enseña 20 candidatos, de más a menos apariciones
      Y al final dice que hay 25 en total
      # En un proyecto real salen más de 1700: la ADR 0013 explica el corte

    @hito6
    Escenario: Ignora lo trivial y lo que no es código del proyecto
      Dados patrones repetidos de menos de 5 líneas
      Y ficheros ignorados por ".gitignore", como "target", "node_modules" o "dist"
      Cuando ejecuto "molde candidates"
      Entonces no aparecen en el informe

    @hito6
    Escenario: No propone lo que ya cubre un snippet
      Dadas tres funciones con la misma estructura que el cuerpo del snippet "axum-get-by-id"
      Cuando ejecuto "molde candidates"
      Entonces ese patrón no aparece como candidato

    @hito6
    Escenario: Los candidatos no molestan mientras escribo
      Cuando edito código con patrones repetidos
      Entonces molde no publica diagnósticos sobre ellos
      Y solo los muestra cuando ejecuto "molde candidates" o el comando "molde.candidates"

  Regla: Medir cuánto se corrige un snippet tras expandirlo

    @futuro
    Escenario: Snippet con placeholders mal pensados
      Dado un snippet que tras expandirse se edita de media más de la mitad de su texto
      Cuando ejecuto "molde stats"
      Entonces aparece marcado para revisar sus placeholders
      # Requiere seguir el rango insertado a través de las ediciones posteriores, por eso queda para el final

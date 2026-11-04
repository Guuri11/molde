# language: es
@scaffold
Característica: Generar features completas desde una feature dorada
  Como desarrollador
  Quiero crear una feature de varios ficheros con un solo comando
  Para no repetir el boilerplate de la arquitectura ni gastar tokens del agente en él

  Antecedentes:
    Dado ".molde/templates/feature-crud/template.toml" con:
      """
      source = "src/features/sample_widget"
      source_name = "sample widget"
      target = "src/features/{snake}"
      exceptions = []
      post = ["cargo fmt", "cargo check"]

      [[inject]]
      file = "src/features/mod.rs"
      marker = "// molde:mods"
      line = "pub mod {snake};"

      [[inject]]
      file = "src/router.rs"
      marker = "// molde:routes"
      line = ".merge({snake}::routes())"
      """
    Y la feature dorada "src/features/sample_widget" compila y sus tests pasan

  @hito7
  Escenario: Se sustituyen todas las variantes del nombre
    Cuando ejecuto "molde scaffold feature-crud line-item"
    Entonces se crea "src/features/line_item" a partir de la feature dorada
    Y en nombres de fichero y en contenido se sustituyen estas variantes:
      | en la feature dorada | en la feature generada |
      | sample_widgets       | line_items             |
      | SampleWidgets        | LineItems              |
      | sample-widgets       | line-items             |
      | sample_widget        | line_item              |
      | SampleWidget         | LineItem               |
      | sampleWidget         | lineItem               |
      | sample-widget        | line-item              |
      | SAMPLE_WIDGET        | LINE_ITEM              |
      | Sample Widget        | Line Item              |
      | sample widget        | line item              |
    # La feature dorada usa un nombre de dos palabras para que cada variante sea inequívoca.
    # Se sustituye primero la forma más larga, para que el plural no quede a medias.

  @hito7
  Escenario: Plural irregular
    Cuando ejecuto "molde scaffold feature-crud person plural=people"
    Entonces "sample_widgets" pasa a ser "people" y "SampleWidgets" pasa a ser "People"

  @hito7
  Escenario: Registro en los marcadores
    Cuando ejecuto "molde scaffold feature-crud line-item"
    Entonces "src/features/mod.rs" tiene "pub mod line_item;" justo después de "// molde:mods"
    Y "src/router.rs" tiene ".merge(line_item::routes())" justo después de "// molde:routes"
    Y se ejecutan "cargo fmt" y "cargo check" y el comando termina con código 0

  @hito7
  Escenario: Todo se valida antes de escribir nada
    Dado que "src/router.rs" no contiene el marcador "// molde:routes"
    Cuando ejecuto "molde scaffold feature-crud line-item"
    Entonces el comando falla indicando el fichero y el marcador que faltan
    Y no se ha creado ni modificado ningún fichero

  @hito7
  Escenario: No sobrescribe una feature existente
    Dado que ya existe "src/features/line_item"
    Cuando ejecuto "molde scaffold feature-crud line-item"
    Entonces el comando falla sin tocar nada

  @hito7
  Escenario: Falla un comando posterior
    Dado que "cargo check" falla tras generar la feature
    Cuando ejecuto "molde scaffold feature-crud line-item"
    Entonces los ficheros generados se quedan en su sitio para revisarlos
    Y molde muestra la salida del comando y termina con un código distinto de 0
    # git permite deshacerlo, y borrar automáticamente ocultaría el error

  @hito7
  Esquema del escenario: Funciona igual en cualquier lenguaje
    Dada una plantilla de <lenguaje> con la feature dorada en "<origen>"
    Y los comandos posteriores <posteriores>
    Cuando genero una feature "line-item"
    Entonces la feature se crea con las mismas reglas de sustitución
    Y los comandos posteriores terminan con código 0

    Ejemplos:
      | lenguaje   | origen                     | posteriores                              |
      | Rust       | src/features/sample_widget | "cargo fmt" y "cargo check"              |
      | TypeScript | src/modules/sample-widget  | "npm run typecheck" y "npm run lint"     |
      | Python     | app/sample_widget          | "ruff format" y "ruff check"             |

  @hito7
  Escenario: Pensado para agentes
    Cuando Claude Code ejecuta "molde scaffold feature-crud line-item"
    Entonces la salida lista los ficheros creados y modificados, uno por línea
    Y el código de salida indica si ha ido bien
    Y no hay ninguna pregunta interactiva

  @hito7
  Escenario: La plantilla se valida en integración continua
    Cuando ejecuto "molde check"
    Entonces comprueba que la feature dorada existe y que todos los marcadores están en su sitio
    Y falla si la plantilla ya no se puede aplicar

  @hito7
  Escenario: Scaffold desde el editor
    Cuando ejecuto el comando "molde.scaffold" con la plantilla "feature-crud" y el nombre "line-item"
    Entonces molde genera la feature igual que la CLI
    Y pide al editor que abra el fichero principal de la feature

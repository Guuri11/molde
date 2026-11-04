# language: es
@hoja-de-ruta
Característica: Hoja de ruta de molde
  Cada hito se puede usar por sí solo desde el día en que se termina.
  Un hito está terminado cuando se cumple su escenario y pasan todos los escenarios con su etiqueta.

  Escenario: Hito 0, esqueleto y prueba de clientes
    Dado un workspace de Rust con los crates molde-core, molde-syntax, molde-store, molde-lsp, molde-cli y molde-acceptance
    Y un servidor de prueba que devuelve un item con una edición adicional y un comando posterior
    Cuando acepto ese item en Neovim con mi configuración de completion
    Entonces se aplica la edición adicional
    Y el servidor recibe el comando
    Y "cargo test" ejecuta todos los .feature, con los pasos aún sin implementar como omitidos
    Y pasan todos los escenarios @hito0
    # Si el cliente falla aquí, se decide cómo resolverlo antes de construir nada encima

  Escenario: Hito 1, snippets por capas en cualquier lenguaje
    Dados snippets de usuario y de proyecto para Rust, TypeScript y Python
    Cuando escribo en cualquiera de esos lenguajes
    Entonces la completion ofrece los snippets que corresponden a cada lenguaje
    Y "molde check" valida todos los ficheros de snippets
    Y pasan todos los escenarios @hito1
    # Desde aquí molde ya puede sustituir a tu colección actual de snippets

  Escenario: Hito 2, contexto con Treesitter
    Dadas las gramáticas de Rust, TypeScript, TSX y Python
    Cuando escribo en cualquier punto de un fichero
    Entonces solo veo los snippets cuyo contexto encaja
    Y pasan todos los escenarios @hito2

  Escenario: Hito 3, imports automáticos
    Cuando acepto un snippet con "requires"
    Entonces los imports que faltan aparecen en su sitio, sin duplicados
    Y pasan todos los escenarios @hito3

  Escenario: Hito 4, captura y recarga en caliente
    Cuando guardo una selección como snippet y edito su prefijo
    Entonces puedo usarlo al momento, en cualquier editor conectado
    Y pasan todos los escenarios @hito4

  Escenario: Hito 5, uso y poda
    Cuando llevo un mes usando molde
    Entonces "molde stats" me dice qué snippets sobran
    Y pasan todos los escenarios @hito5

  Escenario: Hito 6, regla de tres
    Cuando ejecuto "molde candidates" sobre un proyecto real
    Entonces recibo propuestas útiles y sin ruido
    Y pasan todos los escenarios @hito6

  Escenario: Hito 7, scaffold
    Cuando genero una feature completa con "molde scaffold"
    Entonces compila a la primera y queda registrada en el proyecto
    Y pasan todos los escenarios @hito7

  Escenario: Lo marcado como futuro
    Dados los escenarios etiquetados @futuro
    Entonces se abordan solo si el uso real demuestra que hacen falta
    # Snippets postfix y medir cuánto se corrige un snippet tras expandirlo

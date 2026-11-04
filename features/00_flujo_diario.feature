# language: es
@flujo
Característica: Flujo de trabajo diario con molde
  Como desarrollador que trabaja en Rust, TypeScript y Python
  Quiero snippets deterministas que entiendan el contexto y traigan sus imports
  Para escribir lo repetitivo rápido, sin depender de la IA, y sin que la colección se degrade

  Antecedentes:
    Dado que molde está instalado y registrado como servidor LSP en mi editor
    Y que el proyecto tiene su carpeta ".molde" versionada en git

  Escenario: Escribo un handler con un snippet del proyecto
    Dado que estoy en el nivel superior de un fichero Rust de "src/features/invoice"
    Y que el proyecto tiene el snippet "axum-get-by-id" con prefijo "axum handler get by id"
    Cuando escribo "ahg"
    Entonces la completion me ofrece ese snippet, marcado como de molde
    Cuando lo acepto
    Entonces se inserta el handler con el cursor en el primer placeholder
    Y se añaden arriba solo los imports que el snippet necesita y faltaban
    Y molde registra un uso de "axum-get-by-id"

  Escenario: Convierto en snippet lo que me acaba de proponer la IA
    Dado que acabo de aceptar una sugerencia de Copilot de varias líneas
    Cuando selecciono el texto aceptado y pido la acción "Guardar selección como snippet"
    Entonces molde añade una entrada al fichero de snippets del proyecto para ese lenguaje
    Y sustituye los términos de dominio y los literales por placeholders
    Y el editor abre el fichero en la entrada nueva para que revise prefijo y descripción
    Cuando guardo el fichero
    Entonces el snippet ya aparece en la completion, sin reiniciar nada

  Escenario: Creo una feature nueva con scaffold y dejo al agente solo lo específico
    Dado que el proyecto tiene la plantilla "feature-crud"
    Cuando ejecuto "molde scaffold feature-crud invoice"
    Entonces se crean los ficheros de la feature a partir de la feature dorada
    Y se registran el módulo y las rutas en los marcadores del proyecto
    Y pasan los comandos posteriores de la plantilla, como el formateo y la compilación
    Cuando le pido a Claude Code que implemente las reglas de negocio de "invoice"
    Entonces el agente parte del esqueleto generado y solo escribe lo que es propio de esa feature

  Escenario: Hago la revisión mensual de la colección
    Cuando ejecuto "molde stats"
    Entonces veo cada snippet con sus usos, su último uso y si es candidato a borrar
    Cuando ejecuto "molde candidates"
    Entonces veo los patrones que se repiten 3 o más veces en el código sin un snippet que los cubra
    Y decido qué borrar y qué crear, y lo subo en un commit

  Escenario: Comparto los snippets con el equipo
    Dado que otra persona clona el repositorio y tiene molde instalado
    Cuando abre el proyecto en un editor con cliente LSP
    Entonces tiene los mismos snippets y plantillas de proyecto que yo
    Y la integración continua ejecuta "molde check" y falla si algún snippet o plantilla es inválido

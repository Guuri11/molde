# 0012. La captura puede pedir el snippet a una IA a través de un comando

Fecha: 9 de noviembre de 2026
Estado: aceptada

## Contexto

La generalización por regla de la [ADR 0008](0008-captura-como-accion-de-codigo.md) se equivoca en código real. En un caso de uso de Rust con 45 líneas convierte en hueco `domain`, `factory`, `event` o `record`, parte `DomainEventRecord` en tres huecos, deja en `requires` los imports del dominio concreto y no puede poner prefijo ni descripción, porque LSP no deja pedir texto libre. Además copia la selección tal cual, con sus logs y sus textos concretos. Revisar cada captura cuesta casi tanto como escribir el snippet a mano.

Una IA hace bien justo esa parte: saber qué es dominio, qué sobra, cómo dejar el código limpio y cómo se llama el snippet. Las razones para no depender de la IA (variación, coste por uso, red) valen para usar un snippet, no para crearlo: la captura se hace una vez, se revisa en el fichero, y el snippet guardado sigue siendo determinista.

Cada persona del equipo usa una herramienta distinta: Claude Code, Copilot CLI, OpenCode o una API key. molde no debe mantener un cliente para cada una.

## Decisión

- La IA se activa con la variable de entorno `MOLDE_CAPTURE_COMMAND`. Sin ella, la captura funciona como dice la ADR 0008.
- El contrato es un comando que lee el prompt por la entrada estándar y escribe la respuesta por la salida estándar. molde lo ejecuta con `sh -c` y no sabe qué IA hay detrás. Comprobado el 9 de noviembre de 2026 capturando un caso de uso de Rust de 45 líneas:

  | Herramienta | `MOLDE_CAPTURE_COMMAND` | Captura real |
  |---|---|---|
  | Claude Code | `claude -p --model haiku` | 32 a 36 s |
  | Copilot CLI | `copilot -s -p "$(cat)"` | 26 a 47 s |
  | OpenCode | `opencode run -m github-copilot/claude-haiku-5.5 "$(cat)"` | 19 s |

  Las tres dejaron el caso de uso sin logs, con 3 o 4 huecos y sin los imports del dominio concreto. OpenCode necesita `-m`: con su modelo gratuito por defecto (`big-pickle`) no respondió en 120 s. Una API key es un script con `curl`. La clave nunca pasa por molde.

- El comando se ejecuta en una carpeta temporal vacía, no en el proyecto, con un límite de `MOLDE_CAPTURE_TIMEOUT` segundos (120 por defecto, por los 47 s de Copilot). Solo se lee la salida estándar: Copilot y OpenCode escriben sus cabeceras en la de error. Si se pasa del límite, molde termina el comando y todo lo que haya lanzado.
- El prompt es de molde. Lleva el lenguaje, la selección, los imports que usa (tal como los calcula la ADR 0008), los contenedores del lenguaje (`queries/<lenguaje>/containers.scm`) y el formato de respuesta. Nada más del fichero. Pide un snippet lo más limpio posible: huecos para los nombres de dominio, con el mismo número donde se repiten; fuera lo concreto de este caso, como logs, comentarios y textos; la estructura intacta; `$` y `}` escapados; y solo los imports que necesita el cuerpo, sin huecos.
- El prompt pide también que no use herramientas. El agente de OpenCode intentaba explorar la carpeta con `ls`, el permiso se le denegaba y terminaba sin responder nada.
- **La IA escribe el snippet entero.** Responde un objeto JSON con el formato de la [ADR 0003](0003-formato-de-snippet.md) más la clave:

  ```json
  {
    "key": "use-case-delete",
    "prefix": "use case delete",
    "description": "Caso de uso de borrado con unit of work y evento de dominio",
    "body": ["pub struct ${1:Delete}${2:Camera}UseCaseImpl {", "…"],
    "requires": ["use std::sync::Arc;", "use uuid::Uuid;"],
    "context": ["source_file"]
  }
  ```

- molde valida la respuesta con las mismas comprobaciones que `molde check`:
  - el cuerpo tiene que expandirse sin errores; si no, la respuesta entera se rechaza
  - cada `requires` tiene que ser un import válido del lenguaje; los que no lo son se descartan
  - cada `context` tiene que estar entre los contenedores del lenguaje; los que no, se descartan
  - `key` se usa si es kebab-case ASCII y está libre en el fichero; si no, `captura-N`
  - un `prefix` vacío pasa a ser la clave con espacios, como en la ADR 0008

  Si se descarta algo, molde lo dice en un aviso.
- Se acepta texto alrededor del JSON, como un bloque `` ```json ``: se usa el primer objeto.
- Si el comando no existe, termina con error, se pasa del límite o la respuesta no es un snippet válido, la captura se hace por regla y molde muestra un aviso con el motivo. La captura nunca se pierde.
- Mientras espera, molde informa del progreso al editor y no bloquea el resto del servidor: la completion sigue respondiendo. Para eso la captura va en dos pasos con el motor libre en medio (`prepare_capture` y `finish_capture`), y la IA se llama en un hilo aparte.

## Consecuencias

- Las capturas salen limpias y con nombre, prefijo, descripción y huecos con sentido.
- La IA puede cambiar el código, y es lo que se busca. A cambio, molde ya no garantiza que expandir el snippet reproduzca la selección: solo comprueba que es un snippet válido. Un cuerpo válido pero incorrecto solo se ve al revisarlo, así que la revisión en el fichero de la ADR 0008 sigue siendo obligatoria.
- Lo mismo con `requires`: molde comprueba que cada import es válido, no que exista ni que el cuerpo lo use.
- Capturar tarda de 20 a 50 segundos en vez de unos milisegundos.
- Dos capturas de la misma selección pueden dar resultados distintos. Se acepta: lo que importa es el snippet guardado.
- La selección y sus imports salen de la máquina hacia la IA configurada. Es una decisión de cada persona al poner la variable.
- Claude Code, Copilot CLI y OpenCode son agentes y pueden usar herramientas. La carpeta vacía evita que carguen las instrucciones del proyecto o lo toquen, pero limitar sus herramientas es cosa del comando configurado.
- molde sigue sin abrir conexiones de red. Si hay red, la abre el comando.
- Los tests de aceptación usan comandos falsos que responden un JSON fijo, fallan o tardan. No dependen de ninguna IA.

## Alternativas descartadas

- Que la IA solo elija los términos y los literales y molde construya el cuerpo: el cuerpo sería reversible por construcción, pero sería la selección tal cual, con sus logs y sus textos. No cumple el objetivo de un snippet limpio.
- Que la IA corrija el borrador de la regla: serían dos fuentes de verdad para el mismo cuerpo.
- Un cliente por proveedor dentro de molde: hay que mantener cada API y guardar claves.
- El comando en `.molde/molde.toml`: se versiona, así que impondría una herramienta a todo el equipo y clonar un repositorio bastaría para que la captura ejecutara un comando ajeno.

# language: es
@captura
Característica: Capturar código como snippet
  Como desarrollador
  Quiero convertir un fragmento en snippet con una sola acción, desde cualquier editor
  Para que los patrones que acepto de la IA, o que escribo yo, pasen a ser deterministas

  Regla: La captura es una acción de código del servidor

    @hito4
    Escenario: La acción solo aparece con una selección de varias líneas
      Dado un fichero TypeScript abierto
      Cuando pido acciones de código con una selección de una línea
      Entonces no aparece "Guardar selección como snippet"
      Cuando pido acciones de código con una selección de 3 líneas
      Entonces aparece "Guardar selección como snippet"

    @hito4
    Escenario: El snippet capturado se revisa en su propio fichero
      Cuando guardo una selección de Python como snippet
      Entonces la entrada nueva se añade a ".molde/snippets/python.json" con una clave única
      Y lleva un prefijo provisional y la descripción vacía
      Y molde pide al editor que abra el fichero en esa entrada
      # LSP no tiene un prompt de texto libre, así que prefijo y descripción se editan en el fichero

  Regla: La generalización es determinista y reversible
    Un término es de dominio si aparece dentro de al menos dos identificadores distintos,
    separando por snake_case, camelCase y PascalCase, y no está en el vocabulario común.

    @hito4
    Escenario: Los términos de dominio pasan a ser placeholders
      Dada la selección de Rust:
        """
        pub async fn get_invoice(
            State(svc): State<Arc<InvoiceService>>,
            Path(id): Path<Uuid>,
        ) -> Result<Json<InvoiceDto>, ApiError> {
            let invoice = svc.find(id).await?;
            Ok(Json(invoice.into()))
        }
        """
      Cuando la guardo como snippet
      Entonces todas las apariciones de "invoice" pasan a ser el placeholder 1
      Y todas las de "Invoice" pasan a ser el placeholder 2
      Y "svc", "id", "State", "Path", "Arc", "Uuid", "Result", "Json" y "ApiError" se quedan como están
      Y el cuerpo termina con el tabstop final "$0"

    @hito4
    Escenario: Los literales pasan a ser tabstops con su valor por defecto
      Dada la selección de Python:
        """
        response = client.get("/health", timeout=5)
        assert response.status_code == 200
        """
      Cuando la guardo como snippet
      Entonces "/health", 5 y 200 pasan a ser tabstops con su valor original como texto por defecto

    @hito4
    Escenario: El vocabulario común no se generaliza
      Dado ".molde/molde.toml" con common_words = ["user"]
      Y una selección donde "user" aparece en "current_user" y en "user_id"
      Cuando la guardo como snippet
      Entonces "user" no se convierte en placeholder

    @hito4
    Escenario: Expandir con los valores por defecto reproduce la selección
      Dada cualquier selección guardada como snippet
      Cuando expando el snippet sin cambiar ningún placeholder
      Entonces el texto insertado es idéntico a la selección original
      # Es la propiedad que se comprueba con muchos fragmentos reales de los tres lenguajes

    @hito4
    Escenario: Se escapan los caracteres especiales de la sintaxis de snippets
      Dada la selección de TypeScript:
        """
        const url = `${base}/invoices/${id}`;
        """
      Cuando la guardo como snippet
      Entonces los "$" y "}" del template literal quedan escapados en el cuerpo
      Y expandirlo reproduce la línea original

  Regla: Los imports que usa la selección se guardan en "requires"

    @hito4
    Escenario: Se copian los imports de paquetes, no los relativos
      Dado un fichero TypeScript con 'import { z } from "zod"' y 'import { db } from "./db"'
      Y una selección que usa "z" y "db"
      Cuando la guardo como snippet
      Entonces "requires" contiene el import de "zod"
      Pero no contiene el de "./db", porque una ruta relativa depende de dónde esté el fichero
      Y la descripción provisional avisa del import omitido para revisarlo

    @hito4
    Escenario: No se copian imports que la selección no usa
      Dado un fichero Python con "import json" y "from pathlib import Path"
      Y una selección que solo usa "Path"
      Cuando la guardo como snippet
      Entonces "requires" contiene solo "from pathlib import Path"

  Regla: Los ficheros de snippets se recargan solos

    @hito4
    Escenario: Recarga en caliente sin depender del editor
      Dado que el servidor está en marcha
      Cuando edito ".molde/snippets/python.json" desde cualquier programa y lo guardo
      Entonces la siguiente completion ya usa la versión nueva

    @hito4
    Escenario: Un fichero inválido no rompe nada
      Dado que guardo ".molde/snippets/rust.json" con un error de sintaxis en la línea 42
      Cuando molde intenta recargarlo
      Entonces sigue usando la última versión válida de ese fichero
      Y muestra un aviso con el fichero y la línea del error

  Regla: Con una IA configurada, la IA escribe el snippet y molde lo valida
    La IA se configura con la variable de entorno MOLDE_CAPTURE_COMMAND: un comando que lee
    el prompt por la entrada estándar y escribe la respuesta por la salida estándar.
    Sin la variable, la captura funciona como dicen las reglas anteriores.

    @hito8
    Escenario: La IA escribe el snippet entero, limpio
      Dado MOLDE_CAPTURE_COMMAND con una IA que responde:
        """
        {
          "key": "axum-get-by-id",
          "prefix": "axum handler get by id",
          "description": "Handler GET por id con servicio y DTO",
          "body": [
            "pub async fn get_${1:invoice}(",
            "    State(svc): State<Arc<${2:Invoice}Service>>,",
            "    Path(id): Path<Uuid>,",
            ") -> Result<Json<${2}Dto>, ApiError> {",
            "    Ok(Json(svc.find(id).await?.into()))",
            "\\}$0"
          ],
          "requires": ["use std::sync::Arc;", "use uuid::Uuid;"],
          "context": ["source_file", "mod_item"]
        }
        """
      Y la selección de Rust:
        """
        pub async fn get_invoice(
            State(svc): State<Arc<InvoiceService>>,
            Path(id): Path<Uuid>,
        ) -> Result<Json<InvoiceDto>, ApiError> {
            let invoice = svc.find(id).await?;
            Ok(Json(invoice.into()))
        }
        """
      Cuando la guardo como snippet
      Entonces la entrada nueva es "axum-get-by-id" con el prefijo, la descripción, el cuerpo, "requires" y "context" que ha respondido la IA
      # La IA puede cambiar el código para dejarlo limpio: aquí junta el find y el Ok en una línea

    @hito8
    Escenario: Un import o un contexto inválidos se descartan sin perder el resto
      Dado MOLDE_CAPTURE_COMMAND con una IA que responde un snippet válido con el import "use serde::{" y el contexto "class_body"
      Cuando guardo una selección de Rust como snippet
      Entonces la entrada se guarda con el cuerpo que ha respondido la IA
      Pero "requires" no contiene "use serde::{"
      Y "context" no contiene "class_body", que no es un contenedor de Rust
      Y molde muestra un aviso con lo que ha descartado

    @hito8
    Escenario: Una clave ocupada o inválida se sustituye por la provisional
      Dado ".molde/snippets/rust.json" con la clave "axum-get-by-id"
      Y MOLDE_CAPTURE_COMMAND con una IA que responde la clave "axum-get-by-id"
      Cuando guardo una selección de Rust como snippet
      Entonces la entrada nueva tiene la clave "captura-1"
      Y la entrada "axum-get-by-id" que ya existía no cambia
      Cuando la IA responde la clave "Get By Id!"
      Entonces la entrada nueva también tiene una clave "captura-N"

    @hito8
    Escenario: Se acepta texto alrededor del JSON
      Dado MOLDE_CAPTURE_COMMAND con una IA que responde:
        """
        Aquí tienes el snippet:
        ```json
        {"key": "health-check", "prefix": "health check", "description": "", "body": ["assert client.get(\"${1:/health}\").status_code == 200$0"], "requires": []}
        ```
        """
      Cuando guardo una selección de Python como snippet
      Entonces la entrada nueva tiene la clave "health-check"

    @hito8
    Esquema del escenario: Si la IA falla, la captura no se pierde
      Dado MOLDE_CAPTURE_COMMAND con una IA que <fallo>
      Cuando guardo una selección de Rust como snippet
      Entonces la entrada se guarda con la generalización por regla y la clave "captura-1"
      Y molde muestra un aviso con el motivo
      Y molde pide al editor que abra el fichero en esa entrada

      Ejemplos:
        | fallo                                                        |
        | no existe                                                    |
        | termina con código de salida 1                               |
        | tarda más que MOLDE_CAPTURE_TIMEOUT                          |
        | responde un texto sin ningún objeto JSON                     |
        | responde un JSON sin "body"                                  |
        | responde un cuerpo con un placeholder sin cerrar             |

    @hito8
    Escenario: La IA solo recibe la selección
      Dado MOLDE_CAPTURE_COMMAND con una IA que guarda lo que recibe y la carpeta donde se ejecuta
      Y un fichero Rust de 200 líneas con 6 imports
      Cuando guardo como snippet 5 líneas que usan 2 de esos imports
      Entonces la IA ha recibido el lenguaje, esas 5 líneas y esos 2 imports
      Pero ninguna otra línea del fichero
      Y el comando se ha ejecutado en una carpeta temporal vacía, fuera del proyecto

    @hito8
    Escenario: Mientras la IA responde, el servidor sigue trabajando
      Dado MOLDE_CAPTURE_COMMAND con una IA que tarda 2 segundos en responder
      Cuando guardo una selección de Rust como snippet
      Entonces el editor recibe un progreso de molde hasta que la entrada está guardada
      Y una completion pedida durante la espera responde sin esperar a la IA

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

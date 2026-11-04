# language: es
@completado
Característica: Completar snippets por capas, con contexto e imports
  Como desarrollador
  Quiero que molde me ofrezca solo los snippets que tienen sentido donde está el cursor
  Para no memorizar prefijos ni arreglar imports a mano

  Regla: Los snippets se cargan por capas y por lenguaje

    @hito1
    Escenario: La capa de proyecto sobrescribe a la de usuario
      Dado un snippet "test-basico" en "~/.config/molde/snippets/rust.json"
      Y otro snippet "test-basico" en ".molde/snippets/rust.json" del proyecto
      Cuando pido completion en un fichero Rust del proyecto
      Entonces solo aparece la versión del proyecto

    @hito1
    Escenario: El nombre del fichero decide el lenguaje
      Dados los ficheros de snippets "rust.json", "typescript.json", "python.json" y "all.json"
      Cuando pido completion en un fichero Python
      Entonces aparecen los snippets de "python.json" y de "all.json"
      Pero no aparecen los de "rust.json" ni los de "typescript.json"

    @hito1
    Escenario: Los ficheros TSX ven también los snippets de TypeScript
      Dados los ficheros de snippets "typescript.json" y "typescriptreact.json"
      Cuando pido completion en un fichero ".tsx"
      Entonces aparecen los snippets de los dos ficheros

    @hito1
    Escenario: El filtrado fuzzy lo hace el editor
      Dado un snippet con prefijo "axum handler get by id"
      Cuando pido completion tras escribir "ahg"
      Entonces molde devuelve el item con el prefijo completo como texto de filtrado
      Y el editor lo encuentra con "ahg" gracias a su propio filtrado fuzzy

    @hito1
    Escenario: Los items de molde se distinguen de los del LSP del lenguaje
      Dado que rust-analyzer y molde están activos en el mismo buffer
      Cuando pido completion
      Entonces los items de molde son de tipo Snippet y muestran "molde" como origen
      Y su prioridad de ordenación se puede ajustar en ".molde/molde.toml"

  Regla: El contexto sintáctico decide qué snippets se ofrecen
    El campo "context" lista los tipos de contenedor de Treesitter donde el snippet tiene sentido.
    El contenedor es el ancestro más cercano del cursor entre los que define cada lenguaje
    en "queries/<lenguaje>/containers.scm".

    @hito2
    Esquema del escenario: Snippet limitado a un contenedor
      Dado un snippet de <lenguaje> con context "<contexto>"
      Cuando pido completion con el cursor <posición>
      Entonces el snippet <resultado>

      Ejemplos:
        | lenguaje   | contexto            | posición                         | resultado  |
        | Rust       | source_file         | en el nivel superior del fichero | aparece    |
        | Rust       | source_file         | dentro del cuerpo de una función | no aparece |
        | TypeScript | class_body          | dentro del cuerpo de una clase   | aparece    |
        | TypeScript | class_body          | en el nivel superior del fichero | no aparece |
        | Python     | function_definition | dentro de una función            | aparece    |
        | Python     | function_definition | en el nivel superior del módulo  | no aparece |

    @hito2
    Escenario: Un snippet sin context aparece en todo su lenguaje
      Dado un snippet de Python sin campo "context"
      Cuando pido completion en cualquier punto de un fichero Python
      Entonces el snippet aparece

    @hito2
    Escenario: Lenguaje sin gramática de Treesitter
      Dado un fichero Go, lenguaje para el que molde no tiene gramática
      Y un fichero de snippets "go.json"
      Cuando pido completion
      Entonces aparecen todos los snippets de "go.json" sin filtrar por contexto
      Y los que declaran "requires" se insertan sin añadir imports
      Y molde avisa una sola vez en el log de que Go no tiene soporte de contexto ni de imports

    @hito2
    Escenario: Añadir un lenguaje no toca el núcleo
      Dada la gramática de Treesitter de Go añadida como dependencia del crate de sintaxis
      Y sus consultas de contenedores y de imports en "queries/go"
      Cuando recompilo molde
      Entonces Go tiene contexto e imports
      Y el crate del núcleo no ha cambiado

  Regla: Los imports de "requires" se añaden si faltan, nombre a nombre

    @hito3
    Escenario: Se añade el import que falta
      Dado un snippet de TypeScript que requiere 'import { z } from "zod"'
      Y un fichero TypeScript sin ese import
      Cuando acepto el snippet
      Entonces el item trae una edición adicional que inserta el import junto al resto de imports
      Y el cursor queda en el primer placeholder del snippet

    @hito3
    Escenario: No se duplica un import que ya existe
      Dado un fichero Python que contiene "from fastapi import APIRouter"
      Y un snippet que requiere "from fastapi import APIRouter"
      Cuando acepto el snippet
      Entonces no se añade ninguna línea de import

    @hito3
    Escenario: Un nombre ya importado por otra vía no se vuelve a importar
      Dado un fichero Rust que contiene "use axum::Json;"
      Y un snippet que requiere "use axum::{extract::Path, Json};"
      Cuando acepto el snippet
      Entonces solo se añade "use axum::extract::Path;"
      # En Rust y en TypeScript importar dos veces el mismo nombre es un error de compilación

    @hito3
    Escenario: Mismo módulo, nombre nuevo
      Dado un fichero Python que contiene "from fastapi import APIRouter"
      Y un snippet que requiere "from fastapi import HTTPException"
      Cuando acepto el snippet
      Entonces se añade "from fastapi import HTTPException" en una línea nueva
      Y fusionar las dos líneas queda en manos del organizador de imports del proyecto

# 0009. La regla de tres compara estructura sintáctica

Fecha: 4 de noviembre de 2026
Estado: aceptada, revisada en [0013](0013-informe-con-los-candidatos-mas-repetidos.md)

## Contexto

`molde candidates` propone snippets para lo que se repite. 2 fragmentos que solo se diferencian en los nombres y los literales son el mismo patrón. Comparar texto no los encontraría.

## Decisión

- La huella de un fragmento es su estructura en el árbol de Treesitter: tipos de nodo y tokens fijos, sin nombres, literales ni comentarios.
- Cuenta como candidato si tiene 5 líneas o más y aparece 3 veces o más.
- Un patrón que solo aparece dentro de las apariciones de otro mayor no se propone aparte.
- No se propone lo que ya cubre un snippet. Para saberlo, molde expande cada snippet con sus valores por defecto y calcula su huella.
- Se respeta `.gitignore` y se saltan los ficheros de más de `max_file_size_mb`.
- Los candidatos se ordenan por número de apariciones y luego por número de líneas.
- molde no publica diagnósticos sobre los candidatos mientras escribes. Solo aparecen al ejecutar `molde candidates` o el comando `molde.candidates`.

## Consecuencias

- Los primeros candidatos en un proyecto real son útiles. En un backend Rust de 242.000 líneas, los 2 primeros fueron un bloque de `unit_of_work.rs` repetido 120 veces y un `return Err(...::ValidationError(...))` repetido 161 veces.
- La lista completa tiene demasiado ruido. En ese mismo proyecto salieron 1.721 candidatos. La hoja de ruta pide "propuestas útiles y sin ruido", así que esta decisión queda pendiente de revisar. Las opciones más baratas son limitar la salida a los primeros o subir los umbrales.
- En ese proyecto el análisis completo tardó unos 10 segundos. Es aceptable para una revisión mensual.

## Alternativas descartadas

- Comparar texto normalizado: no detecta el mismo patrón con nombres distintos.
- Ventanas de N líneas: generan muchos solapes y cortan construcciones por la mitad.

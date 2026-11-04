# Decisiones de arquitectura

Cada fichero recoge una decisión: el problema, lo que se decidió y lo que implica. Si una decisión cambia, no se edita: se escribe una nueva que la sustituye y se marca la antigua como sustituida.

| Número | Decisión | Estado |
|---|---|---|
| [0001](0001-arquitectura-hexagonal.md) | Arquitectura hexagonal con un núcleo sin IO | Aceptada |
| [0002](0002-treesitter-para-contexto-e-imports.md) | Treesitter para el contexto y los imports | Aceptada |
| [0003](0003-formato-de-snippet.md) | VSCode JSON con `requires` y `context` | Aceptada |
| [0004](0004-imports-en-el-propio-item.md) | Los imports viajan en el propio item de completion | Aceptada |
| [0005](0005-registro-de-usos.md) | Registro de usos con un comando y SQLite fuera del repositorio | Aceptada |
| [0006](0006-recarga-por-fecha-y-tamano.md) | Recarga por fecha y tamaño en vez de `notify` | Aceptada |
| [0007](0007-variantes-como-placeholders.md) | Variantes de mayúsculas como placeholders independientes | Aceptada |
| [0008](0008-captura-como-accion-de-codigo.md) | La captura es una acción de código que se revisa en el fichero | Aceptada |
| [0009](0009-regla-de-tres-por-estructura.md) | La regla de tres compara estructura sintáctica | Aceptada, con revisión pendiente |
| [0010](0010-scaffold-desde-feature-dorada.md) | Scaffold desde una feature dorada | Aceptada |
| [0011](0011-especificacion-ejecutable.md) | La especificación en Gherkin es la batería de aceptación | Aceptada |

## Plantilla

```markdown
# NNNN. Título en forma de decisión

Fecha: d de mes de aaaa
Estado: propuesta | aceptada | sustituida por NNNN

## Contexto

Qué problema hay y qué lo condiciona.

## Decisión

Qué se hace, en presente.

## Consecuencias

Qué se gana, qué se pierde y qué queda pendiente.

## Alternativas descartadas

Qué más se consideró y por qué no.
```

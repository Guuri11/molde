# 0013. El informe de candidatos enseña los 20 más repetidos

Fecha: 9 de noviembre de 2026
Estado: aceptada

## Contexto

La [ADR 0009](0009-regla-de-tres-por-estructura.md) dejó pendiente el ruido: en un backend real de Rust salen 1751 candidatos y el informe ocupa 2,5 MB. Los primeros son útiles, el resto no se lee. La ADR 0009 proponía dos arreglos baratos: limitar la salida o subir los umbrales.

## Decisión

- `molde candidates` y `molde.candidates` enseñan solo los 20 primeros. Ya vienen ordenados de más a menos apariciones y, a igualdad, de más a menos líneas.
- Al final, una línea dice cuántos hay en total y que `molde candidates --save N` guarda el N.
- `--save N` sigue contando sobre la lista completa, así que el número del informe es el que se guarda.

## Consecuencias

- En ese backend, el informe pasa de 2,5 MB a 145 KB. Sigue ocupando bastante porque cada candidato lista todas sus apariciones: el primero tiene 215.
- El análisis tarda lo mismo: recorre todo el proyecto para poder ordenar.
- No hay forma de ver la lista completa desde el editor. Si hiciera falta, sería una opción de la CLI.

## Alternativas descartadas

- Subir los umbrales de líneas o de repeticiones: quita ruido, pero también patrones útiles, y la cantidad sigue dependiendo del tamaño del proyecto.

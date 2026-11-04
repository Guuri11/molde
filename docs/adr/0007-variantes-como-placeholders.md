# 0007. Variantes de mayúsculas como placeholders independientes

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Un mismo término de dominio aparece con distintas mayúsculas: `invoice` en `get_invoice` e `Invoice` en `InvoiceService`. La sintaxis de VSCode permite derivar una forma de otra con transformaciones (`${1/(.*)/${1:/capitalize}/}`), pero no todos los clientes las aplican, y un cliente que no las entiende inserta texto roto.

## Decisión

- Cada variante de mayúsculas es un placeholder propio. `invoice` es el 1 e `Invoice` es el 2.
- Las apariciones de una misma variante repiten su placeholder (`${1}`).
- molde no genera transformaciones.

## Consecuencias

- El snippet funciona igual en cualquier cliente.
- Al usarlo hay que escribir el nombre una vez por variante.
- La captura cumple una propiedad que comprueban los tests: expandir el snippet sin tocar los placeholders reproduce el texto original.

## Alternativas descartadas

- Transformaciones de VSCode: menos teclas, pero dependen del cliente.
- Un solo placeholder y adivinar las mayúsculas al insertar: LSP no deja al servidor intervenir mientras se rellenan los placeholders.

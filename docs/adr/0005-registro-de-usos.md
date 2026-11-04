# 0005. Registro de usos con un comando y SQLite fuera del repositorio

Fecha: 4 de noviembre de 2026
Estado: aceptada

## Contexto

Para proponer qué snippets borrar, molde necesita saber cuáles se usan. LSP no avisa al servidor cuando se acepta un item. Lo único fiable es un comando que el cliente ejecuta al aceptarlo.

Los usos son personales: no deben ir al repositorio ni provocar conflictos entre personas.

## Decisión

- Cada item lleva el comando `molde.recordUse` con la capa, la clave, el lenguaje y la raíz del proyecto.
- Los usos se guardan en `~/.local/share/molde/usage.sqlite`, en modo WAL y con espera si la base está ocupada, porque puede haber varios editores abiertos.
- La identidad de un snippet incluye la capa. Un snippet de usuario suma los usos de todos los proyectos. Uno de proyecto, solo los de su proyecto.
- La primera vez que molde ve un snippet guarda su fecha de alta. Un snippet que nunca se ha usado cuenta desde esa fecha.
- Es candidato a borrar si han pasado `unused_days` (60 por defecto) desde su último uso o, si no tiene, desde su alta.
- molde no estima usos a partir de `completionItem/resolve`.

## Consecuencias

- Si el cliente no ejecuta el comando, los snippets funcionan igual, pero `molde stats` no tiene datos. El Hito 0 comprueba esto antes de construir nada encima.
- Cambiar el prefijo o la descripción de un snippet conserva su historial. Cambiar su clave lo empieza de cero.
- Mover un snippet de proyecto a usuario también empieza su historial de cero, porque cambia la capa.
- Borrar `usage.sqlite` no rompe nada: solo se pierde el historial.

## Alternativas descartadas

- Guardar los usos en `.molde/`: ensucia el repositorio y mezcla los hábitos de todo el equipo.
- Contar los `resolve`: el cliente lo pide al resaltar un item, no al aceptarlo.

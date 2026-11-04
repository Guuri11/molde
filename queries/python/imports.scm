; @import: sentencias de import. @header: lo que tiene que quedar por encima
; (shebang, codificación y docstring del módulo).
(import_statement) @import
(import_from_statement) @import
(future_import_statement) @import
(module (expression_statement (string)) @header)
((comment) @header (#match? @header "^#!|^#.*coding[:=]"))

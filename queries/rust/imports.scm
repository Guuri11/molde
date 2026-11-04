; @import: sentencias de import. @header: lo que tiene que quedar por encima.
; @scope: ámbitos con sus propios imports, además del fichero.
(use_declaration) @import
(inner_attribute_item) @header
(line_comment (inner_doc_comment_marker)) @header
(block_comment (inner_doc_comment_marker)) @header
(mod_item body: (declaration_list) @scope)

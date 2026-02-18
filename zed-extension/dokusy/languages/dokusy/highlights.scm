; Comments
(line_comment) @comment
(block_comment) @comment

; Literals
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @boolean

; Identifiers
(identifier) @variable
(field_identifier) @property

; Function definitions and calls
(function_item
  name: (identifier) @function)

(call_expression
  function: (identifier) @function)

(parameters
  (parameter
    pattern: (identifier) @variable.parameter))

; Builtin
((call_expression
   function: (identifier) @function.builtin)
 (#eq? @function.builtin "print"))

; Types
(primitive_type) @type.builtin
(type_identifier) @type

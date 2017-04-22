require_relative "operator_definition"

module BergLang
    class Parser
        #
        # Handles parsing of operators, with precedence. See Parser for the actual defined list of
        # operators.
        #
        module OperatorList
            def self.define(*groups)
                precedence = 1
                operators = groups.flat_map do |operator_defs|
                    operator_defs = define_operators(precedence, *operator_defs)
                    precedence += 1
                    operator_defs
                end.group_by { |operator| operator.key }
                operators = operators.map do |key, operators|
                    [ key, operators.map { |operator| [ operator.type, operator ] }.to_h ]
                end
                operators.sort_by { |key, value| key.is_a?(String) ? -key.size : 0 }.to_h
            end

            def self.berg_operators
                @berg_operators ||= define(
                    [
                        # :BareDeclaration is handled outside the normal arity resolution rules because the rule is <!bareword>:bareword,
                        # which doesn't fit normal rules. Will have to think if there is a way ...
                        { string: ":", type: :prefix, resolve_manually: true },
                    ],
                    ". postfix.-- postfix.++",
                    "prefix.-- prefix.++ prefix.- prefix.+ prefix.!",
                    "* / %",
                    "+ -",
                    "> >= < <=",
                    "== !=",
                    "postfix.+ postfix.* postfix.?",
                    "&&",
                    "|| ??",
                    "->",
                    [
                        { string: ":",   direction: :right, declaration: true, opens_indent_block: true, },
                        { string: "=",   direction: :right, declaration: true, },
                        { string: "+=",  direction: :right, declaration: true, },
                        { string: "-=",  direction: :right, declaration: true, },
                        { string: "*=",  direction: :right, declaration: true, },
                        { string: "/=",  direction: :right, declaration: true, },
                        { string: "%=",  direction: :right, declaration: true, },
                        { string: "||=", direction: :right, declaration: true, },
                        { string: "&&=", direction: :right, declaration: true, },
                        { string: "??=", direction: :right, declaration: true, },
                    ],
                    [ ",", { string: ",", type: :postfix, can_be_sticky: false } ],
                    # TODO unsure if this is the right spot for intersect/union. Maybe closer to - and +
                    "&",
                    "|",
                    [ { key: :call } ],
                    [ "\n ;", { string: ";", type: :postfix, can_be_sticky: false } ],
                    # Delimiters want everything as children.
                    [
                        { key: :indent, type: :open,  ended_by: :undent,   direction: :right },
                        { key: :undent, type: :close, started_by: :indent, direction: :right },
                        { string: "(",  type: :open,  ended_by: ")",       direction: :right },
                        { string: ")",  type: :close, started_by: "(",     direction: :right },
                        { string: "{",  type: :open,  ended_by: "}",       direction: :right },
                        { string: "}",  type: :close, started_by: "{",     direction: :right },
                        { key: :sof,    type: :open,  ended_by: :eof,      direction: :right },
                        { key: :eof,    type: :close, started_by: :sof,    direction: :right },
                    ],
                )
            end

            private

            def self.define_operators(precedence, *operator_defs)
                #
                # Process the nice operator string definitions
                #
                operator_defs.flat_map do |operator_def|
                    if operator_def.is_a?(String)
                        # String is like "* / + *"
                        operator_def = operator_def.split(/ /)

                        # If string starts with "right", like "right = += -=", use that as direction
                        if %w{left right}.include?(operator_def.first)
                            direction = operator_def.shift.to_sym
                        else
                            direction = :left
                        end

                        # Parse through looking for prefix, infix, etc. "++.postfix --.postfix"
                        operator_def.map do |operator_string|
                            if operator_string =~ /^(.+)\.(.+)$/
                                OperatorDefinition.new(precedence: precedence, string: $2, type: $1.to_sym, direction: direction)
                            else
                                OperatorDefinition.new(precedence: precedence, string: operator_string, direction: direction)
                            end
                        end
                    else
                        OperatorDefinition.new(precedence: precedence, **operator_def)
                    end
                end
            end
        end
    end
end
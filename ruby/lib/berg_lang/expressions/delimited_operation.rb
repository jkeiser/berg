require_relative "../expression"

module BergLang
    module Expressions
        class DelimitedOperation < Expression
            attr_reader :open
            attr_reader :expression
            attr_reader :close

            def source_range
                SourceRange.span(open, close)
            end

            def initialize(open, expression, close)
                @open = open
                @expression = expression
                @close = close
            end

            def to_s
                "#{open}#{expression}#{close}"
            end
        end
    end
end

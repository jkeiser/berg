require_relative "operator_list"
require_relative "syntax_errors"
require_relative "../ast/bareword"
require_relative "../ast/float_literal"
require_relative "../ast/hexadecimal_literal"
require_relative "../ast/imaginary_literal"
require_relative "../ast/integer_literal"
require_relative "../ast/octal_literal"
require_relative "../ast/operator"
require_relative "../ast/string_literal"
require_relative "../ast/unrecognized_character"
require_relative "../ast/whitespace"

module BergLang
    class Parser
        #
        # Parses Berg.
        #
        class Tokenizer
            attr_reader :source
            attr_reader :output

            def initialize(source, output)
                @source = source
                @output = output
                @token = Ast::Operator.new(source.create_empty_range, all_operators[:sof])
            end

            def token
                if @token == :next
                    @token = parse_whitespace
                    @token ||= parse_number
                    @token ||= parse_operator
                    @token ||= parse_string
                    @token ||= parse_bareword
                    @token ||= create_eof_token if source.eof?
                    if !token
                        raise syntax_errors.unrecognized_character(Ast::UnrecognizedCharacter.new(source.match(/./)))
                    end
                    @token
                else
                    @token
                end
            end

            # Pick up the current token, and ensure we will pick up a new token next time.
            def advance_token
                result = self.token
                case result
                when eof_token
                    @token = nil
                when nil
                else
                    @token = :next
                end
                result
            end

            def all_operators
                OperatorList.berg_operators
            end

            private

            def syntax_errors
                SyntaxErrors.new
            end

            def parse_whitespace
                match = source.match(/\A((((?<newline>\n)(?<indent>[ \t]*))|\s)+|#[^\n]+)+/)
                Ast::Whitespace.new(match) if match
            end

            def parse_operator
                match = source.match(operators_regexp)
                if match
                    Ast::Operator.new(match, all_operators[match.string])
                end
            end

            def parse_bareword
                match = source.match(/\A(\w|[_$])+/)
                Ast::Bareword.new(match) if match
            end

            def parse_string
                if source.peek == '"'
                    match = source.match(/\A"(\\.|[^\\"]+)*"/m)
                    if match
                        Ast::StringLiteral.new(match)
                    else
                        match = source.match(/\A"(\\.|[^\\"]+)*/m)
                        raise syntax_errors.unclosed_string(match)
                    end
                end
            end

            def eof_token
                @eof_token
            end

            def create_eof_token
                @eof_token ||= Ast::Operator.new(source.create_empty_range, all_operators[:eof])
            end

            def parse_number
                # Handle hex literals (0xDEADBEEF)
                # prefix integer
                match = source.match /\A(?<prefix>0[xX])(?<integer>(\d|[A-Fa-f])+)/
                if match
                    illegal_word_characters = source.match /\A(\w|[_$])+/
                    # Word characters immediately following a number is illegal.
                    if illegal_word_characters
                        raise syntax_errors.variable_name_starting_with_an_integer(SourceRange.span(match, illegal_word_characters))
                    end
                    return Ast::HexadecimalLiteral.new(match)
                end

                #
                # Handle floats, imaginaries and integers (hex is later in this function)
                #
                # integer (. decimal)? (e expsign? exponent)? i?
                match = source.match /\A(?<integer>\d+)((\.)(?<decimal>\d+))?((e)(?<expsign>[-+])?(?<exp>\d+))?(?<imaginary>i)?/i
                if match
                    illegal_word_characters = source.match /\A(\w|[_$])+/
                    # Word characters immediately following a number is illegal.
                    if illegal_word_characters
                        if !match[:exp] && !match[:imaginary] && illegal_word_characters.string.downcase == "e"
                            raise syntax_errors.empty_exponent(SourceRange.span(match, illegal_word_characters))
                        elsif match[:decimal]
                            raise syntax_errors.float_with_trailing_identifier(SourceRange.span(match, illegal_word_characters))
                        else
                            raise syntax_errors.variable_name_starting_with_an_integer(SourceRange.span(match, illegal_word_characters))
                        end
                    end

                    is_imaginary = match[:imaginary]
                    is_float = match[:decimal] || match[:exp]
                    is_octal = !is_float && match[:integer] && match[:integer].length > 1 && match["integer"].start_with?("0")
                    if is_imaginary
                        Ast::ImaginaryLiteral.new(match)

                    elsif is_float
                        Ast::FloatLiteral.new(match)

                    elsif is_octal
                        if match[:integer] =~ /[89]/
                            raise syntax_errors.illegal_octal_digit(match)
                        end
                        Ast::OctalLiteral.new(match)

                    elsif match[:integer]
                        Ast::IntegerLiteral.new(match)

                    else
                        raise syntax_errors.internal_error(match, "ERROR: number that doesn't fit any category: #{match.string}")
                    end
                end
            end

            def operators_regexp
                @operators_regexp ||= Regexp.new(
                    "\\A(" +
                    all_operators.keys.select { |key| key.is_a?(String) }
                                      .sort_by { |key| -key.length }
                                      .map { |key| Regexp.escape(key) }
                                      .join("|") +
                    ")"
                )
            end

        end
    end
end
use ast::intern_pool::Pool;
use ast::{IdentifierIndex,LiteralIndex};
use std::ops::Range;
use compiler::compile_errors::SourceCompileErrors;
use ast::token::Token::*;
use ast::token::PrefixToken::*;
use ast::token::PostfixToken::*;
use ast::token::InfixToken::*;
use ast::token::TermToken::*;
use parser::scanner;
use parser::scanner::Symbol;
use public::*;

#[derive(Debug)]
enum Need {
    Term,
    Operand(Range<ByteIndex>),
    Operator,
}

impl Need {
    fn after(token: Token, start: ByteIndex, end: ByteIndex) -> Need {
        match token {
            Infix(_)|Prefix(PrefixOperator(_)) => Need::Operand(start..end),
            Prefix(Open(_)) => Need::Term,
            Postfix(_)|Term(_) => Need::Operator,
        }
    }
    fn before(token: Token, start: ByteIndex, end: ByteIndex) -> Need {
        match token {
            Infix(_)|Postfix(PostfixOperator(_)) => Need::Operand(start..end),
            Postfix(Close(_)) => Need::Term,
            Term(_)|Prefix(_) => Need::Operator,
        }
    }
}

///
/// Breaks a file into a series of Tokens, calling the given function for each
/// token.
/// 
pub(crate) fn tokenize<F: FnMut(Token,Range<ByteIndex>)->()>(
    buffer: &[u8],
    errors: &mut SourceCompileErrors,
    identifiers: &mut Pool<IdentifierIndex>,
    literals: &mut Pool<LiteralIndex>,
    mut on_token: F
) {
    let mut start = ByteIndex(0);
    let mut need = Need::Term;
    while let Some((symbol, index)) = scanner::next(buffer, start) {
        println!("{:?}-{:?}: {:?}", start, index, symbol);
        let token = match symbol {
            Symbol::Integer => Some(Term(IntegerLiteral(unsafe { literals.add_utf8_unchecked(buffer, start, index) }))),
            Symbol::Open => Some(Prefix(Open(unsafe { identifiers.add_utf8_unchecked(buffer, start, index) }))),
            Symbol::Close => Some(Postfix(Close(unsafe { identifiers.add_utf8_unchecked(buffer, start, index) }))),
            Symbol::Operator => match need {
                Need::Term|Need::Operand(_) => Some(Prefix(PrefixOperator(unsafe { identifiers.add_utf8_unchecked(buffer, start, index) }))),
                Need::Operator => {
                    if scanner::next_has_left_operand(buffer, index) {
                        Some(Postfix(PostfixOperator(unsafe { identifiers.add_utf8_unchecked(buffer, start, index) })))
                    } else {
                        Some(Infix(InfixOperator(unsafe { identifiers.add_utf8_unchecked(buffer, start, index) })))
                    }
                },
            },
            Symbol::UnsupportedCharacters => {
                report_valid_utf8(errors, CompileErrorType::UnsupportedCharacters, start..index, buffer);
                None
            },
            Symbol::InvalidUtf8Bytes => { errors.report_invalid_utf8(start..index, buffer); None },
        };

        // Report if there are missing operands, and insert the MissingOperand/NoExpression/MissingInfix token.
        if let Some(token) = token {
            let missing_token = report_missing_operands(need, buffer, token, start, index, errors);
            if let Some(missing_token) = missing_token {
                on_token(missing_token, start..start);
            }
            on_token(token, start..index);
            need = Need::after(token, start, index);
        }

        start = index
    }
    match need {
        Need::Term => on_token(Term(NoExpression), start..start),
        Need::Operand(range) => {
            report_valid_utf8(errors, CompileErrorType::MissingRightOperand, range, buffer);
            on_token(Term(MissingOperand), start..start);
        },
        Need::Operator => {}
    }
}

fn report_missing_operands(
    after_prev: Need,
    buffer: &[u8],
    token: Token,
    start: ByteIndex,
    end: ByteIndex,
    errors: &mut SourceCompileErrors
) -> Option<Token> {
    use parser::tokenizer::Need::*;
    let result = match (after_prev, Need::before(token, start, end)) {
        (Operator,Term)|(Operator,Operand(_))|(Term,Operator)|(Operand(_),Operator) => None,
        (Operator,Operator) => Some(Infix(MissingInfix)),
        (Term,Term) => Some(Token::Term(NoExpression)),
        (Term,Operand(second)) => {
            report_valid_utf8(errors, CompileErrorType::MissingLeftOperand, second, buffer);
            Some(Token::Term(MissingOperand))
        },
        (Operand(first),Term) => {
            report_valid_utf8(errors, CompileErrorType::MissingRightOperand, first, buffer);
            Some(Token::Term(MissingOperand))
        },
        (Operand(first),Operand(second)) => {
            report_valid_utf8(errors, CompileErrorType::MissingOperandsBetween, first.start..second.end, buffer);
            Some(Token::Term(MissingOperand))
        },
    };
    println!("Thus we return {:?}", result);
    result
}

fn report_valid_utf8(errors: &mut SourceCompileErrors, error_type: CompileErrorType, range: Range<ByteIndex>, buffer: &[u8]) {
    unsafe { errors.report_at_utf8_unchecked(error_type, range, buffer) }
}

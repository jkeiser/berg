pub mod char_data;
pub mod scanner;
pub mod syntax_expression;

use public::*;
use parser::scanner::Scanner;

/// Shared parsing state
#[derive(Debug)]
struct Parser<'s, 'c: 's> {
    pub scanner: Scanner<'s, 'c>,
    pub char_data: CharData,
    pub expressions: Vec<SyntaxExpression>,
}

pub fn parse<'s>(compiler: &'s Compiler, source: SourceIndex) {
    let (char_data, expressions) = compiler.with_source(source, |s| {
        s.source().with_buffer(compiler, source, |raw_buffer| {
            let scanner = Scanner::new(compiler, source, raw_buffer);
            let parser = Parser::new(scanner);
            parser.parse()
        })
    });
    compiler.with_source_mut(source, |s| {
        s.parse_complete(char_data, expressions);
    });
}

impl<'s, 'c: 's> Parser<'s, 'c> {
    pub fn new(scanner: Scanner<'s, 'c>) -> Self {
        let char_data = CharData::new();
        let expressions = vec![];
        Parser { scanner, char_data, expressions }
    }

    pub fn parse(mut self) -> (CharData, Vec<SyntaxExpression>) {
        while self.step() {}
        (self.char_data, self.expressions)
    }

    fn step(&mut self) -> bool {
        if self.scanner.index >= self.scanner.len() {
            return false;
        }
        
        if self.scan_term() {
            return true;
        }
        
        // Unsupported characters or invalid UTF-8 must be here.
        let start = self.scanner.index;
        let mut string = String::new();
        // If there are valid UTF-8 chars, they are just unsupported. Report 
        if self.scanner.take_valid_char(&mut string) {
            while !self.scan_term() && self.scanner.take_valid_char(&mut string) {}
            self.scanner.compiler.report_at(UnsupportedCharacters, self.scanner.source, start, &string);
        } else {
            // Invalid UTF-8. Read invalid characters until you find something valid.
            let mut bytes: Vec<u8> = vec![];
            self.scanner.take_byte(&mut bytes);
            while self.scanner.index < self.scanner.len() && !self.scanner.is_valid_char() {
                self.scanner.take_byte(&mut bytes);
            }
            self.scanner.compiler.report_invalid_bytes(InvalidUtf8, self.scanner.source, start, &bytes);
        }
        true
    }

    fn scan_term(&mut self) -> bool {
        let mut index = self.scanner.index;
        match self.scanner[index] {
            b'0'...b'9' => {
                index = self.scanner.match_all(&(b'0'..=b'9'), index + 1);
                self.token(IntegerLiteral, index)
            },
            _ => false,
        }
    }

    fn token(&mut self, expression_type: SyntaxExpressionType, end: ByteIndex) -> bool {
        let expression = self.scanner.take_token(expression_type, end);
        self.expressions.push(expression);
        true
     }
}

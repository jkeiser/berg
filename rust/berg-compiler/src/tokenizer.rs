use berg::*;
use berg::SyntaxExpressionType::*;
use compile_errors::*;
use source_reader::*;

use std::mem;
use std::ops::Range;

pub struct Tokenizer<'a, R: SourceReader + 'a> {
    reader: &'a mut R,
    start: GraphemeIndex,
    buffer: String,
    expressions: Vec<SyntaxExpression>,
}

impl<'a, R: SourceReader + 'a> Tokenizer<'a, R> {
    pub fn new(reader: &'a mut R) -> Tokenizer<'a, R> {
        let start = reader.index();
        let buffer = String::new();
        let expressions = vec![];
        Tokenizer { reader, start, buffer, expressions }
    }
    pub fn open(&mut self, berg: &Berg) -> bool {
        self.reader.open(berg)
    }
    pub fn close(self) -> Vec<SyntaxExpression> {
        self.expressions
    }

    pub fn next(&mut self) -> Option<usize> {
        // IntegerLiteral
        if self.read_if(Self::is_digit) {
            self.consume_while(IntegerLiteral, Self::is_digit)

        // EOF
        } else if self.reader.peek().is_none() {
            None

        // Unsupported character
        } else {
            self.discard_while(Self::is_unsupported);
            let range = self.range();
            self.report(UnsupportedCharacters(range));
            None
        }
    }

    fn range(&self) -> Range<GraphemeIndex> {
        Range { start: self.start, end: self.reader.index()-1 }
    }

    fn is_digit(ch: char) -> bool { ch >= '0' && ch <= '9' }
    fn is_significant(ch: char) -> bool { Self::is_digit(ch)  }
    fn is_unsupported(ch: char) -> bool { !Self::is_significant(ch) }

    fn read_if(&mut self, valid_char: fn(char) -> bool) -> bool {
        if let Some(ch) = self.reader.read_if(valid_char) {
            self.buffer.push(ch);
            true
        } else {
            false
        }
    }
    fn read_while(&mut self, valid_char: fn(char) -> bool) -> bool {
        self.reader.read_while(valid_char, &mut self.buffer)
    }

    fn consume(&mut self, expression_type: SyntaxExpressionType) -> Option<usize> {
        if self.buffer.len() == 0 {
            return None;
        }

        let mut expression = SyntaxExpression {
            expression_type,
            string: String::new(),
            start: self.start,
        };
        mem::swap(&mut self.buffer, &mut expression.string);
        self.start = self.reader.index();
        self.expressions.push(expression);
        Some(self.expressions.len()-1)
    }
    fn consume_while(&mut self, expression_type: SyntaxExpressionType, valid_char: fn(char) -> bool) -> Option<usize> {
        self.read_while(valid_char);
        self.consume(expression_type)
    }
    fn discard(&mut self) {
        self.start = self.reader.index();
        self.buffer = String::new();
    }
    fn discard_while(&mut self, valid_char: fn(char) -> bool) -> bool {
        let result = self.read_while(valid_char);
        self.discard();
        result
    }

    fn report(&mut self, error: CompileError) { self.reader.report(error); }
}

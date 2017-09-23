mod internals;
mod source_reader;
mod file_source_reader;
mod string_source_reader;
mod parse_result;
mod syntax_expression;
mod tokenizer;

pub use parser::parse_result::ParseResult;
pub use parser::syntax_expression::SyntaxExpression;
pub use parser::syntax_expression::SyntaxExpressionType;
use parser::internals::*;

/// Shared parsing state
pub struct Parser<'a, R: SourceReader<'a>> {
    tokenizer: Tokenizer<'a, R>,
}

pub fn parse<'a>(source: &'a Source, berg: &Berg) -> ParseResult<'a> {
    match *source {
        Source::File(..) => Parser::<FileSourceReader<'a>>::parse(source, berg),
        Source::String(..) => Parser::<StringSourceReader<'a>>::parse(source, berg),
    }
}

impl<'a, R: SourceReader<'a>> Parser<'a, R> {
    fn parse(source: &'a Source, berg: &Berg) -> ParseResult<'a> {
        let mut parser = Self::from_source(source);
        if parser.open(berg) {
            while parser.step() {};
        }
        parser.close()
    }
    fn from_source(source: &'a Source) -> Parser<'a, R> {
        let tokenizer = Tokenizer::from_source(source);
        Parser { tokenizer }
    }
    fn open(&mut self, berg: &Berg) -> bool {
        self.tokenizer.open(berg)
    }

    fn step(&mut self) -> bool {
        if let Some(_) = self.tokenizer.next() {
            true
        } else {
            false
        }
    }

    fn close(self) -> ParseResult<'a> {
        let (metadata, expressions, errors) = self.tokenizer.close();
        ParseResult { metadata, expressions, errors }
    }
}

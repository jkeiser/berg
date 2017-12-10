mod grouper;
mod sequencer;
mod tokenizer;

use compiler::Compiler;
use compiler::source_data::{ParseData,SourceIndex};

pub(super) fn parse<'c>(compiler: &Compiler<'c>, source: SourceIndex) -> ParseData
{
    let ast_builder = grouper::Grouper::new(compiler, source);
    let tokenizer = tokenizer::Tokenizer::new(ast_builder);
    let mut sequencer = sequencer::Sequencer::new(tokenizer);
    sequencer.tokenize();
    sequencer.complete()
}

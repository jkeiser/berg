use parser::internals::*;

// ExpressionType, String, LeftChild, RightChild
#[derive(Debug)]
pub struct SyntaxExpression {
    pub expression_type: SyntaxExpressionType,
    pub string: String,
    pub start: usize,
}

impl<'a> fmt::Display for SyntaxExpression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.expression_type {
            IntegerLiteral => write!(f, "{:?}", self.string)
        }
    }
}

#[derive(Debug)]
pub enum SyntaxExpressionType {
    IntegerLiteral,
}

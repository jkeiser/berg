use std::borrow::Cow;
use syntax::OperandPosition::*;
use syntax::char_data::CharData;
use value::BergError;
use eval::{Expression, RootRef};
use parser::{ByteRange, SourceRef};
use std;
use std::fmt::{Display, Formatter, Result};
use std::{io, u32};
use std::rc::Rc;
use syntax::Token;
use util::indexed_vec::IndexedVec;
use util::intern_pool::{InternPool, StringPool};

index_type! {
    pub struct AstIndex(pub u32) <= u32::MAX;
    pub struct BlockIndex(pub u32) <= u32::MAX;
    pub struct IdentifierIndex(pub u32) <= u32::MAX;
    pub struct LiteralIndex(pub u32) <= u32::MAX;
    pub struct FieldIndex(pub u32) <= u32::MAX;
}

pub type Tokens = IndexedVec<Token, AstIndex>;
pub type TokenRanges = IndexedVec<ByteRange, AstIndex>;

// So we can signify that something is meant to be a *difference* between indices.
pub type AstDelta = Delta<AstIndex>;

#[derive(Clone, Debug)]
pub struct Field {
    pub name: IdentifierIndex,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ExpressionBoundaryError {
    CloseWithoutOpen,
    OpenWithoutClose,
    OpenError, // Error opening or reading the source file
    None,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Fixity {
    Term,
    Infix,
    Prefix,
    Postfix,
    Open,
    Close,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ExpressionBoundary {
    PrecedenceGroup,
    CompoundTerm,
    Parentheses,
    CurlyBraces,
    Source,
    Root,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperandPosition {
    Left,
    Right,
    PrefixOperand,
    PostfixOperand,
}

#[derive(Debug)]
pub struct AstBlock {
    pub boundary: ExpressionBoundary,
    pub parent: Delta<BlockIndex>,
    pub scope_start: FieldIndex,
}

#[derive(Clone)]
pub struct AstRef<'a>(Rc<AstData<'a>>);

#[derive(Debug)]
pub struct AstData<'a> {
    pub source: SourceRef<'a>,
    pub char_data: CharData,
    pub identifiers: InternPool<IdentifierIndex>, // NOTE: if needed we can save space by removing or clearing the StringPool after making this readonly.
    pub literals: StringPool<LiteralIndex>,
    pub tokens: Tokens,
    pub token_ranges: TokenRanges,
    pub blocks: IndexedVec<AstBlock, BlockIndex>,
    pub fields: IndexedVec<Field, FieldIndex>,
    pub file_open_error: Option<(BergError, io::Error)>,
}

impl<'a> AstData<'a> {
    pub fn new(source: SourceRef<'a>) -> AstData<'a> {
        let identifiers = source.root().identifiers();
        let fields = source
            .root()
            .field_names()
            .map(|name| Field { name: *name })
            .collect();
        AstData {
            source,
            identifiers,
            fields,

            char_data: Default::default(),
            literals: Default::default(),
            blocks: Default::default(),
            tokens: Default::default(),
            token_ranges: Default::default(),
            file_open_error: None,
        }
    }
}

impl<'a> AstRef<'a> {
    pub fn new(data: AstData<'a>) -> Self {
        AstRef(Rc::new(data))
    }

    pub fn source(&self) -> &SourceRef<'a> {
        &self.0.source
    }
    pub fn root(&self) -> &RootRef {
        self.source().root()
    }

    pub fn expression(&self) -> Expression {
        assert_ne!(self.0.tokens.len(), 0);
        Expression(AstIndex(0))
    }

    pub fn char_data(&self) -> &CharData {
        &self.0.char_data
    }
    pub fn identifiers(&self) -> &InternPool<IdentifierIndex> {
        &self.0.identifiers
    }
    pub fn literals(&self) -> &StringPool<LiteralIndex> {
        &self.0.literals
    }
    pub fn tokens(&self) -> &IndexedVec<Token, AstIndex> {
        &self.0.tokens
    }
    pub fn token_ranges(&self) -> &IndexedVec<ByteRange, AstIndex> {
        &self.0.token_ranges
    }
    pub fn fields(&self) -> &IndexedVec<Field, FieldIndex> {
        &self.0.fields
    }
    pub fn blocks(&self) -> &IndexedVec<AstBlock, BlockIndex> {
        &self.0.blocks
    }

    pub fn token(&self, index: AstIndex) -> &Token {
        &self.0.tokens[index]
    }
    pub fn token_string(&self, index: AstIndex) -> Cow<str> {
        self.0.tokens[index].to_string(self)
    }
    pub fn token_range(&self, index: AstIndex) -> ByteRange {
        self.0.token_ranges[index].clone()
    }
    pub fn identifier_string(&self, index: IdentifierIndex) -> &str {
        &self.0.identifiers[index]
    }
    pub fn literal_string(&self, index: LiteralIndex) -> &str {
        &self.0.literals[index]
    }
    pub fn open_error(&self) -> &BergError {
        &self.0.file_open_error.as_ref().unwrap().0
    }
    pub fn open_io_error(&self) -> &io::Error {
        &self.0.file_open_error.as_ref().unwrap().1
    }
    pub fn field_name(&self, index: FieldIndex) -> &str {
        self.identifier_string(self.fields()[index].name)
    }
}

impl<'a> fmt::Debug for AstRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ast({:?})", self.source().name())
    }
}

impl<'a> AstData<'a> {
    pub fn push_token(&mut self, token: Token, range: ByteRange) -> AstIndex {
        self.tokens.push(token);
        self.token_ranges.push(range)
    }

    pub fn insert_token(&mut self, index: AstIndex, token: Token, range: ByteRange) {
        self.tokens.insert(index, token);
        self.token_ranges.insert(index, range);
    }

    pub fn next_index(&self) -> AstIndex {
        self.tokens.next_index()
    }
}

impl<'a> Display for AstRef<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "Tokens:")?;
        let mut index = AstIndex(0);
        while index < self.tokens().len() {
            let range = self.char_data().range(&self.token_range(index));
            writeln!(
                f,
                "[{}] {} {:?}",
                range,
                self.token_string(index),
                self.token(index)
            )?;
            index += 1;
        }
        Ok(())
    }
}

impl OperandPosition {
    pub(crate) fn get(self, expression: Expression, ast: &AstRef) -> Expression {
        match self {
            Left | PostfixOperand => expression.left_expression(ast),
            Right | PrefixOperand => expression.right_expression(ast),
        }
    }
}

impl Display for OperandPosition {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let string = match *self {
            Left | PostfixOperand => "left side",
            Right | PrefixOperand => "right side",
        };
        write!(f, "{}", string)
    }
}

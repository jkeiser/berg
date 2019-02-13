use crate::eval::BlockRef;
use crate::syntax::{
    AstIndex, AstRef, ByteRange, ExpressionTreeWalker, ExpressionRef, FieldIndex, Fixity, IdentifierIndex,
    LineColumnRange, LiteralIndex, OperandPosition, RawLiteralIndex,
};
use crate::value::*;
use std::fmt;

///
/// Standard berg error.
/// 
/// Contains a BergError and a stack of error locations.
///
#[derive(Debug, Clone)]
pub struct Error<'a> {
    pub error: BergError<'a>,
    pub expression: ExpressionRef<'a>,
}

///
/// Standard berg error.
/// 
/// This class is generally used to determine the type of an error, or for
/// implementors to create local errors without having to know an expression's
/// location. An Error or EvalError is needed to give it a source location that
/// can actually be reported.
/// 
#[derive(Debug, Clone)]
pub enum BergError<'a> {
    // File open errors

    ///
    /// The source file to be read could not be found.
    /// 
    SourceNotFound,

    ///
    /// There was an I/O error opening the source file. The file may or may not
    /// exist (depending on the type of the IoOpenError).
    /// 
    IoOpenError,

    ///
    /// There was an I/O error reading the source file.
    ///
    IoReadError,

    ///
    /// There was an error determining the current directory.
    ///
    CurrentDirectoryError,

    ///
    /// A source file was more than 32-bits (4GB).
    /// 
    SourceTooLarge(usize),

    // Code errors
    InvalidUtf8(RawLiteralIndex),
    UnsupportedCharacters(LiteralIndex),
    IdentifierStartsWithNumber(LiteralIndex),
    MissingOperand,
    AssignmentTargetMustBeIdentifier,
    RightSideOfDotMustBeIdentifier,
    OpenWithoutClose,
    CloseWithoutOpen,
    UnsupportedOperator(Box<BergResult<'a>>, Fixity, IdentifierIndex),
    DivideByZero,
    NoSuchField(FieldIndex),
    FieldNotSet(FieldIndex),
    CircularDependency,
    // TODO stop boxing BergVals
    BadType(Box<BergResult<'a>>, &'static str),
    BadOperandType(OperandPosition, Box<BergResult<'a>>, &'static str),
    PrivateField(BlockRef<'a>, IdentifierIndex),
    NoSuchPublicField(BlockRef<'a>, IdentifierIndex),
    NoSuchPublicFieldOnValue(Box<BergResult<'a>>, IdentifierIndex),
    NoSuchPublicFieldOnRoot(IdentifierIndex),
    ImmutableFieldOnRoot(FieldIndex),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ErrorCode {
    // Compile errors related to system (source)
    SourceNotFound = 101,
    IoOpenError,
    IoReadError,
    CurrentDirectoryError,
    SourceTooLarge,

    // Compile errors related to format (tokenizer)
    InvalidUtf8 = 201,
    UnsupportedCharacters,
    IdentifierStartsWithNumber,

    // Compile errors related to structure (parser)
    MissingOperand = 301,
    AssignmentTargetMustBeIdentifier,
    RightSideOfDotMustBeIdentifier,
    OpenWithoutClose,
    CloseWithoutOpen,

    // Compile errors related to type (checker)
    UnsupportedOperator = 1001,
    DivideByZero,
    BadType,
    NoSuchField,
    NoSuchPublicField,
    FieldNotSet,
    CircularDependency,
    PrivateField,
    ImmutableField,
}

#[derive(Debug, Clone)]
pub enum ErrorLocation<'a> {
    Generic,
    SourceOnly(AstRef<'a>),
    SourceExpression(AstRef<'a>, AstIndex),
    SourceRange(AstRef<'a>, ByteRange),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ExpressionErrorPosition {
    Expression,
    ImmediateLeftOperand,
    LeftOperand,
    RightOperand,
}

impl<'a> ErrorLocation<'a> {
    pub fn range(&self) -> LineColumnRange {
        match self {
            ErrorLocation::SourceExpression(ast, _) | ErrorLocation::SourceRange(ast, _) => {
                ast.char_data.range(&self.byte_range())
            }
            _ => unreachable!(),
        }
    }
    pub fn byte_range(&self) -> ByteRange {
        match self {
            ErrorLocation::SourceExpression(ast, index) => {
                ExpressionTreeWalker::new((), ast, *index).byte_range()
            }
            ErrorLocation::SourceRange(_, range) => range.clone(),
            _ => unreachable!(),
        }
    }
}

impl<'a> Error<'a> {
    pub fn new(error: BergError<'a>, expression: ExpressionRef<'a>) -> Self {
        Error {
            error,
            expression,
        }
    }

    pub fn err<T, E: From<Self>>(self) -> Result<T, E> {
        Err(E::from(self))
    }

    pub fn code(&self) -> ErrorCode {
        self.error.code()
    }

    pub fn expression(&self) -> ExpressionRef<'a> {
        self.expression.clone()
    }

    pub fn location(&self) -> ErrorLocation<'a> {
        use self::BergError::*;
        use self::ErrorLocation::*;
        let expression = self.expression();
        match self.error {
            // File open errors
            CurrentDirectoryError => ErrorLocation::Generic,
            SourceNotFound | IoOpenError | IoReadError | SourceTooLarge(..) => {
                SourceOnly(expression.ast)
            }

            MissingOperand => {
                let range = expression.ast.token_ranges[expression.expression().parent_expression().root_index()].clone();
                SourceRange(expression.ast, range)
            }

            UnsupportedOperator(..) => {
                let range = expression.ast.token_ranges[expression.expression().root_index()].clone();
                SourceRange(expression.ast, range)
            }
            BadOperandType(position, ..) => {
                let operand = expression.expression().child_expression(position).root_index();
                SourceExpression(expression.ast, operand)
            }


            DivideByZero | RightSideOfDotMustBeIdentifier => {
                let operand = expression.expression().right_expression().root_index();
                SourceExpression(expression.ast, operand)
            }

            OpenWithoutClose => {
                let range =
                    expression.ast.token_ranges[expression.expression().open_operator()].clone();
                SourceRange(expression.ast, range)
            }

            CloseWithoutOpen => {
                let range =
                    expression.ast.token_ranges[expression.expression().close_operator()].clone();
                SourceRange(expression.ast, range)
            }

            // Expression errors
            InvalidUtf8(..)
            | UnsupportedCharacters(..)
            | IdentifierStartsWithNumber(..)
            | AssignmentTargetMustBeIdentifier
            | NoSuchField(..)
            | NoSuchPublicField(..)
            | NoSuchPublicFieldOnValue(..)
            | NoSuchPublicFieldOnRoot(..)
            | FieldNotSet(..)
            | CircularDependency
            | ImmutableFieldOnRoot(..)
            | PrivateField(..)
            | BadType(..) => ErrorLocation::SourceExpression(expression.ast, expression.root),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ErrorCode::*;
        let string = match *self {
            SourceNotFound => "SourceNotFound",
            IoOpenError => "IoOpenError",
            IoReadError => "IoReadError",
            CurrentDirectoryError => "CurrentDirectoryError",
            SourceTooLarge => "SourceTooLarge",
            InvalidUtf8 => "InvalidUtf8",
            UnsupportedCharacters => "UnsupportedCharacters",
            IdentifierStartsWithNumber => "IdentifierStartsWithNumber",
            MissingOperand => "MissingOperand",
            AssignmentTargetMustBeIdentifier => "AssignmentTargetMustBeIdentifier",
            RightSideOfDotMustBeIdentifier => "RightSideOfDotMustBeIdentifier",
            OpenWithoutClose => "OpenWithoutClose",
            CloseWithoutOpen => "CloseWithoutOpen",
            UnsupportedOperator => "UnsupportedOperator",
            DivideByZero => "DivideByZero",
            BadType => "BadType",
            NoSuchField => "NoSuchField",
            NoSuchPublicField => "NoSuchPublicField",
            FieldNotSet => "FieldNotSet",
            CircularDependency => "CircularDependency",
            PrivateField => "PrivateField",
            ImmutableField => "ImmutableField",
        };
        write!(f, "{}", string)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BergError::*;
        let expression = self.expression();
        match self.error {
            SourceNotFound => write!(
                f,
                "I/O error getting current directory path {:?} ({}): {}",
                expression.ast.source.absolute_path().unwrap(),
                expression.ast.source.name(),
                expression.ast.open_io_error()
            ),
            IoOpenError => write!(
                f,
                "I/O error opening {:?} ({}): {}",
                expression.ast.source.absolute_path().unwrap(),
                expression.ast.source.name(),
                expression.ast.open_io_error()
            ),
            IoReadError => write!(
                f,
                "I/O error reading {:?} ({}): {}",
                expression.ast.source.absolute_path().unwrap(),
                expression.ast.source.name(),
                expression.ast.open_io_error()
            ),
            CurrentDirectoryError => write!(
                f,
                "I/O error getting current directory to determine path of {:?}: {}",
                expression.ast.source.name(),
                expression.ast.root().root_path().as_ref().unwrap_err()
            ),
            SourceTooLarge(size) => write!(
                f,
                "SourceRef file {} too large ({} bytes): source files greater than 4GB are unsupported.",
                expression.ast.source.name(),
                size
            ),
            InvalidUtf8(raw_literal) => {
                write!(f, "Invalid UTF-8 bytes! Perhaps this isn't a Berg UTF-8 source file? Invalid bytes: '")?;
                let bytes = expression.ast.raw_literal_string(raw_literal);
                // Only print up to the first 12 bytes to prevent the error message from being ridiculous
                let print_max = 12.min(bytes.len());
                for byte in &bytes[0..print_max] {
                    write!(f, "{:2X}", byte)?;
                }
                if print_max > 12 {
                    write!(f, "...")?;
                }
                write!(f, "'")
            }
            UnsupportedCharacters(literal) => write!(f, "Unsupported Unicode characters! Perhaps this isn't a Berg source file? Unsupported characters: '{}'", expression.ast.literal_string(literal)),
            OpenWithoutClose => write!(
                f,
                "Open '{}' found without a matching close '{}'.",
                expression.expression().open_token().to_string(&expression.ast),
                expression.expression().boundary().close_string()
            ),
            CloseWithoutOpen => write!(
                f,
                "Close '{}' found without a matching open '{}'.",
                expression.expression().close_token().to_string(&expression.ast),
                expression.expression().boundary().open_string()
            ),
            UnsupportedOperator(ref value, fixity, identifier) => write!(
                f,
                "Unsupported {} operator {} on value {}",
                fixity,
                expression.ast.identifier_string(identifier),
                value.display()
            ),
            DivideByZero => write!(
                f,
                "Division by zero is illegal. Perhaps you meant a different number on the right hand side of the '{}'?",
                expression.expression().token().to_string(&expression.ast)
            ),
            NoSuchField(field_index) => write!(
                f,
                "No such field: '{}'",
                expression.ast.field_name(field_index)
            ),
            FieldNotSet(field_index) => write!(
                f,
                "Field '{}' was declared, but never set to a value!",
                expression.ast.field_name(field_index)
            ),
            NoSuchPublicField(ref block, name) => write!(
                f,
                "No field '{}' exists on '{}'! Perhaps it's a misspelling?",
                expression.ast.identifier_string(name),
                block
            ),
            NoSuchPublicFieldOnValue(ref value, name) => write!(
                f,
                "No field '{}' exists on '{}'! Perhaps it's a misspelling?",
                expression.ast.identifier_string(name),
                value.display()
            ),
            NoSuchPublicFieldOnRoot(name) => write!(
                f,
                "No field '{}' exists on the root! Also, how did you manage to do '.' on the root?",
                expression.ast.identifier_string(name)
            ),
            PrivateField(ref value, name) => write!(
                f,
                "Field '{}' on '{}' is private and cannot be accessed with '.'! Perhaps you meant to declare the field with ':{}' instead of '{}'?",
                expression.ast.identifier_string(name),
                value,
                expression.ast.identifier_string(name),
                expression.ast.identifier_string(name)
            ),
            ImmutableFieldOnRoot(field_index) => write!(
                f,
                "'{}' cannot be modified!",
                expression.ast.field_name(field_index)
            ),
            IdentifierStartsWithNumber(literal) => write!(
                f,
                "Field names must start with letters or '_', but '{}' starts with a number! You may have mistyped the field name, or missed an operator?",
                expression.ast.literal_string(literal)
            ),
            CircularDependency => write!(
                f,
                "Circular dependency at '{}'!",
                expression
            ),
            MissingOperand => write!(
                f,
                "Operator {} has no value on {} to operate on!",
                expression.expression().parent_expression().token().to_string(&expression.ast),
                expression.expression().operand_position()
            ),
            AssignmentTargetMustBeIdentifier => write!(
                f,
                "The assignment operator '{operator}' must have a field declaration or name on {position} (like \":foo {operator} ...\" or \"foo {operator} ...\": {position} is currently {operand}.",
                operator = expression.expression().parent_expression().token().to_string(&expression.ast),
                position = expression.expression().operand_position(),
                operand = expression,
            ),
            RightSideOfDotMustBeIdentifier => write!(
                f,
                "The field access operator '{operator}' must have an identifier on the right side (like \"{left}.FieldName\"): currently it is '{right}'.",
                operator = expression.expression().token().to_string(&expression.ast),
                left = expression.expression().left_expression(),
                right = expression.expression().right_expression(),
            ),
            BadOperandType(position,ref actual_value,expected_type) => write!(
                f,
                "The value of '{operand}' is {actual_value}, but {position} '{operator}' must be an {expected_type}!",
                operand = expression.expression().child_expression(position),
                actual_value = actual_value.display(),
                position = position,
                operator = expression.expression().token_string(),
                expected_type = expected_type
            ),
            BadType(ref actual_value,expected_type) => write!(
                f,
                "The value of '{}' is {}, but we expected {}!",
                expression,
                actual_value.display(),
                expected_type
            ),
        }
    }
}

impl<'a> BergError<'a> {
    pub fn at_location(self, expression: impl Into<ExpressionRef<'a>>) -> Error<'a> {
        Error::new(self, expression.into())
    }

    pub fn err<T>(self) -> BergResult<'a, T> {
        Err(ControlVal::ExpressionError(self, ExpressionErrorPosition::Expression))
    }

    pub fn operand_err<T>(self, position: ExpressionErrorPosition) -> BergResult<'a, T> {
        Err(ControlVal::ExpressionError(self, position))
    }

    pub fn code(&self) -> ErrorCode {
        use self::BergError::*;
        match *self {
            // File open errors
            SourceNotFound => ErrorCode::SourceNotFound,
            IoOpenError => ErrorCode::IoOpenError,
            IoReadError => ErrorCode::IoReadError,
            CurrentDirectoryError => ErrorCode::CurrentDirectoryError,
            SourceTooLarge(..) => ErrorCode::SourceTooLarge,

            // Expression errors
            InvalidUtf8(..) => ErrorCode::InvalidUtf8,
            UnsupportedCharacters(..) => ErrorCode::UnsupportedCharacters,
            IdentifierStartsWithNumber(..) => ErrorCode::IdentifierStartsWithNumber,
            MissingOperand => ErrorCode::MissingOperand,
            AssignmentTargetMustBeIdentifier => ErrorCode::AssignmentTargetMustBeIdentifier,
            RightSideOfDotMustBeIdentifier => ErrorCode::RightSideOfDotMustBeIdentifier,
            OpenWithoutClose => ErrorCode::OpenWithoutClose,
            CloseWithoutOpen => ErrorCode::CloseWithoutOpen,

            // Compile errors related to type (checker)
            UnsupportedOperator(..) => ErrorCode::UnsupportedOperator,
            DivideByZero => ErrorCode::DivideByZero,
            NoSuchField(..) => ErrorCode::NoSuchField,
            NoSuchPublicField(..) | NoSuchPublicFieldOnValue(..) | NoSuchPublicFieldOnRoot(..) => {
                ErrorCode::NoSuchPublicField
            }
            PrivateField(..) => ErrorCode::PrivateField,
            FieldNotSet(..) => ErrorCode::FieldNotSet,
            CircularDependency => ErrorCode::CircularDependency,
            ImmutableFieldOnRoot(..) => ErrorCode::ImmutableField,
            BadOperandType(..) | BadType(..) => ErrorCode::BadType,
        }
    }
}

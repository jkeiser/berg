use crate::eval::{ExpressionEvaluator, ScopeRef};
use crate::syntax::{
    Ast, ExpressionTreeWalker, AstIndex, AstRef, BlockIndex, ExpressionRef, FieldError, FieldIndex, IdentifierIndex,
};
use crate::value::implement::*;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::mem;
use std::rc::Rc;

///
/// A block represents the execution of an expression, including the next
/// expression to execute, as well as the scope (field values and parent block)
/// and input (a BergResult).
///
#[derive(Clone)]
pub struct BlockRef<'a>(Rc<RefCell<BlockData<'a>>>);

#[derive(Debug)]
struct BlockData<'a> {
    expression: AstIndex,
    state: BlockState<'a>,
    index: BlockIndex,
    fields: Vec<BlockFieldValue<'a>>,
    parent: ScopeRef<'a>,
    input: BergResult<'a>,
}

#[derive(Debug)]
enum BlockState<'a> {
    Ready,
    Running,
    NextVal,
    Complete(BergResult<'a>),
}

#[derive(Debug, Clone)]
enum BlockFieldValue<'a> {
    NotDeclared,
    NotSet,
    Value(BergVal<'a>),
}

impl<'a> BlockRef<'a> {
    ///
    /// Create a new block that will run the given expression against the
    /// given scope and input.
    ///
    pub fn new(
        expression: AstIndex,
        index: BlockIndex,
        parent: ScopeRef<'a>,
        input: BergResult<'a>,
    ) -> Self {
        BlockRef(Rc::new(RefCell::new(BlockData {
            expression,
            index,
            state: BlockState::Ready,
            fields: Default::default(),
            parent,
            input,
        })))
    }

    pub fn create_child_block(&self, expression: AstIndex, index: BlockIndex) -> Self {
        Self::new(expression, index, ScopeRef::BlockRef(self.clone()), BergVal::empty_tuple().ok())
    }

    pub fn apply(&self, input: RightOperand<'a, impl BergValue<'a>>) -> BergResult<'a> {
        let block = self.0.borrow();
        let input = input.into_val();
        // Evaluate immediately and take the result.
        let new_block = Self::new(
            block.expression,
            block.index,
            block.parent.clone(),
            input,
        );
        new_block.take_result(BlockState::Complete(BergVal::empty_tuple().ok())).evaluate()
    }

    fn take_result(&self, replace_with: BlockState<'a>) -> BergResult<'a> {
        self.ensure_evaluated()?;
        let mut block = self.0.borrow_mut();
        match block.state {
            BlockState::Running | BlockState::NextVal => {
                block.delocalize_error(BergError::CircularDependency)
            }
            BlockState::Complete(_) => {
                if let BlockState::Complete(result) = mem::replace(&mut block.state, replace_with) {
                    result
                } else {
                    unreachable!()
                }
            }
            BlockState::Ready => unreachable!(),
        }
    }

    fn replace_result(&self, result: BergResult<'a>) {
        let mut block = self.0.borrow_mut();
        match block.state {
            BlockState::NextVal => {
                block.state = BlockState::Complete(result);
            }
            _ => unreachable!(
                "Block didn't stay in NextVal state by itself: {:?}",
                block.state
            ),
        }
    }

    fn clone_result(&self) -> BergResult<'a> {
        self.ensure_evaluated()?;
        let block = self.0.borrow();
        match &block.state {
            BlockState::Running | BlockState::NextVal => block.delocalize_error(BergError::CircularDependency),
            BlockState::Complete(ref result) => result.clone(),
            BlockState::Ready => unreachable!(),
        }
    }

    fn ensure_evaluated(&self) -> Result<(), ErrorVal<'a>> {
        // Check if the block has already been run (and don't re-run)
        let (ast, expression, index) = {
            let mut block = self.0.borrow_mut();
            match block.state {
                BlockState::Running | BlockState::NextVal => {
                    return block.delocalize_error(BergError::CircularDependency);
                }
                BlockState::Complete(_) => return Ok(()),
                BlockState::Ready => {}
            }
            block.state = BlockState::Running;
            let ast = block.ast();
            let index = block.index;
            block.fields.resize(ast.blocks[index].scope_count.into(), BlockFieldValue::NotDeclared);
            (ast, block.expression, index)
        };
        // Run the block and stash the result
        let scope = ScopeRef::BlockRef(self.clone());
        let expression = ExpressionEvaluator::new(&scope, &ast, expression);
        let indent = "  ".repeat(expression.depth());
        println!("{}|----------------------------------------", indent);
        println!("{}| Block evaluating {:?}", indent, expression);
        let result = expression.evaluate_inner(ast.blocks[index].boundary).into_val();
        println!("{}|       evaluated to {}", indent, result.display());
        self.0.borrow_mut().state = BlockState::Complete(result);
        println!("{}| Block state after evaluation: {}", indent, self);
        println!("{}| Block evaluated {:?} to {}", indent, expression, self.clone_result().display());
        println!("{}", indent);
        Ok(())
    }

    pub fn local_field(&self, index: FieldIndex, ast: &Ast) -> EvalResult<'a> {
        use BlockFieldValue::*;
        let block = self.0.borrow();
        let scope_start = ast.blocks[block.index].scope_start;
        if index >= scope_start {
            let scope_index: usize = (index - scope_start).into();
            match block.fields.get(scope_index) {
                Some(Value(value)) => value.clone().eval_val(),
                Some(NotSet) => BergError::FieldNotSet(index).err(),
                Some(NotDeclared) => BergError::NoSuchField(index).err(),
                None => BergError::NoSuchField(index).err(),
            }
        } else {
            block.parent.local_field(index, ast)
        }
    }

    pub fn declare_field(&self, field_index: FieldIndex, ast: &Ast) -> Result<(), ErrorVal<'a>> {
        // Make sure we have enough spots to put the field
        let (input, block_field_index) = {
            let mut block = self.0.borrow_mut();
            let scope_start = ast.blocks[block.index].scope_start;
            // The index we intend to insert this at
            let block_field_index: usize = (field_index - scope_start).into();
            if let BlockFieldValue::NotDeclared = block.fields[block_field_index] {
                // The only known way to declare a field in an object (right now) is to set it while
                // running.
                assert!(match block.state {
                    BlockState::Running => true,
                    _ => false,
                });
                // Steal the input value so we can next_val() it without fear.
                if let Err(ref error) = block.input {
                    return Err(error.clone());
                }
                let input = mem::replace(&mut block.input, BergError::CircularDependency.err());
                (input.unwrap(), block_field_index)
            } else {
                return Ok(());
            }
        };

        // Move the value forward here, outside the lock, so we don't panic
        // (we will instead see CircularReference errors).
        let next_val = input.next_val()?;

        // Put input back, and set the field to the value we got!
        let mut block = self.0.borrow_mut();
        block.fields[block_field_index] = match next_val {
            None => BlockFieldValue::NotSet,
            Some(NextVal { head, tail }) => {
                block.input = tail;
                BlockFieldValue::Value(head)
            }
        };
        Ok(())
    }

    pub fn set_local_field(
        &self,
        field_index: FieldIndex,
        value: BergVal<'a>,
        ast: &Ast,
    ) -> Result<(), ErrorVal<'a>> {
        let scope_start = ast.blocks[self.0.borrow().index].scope_start;
        if field_index < scope_start {
            return self
                .0
                .borrow()
                .parent
                .set_local_field(field_index, value, ast);
        }
        println!(
            "Set {} to {:?}: {}",
            ast.identifier_string(ast.fields[field_index].name),
            value,
            self
        );
        {
            let mut block = self.0.borrow_mut();
            let index: usize = (field_index - scope_start).into();
            block.fields[index] = BlockFieldValue::Value(value);
        }
        println!("Now we are {}", self);
        Ok(())
    }

    pub fn ast(&self) -> AstRef<'a> {
        self.0.borrow().ast()
    }

    pub fn field_error<T>(&self, error: FieldError, name: IdentifierIndex) -> Result<T, ErrorVal<'a>> {
        use FieldError::*;
        match error {
            NoSuchPublicField => BergError::NoSuchPublicField(self.clone(), name).err(),
            PrivateField => BergError::PrivateField(self.clone(), name).err(),
        }
    }
}

impl<'a> From<&BlockData<'a>> for ExpressionRef<'a> {
    fn from(from: &BlockData<'a>) -> Self {
        ExpressionRef::new(from.ast(), from.expression)
    }
}
impl<'p, 'a: 'p> From<Ref<'p, BlockData<'a>>> for ExpressionRef<'a> {
    fn from(from: Ref<'p, BlockData<'a>>) -> Self {
        ExpressionRef::new(from.ast(), from.expression)
    }
}
impl<'p, 'a: 'p> From<RefMut<'p, BlockData<'a>>> for ExpressionRef<'a> {
    fn from(from: RefMut<'p, BlockData<'a>>) -> Self {
        ExpressionRef::new(from.ast(), from.expression)
    }
}

impl<'a> From<BlockRef<'a>> for BergVal<'a> {
    fn from(from: BlockRef<'a>) -> Self {
        BergVal::BlockRef(from)
    }
}

impl<'a> From<BlockRef<'a>> for EvalVal<'a> {
    fn from(from: BlockRef<'a>) -> Self {
        BergVal::from(from).into()
    }
}

impl<'a> BergValue<'a> for BlockRef<'a> {
    fn next_val(self) -> Result<Option<NextVal<'a>>, ErrorVal<'a>> {
        // Get the current result, and prevent anyone else from retrieving this value while we change it
        // by marking state as NextVal
        let next_val = self.take_result(BlockState::NextVal).next_val()?;
        match next_val {
            None => Ok(None),
            Some(NextVal { head, tail }) => {
                self.replace_result(tail);
                Ok(Some(NextVal { head, tail: self.into_val() }))
            }
        }
    }

    fn into_val(self) -> BergResult<'a> {
        self.ok()
    }
    fn eval_val(self) -> EvalResult<'a> {
        self.ok()
    }
    fn evaluate(self) -> BergResult<'a> {
        self.clone_result()
    }
    fn at_position(self, _new_position: ExpressionErrorPosition) -> BergResult<'a> {
        self.ok()
    }

    fn into_native<T: TryFromBergVal<'a>>(self) -> Result<T, ErrorVal<'a>> {
        self.clone_result().into_native()
    }
    fn try_into_native<T: TryFromBergVal<'a>>(self) -> Result<Option<T>, ErrorVal<'a>> {
        self.clone_result().try_into_native()
    }

    fn infix(self, operator: IdentifierIndex, right: RightOperand<'a, impl BergValue<'a>>) -> EvalResult<'a> {
        use crate::syntax::identifiers::*;

        match operator {
            DOT => default_infix(self, operator, right),
            APPLY => self.apply(right)?.ok(),
            _ => self.clone_result().infix(operator, right),
        }
    }

    fn infix_assign(self, operator: IdentifierIndex, right: RightOperand<'a, impl BergValue<'a>>) -> EvalResult<'a> {
        self.clone_result().infix_assign(operator, right)
    }

    fn prefix(self, operator: IdentifierIndex) -> EvalResult<'a> {
        // Closures report their own internal error instead of local ones.
        self.clone_result().prefix(operator)
    }

    fn postfix(self, operator: IdentifierIndex) -> EvalResult<'a> {
        self.clone_result().postfix(operator)
    }

    fn subexpression_result(self, boundary: ExpressionBoundary) -> EvalResult<'a> {
        default_subexpression_result(self, boundary)
    }

    fn field(self, name: IdentifierIndex) -> EvalResult<'a> {
        println!(
            "====> get {} on {}",
            self.ast().identifier_string(name),
            self
        );
        // Always try to get the field from the inner result first
        let current = self.clone_result();
        println!("got from {:?}", self);
        let current = current?;
        println!("current = {}", current);
        match current.field(name) {
            // If we couldn't find the field on the inner value, see if our block has the field
            Err(ErrorVal::ExpressionError(ref error, ExpressionErrorPosition::Expression)) if error.code() == ErrorCode::NoSuchPublicField => {
                println!("====> no such public {} on current", self.ast().identifier_string(name));
                let ast = self.ast();
                let index = {
                    let block = self.0.borrow();
                    let ast_block = &ast.blocks[block.index];
                    let index = ast_block.public_field_index(block.index, name, &ast);
                    index.or_else(|error| self.field_error(error, name))?
                };
                self.local_field(index, &ast)
            }
            Ok(value) => { println!("====> got {:?} for {} on {}", value, self.ast().identifier_string(name), self); Ok(value) }
            Err(error) => { println!("====> error {} for {} on {}", error, self.ast().identifier_string(name), self); Err(error) }
            // result => result,
        }
    }

    fn set_field(&mut self, name: IdentifierIndex, value: BergVal<'a>) -> Result<(), ErrorVal<'a>> {
        self.ensure_evaluated()?;

        // Figure out the field index from its name
        let ast = self.ast();
        let index = {
            let block = self.0.borrow();
            let ast_block = &ast.blocks[block.index];
            ast_block
                .public_field_index(block.index, name, &ast)
                .or_else(|error| self.field_error(error, name))?
        };

        // Set the field.
        self.set_local_field(index, value, &ast)
    }
}

impl<'a> BlockData<'a> {
    pub fn ast(&self) -> AstRef<'a> {
        self.parent.ast()
    }
    pub fn delocalize_error<T>(&self, error: BergError<'a>) -> Result<T, ErrorVal<'a>> {
        Err(ErrorVal::Error(error.at_location(ExpressionRef::new(self.ast(), self.expression))))
    }
}

impl<'a> fmt::Display for BlockRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BlockFieldValue::*;
        let block = self.0.borrow();
        let ast = self.ast();
        write!(f, "block({}", block.state)?;
        match &block.input {
            Ok(BergVal::Tuple(tuple)) if tuple.is_empty() => {},
            input => write!(f, ", input: {}", input.display())?,
        }
        if !block.fields.is_empty() {
            write!(f, ", fields: {{")?;
            let mut is_first_field = true;
            let scope_start = ast.blocks[block.index].scope_start;
            for (index, field_value) in block.fields.iter().enumerate() {
                if is_first_field {
                    is_first_field = false;
                } else {
                    write!(f, ", ")?;
                }

                let field = &ast.fields[scope_start + index];
                let name = ast.identifier_string(field.name);

                match field_value {
                    Value(value) => write!(f, "{}: {}", name, value)?,
                    NotDeclared => write!(f, "{}: <undeclared>", name)?,
                    NotSet => write!(f, "{}: <not set>", name)?,
                }
            }
            write!(f, "}}")?;
        }
        write!(f, ", expression: {}", ExpressionTreeWalker::basic(&ast, block.expression))?;
        write!(f, ")")
    }
}

impl<'a> fmt::Display for BlockState<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockState::Complete(result) => write!(f, "Complete({})", result.display()),
            BlockState::Ready => write!(f, "Ready",),
            BlockState::Running => write!(f, "Running"),
            BlockState::NextVal => write!(f, "NextVal"),
        }
    }
}

impl<'a> fmt::Debug for BlockRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BlockFieldValue::*;
        let block = self.0.borrow();
        let ast = self.ast();
        write!(f, "BlockRef {{ state: {:?}", block.state)?;
        match &block.input {
            Ok(BergVal::Tuple(tuple)) if tuple.is_empty() => {},
            input => write!(f, ", input: {:?}", input)?,
        }
        if !block.fields.is_empty() {
            write!(f, ", fields: {{")?;
            let mut is_first_field = true;
            let scope_start = ast.blocks[block.index].scope_start;
            for (index, field_value) in block.fields.iter().enumerate() {
                if is_first_field {
                    is_first_field = false;
                } else {
                    write!(f, ", ")?;
                }

                let field = &ast.fields[scope_start + index];
                let name = ast.identifier_string(field.name);

                match field_value {
                    Value(value) => write!(f, "{}: {:?}", name, value)?,
                    NotDeclared => write!(f, "{}: <undeclared>", name)?,
                    NotSet => write!(f, "{}: <not set>", name)?,
                }
            }
            write!(f, "}}")?;
        }
        write!(
            f,
            ", expression: {:?}, parent: {:?}",
            ExpressionTreeWalker::basic(&ast, block.expression), block.parent
        )?;
        write!(f, "}}")
    }
}

impl<'a> PartialEq for BlockRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

use syntax::IdentifierIndex;
use util::try_from::TryFrom;
use value::*;

impl TypeName for IdentifierIndex {
    const TYPE_NAME: &'static str = "identifier";
}

impl<'a> BergValue<'a> for IdentifierIndex {
    fn infix(self, operator: IdentifierIndex, scope: &mut ScopeRef<'a>, right: Operand, ast: &AstRef<'a>) -> EvalResult<'a> {
        use syntax::identifiers::EQUAL_TO;
        match operator {
            EQUAL_TO => match right.evaluate(scope, ast)?.downcast::<IdentifierIndex>() {
                Ok(identifier) if identifier == self => true.ok(),
                _ => false.ok(),
            },
            _ => default_infix(self, operator, scope, right, ast),
        }
    }
}

impl<'a> From<IdentifierIndex> for BergVal<'a> {
    fn from(value: IdentifierIndex) -> Self {
        BergVal::Identifier(value)
    }
}

impl<'a> TryFrom<BergVal<'a>> for IdentifierIndex {
    type Error = BergVal<'a>;
    fn try_from(from: BergVal<'a>) -> Result<Self, Self::Error> {
        match from {
            BergVal::Identifier(value) => Ok(value),
            _ => Err(from),
        }
    }
}

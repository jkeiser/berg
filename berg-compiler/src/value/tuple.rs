use crate::syntax::IdentifierIndex;
use crate::value::implement::*;
use std::iter::FromIterator;
use std::fmt;

///
/// A discrete series of values.
///
/// Note: Tuples are generally stored in *reverse* order, since the typical
/// operation for a tuple is to take the first value and return the next.
///
#[derive(Debug, Clone)]
pub struct Tuple<'a>(Vec<BergVal<'a>>);

impl<'a> Tuple<'a> {
    pub fn from_values(iter: impl DoubleEndedIterator<Item = BergVal<'a>>) -> Self {
        Self::from_reversed(iter.rev())
    }
    pub fn from_reversed(iter: impl Iterator<Item = BergVal<'a>>) -> Self {
        Tuple(iter.collect())
    }
    pub fn from_reversed_vec(vec: Vec<BergVal<'a>>) -> Self {
        Tuple(vec)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for Tuple<'a> {
    type Item = BergVal<'a>;
    type IntoIter = std::iter::Rev<<Vec<BergVal<'a>> as IntoIterator>::IntoIter>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().rev()
    }
}

impl<'a> FromIterator<BergVal<'a>> for Tuple<'a> {
    // Sadly, I don't think there is a way to specialize this for ExactSizeIterators.
    // So we have to build it the old fashioned way.
    fn from_iter<I: IntoIterator<Item = BergVal<'a>>>(iter: I) -> Self {
        Tuple::from(iter.into_iter().collect::<Vec<BergVal<'a>>>())
    }
}

impl<'a> From<Vec<BergVal<'a>>> for Tuple<'a> {
    fn from(mut from: Vec<BergVal<'a>>) -> Self {
        from.reverse();
        Tuple(from)
    }
}

impl<'a, 'p> IntoIterator for &'p Tuple<'a> {
    type Item = &'p BergVal<'a>;
    type IntoIter = std::iter::Rev<<&'p Vec<BergVal<'a>> as IntoIterator>::IntoIter>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).iter().rev()
    }
}

impl<'a> BergValue<'a> for Tuple<'a> {
    fn infix(self, operator: IdentifierIndex, right: RightOperand<'a, impl BergValue<'a>>) -> EvalResult<'a> {
        default_infix(self, operator, right)
    }
    fn infix_assign(self, operator: IdentifierIndex, right: RightOperand<'a, impl BergValue<'a>>) -> EvalResult<'a> {
        default_infix_assign(self, operator, right)
    }
    fn postfix(self, operator: IdentifierIndex) -> EvalResult<'a> {
        default_postfix(self, operator)
    }
    fn prefix(self, operator: IdentifierIndex) -> EvalResult<'a> {
        default_prefix(self, operator)
    }
    fn subexpression_result(self, boundary: ExpressionBoundary) -> EvalResult<'a> {
        default_subexpression_result(self, boundary)
    }

    fn field(self, name: IdentifierIndex) -> BergResult<'a> {
        default_field(self, name)
    }
    fn set_field(&mut self, name: IdentifierIndex, value: BergVal<'a>) -> Result<(), ErrorVal<'a>> {
        default_set_field(self, name, value)
    }

    fn next_val(mut self) -> Result<Option<NextVal<'a>>, ErrorVal<'a>> {
        match self.0.pop() {
            Some(value) => Ok(Some(NextVal { head: value, tail: Ok(self.0.into()) })),
            None => Ok(None),
        }
    }
    fn into_val(self) -> BergResult<'a> {
        self.ok()
    }
    fn eval_val(self) -> EvalResult<'a> {
        self.ok()
    }
    fn at_position(self, _new_position: ExpressionErrorPosition) -> BergResult<'a> {
        self.ok()
    }
    fn into_native<T: TryFromBergVal<'a>>(mut self) -> Result<T, ErrorVal<'a>> {
        if self.0.len() == 1 {
            Ok(self.0.pop().unwrap().into_native()?)
        } else {
            BergError::BadOperandType(Box::new(Ok(BergVal::Tuple(self))), T::TYPE_NAME).err()
        }
    }
    fn try_into_native<T: TryFromBergVal<'a>>(mut self) -> Result<Option<T>, ErrorVal<'a>> {
        if self.0.len() == 1 {
            Ok(Some(self.0.pop().unwrap().into_native()?))
        } else {
            Ok(None)
        }
    }
}

impl<'a> fmt::Display for Tuple<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut is_first = true;
        for elem in self {
            if is_first {
                is_first = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, "{}", elem)?;
        }
        write!(f, ")")
    }
}

impl<'a> From<Tuple<'a>> for BergVal<'a> {
    fn from(from: Tuple<'a>) -> Self {
        BergVal::Tuple(from)
    }
}

impl<'a> From<Tuple<'a>> for EvalVal<'a> {
    fn from(from: Tuple<'a>) -> Self {
        BergVal::from(from).into()
    }
}

impl<'a> From<Tuple<'a>> for Vec<BergVal<'a>> {
    fn from(from: Tuple<'a>) -> Self {
        from.0
    }
}

impl<'a> TryFromBergVal<'a> for Tuple<'a> {
    const TYPE_NAME: &'static str = "Tuple";
    fn try_from_berg_val(from: EvalVal<'a>) -> Result<Result<Self, BergVal<'a>>, ErrorVal<'a>> {
        match from.into_val()? {
            BergVal::Tuple(value) => Ok(Ok(value)),
            value => Ok(Err(value)),
        }
    }
}

impl<'a> FromIterator<BergVal<'a>> for BergVal<'a> {
    // Sadly, it doesn't seem we can specialize this for the happy case where iter is an ExactSizeIterator.
    fn from_iter<I: IntoIterator<Item = BergVal<'a>>>(iter: I) -> Self {
        BergVal::Tuple(Tuple::from_iter(iter))
    }
}
impl<'a> From<Vec<BergVal<'a>>> for BergVal<'a> {
    fn from(from: Vec<BergVal<'a>>) -> Self {
        BergVal::Tuple(Tuple::from(from))
    }
}
impl<'a> From<Box<[BergVal<'a>]>> for BergVal<'a> {
    fn from(from: Box<[BergVal<'a>]>) -> Self {
        BergVal::from(from.into_vec())
    }
}

macro_rules! from_sized_array {
    ($($size:tt),*) => {
        $(
            impl<'a> From<[BergVal<'a>; $size]> for BergVal<'a> {
                fn from(from: [BergVal<'a>; $size]) -> Self {
                    // Put it in a box so we can convert to Vec.
                    let from: Box<[BergVal<'a>]> = Box::new(from);
                    BergVal::from(from.into_vec())
                }
            }
        )*
    }
}

from_sized_array! { 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31 }

pub use berg_compiler::test::expect;
pub use berg_compiler::ErrorCode::*;
pub use berg_compiler::*;
pub use num::BigRational;
pub use std::iter::FromIterator;
pub use std::str::FromStr;

mod block;
mod parser;
mod primitives;

#[macro_export]
macro_rules! tuple {
    ( $( $x:tt ),* ) => { BergVal::from_iter(vec![ $( val!($x) ),* ].drain(..)) };
}

#[macro_export]
macro_rules! val {
    ( [ $( $x:tt ),* ] ) => { tuple!( $( $x ),* ) };
    ( ( $( $x:tt ),+ ) ) => { tuple!( $( $x ),+ ) };
    ( ( $x:expr ) ) => { val!($x) };
    ( $x:expr ) => { BergVal::from($x) };
}
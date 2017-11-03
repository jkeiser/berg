use indexed_vec::Delta;
use compiler::source_data::{ByteSlice,ByteIndex};
use parser::scanner::ByteType::*;
use parser::scanner::CharType::*;

#[derive(Debug,Copy,Clone)]
pub enum Symbol {
    Integer,
    Operator,
    Open,
    Close,
    UnsupportedCharacters,
    InvalidUtf8Bytes,
}

pub fn next(buffer: &ByteSlice, mut index: ByteIndex) -> Option<(Symbol, ByteIndex)> {
    if index >= buffer.len() {
        return None;
    }

    let byte_type = ByteType::from(buffer[index]);
    index += 1;
    let symbol = match byte_type {
        Char(Digit) => { read_bytes_while(buffer, &mut index, byte_type); Symbol::Integer },
        Char(Operator) => { read_bytes_while(buffer, &mut index, byte_type); Symbol::Operator },
        Char(Open) => Symbol::Open,
        Char(Close) => Symbol::Close,
        InvalidUtf8|Char(Unsupported)|Utf8LeadingByte(_) => { unsupported_or_invalid_utf8(buffer, &mut index, byte_type) },
    };
    Some((symbol, index))
}

pub fn next_has_left_operand(
    buffer: &ByteSlice,
    index: ByteIndex
) -> bool {
    if index < buffer.len() {
        match peek_char(buffer, index) {
            Some((Digit, _))|Some((Open, _)) => false,
            _ => true,
        }
    } else {
        true
    }
}

fn unsupported_or_invalid_utf8(
    buffer: &ByteSlice,
    index: &mut ByteIndex,
    byte_type: ByteType,
) -> Symbol {
    match byte_type {
        Char(Unsupported) => { read_many_unsupported(buffer, index); Symbol::UnsupportedCharacters },
        InvalidUtf8 => { read_many_invalid_utf8(buffer, index); Symbol::InvalidUtf8Bytes },
        Utf8LeadingByte(char_length) => {
            if is_valid_utf8_char(buffer, *index, char_length) {
                *index += char_length;
                *index -= 1;
                read_many_unsupported(buffer, index);
                Symbol::UnsupportedCharacters
            } else {
                read_many_invalid_utf8(buffer, index);
                Symbol::InvalidUtf8Bytes
            }
        },
        Char(_) => unreachable!(),
    }
}

fn read_bytes_while(buffer: &ByteSlice, index: &mut ByteIndex, byte_type: ByteType) {
    while *index < buffer.len() && ByteType::from(buffer[*index]) == byte_type {
        *index += 1;
    }
}

fn read_many_unsupported(buffer: &ByteSlice, index: &mut ByteIndex) {
    while *index < buffer.len() {
        if let Some((Unsupported, char_length)) = peek_char(buffer, *index) {
            *index += char_length;
        } else {
            break;
        }
    }
}

fn read_many_invalid_utf8(buffer: &ByteSlice, index: &mut ByteIndex) {
    while *index < buffer.len() && peek_char(buffer, *index).is_none() {
        *index += 1;
    }
}

// #[inline(always)]
fn peek_char(buffer: &ByteSlice, index: ByteIndex) -> Option<(CharType, Delta<ByteIndex>)> {
    match ByteType::from(buffer[index]) {
        ByteType::Char(char_type) => Some((char_type, Delta(ByteIndex(1)))),
        ByteType::InvalidUtf8 => None,
        ByteType::Utf8LeadingByte(n) => peek_char_utf8_leading(buffer, index, n),
    }
}

fn peek_char_utf8_leading(buffer: &ByteSlice, index: ByteIndex, char_length: Delta<ByteIndex>) -> Option<(CharType, Delta<ByteIndex>)> {
    if is_valid_utf8_char(buffer, index, char_length) {
        Some((CharType::Unsupported, char_length))
    } else {
        None
    }
}

fn is_valid_utf8_char(buffer: &ByteSlice, index: ByteIndex, char_length: Delta<ByteIndex>) -> bool {
    if index + char_length > buffer.len() {
        return false;
    }
    match char_length {
        Delta(ByteIndex(2)) => ByteType::is_utf8_cont(buffer[index+1]),
        Delta(ByteIndex(3)) => ByteType::is_utf8_cont(buffer[index+1]) && ByteType::is_utf8_cont(buffer[index+2]),
        Delta(ByteIndex(4)) => ByteType::is_utf8_cont(buffer[index+1]) && ByteType::is_utf8_cont(buffer[index+2]) && ByteType::is_utf8_cont(buffer[index+3]),
        _ => unreachable!()
    }
}

#[derive(Debug,Copy,Clone,PartialEq)]
enum CharType {
    Digit,
    Operator,
    Open,
    Close,
    Unsupported,
}

#[derive(Debug,Copy,Clone,PartialEq)]
enum ByteType {
    Char(CharType),
    InvalidUtf8,
    Utf8LeadingByte(Delta<ByteIndex>),
}

impl From<u8> for ByteType {
    fn from(byte: u8) -> Self {
        match byte {
            b'+'|b'-'|b'*'|b'/' => Char(Operator),
            b'0'...b'9' => Char(Digit),
            b'(' => Char(Open),
            b')' => Char(Close),
            _ => ByteType::from_generic(byte)
        }
    }
}

impl ByteType {
    fn from_generic(byte: u8) -> Self {
        use parser::scanner::ByteType::*;
        match byte {
            0b0000_0000...0b0111_1111 => Char(CharType::Unsupported),
            0b1100_0000...0b1101_1111 => Utf8LeadingByte(Delta(ByteIndex(2))),
            0b1110_0000...0b1110_1111 => Utf8LeadingByte(Delta(ByteIndex(3))),
            0b1111_0000...0b1111_0111 => Utf8LeadingByte(Delta(ByteIndex(4))),
            _ => InvalidUtf8,
        }
    }

    fn is_utf8_cont(byte: u8) -> bool {
        byte >= 0b1000_0000 && byte < 0b1011_1111
    }
}

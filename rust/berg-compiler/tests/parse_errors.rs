#[macro_use]
mod compiler_test;

compiler_tests! {
    unsupported_characters: "`" => error(UnsupportedCharacters@0) result(nothing),
    unsupported_characters_multiple: "``" => error(UnsupportedCharacters@[0-1]) result(nothing),
    unsupported_characters_then_ok: "`1" => error(UnsupportedCharacters@0) result(1),
    unsupported_characters_multiple_then_ok: "``1" => error(UnsupportedCharacters@[0-1]) result(1),

    invalid_utf8_no_leading_byte:  [0b1000_0000]                           => error(InvalidUtf8@0) result(nothing),
    invalid_utf8_invalid_byte:     [0b1111_1000]                           => error(InvalidUtf8@0) result(nothing),
    invalid_utf8_multiple:         [0b1000_0000, 0b1111_1000, 0b1000_0000] => error(InvalidUtf8@[0-2]) result(nothing),
    invalid_utf8_multiple_then_ok: [0b1000_0000, 0b1111_1000, 0b1000_0000, b'1'] => error(InvalidUtf8@[0-2]) result(1),

    invalid_utf8_too_small_2:     [0b1100_0000, b'1']                     => error(InvalidUtf8@0) result(1),
    invalid_utf8_too_small_eof_2: [0b1100_0000]                           => error(InvalidUtf8@0) result(nothing),
    invalid_utf8_too_small_3:     [0b1110_0000, 0b1000_0000, b'1']        => error(InvalidUtf8@[0-1]) result(1),
    invalid_utf8_too_small_eof_3: [0b1110_0000, 0b1000_0000]              => error(InvalidUtf8@[0-1]) result(nothing),
    invalid_utf8_too_small_4:     [0b1110_0000, 0b1000_0000, b'1']        => error(InvalidUtf8@[0-1]) result(1),
    invalid_utf8_too_small_eof_4: [0b1111_0000, 0b1000_0000, 0b1000_0000] => error(InvalidUtf8@[0-2]) result(nothing),

    unsupported_and_invalid: [b'`', 0b1000_0000] => errors(UnsupportedCharacters@0, InvalidUtf8@1),
    unsupported_and_invalid_multiple: [b'`', b'`', 0b1000_0000, 0b1000_0000] => errors(UnsupportedCharacters@[0-1], InvalidUtf8@[2-3]),
    invalid_and_unsupported: [0b1000_0000, b'`'] => errors(InvalidUtf8@0, UnsupportedCharacters@1),
    invalid_and_unsupported_multiple: [0b1000_0000, 0b1000_0000, b'`', b'`'] => errors(InvalidUtf8@[0-1], UnsupportedCharacters@[2-3]),
}
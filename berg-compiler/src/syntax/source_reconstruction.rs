use crate::syntax::identifiers::*;
use crate::syntax::{Ast, AstIndex, ByteIndex, ByteRange, Token, ExpressionToken, OperatorToken, TermToken};
use std::cmp;
use std::fmt;
use std::io;
use std::io::Read;

///
/// Reconstructs a range of source from the parsed AST.
/// 
/// All data in an AST is preserved, which is what makes this possible.
/// 
pub struct SourceReconstruction<'p, 'a: 'p> {
    ast: &'p Ast<'a>,
    range: ByteRange,
}

///
/// An io::Reader over an AST that yields the same data as the original source.
/// 
/// Uses [`SourceReconstruction`] to do the formatting.
/// 
pub struct SourceReconstructionReader<'p, 'a: 'p> {
    iterator: SourceReconstructionIterator<'p, 'a>,
    buffered: Option<&'p [u8]>,
}

///
/// Iterates through the AST, yielding &strs that reconstruct the file.
/// 
/// Works by iterating in parallel through tokens, whitespace and line starts,
/// and picking whichever one covers the current range.
/// 
struct SourceReconstructionIterator<'p, 'a: 'p> {
    /// The AST we're reconstructing.
    ast: &'p Ast<'a>,
    /// The current byte index (corresponding to the original file).
    index: ByteIndex,
    /// The end of the range we're reconstructing (non-inclusive)
    end: ByteIndex,
    /// The next token we'll need to reconstruct.
    ast_index: AstIndex,
    /// The next whitespace we'll need to print.
    whitespace_index: usize,
    /// The next line start.
    line_start_index: usize,
}

impl<'p, 'a: 'p> SourceReconstruction<'p, 'a> {
    ///
    /// Create a reconstructor for the given range.
    /// 
    /// # Arguments
    /// 
    /// * `ast` - The AST containing the parsed information
    /// * 
    /// 
    pub fn new(ast: &'p Ast<'a>, range: ByteRange) -> Self {
        SourceReconstruction { ast, range }
    }
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        // Set up the buffer.
        let size = usize::from(self.range.end - self.range.start);
        let mut buffer = Vec::with_capacity(size);
        unsafe { buffer.set_len(size) };

        // Read!
        let mut reader = SourceReconstructionReader::new(self.ast, self.range.clone());
        let size = reader.read(buffer.as_mut_slice()).unwrap();
        assert!(size == buffer.len());
        buffer
    }
}

impl<'p, 'a: 'p> fmt::Display for SourceReconstruction<'p, 'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let iterator = SourceReconstructionIterator::new(self.ast, self.range.clone());
        for bytes in iterator {
            write!(f, "{}", String::from_utf8_lossy(bytes))?;
        }
        Ok(())
    }
}

impl<'p, 'a: 'p> SourceReconstructionReader<'p, 'a> {
    pub fn new(ast: &'p Ast<'a>, range: ByteRange) -> Self {
        SourceReconstructionReader {
            iterator: SourceReconstructionIterator::new(ast, range),
            buffered: None,
        }
    }

    fn next(&mut self) -> Option<&'p [u8]> {
        if let Some(buffer) = self.buffered.take() {
            Some(buffer)
        } else {
            self.iterator.next()
        }
    }
}

impl<'p, 'a: 'p> io::Read for SourceReconstructionReader<'p, 'a> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut read = 0;
        while let Some(mut bytes) = self.next() {
            assert!(!bytes.is_empty());

            read += bytes.read(&mut buffer[read..])?;
            if !bytes.is_empty() {
                // If we filled the buffer, stash it and return.
                self.buffered = Some(bytes);
                break;
            }
        }
        Ok(read)
    }
}

impl<'p, 'a: 'p> SourceReconstructionIterator<'p, 'a> {
    fn new(ast: &'p Ast<'a>, range: ByteRange) -> Self {
        assert!(ast.tokens.len() > 0);
        let index = range.start;
        SourceReconstructionIterator {
            ast,
            index,
            end: range.end,
            ast_index: find_ast_index(ast, index),
            whitespace_index: find_whitespace_index(ast, index),
            line_start_index: 0,
        }
    }
}

impl<'p, 'a: 'p> Iterator for SourceReconstructionIterator<'p, 'a> {
    type Item = &'p [u8];
    fn next(&mut self) -> Option<&'p [u8]> {
        if self.index >= self.end {
            return None;
        }

        // Get the next thing (be it token or whitespace)
        let (start, mut bytes) = self
            .next_token()
            .or_else(|| self.next_whitespace_range())
            .or_else(|| self.next_newline())
            .unwrap_or_else(|| (self.index, b" "));

        // Clip the beginning of the string if it starts earlier than index.
        match start.cmp(&self.index) {
            cmp::Ordering::Equal => {}
            cmp::Ordering::Less => {
                bytes = &bytes[(self.index - start).into()..];
            }
            cmp::Ordering::Greater => unreachable!(),
        }
        // Clip the end of the string if it goes past the end.
        if self.index + bytes.len() > self.end {
            bytes = &bytes[..(self.end - self.index).into()]
        }
        // Increment the index, and return!
        self.index += bytes.len();
        Some(bytes)
    }
}

impl<'p, 'a: 'p> SourceReconstructionIterator<'p, 'a> {
    fn next_token(&mut self) -> Option<(ByteIndex, &'p [u8])> {
        let token_ranges = &self.ast.token_ranges;
        let token_range = &self.ast.token_ranges[self.ast_index];
        if self.index >= token_range.start && self.index < token_range.end {
            // Grab the string we are returning this time.
            let result = self.token_bytes(token_range.start, self.ast.tokens[self.ast_index]);
            let end = match result {
                Some((start, bytes)) => start + bytes.len(),
                None => token_range.end,
            };

            // Skip to the next non-empty ast index now that we've passed this token.
            if end >= token_range.end {
                while self.ast_index + 1 < token_ranges.len() {
                    self.ast_index += 1;
                    if token_ranges[self.ast_index].start < token_ranges[self.ast_index].end {
                        break;
                    }
                }
            }

            // Return the string.
            result
        } else {
            None
        }
    }

    fn next_whitespace_range(&mut self) -> Option<(ByteIndex, &'p [u8])> {
        // If the current whitespace range includes us, return that string.
        if let Some((whitespace, whitespace_start)) = self.ast.char_data.whitespace_ranges.get(self.whitespace_index) {
            if *whitespace_start <= self.index {
                self.whitespace_index += 1;
                let whitespace_character = self.ast.char_data.whitespace_characters.resolve(*whitespace).unwrap();
                assert!(*whitespace_start + whitespace_character.len() > self.index, "whitespace {:?} at {} got skipped somehow! Current index is {}.", whitespace_character, whitespace_start, self.index);
                return Some((*whitespace_start, whitespace_character.as_bytes()));
            }
        }

        None
    }

    ///
    /// Write out \n if we have a line ending without any character data.
    /// 
    fn next_newline(&mut self) -> Option<(ByteIndex, &'p [u8])> {
        // If this is a line start, increment the line start index and return "\n".
        while let Some(line_start) = self.ast.char_data.line_starts.get(self.line_start_index) {
            // We haven't reached the line ending yet.
            if *line_start > self.index + 1 {
                break;
            }

            // We may have to skip a few line starts if some of them had alternate
            // line endings like \r or \r\n in them (and therefore we found the space
            // string in next_newline()).
            self.line_start_index += 1;
            if *line_start == self.index + 1 {
                return Some((self.index, b"\n"));
            }
        }

        None
    }

    fn token_bytes(&self, token_start: ByteIndex, token: Token) -> Option<(ByteIndex, &'p [u8])> {
        use Token::*;
        use ExpressionToken::*;
        use OperatorToken::*;
        use TermToken::*;

        let bytes = match token {
            Expression(token) => match token {
                Term(token) => match token {
                    IntegerLiteral(literal) | ErrorTerm(.., literal) => {
                        self.ast.literal_string(literal).as_bytes()
                    }
                    RawErrorTerm(.., raw_literal) => &self.ast.raw_literals[raw_literal],

                    FieldReference(field) => self
                        .ast
                        .identifier_string(self.ast.fields[field].name)
                        .as_bytes(),

                    RawIdentifier(identifier) => self.ast.identifier_string(identifier).as_bytes(),
                    MissingExpression => unreachable!(),
                }
                PrefixOperator(identifier) => self.ast.identifier_string(identifier).as_bytes(),
                Open(_, boundary, _) => boundary.open_string().as_bytes(),
            }
            Operator(token) => match token {
                InfixOperator(NEWLINE) => return None,
                InfixOperator(APPLY) => unreachable!(),

                InfixOperator(identifier)
                | PostfixOperator(identifier) => self.ast.identifier_string(identifier).as_bytes(),

                InfixAssignment(identifier) => {
                    // Because of how InfixAssignment works, we store the str for the "+" and assume the "="
                    let bytes = self.ast.identifier_string(identifier).as_bytes();
                    if self.index == token_start + bytes.len() {
                        return Some((token_start + bytes.len(), b"="));
                    } else {
                        bytes
                    }
                }

                Close(_, boundary) | CloseBlock(_, boundary) => boundary.close_string().as_bytes(),
            }
        };
        Some((token_start, bytes))
    }
}

fn find_ast_index(ast: &Ast, index: ByteIndex) -> AstIndex {
    let ast_index = ast.token_ranges.iter().position(|range| range.end > index);
    ast_index.unwrap_or_else(|| ast.token_ranges.len().into())
}

fn find_whitespace_index(ast: &Ast, index: ByteIndex) -> usize {
    // Get the first whitespace that starts *at* or *after* the given index.
    if let Some(next_whitespace) = ast.char_data.whitespace_ranges.iter().position(|(_, start)| *start >= index) {
        // If there is a whitespace starting *after* the given index, check if the previous one *intersects* the index.
        if next_whitespace > 0 {
            let (whitespace, start) = ast.char_data.whitespace_ranges[next_whitespace - 1];
            if start + ast.char_data.whitespace_characters.resolve(whitespace).unwrap().len() > index {
                return next_whitespace - 1;
            }
        }
        return next_whitespace
    }
    ast.char_data.whitespace_ranges.len()
}

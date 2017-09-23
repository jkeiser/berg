use internals::*;

#[derive(Debug)]
pub enum CompileError {
    SourceNotFound(io::Error),
    InvalidUtf8(Range<usize>),
    UnsupportedCharacters(Range<usize>),
    IoOpenError(io::Error),
    IoReadError(usize,io::Error),
    IoCurrentDirectoryError(PathBuf,io::ErrorKind,String),
}

impl CompileError {
    pub fn code(&self) -> u16 {
        match *self {
            // Codes 1-100, and x000 (1000, 2000, ...) are unused. Hard to google.
            // Syntax errors: 101-999
            SourceNotFound(..) => 101,
            InvalidUtf8(..) => 102,
            UnsupportedCharacters(..) => 103,

            // Type errors: 1001-1999

            // System errors: 9001-9999
            IoOpenError(..) => 9001,
            IoReadError(..) => 9002,
            IoCurrentDirectoryError(..) => 9003,
        }
    }
    pub fn format<'a>(&self, f: &mut fmt::Formatter, metadata: &SourceMetadata<'a>) -> fmt::Result {
        write!(f, "{:?} {:?} - BRGR-{} {}", metadata.source().name(), self.range(&metadata), self.code(), self.message(metadata.source()))
    }
    pub fn range<'a>(&self, metadata: &SourceMetadata<'a>) -> Range<LineColumn> {
        match *self {
            SourceNotFound(..)|IoOpenError(..)|IoCurrentDirectoryError(..) => Range { start: LineColumn::none(), end: LineColumn::none() },
            InvalidUtf8(ref range)|UnsupportedCharacters(ref range) => metadata.range(range),
            IoReadError(loc, _) => metadata.range(&(loc..loc)),
        }
    }
    pub fn message(&self, source: &Source) -> String {
        match *self {
            SourceNotFound(ref error) => format!("File not found: {:?} (error: {})", source.name(), error),
            UnsupportedCharacters(_) => format!("Unsupported characters"),
            InvalidUtf8(_) => format!("Invalid UTF-8"),
            IoOpenError(ref error) => format!("I/O error opening {:?}: {}", source.name(), error),
            IoReadError(_, ref error) => format!("I/O error while reading {:?}: {}", source.name(), error),
            IoCurrentDirectoryError(ref path, _, ref message) => format!("I/O error getting current directory to read relative path {:?}: {}", path, message),
        }
    }
}

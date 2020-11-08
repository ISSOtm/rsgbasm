mod expression;
mod instruction;
mod lexer;
mod section;
mod symbol;
use crate::lexer::{Lexer, Location, LocationSpan, TokType};
use crate::parser::AsmParser;
use crate::symbol::Symbol;
use lalrpop_util::lalrpop_mod;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read};
use std::rc::{Rc, Weak};

lalrpop_mod!(parser);

type ParseError = lalrpop_util::ParseError<Location, TokType, AssemblerError>;

#[derive(Debug)]
pub struct Error {
    err: ParseError,
}

fn write_expected_tokens(fmt: &mut Formatter, expected: &Vec<String>) -> Result<(), fmt::Error> {
    let mut items = expected.iter();
    write!(fmt, "{}", items.next().unwrap())?;

    let mut item = items.next();
    while let Some(tok) = item {
        let next = items.next();
        match next {
            Some(_) => write!(fmt, ", {}", tok)?,
            None => write!(fmt, " or {}", tok)?,
        }
        item = next;
    }
    Ok(())
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        use lalrpop_util::ParseError::*;

        match &self.err {
            InvalidToken { location } => write!(fmt, "Invalid token at {}", location),
            UnrecognizedEOF { location, expected } => {
                write!(fmt, "Unexpected EOF at {}; expected ", location)?;
                debug_assert_ne!(expected.len(), 0);
                write_expected_tokens(fmt, expected)
            }
            UnrecognizedToken {
                token: (begin, tok_type, end),
                expected,
            } => {
                write!(
                    fmt,
                    "Unexpected {} at {}; expected ",
                    tok_type,
                    LocationSpan::new(begin, end)
                )?;
                debug_assert_ne!(expected.len(), 0);
                write_expected_tokens(fmt, expected)
            }
            ExtraToken {
                token: (begin, tok_type, end),
            } => write!(
                fmt,
                "Unexpected {} at {}; expected no more tokens",
                tok_type,
                LocationSpan::new(begin, end)
            ),
            User { error } => error.fmt(fmt),
        }
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self { err }
    }
}

impl From<AssemblerError> for Error {
    fn from(err: AssemblerError) -> Self {
        Self { err: err.into() }
    }
}

#[derive(Debug)]
pub enum Warning {
    //
}

#[derive(Debug)]
pub enum AssemblerError {
    // Lexer errors
    BadInterpFmt(String),
    CharAfterLineCont(char),
    GarbageChar(char),
    EmptyFract,
    EmptyGfx,
    EmptyHex,
    EmptyInterpFmt,
    EmptyInterpName,
    EmptyOct,
    IllegalEscape(char),
    IllegalEscapeEOF,
    IllegalInterpChar(char),
    LineContEOF,
    MultipleInterpFmt,
    UntermInterp,
    UntermString,

    // Logic errors
    AssertFailure(Option<String>),
    LdHLHL,
    LocalInMainScope(String),

    // Expression errors
    ExprNotConstant,

    // Symbol errors
    SymbolRedef,
}

#[derive(Debug)]
pub enum Diagnostic {
    Warning(Warning),
    Error(Error),
}

pub type DiagCallback = dyn Fn(Diagnostic);

impl Display for AssemblerError {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::BadInterpFmt(s) => write!(fmt, "Bad interpolation format \"{}\"", s),
            Self::CharAfterLineCont(c) => write!(
                fmt,
                "Begun line continuation, but encountered character '{}'",
                c
            ),
            Self::GarbageChar(c) => write!(fmt, "Garbage char '{}'", c),
            Self::EmptyFract => write!(fmt, "Invalid fixed-point constant, no digits after '.'"),
            Self::EmptyGfx => write!(fmt, "Invalid gfx constant, no digits after '`'"),
            Self::EmptyHex => write!(fmt, "Invalid hex constant, no digits after '$'"),
            Self::EmptyInterpFmt => write!(fmt, "Empty interpolation format"),
            Self::EmptyInterpName => write!(fmt, "Empty interpolation name"),
            Self::EmptyOct => write!(fmt, "Invalid octal constant, no digits after '&'"),
            Self::IllegalEscape(c) => write!(fmt, "Illegal character escape '{}'", c),
            Self::IllegalEscapeEOF => write!(fmt, "Illegal character escape at end of input"),
            Self::IllegalInterpChar(c) => write!(fmt, "Illegal character '{}' in interpolation", c),
            Self::LineContEOF => write!(fmt, "Line continuation at end of file"),
            Self::MultipleInterpFmt => write!(fmt, "Multiple interpolation formats"),
            Self::UntermInterp => write!(fmt, "Unterminated interpolation"),
            Self::UntermString => write!(fmt, "Unterminated string"),

            Self::AssertFailure(Some(s)) => write!(fmt, "Assertion failure: {}", s),
            Self::AssertFailure(None) => write!(fmt, "Assertion failure"),
            Self::LdHLHL => write!(fmt, "ld [hl], [hl] is not a valid instruction"),
            Self::LocalInMainScope(name) => write!(fmt, "Local symbol \"{}\" in main scope", name),

            Self::ExprNotConstant => write!(fmt, "Expression is not constant"),

            Self::SymbolRedef => write!(fmt, "Redefined symbol"),
        }
    }
}

#[derive(Debug)]
pub enum AssertType {
    Warn,
    Error,
    Fatal,
}

pub struct Assembler<'a> {
    symbols: RefCell<HashMap<Rc<String>, Symbol>>,
    sym_scope: RefCell<Option<Weak<Symbol>>>,

    // Callbacks
    diagnose: &'a DiagCallback,
}

impl<'a> Assembler<'a> {
    // === Contructor ===

    pub fn new(diagnose: &'a DiagCallback) -> Self {
        Self {
            symbols: RefCell::new(HashMap::new()),
            sym_scope: RefCell::new(None),

            diagnose,
        }
    }

    // === Main call ===

    pub fn assemble(&mut self, mut f: impl Read) -> Result<(), io::Error> {
        // Init all
        self.symbols.borrow_mut().clear();

        self.add_symbol(Symbol::new_equ("_RS".to_string(), 0))
            .unwrap();

        // FIXME: reading the whole file as a string sucks, using an Iterator over chars would be much better
        let mut s = String::new();
        f.read_to_string(&mut s)?;

        let lexer_state = RefCell::new(Lexer::new_state());
        let lexer = Lexer::new(s.chars(), &lexer_state, self.diagnose, &self);

        if let Err(err) = AsmParser::new().parse(self, &lexer_state, lexer) {
            (self.diagnose)(Diagnostic::Error(err.into()));
        }
        Ok(())
    }

    // === Error reporting ===

    pub fn assert(
        &self,
        assert_type: AssertType,
        expr: i32,
        msg: Option<String>,
    ) -> Result<(), AssemblerError> {
        if expr == 0 {
            return Ok(());
        }
        match assert_type {
            AssertType::Warn => unimplemented!(),
            AssertType::Error => unimplemented!(),
            AssertType::Fatal => Err(AssemblerError::AssertFailure(msg)),
        }
    }

    // === Symbol management ===

    pub fn get_symbol_scope(&self) -> Option<Rc<Symbol>> {
        self.sym_scope
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
    }

    pub fn set_symbol_scope(&self, scope: Weak<Symbol>) {
        self.sym_scope.replace(Some(scope));
    }

    pub fn expand_sym_name(&self, name: String) -> Result<String, AssemblerError> {
        if !name.starts_with('.') {
            Ok(name)
        } else {
            if let Some(scope) = self.get_symbol_scope() {
                Ok(format!("{}{}", scope.get_name(), name))
            } else {
                Err(AssemblerError::LocalInMainScope(name))
            }
        }
    }

    pub fn find_symbol(&self, name: &String) -> Option<Ref<Symbol>> {
        self.symbols.borrow().get(name).map(|sym| Ref::map(sym))
    }

    pub fn add_symbol(&self, sym: Symbol) -> Result<(), AssemblerError> {
        if let Some(other) = self.symbols.borrow_mut().get_mut(sym.get_name()) {
            other.redefine(sym)?;
            Ok(())
        } else {
            self.symbols
                .borrow_mut()
                .insert(Rc::clone(sym.get_name()), sym);
            Ok(())
        }
    }

    pub(crate) fn advance_rs(&self, offset: i32) -> i32 {
        let rs = self
            .symbols
            .borrow_mut()
            .get_mut(&"_RS".to_string())
            .unwrap();
        let val = rs.get_value().unwrap();
        rs.set_value(val + offset);

        val
    }
}

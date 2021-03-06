use core::iter::Iterator;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::ExecutionContext;
use crate::mm::Vector;
use crate::mm::String;
use crate::mm::AllocError;
use crate::error::Error;
use crate::xc_err;

#[derive(Debug, PartialEq)]
pub enum ParseErrorData {
    ReachedEnd,
    NotImplemented,
    Alloc(AllocError),
    IllegalChar(char),
    UnexpectedChar(char),
    UnexpectedToken,
}
pub type ParseError<'a> = Error<'a, ParseErrorData>;

#[derive(Debug, Copy, Clone)]
pub struct Source<'s> {
    content: &'s str,
    name: &'s str,
}

#[derive(Debug)]
pub struct SourceSlice<'s> {
    source: &'s Source<'s>,
    start_offset: usize,
    end_offset: usize,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

#[derive(Debug)]
pub struct Token<'s, T> {
    data: T,
    source_slice: SourceSlice<'s>,
}

#[derive(Debug, PartialEq)]
pub struct CharInfo {
    codepoint: char,
    width: u8,
    size: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BasicTokenType {
    End,
    Identifier,
    Dot,
    Comma,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BasicTokenTypeBitmap(u64);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BasicTokenTypeBitmapIterator {
    mask: u64,
    pos: u8,
}

#[derive(Debug, PartialEq)]
pub enum BasicTokenData<'a> {
    End,
    //BoolLiteral(bool),
    //U64Literal(u64),
    //StringLiteral(String<'a>),
    //BinLiteral(Vector<'a, u8>),
    Identifier(String<'a>),
    //OpenParen,
    //CloseParen,
    //OpenSquareBracket,
    //CloseSquareBracket,
    //LessThan,
    //GreaterThan,
    //Tilde,
    //Exclamation,
    //Percent,
    //Caret,
    //Ampersand,
    //Star,
    //Minus,
    //Plus,
    //Equal,
    //Pipe,
    //Slash,
    //DoubleLessThan,
    //DoubleGreatedThan,
    //Comma,
    Dot,
    Comma,
    //QuestionMark,
    //Colon,
}

#[derive(Debug, PartialEq)]
pub enum PrimaryExpr<'a> {
    Identifier(String<'a>),
}

#[derive(Debug, PartialEq)]
pub enum PostfixRoot<'a> {
    Primary(PrimaryExpr<'a>), // points to foo in foo.bar
    // Implied... for expressions like .bla (points to the empty space before .)
}

#[derive(Debug, PartialEq)]
pub enum PostfixItem<'a> {
    Property(String<'a>), // points to bar or baz in foo.bar.baz
    // Subscript(ExprList<'a>), // a[b, c]
    // Call(ExprList<'a>), // a(b, c)
}

#[derive(Debug, PartialEq)]
pub struct PostfixExpr<'a> {
    pub root: PostfixRoot<'a>,
    pub items: Vector<'a, PostfixItem<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Expr<'a> {
    Postfix(PostfixExpr<'a>),
}

#[derive(Debug, PartialEq)]
pub struct ExprList<'a> {
    items: Vector<'a, Expr<'a>>,
}

pub struct Parser<'s, 't> {
    source: &'s Source<'s>,
    exectx: ExecutionContext<'t>,
    lookup_token: Option<Token<'s, BasicTokenData<'t>>>,
    cr_lf_to_lf: bool,
    cr_to_lf: bool,
    tab_width: Option<u8>,
    remaining_text: &'s str,
    current_line: u32,
    current_column: u32,
}

impl<'a> From<AllocError> for ParseError<'a> {
    fn from(e: AllocError) -> Self {
        ParseError::with_str(ParseErrorData::Alloc(e), "alloc error")
    }
}

impl<'a, T> From<(AllocError, T)> for ParseError<'a> {
    fn from(e: (AllocError, T)) -> Self {
        ParseError::with_str(ParseErrorData::Alloc(e.0), "alloc error")
    }
}

impl BasicTokenType {
    pub fn name(&self) -> &'static str {
        match self {
            BasicTokenType::End => "end-of-file",
            BasicTokenType::Identifier => "identifier",
            BasicTokenType::Dot => "dot",
            BasicTokenType::Comma => "comma",
        }
    }
    pub fn to_bitmap(&self) -> BasicTokenTypeBitmap {
        BasicTokenTypeBitmap(1_u64 << (*self as usize))
    }
    pub fn from_u8(v: u8) -> Option<BasicTokenType> {
        if v == (BasicTokenType::End as u8) {
            Some(BasicTokenType::End)
        } else if v == (BasicTokenType::Identifier as u8) {
            Some(BasicTokenType::Identifier)
        } else if v == (BasicTokenType::Dot as u8) {
            Some(BasicTokenType::Dot)
        } else if v == (BasicTokenType::Comma as u8) {
            Some(BasicTokenType::Comma)
        } else {
            None
        }
    }
}

impl Display for BasicTokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.name(), f)
    }
}

impl BasicTokenTypeBitmap {
    pub fn new() -> Self {
        BasicTokenTypeBitmap(0)
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn contains(&self, btt: BasicTokenType) -> bool {
        ((self.0 >> (btt as usize)) & 1) != 0
    }
    pub fn len(&self) -> u32 {
        self.0.count_ones()
    }
    pub fn add_types(&mut self, l: &[BasicTokenType]) {
        for t in l {
            self.0 |= t.to_bitmap().0;
        }
    }
    pub fn from_list(l: &[BasicTokenType]) -> Self {
        let mut b = Self::new();
        b.add_types(l); b
    }
    pub fn iter(&self) -> BasicTokenTypeBitmapIterator {
        BasicTokenTypeBitmapIterator {
            mask: self.0,
            pos: 0,
        }
    }
}

impl Iterator for BasicTokenTypeBitmapIterator {
    type Item = BasicTokenType;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < 64 && (self.mask & (1_u64 << self.pos)) == 0 { self.pos += 1; }
        if self.pos == 64 {
            None
        } else {
            let p = self.pos;
            self.pos += 1;
            BasicTokenType::from_u8(p)
        }
    }
}

impl Display for BasicTokenTypeBitmap {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_empty() {
            Display::fmt("no types", f)
        } else {
            let mut first = true;
            for btt in self.iter() {
                if !first {
                    write!(f, ", ")?;
                } else {
                    first = false;
                }
                Display::fmt(&btt, f)?;
            }
            Ok(())
        }
    }
}

impl<'t> BasicTokenData<'t> {
    pub fn to_type(&self) -> BasicTokenType {
        match self {
            BasicTokenData::End => BasicTokenType::End,
            BasicTokenData::Identifier(_) => BasicTokenType::Identifier,
            BasicTokenData::Dot => BasicTokenType::Dot,
            BasicTokenData::Comma => BasicTokenType::Comma,
        }
    }
    pub fn type_str(&self) -> &'static str {
        self.to_type().name()
    }
    pub fn unwrap_identifier_data(self) -> String<'t> {
        if let BasicTokenData::Identifier(s) = self { s } else {
            panic!("expecting Identifier, not {:?}", self);
        }
    }
}

impl<'t> Display for BasicTokenData<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            BasicTokenData::End => "<end-of-file>".fmt(f),
            BasicTokenData::Dot => "'.'".fmt(f),
            BasicTokenData::Comma => "','".fmt(f),
            BasicTokenData::Identifier(s) => s.fmt(f),
        }
    }
}

impl<'t> Display for PrimaryExpr<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PrimaryExpr::Identifier(s) => s.fmt(f),
        }
    }
}

impl<'t> Display for PostfixRoot<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PostfixRoot::Primary(pe) => pe.fmt(f),
        }
    }
}

impl<'t> Display for PostfixItem<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PostfixItem::Property(s) => write!(f, ".{}", s),
        }
    }
}

impl<'t> Display for PostfixExpr<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.root)?;
        for i in self.items.as_slice() {
            write!(f, "{}", i)?;
        }
        Ok(())
    }
}

impl<'t> From<PostfixExpr<'t>> for Expr<'t> {
    fn from(pe: PostfixExpr<'t>) -> Expr<'t> {
        Expr::Postfix(pe)
    }
}

impl<'t> Display for Expr<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Expr::Postfix(pfe) => pfe.fmt(f),
        }
    }
}

impl<'s, 't> From<Token<'s, PostfixExpr<'t>>> for Token<'s, Expr<'t>> {
    fn from(src: Token<'s, PostfixExpr<'t>>) -> Self {
        Token {
            data: src.data.into(),
            source_slice: src.source_slice,
        }
    }
}

impl<'t> ExprList<'t> {
    pub fn unwrap_items(self) -> Vector<'t, Expr<'t>> {
        self.items
    }
}
impl<'t> Display for ExprList<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut write_sep = false;
        for e in self.items.as_slice() {
            if write_sep {
                write!(f, ", ")?;
            } else {
                write_sep = true;
            }
            e.fmt(f)?;
        }
        Ok(())
    }
}

impl<'s> Source<'s> {
    pub fn new(content: &'s str, name: &'s str) -> Self {
        Source { content, name }
    }
    pub fn get_content(&self) -> &'s str {
        self.content
    }

    pub fn get_name(&self) -> &'s str {
        self.name
    }
}

impl<'s> SourceSlice<'s> {
    pub fn as_str(&self) -> &'s str {
        &self.source.content[self.start_offset..self.end_offset]
    }
    pub fn update_end<'t>(&mut self, tail: &SourceSlice<'t>) {
        self.end_offset = tail.end_offset;
        self.end_line = tail.end_line;
        self.end_column = tail.end_column;
    }
}

impl<'s, T> Token<'s, T> {
    pub fn to_parts(self) -> (T, SourceSlice<'s>) {
        (self.data, self.source_slice)
    }
    pub fn unwrap_data(self) -> T {
        self.data
    }
}

impl<'s, 't> Parser<'s, 't> {

    pub fn new(src: &'s Source<'s>, xc: &ExecutionContext<'t>) -> Self {
        Parser {
            source: src,
            exectx: xc.to_non_logging(),
            lookup_token: None,
            cr_lf_to_lf: true,
            cr_to_lf: true,
            tab_width: None,
            remaining_text: src.content,
            current_line: 1,
            current_column: 1,
        }
    }
    pub fn set_new_line_handling(&mut self, cr_lf_to_lf: bool, cr_to_lf: bool) {
        self.cr_lf_to_lf = cr_lf_to_lf;
        self.cr_to_lf = cr_to_lf;
    }

    pub fn set_tab_handling(&mut self, tab_width: Option<u8>) {
        self.tab_width = tab_width;
    }

    pub fn is_whitespace(&self, ch: char) -> bool {
        ch == ' ' || ch == '\n' || ch == '\r' || (ch == '\t' && self.tab_width.is_some())
    }
    pub fn is_legal_char(&self, ch: char) -> bool {
        ch < '\x7F' && (ch >= ' ' || self.is_whitespace(ch))
    }

    pub fn peek_raw_char(&self) -> Option<CharInfo> {
        let mut it = self.remaining_text.chars();
        it.next().map(|ch|
            if (ch as u32) < 32 {
                if ch == '\r' {
                    if self.cr_lf_to_lf && Some('\n') == it.next() {
                        CharInfo { codepoint: '\n', width: 0, size: 2 }
                    } else if self.cr_to_lf {
                        CharInfo { codepoint: '\n', width: 0, size: 1 }
                    } else {
                        CharInfo { codepoint: '\r', width: 0, size: 1 }
                    }
                } else {
                    CharInfo { codepoint: ch, width: 0, size: 1 }
                }
            } else {
                CharInfo { codepoint: ch, width: 1, size: ch.len_utf8() as u8 }
            })
    }
    pub fn peek_char(&mut self) -> Result<CharInfo, ParseError<'t>> {
        self.peek_raw_char()
            .ok_or_else(|| Error::with_str(ParseErrorData::ReachedEnd, "reached end of source file"))
            .and_then(|ci| {
                if self.is_legal_char(ci.codepoint) {
                    Ok(ci)
                } else {
                    Err(Error::with_str(ParseErrorData::IllegalChar(ci.codepoint), "illegal char"))
                }
            })
    }

    pub fn consume_char(&mut self, ci: CharInfo) {
        debug_assert!(ci.size > 0);
        match ci.codepoint {
            '\n' | '\r' => {
                self.current_line += 1;
                self.current_column = 1;
            },
            '\t' => if let Some(w) = self.tab_width {
                let w = w as u32;
                self.current_column = ((self.current_column - 1) / w + 1) * w + 1;
            },
            _ => { self.current_column += ci.width as u32; }
        }
        self.remaining_text = &self.remaining_text[(ci.size as usize)..];
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(ci) = self.peek_raw_char() {
            if !self.is_whitespace(ci.codepoint) { break; }
            self.consume_char(ci);
        }
    }

    pub fn current_offset(&self) -> usize {
        (self.remaining_text.as_ptr() as usize)
            - (self.source.content.as_ptr() as usize)
    }
    pub fn here(&self) -> SourceSlice<'s> {
        SourceSlice {
            source: &self.source,
            start_offset: self.current_offset(),
            end_offset: self.current_offset(),
            start_line: self.current_line,
            start_column: self.current_column,
            end_line: self.current_line,
            end_column: self.current_column,
        }
    }
    fn end_slice_here(&self, ss: &mut SourceSlice<'s>) {
        ss.end_offset = self.current_offset();
        ss.end_line = self.current_line;
        ss.end_column = self.current_column;
    }

    pub fn can_start_identifier(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    pub fn is_valid_identifier_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn parse_identifier(
        &mut self,
    ) -> Result<Token<'s, BasicTokenData<'t>>, ParseError<'t>> {
        let mut id = self.exectx.string();
        let mut source_slice = self.here();
        while let Ok(ci) = self.peek_char() {
            if !Parser::is_valid_identifier_char(ci.codepoint) { break; }
            id.push(ci.codepoint)?;
            self.consume_char(ci);
        }
        self.end_slice_here(&mut source_slice);
        Ok(Token {
            data: BasicTokenData::Identifier(id),
            source_slice: source_slice
        })
    }

    pub fn parse_basic_token(
        &mut self
    ) -> Result<Token<'s, BasicTokenData<'t>>, ParseError<'t>> {
        self.skip_whitespace();
        if self.remaining_text.is_empty() {
            return Ok(Token {
                data: BasicTokenData::End,
                source_slice: self.here()
            })
        }
        let c = self.peek_char()?;
        if Parser::can_start_identifier(c.codepoint) {
            return self.parse_identifier();
        }
        let mut ss = self.here();
        let td = match c.codepoint {
            '.' => {
                self.consume_char(c);
                BasicTokenData::Dot
            },
            ',' => {
                self.consume_char(c);
                BasicTokenData::Comma
            },
            _ => {
                let cp = c.codepoint;
                self.consume_char(c);
                return Err(xc_err!(self.exectx, ParseErrorData::UnexpectedChar(cp), "unexpected char", "unexpected char {:?} at {}:{}", cp, ss.start_line, ss.start_column));
            },
        };
        self.end_slice_here(&mut ss);
        Ok(Token {
            data: td,
            source_slice: ss,
        })
    }

    pub fn preview_next_token(
        &mut self
    ) -> Result<&Token<'s, BasicTokenData<'t>>, ParseError<'t>> {
        if self.lookup_token.is_none() {
            self.lookup_token = Some(self.parse_basic_token()?);
        }
        Ok(self.lookup_token.as_ref().unwrap())
    }

    pub fn get_next_token(
        &mut self
    ) -> Result<Token<'s, BasicTokenData<'t>>, ParseError<'t>> {
        self.preview_next_token()?;
        Ok(self.lookup_token.take().unwrap())
    }

    pub fn expect_token(
        &mut self,
        expected: BasicTokenTypeBitmap,
    ) -> Result<Token<'s, BasicTokenData<'t>>, ParseError<'t>> {
        let t = self.get_next_token()?;
        if expected.contains(t.data.to_type()) {
            Ok(t)
        } else {
            Err(xc_err!(self.exectx, ParseErrorData::UnexpectedToken, "unexpected token", "expecting [{}] not {} at {}:{}", expected, t.data.type_str(), t.source_slice.start_line, t.source_slice.start_column))
        }
    }

    pub fn get_identifier_str(
        &mut self
    ) -> Result<String<'t>, ParseError<'t>> {
        Ok(self.expect_token(BasicTokenType::Identifier.to_bitmap())?.data.unwrap_identifier_data())
    }

    // pub fn is_next_token_matching(
    //     &mut self,
    //     desired: BasicTokenTypeBitmap,
    // ) -> Result<bool, ParseError<'t>> {
    //     let t = self.preview_next_token()?;
    //     Ok(desired.contains(t.data.to_type()))
    // }

    pub fn get_token_matching_types(
        &mut self,
        desired: BasicTokenTypeBitmap,
    ) -> Result<Option<Token<'s, BasicTokenData<'t>>>, ParseError<'t>> {
        let t = self.preview_next_token()?;
        if desired.contains(t.data.to_type()) {
            self.get_next_token().map(|t| Some(t))
        } else {
            Ok(None)
        }
    }

    pub fn parse_primary_expr(
        &mut self,
    ) -> Result<Token<'s, PrimaryExpr<'t>>, ParseError<'t>> {
        let t = self.get_next_token()?;
        if let BasicTokenData::Identifier(id) = t.data {
            Ok(Token {
                data: PrimaryExpr::Identifier(id),
                source_slice: t.source_slice,
            })
        } else {
            Err(xc_err!(self.exectx, ParseErrorData::UnexpectedToken, "identifier expected", "identifier expected at {}:{}", t.source_slice.start_line, t.source_slice.start_column))
        }
    }

    pub fn parse_postfix_expr(
        &mut self,
    ) -> Result<Token<'s, PostfixExpr<'t>>, ParseError<'t>> {
        let mut ss = self.here();
        let mut pfx_expr = PostfixExpr {
            root: PostfixRoot::Primary(self.parse_primary_expr()?.data),
            items: self.exectx.vector(),
        };
        self.end_slice_here(&mut ss);
        while let Some(_dot) = self.get_token_matching_types(
            BasicTokenType::Dot.to_bitmap())? {
            let id_str = self.get_identifier_str()?;
            pfx_expr.items.push(PostfixItem::Property(id_str))?;
            self.end_slice_here(&mut ss);
        }
        Ok(Token {
            data: pfx_expr,
            source_slice: ss,
        })
    }

    pub fn parse_expr(
        &mut self,
    ) -> Result<Token<'s, Expr<'t>>, ParseError<'t>> {
        Ok(self.parse_postfix_expr()?.into())
    }

    pub fn parse_expr_list(
        &mut self,
    ) -> Result<Token<'s, ExprList<'t>>, ParseError<'t>> {
        let mut ss = self.here();
        let mut iv = self.exectx.vector();
        {
            let t = self.parse_expr()?;
            iv.push(t.data)?;
            ss.update_end(&t.source_slice);
        }
        while let Some(_comma) = self.get_token_matching_types(
            BasicTokenType::Comma.to_bitmap())? {
            let t = self.parse_expr()?;
            iv.push(t.data)?;
            ss.update_end(&t.source_slice);
        }
        Ok(Token{
            data: ExprList {
                items: iv
            },
            source_slice: ss,
        })
    }

}

#[cfg(test)]
mod tests {
    use crate::mm::SingleAlloc;
    use crate::mm::Allocator;
    use core::fmt::Write;

    use super::*;

    #[test]
    fn parse_error_from_alloc() {
        let pe: ParseError<'_> = AllocError::NotEnoughMemory.into();
        assert_eq!(*pe.get_data(), ParseErrorData::Alloc(AllocError::NotEnoughMemory));
        let pe: ParseError<'_> = (AllocError::UnsupportedOperation, 0).into();
        assert_eq!(*pe.get_data(), ParseErrorData::Alloc(AllocError::UnsupportedOperation));
    }

    #[test]
    fn basic_token_type_bitmap_iterate() {
        let b = BasicTokenTypeBitmap::from_list(&[BasicTokenType::End, BasicTokenType::Identifier]);
        let mut i = b.iter();
        assert_eq!(b.len(), 2);
        assert_eq!(i.next(), Some(BasicTokenType::End));
        assert_eq!(i.next(), Some(BasicTokenType::Identifier));
        assert_eq!(i.next(), None);
    }

    #[test]
    fn basic_token_type_names() {
        assert_eq!(BasicTokenType::Identifier.name(), "identifier");
        assert_eq!(BasicTokenType::Dot.name(), "dot");
        assert_eq!(BasicTokenType::Comma.name(), "comma");

        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 2048];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let e = xc_err!(xc, (), "-grr-", "{}", BasicTokenType::End.to_bitmap());
        assert_eq!(e.get_msg(), "end-of-file");
    }

    #[test]
    fn empty_basic_token_type_bitmap_display() {
        use core::fmt::Write;
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 2048];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let mut s = xc.string();
        write!(s, "{}", BasicTokenTypeBitmap(0)).unwrap();
        assert_eq!(s.as_str(), "no types");
    }

    #[test]
    fn basic_token_type_bitmap_display() {
        use core::fmt::Write;
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 2048];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let mut s = xc.string();
        write!(s, "{}", BasicTokenTypeBitmap::from_list(&[BasicTokenType::End, BasicTokenType::Dot])).unwrap();
        assert_eq!(s.as_str(), "end-of-file, dot");
    }

    #[test]
    fn source_new() {
        let c;
        let n;
        {
            let src = Source::new("bla", "-");
            c = src.get_content();
            n = src.get_name();
        }
        assert_eq!(c, "bla");
        assert_eq!(n, "-");
    }

    #[test]
    fn peek_raw_ctl_char() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\x0B", "-");
        let p = Parser::new(&src, &xc);
        let ucp = CharInfo { codepoint: '\x0B', width: 0, size: 1 };
        assert_eq!(p.peek_raw_char().unwrap(), ucp);
    }

    #[test]
    fn peek_raw_large_code_point() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\u{10348}", "-");
        let p = Parser::new(&src, &xc);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\u{10348}', width: 1, size: 4 });
    }

    #[test]
    fn peek_cr_lf_no_conv() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src, &xc);
        p.set_new_line_handling(false, false);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\r', width: 0, size: 1 });
    }

    #[test]
    fn peek_cr_lf_all_conv() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src, &xc);
        p.set_new_line_handling(true, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 2 });
    }
    #[test]
    fn peek_cr_lf_part_conv() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src, &xc);
        p.set_new_line_handling(false, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 1 });
    }

    #[test]
    fn skip_whitespace() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\r\n\n\r      a", "-");
        let mut p = Parser::new(&src, &xc);
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: 'a', width: 1, size: 1 });
    }

    #[test]
    fn skip_whitespace_to_end() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\r\n\n\r      ", "-");
        let mut p = Parser::new(&src, &xc);
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char(), None);
    }

    #[test]
    fn peek_char_at_end() {
        let src = Source::new("", "-");
        let xc = ExecutionContext::nop();
        let mut p = Parser::new(&src, &xc);
        assert_eq!(*p.peek_char().unwrap_err().get_data(), ParseErrorData::ReachedEnd)
    }

    #[test]
    fn peek_illegal_char() {
        let src = Source::new("\x01", "-");
        let xc = ExecutionContext::nop();
        let mut p = Parser::new(&src, &xc);
        assert_eq!(*p.peek_char().unwrap_err().get_data(), ParseErrorData::IllegalChar('\x01'))
    }

    #[test]
    fn peek_char() {
        let src = Source::new("!", "-");
        let xc = ExecutionContext::nop();
        let mut p = Parser::new(&src, &xc);
        assert_eq!(p.peek_char().unwrap(), CharInfo { codepoint: '!', width: 1, size: 1 });
    }

    #[test]
    fn consume_tab() {
        let xc = ExecutionContext::nop();
        let src = Source::new("\t", "-");
        let mut p = Parser::new(&src, &xc);
        p.set_tab_handling(Some(5));
        let ci = CharInfo { codepoint: '\t', width: 0, size: 1 };
        assert_eq!(p.peek_char().unwrap(), ci);
        p.consume_char(ci);
        assert_eq!(p.current_line, 1);
        assert_eq!(p.current_column, 6);

    }

    #[test]
    fn only_whitespaces_produce_end_token() {
        let xc = ExecutionContext::nop();
        let src = Source::new("       \n         ", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        assert_eq!(t.data, BasicTokenData::End);
    }

    #[test]
    fn identifier_token() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("  best.worst", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        assert_eq!(t.source_slice.as_str(), "best");
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 3));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 7));
        assert_eq!(t.data.unwrap_identifier_data().as_str(), "best");
    }

    #[test]
    #[should_panic(expected = "expecting Identifier, not")]
    fn unwrap_identifier_data_from_dot() {
        BasicTokenData::Dot.unwrap_identifier_data();
    }

    #[test]
    fn identifier_token_oom() {
        use crate::mm::AllocError;
        let xc = ExecutionContext::nop();
        let src = Source::new("  best.worst", "-");
        let mut p = Parser::new(&src, &xc);
        assert_eq!(*p.parse_basic_token().unwrap_err().get_data(), ParseErrorData::Alloc(AllocError::UnsupportedOperation));
    }

    #[test]
    fn dot_token() {
        let xc = ExecutionContext::nop();
        let src = Source::new(".a", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        assert_eq!(t.data, BasicTokenData::Dot);
        assert_eq!(t.source_slice.as_str(), ".");
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 1));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 2));
    }

    #[test]
    fn comma_token() {
        let xc = ExecutionContext::nop();
        let src = Source::new(" ,\n", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        assert_eq!(t.data, BasicTokenData::Comma);
        assert_eq!(t.source_slice.as_str(), ",");
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 2));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 3));
        let t = p.parse_basic_token().unwrap();
        assert_eq!(t.data, BasicTokenData::End);
    }

    #[test]
    fn next_token_encounters_bad_char() {
        let xc = ExecutionContext::nop();
        let src = Source::new("`", "-");
        let mut p = Parser::new(&src, &xc);
        let e = p.parse_basic_token().unwrap_err();
        assert_eq!(*e.get_data(), ParseErrorData::UnexpectedChar('`'));
        assert_eq!(e.get_msg(), "unexpected char");
    }

    #[test]
    fn next_token_encounters_bad_char_with_error_msg() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("`", "-");
        let mut p = Parser::new(&src, &xc);
        let e = p.parse_basic_token().unwrap_err();
        assert_eq!(*e.get_data(), ParseErrorData::UnexpectedChar('`'));
        assert_eq!(e.get_msg(), "unexpected char '`' at 1:1");
    }

    #[test]
    fn token_to_parts() {
        let xc = ExecutionContext::nop();
        let src = Source::new(".a", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        let (d, ss) = t.to_parts();
        assert_eq!(d, BasicTokenData::Dot);
        assert_eq!(ss.as_str(), ".");
        assert_eq!((ss.start_line, ss.start_column), (1, 1));
        assert_eq!((ss.end_line, ss.end_column), (1, 2));
    }

    #[test]
    fn token_unwrap_data() {
        let xc = ExecutionContext::nop();
        let src = Source::new(".a", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_basic_token().unwrap();
        let d = t.unwrap_data();
        assert_eq!(d, BasicTokenData::Dot);
    }

    #[test]
    fn id_as_primary_expr() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo.bar", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_primary_expr().unwrap();
        assert_eq!(t.data, PrimaryExpr::Identifier(String::map_str("foo")));
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 1));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 4));
    }

    #[test]
    fn dot_as_primary_expr() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new(" .bar", "-");
        let mut p = Parser::new(&src, &xc);
        let e = p.parse_primary_expr().unwrap_err();
        assert_eq!(*e.get_data(), ParseErrorData::UnexpectedToken);
        assert_eq!(e.get_msg(), "identifier expected at 1:2");
    }

    #[test]
    fn id_dot_chain_postfix_expr() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo .bar baz", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_postfix_expr().unwrap();
        assert_eq!(t.source_slice.as_str(), "foo .bar");
        assert_eq!(p.get_identifier_str().unwrap().as_str(), "baz");
    }

    #[test]
    fn postfix_dot_dot() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 2048];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo .bar.  .", "-");
        let mut p = Parser::new(&src, &xc);
        let e = p.parse_postfix_expr().unwrap_err();
        assert_eq!(*e.get_data(), ParseErrorData::UnexpectedToken);
        assert_eq!(e.get_msg(), "expecting [identifier] not dot at 1:12");

    }

    #[test]
    fn expr_list_2_items() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 2048];
        let a = BumpAllocator::new(&mut buffer);
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo .bar , \nmoo\n. mar baz", "-");
        let mut p = Parser::new(&src, &xc);
        let t = p.parse_expr_list().unwrap();
        extern crate std; use std::dbg; dbg!(t.source_slice.as_str());
        assert_eq!(t.data.items.len(), 2);
        assert_eq!(t.source_slice.as_str(), "foo .bar , \nmoo\n. mar");
    }

    #[test]
    fn display_basic_token_data() {
        use crate::mm::SingleAlloc;
        use crate::mm::Allocator;
        use core::fmt::Write;
        let mut buffer = [0; 2048];
        let a = SingleAlloc::new(&mut buffer);

        {
            let mut s = String::new(a.to_ref());
            write!(s, "{}", BasicTokenData::End).unwrap();
            assert_eq!(s.as_str(), "<end-of-file>");
        }

        {
            let mut s = String::new(a.to_ref());
            write!(s, "{}", BasicTokenData::Identifier(String::map_str("abc"))).unwrap();
            assert_eq!(s.as_str(), "abc");
        }

        {
            let mut s = String::new(a.to_ref());
            write!(s, "{}", BasicTokenData::Dot).unwrap();
            assert_eq!(s.as_str(), "'.'");
        }

        {
            let mut s = String::new(a.to_ref());
            write!(s, "{}", BasicTokenData::Comma).unwrap();
            assert_eq!(s.as_str(), "','");
        }
    }

    #[test]
    fn display_primary_expr_identifier() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let x = PrimaryExpr::Identifier(String::map_str("abc"));
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "abc");
    }

    #[test]
    fn display_postfix_root_primary() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let x = PrimaryExpr::Identifier(String::map_str("abc"));
        let x = PostfixRoot::Primary(x);
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "abc");
    }

    #[test]
    fn display_postfix_item_property() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let x = PostfixItem::Property(String::map_str("abc"));
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), ".abc");
    }

    #[test]
    fn display_postfix_expr_0() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let x = PostfixExpr {
            root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
            items: Vector::map_slice(&[]),
        };
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "a");
    }

    #[test]
    fn display_postfix_expr_1() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let items = [ PostfixItem::Property(String::map_str("b")), ];
        let x = PostfixExpr {
            root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
            items: Vector::map_slice(&items),
        };
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "a.b");
    }

    #[test]
    fn display_expr_postfix() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let x = Expr::Postfix(PostfixExpr {
            root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
            items: Vector::map_slice(&[]),
        });
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "a");
    }

    #[test]
    fn display_expr_list_0() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let items = [];
        let x = ExprList { items: Vector::map_slice(&items), };
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn display_expr_list_1() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let items = [
            Expr::Postfix(PostfixExpr {
                root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
                items: Vector::map_slice(&[]),
            }),
        ];
        let x = ExprList { items: Vector::map_slice(&items), };
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "a");
    }

    #[test]
    fn display_expr_list_2() {
        let mut buffer = [0_u8; 256];
        let a = SingleAlloc::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        let items = [
            Expr::Postfix(PostfixExpr {
                root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
                items: Vector::map_slice(&[]),
            }),
            Expr::Postfix(PostfixExpr {
                root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("b"))),
                items: Vector::map_slice(&[]),
            }),
        ];
        let x = ExprList { items: Vector::map_slice(&items), };
        write!(s, "{}", x).unwrap();
        assert_eq!(s.as_str(), "a, b");
    }

    #[test]
    fn unwrap_expr_list_items() {
        let items = [
            Expr::Postfix(PostfixExpr {
                root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("a"))),
                items: Vector::map_slice(&[]),
            }),
            Expr::Postfix(PostfixExpr {
                root: PostfixRoot::Primary(PrimaryExpr::Identifier(String::map_str("b"))),
                items: Vector::map_slice(&[]),
            }),
        ];
        let x = ExprList { items: Vector::map_slice(&items), };
        let v = x.unwrap_items();
        assert_eq!(v.len(), 2);
    }
}



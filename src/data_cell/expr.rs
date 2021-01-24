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
type ParseError<'a> = Error<'a, ParseErrorData>;

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
    //QuestionMark,
    //Colon,
}

#[derive(Debug, PartialEq)]
pub enum PrimaryExpr<'a> {
    Identifier(String<'a>),
}

pub enum PostfixRoot<'a> {
    Primary(PrimaryExpr<'a>), // points to foo in foo.bar
    // Implied... for expressions like .bla (points to the empty space before .)
}

pub enum PostfixItem<'a> {
    Attr(String<'a>), // points to bar or baz in foo.bar.baz
    // Subscript(ExprList<'a>), // a[b, c]
    // Call(ExprList<'a>), // a(b, c)
}

pub struct PostfixExpr<'a> {
    root: PostfixRoot<'a>,
    items: Vector<'a, PostfixItem<'a>>,
}

pub struct Parser<'s> {
    source: &'s Source<'s>,
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
}

impl<'s> Parser<'s> {
    pub fn new(src: &'s Source<'s>) -> Self {
        Parser {
            source: src,
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
    pub fn peek_char<'x>(
        &mut self,
        _xc: &mut ExecutionContext<'x>
    ) -> Result<CharInfo, ParseError<'x>> {
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

    fn parse_identifier<'a, 'x>(
        &'a mut self,
        xc: &mut ExecutionContext<'x>
    ) -> Result<Token<'a, BasicTokenData<'x>>, ParseError<'x>> {
        let mut id = xc.string();
        let mut source_slice = self.here();
        while let Ok(ci) = self.peek_char(xc) {
            if !Parser::is_valid_identifier_char(ci.codepoint) {
                break;
            }
            id.push(ci.codepoint)?;
            self.consume_char(ci);
        }
        self.end_slice_here(&mut source_slice);
        Ok(Token {
            data: BasicTokenData::Identifier(id),
            source_slice: source_slice
        })
    }

    pub fn parse_basic_token<'a, 'x>(
        &'a mut self,
        xc: &mut ExecutionContext<'x>
    ) -> Result<Token<'a, BasicTokenData<'x>>, ParseError<'x>> {
        self.skip_whitespace();
        if self.remaining_text.is_empty() {
            return Ok(Token {
                data: BasicTokenData::End,
                source_slice: self.here()
            })
        }
        let c = self.peek_char(xc)?;
        if Parser::can_start_identifier(c.codepoint) {
            return self.parse_identifier(xc);
        }
        let mut ss = self.here();
        let td = match c.codepoint {
            '.' => {
                self.consume_char(c);
                BasicTokenData::Dot
            },
            _ => {
                let cp = c.codepoint;
                self.consume_char(c);
                return Err(xc_err!(xc, ParseErrorData::UnexpectedChar(cp), "unexpected char", "unexpected char {:?} at {}:{}", cp, ss.start_line, ss.start_column));
            }
        };
        self.end_slice_here(&mut ss);
        Ok(Token {
            data: td,
            source_slice: ss,
        })
    }

    pub fn parse_primary_expr<'a, 'x>(
        &'a mut self,
        xc: &mut ExecutionContext<'x>
    ) -> Result<Token<'a, PrimaryExpr<'x>>, ParseError<'x>> {
        let t = self.parse_basic_token(xc)?;
        if let BasicTokenData::Identifier(id) = t.data {
            Ok(Token {
                data: PrimaryExpr::Identifier(id),
                source_slice: t.source_slice,
            })
        } else {
            Err(xc_err!(xc, ParseErrorData::UnexpectedToken, "identifier expected", "identifier expected at {}:{}", t.source_slice.start_line, t.source_slice.start_column))
        }
    }

    pub fn parse_postfix_expr<'a, 'x>(
        &'a mut self,
        xc: &mut ExecutionContext<'x>
    ) -> Result<Token<'a, PostfixExpr<'x>>, ParseError<'x>> {
        let pe = self.parse_primary_expr(xc)?;
        panic!("aaaaaa");
    }


}

#[cfg(test)]
mod tests {
    use super::*;

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
        let src = Source::new("\x0B", "-");
        let p = Parser::new(&src);
        let ucp = CharInfo { codepoint: '\x0B', width: 0, size: 1 };
        assert_eq!(p.peek_raw_char().unwrap(), ucp);
    }

    #[test]
    fn peek_raw_large_code_point() {
        let src = Source::new("\u{10348}", "-");
        let p = Parser::new(&src);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\u{10348}', width: 1, size: 4 });
    }

    #[test]
    fn peek_cr_lf_no_conv() {
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src);
        p.set_new_line_handling(false, false);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\r', width: 0, size: 1 });
    }

    #[test]
    fn peek_cr_lf_all_conv() {
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src);
        p.set_new_line_handling(true, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 2 });
    }
    #[test]
    fn peek_cr_lf_part_conv() {
        let src = Source::new("\r\n", "-");
        let mut p = Parser::new(&src);
        p.set_new_line_handling(false, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 1 });
    }

    #[test]
    fn skip_whitespace() {
        let src = Source::new("\r\n\n\r      a", "-");
        let mut p = Parser::new(&src);
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: 'a', width: 1, size: 1 });
    }

    #[test]
    fn skip_whitespace_to_end() {
        let src = Source::new("\r\n\n\r      ", "-");
        let mut p = Parser::new(&src);
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char(), None);
    }

    #[test]
    fn peek_char_at_end() {
        let src = Source::new("", "-");
        let mut p = Parser::new(&src);
        let mut xc = ExecutionContext::nop();
        assert_eq!(*p.peek_char(&mut xc).unwrap_err().get_data(), ParseErrorData::ReachedEnd)
    }

    #[test]
    fn peek_illegal_char() {
        let src = Source::new("\x01", "-");
        let mut p = Parser::new(&src);
        let mut xc = ExecutionContext::nop();
        assert_eq!(*p.peek_char(&mut xc).unwrap_err().get_data(), ParseErrorData::IllegalChar('\x01'))
    }

    #[test]
    fn peek_char() {
        let src = Source::new("!", "-");
        let mut p = Parser::new(&src);
        let mut xc = ExecutionContext::nop();
        assert_eq!(p.peek_char(&mut xc).unwrap(), CharInfo { codepoint: '!', width: 1, size: 1 });
    }

    #[test]
    fn consume_tab() {
        let src = Source::new("\t", "-");
        let mut p = Parser::new(&src);
        p.set_tab_handling(Some(5));
        let mut xc = ExecutionContext::nop();
        let ci = CharInfo { codepoint: '\t', width: 0, size: 1 };
        assert_eq!(p.peek_char(&mut xc).unwrap(), ci);
        p.consume_char(ci);
        assert_eq!(p.current_line, 1);
        assert_eq!(p.current_column, 6);

    }

    #[test]
    fn only_whitespaces_produce_end_token() {
        let mut xc = ExecutionContext::nop();
        let src = Source::new("       \n         ", "-");
        let mut p = Parser::new(&src);
        let t = p.parse_basic_token(&mut xc).unwrap();
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
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("  best.worst", "-");
        let mut p = Parser::new(&src);
        let t = p.parse_basic_token(&mut xc).unwrap();
        assert_eq!(t.source_slice.as_str(), "best");
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 3));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 7));
        let s = if let BasicTokenData::Identifier(x) = t.data { x } else { String::map_str("-grr-") };
        assert_eq!(s.as_str(), "best");
    }

    #[test]
    fn identifier_token_oom() {
        use crate::mm::AllocError;
        let mut xc = ExecutionContext::nop();
        let src = Source::new("  best.worst", "-");
        let mut p = Parser::new(&src);
        assert_eq!(*p.parse_basic_token(&mut xc).unwrap_err().get_data(), ParseErrorData::Alloc(AllocError::UnsupportedOperation));

    }

    #[test]
    fn dot_token() {
        let mut xc = ExecutionContext::nop();
        let src = Source::new(".a", "-");
        let mut p = Parser::new(&src);
        let t = p.parse_basic_token(&mut xc).unwrap();
        assert_eq!(t.data, BasicTokenData::Dot);
        assert_eq!(t.source_slice.as_str(), ".");
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 1));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 2));
    }

    #[test]
    fn next_token_encounters_bad_char() {
        let mut xc = ExecutionContext::nop();
        let src = Source::new("`", "-");
        let mut p = Parser::new(&src);
        assert_eq!(*p.parse_basic_token(&mut xc).unwrap_err().get_data(), ParseErrorData::UnexpectedChar('`'));
    }

    #[test]
    fn id_as_primary_expr() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo.bar", "-");
        let mut p = Parser::new(&src);
        let t = p.parse_primary_expr(&mut xc).unwrap();
        assert_eq!(t.data, PrimaryExpr::Identifier(String::map_str("foo")));
        assert_eq!((t.source_slice.start_line, t.source_slice.start_column), (1, 1));
        assert_eq!((t.source_slice.end_line, t.source_slice.end_column), (1, 4));
    }

    #[test]
    fn id_dot_chain_postfix_expr() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::LogLevel;
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        let src = Source::new("foo .bar/3", "-");
        let mut p = Parser::new(&src);
        let t = p.parse_postfix_expr(&mut xc).unwrap();
        assert_eq!(t.source_slice.as_str(), "foo.bar");
    }

}


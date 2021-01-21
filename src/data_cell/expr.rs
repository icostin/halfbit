use crate::ExecutionContext;
use crate::mm::String;
use crate::error::Error;
//use crate::mm::Vector;

/*

bool_lit := 'false' | 'true'
u64_lit := digits
bin_lit := b"bla"
str_lit := "utf8-bla"
id := letter alnum+

Primary :=
  BoolLiteral
| U64Literal
| BinLiteral
| StringLiteral
| Identifier
| OpenParen Expression

 */

#[derive(Debug, PartialEq)]
pub enum ParseErrorData {
    NotImplemented,
    ReachedEnd,
    IllegalChar(char),
}
type ParseError<'a> = Error<'a, ParseErrorData>;

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

#[derive(Copy, Clone)]
pub struct Source<'s> {
    content: &'s str,
    name: &'s str,
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

pub struct SourceExtract<'a, 's> {
    source: &'a Source<'s>,
    start_offset: usize,
    end_offset: usize,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

#[derive(Debug, PartialEq)]
pub struct CharInfo {
    codepoint: char,
    width: u8,
    size: u8,
}

pub struct Parser<'s> {
    source: Source<'s>,
    cr_lf_to_lf: bool,
    cr_to_lf: bool,
    tab_width: Option<u8>,
    remaining_text: &'s str,
    current_line: u32,
    current_column: u32,
}
impl<'s> Parser<'s> {
    pub fn new(src: Source<'s>) -> Self {
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
                CharInfo { codepoint: ch, width: 1, size: 1 }
            })
    }
    pub fn peek_char<'x>(
        &mut self,
        _xc: &mut ExecutionContext<'x>
    ) -> Result<CharInfo, ParseError<'x>> {
        self.peek_raw_char()
            .ok_or_else(|| Error::with_str(ParseErrorData::ReachedEnd, "reached end of source file"))
            .and_then(|ci| {
                let cp = ci.codepoint as u32;
                if (cp < 32 && !self.is_whitespace(ci.codepoint)) || cp >= 127 {
                    Err(Error::with_str(ParseErrorData::IllegalChar(ci.codepoint), "illegal char"))
                } else {
                    Ok(ci)
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
            '\t' => {
                if let Some(w) = self.tab_width {
                    let w = w as u32;
                    self.current_column = ((self.current_column - 1) / w + 1) * w + 1;
                }
            }
            _ => {
                self.current_column += ci.width as u32;
            }
        }
        self.remaining_text = &self.remaining_text[(ci.size as usize)..];
    }

    pub fn is_whitespace(&self, ch: char) -> bool {
        ch == ' ' || ch == '\n' || ch == '\r' || (ch == '\t' && self.tab_width.is_some())
    }
    pub fn skip_whitespace<'x>(&mut self) {
        while let Some(ci) = self.peek_raw_char() {
            if !self.is_whitespace(ci.codepoint) { break; }
            self.consume_char(ci);
        }
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
        let p = Parser::new(Source::new("\x0B", "-"));
        let ucp = CharInfo { codepoint: '\x0B', width: 0, size: 1 };
        assert_eq!(p.peek_raw_char().unwrap(), ucp);
    }

    fn peek_raw_large_code_point() {
        let p = Parser::new(Source::new("\u{10348}", "-"));
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\u{10348}', width: 0, size: 4 });
    }


    #[test]
    fn peek_cr_lf_no_conv() {
        let mut p = Parser::new(Source::new("\r\n", "-"));
        p.set_new_line_handling(false, false);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\r', width: 0, size: 1 });
    }

    #[test]
    fn peek_cr_lf_all_conv() {
        let mut p = Parser::new(Source::new("\r\n", "-"));
        p.set_new_line_handling(true, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 2 });
    }
    #[test]
    fn peek_cr_lf_part_conv() {
        let mut p = Parser::new(Source::new("\r\n", "-"));
        p.set_new_line_handling(false, true);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: '\n', width: 0, size: 1 });
    }

    #[test]
    fn skip_whitespace() {
        let mut p = Parser::new(Source::new("\r\n\n\r      a", "-"));
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char().unwrap(), CharInfo { codepoint: 'a', width: 1, size: 1 });
    }

    #[test]
    fn skip_whitespace_to_end() {
        let mut p = Parser::new(Source::new("\r\n\n\r      ", "-"));
        p.skip_whitespace();
        assert_eq!(p.current_line, 4);
        assert_eq!(p.current_column, 7);
        assert_eq!(p.peek_raw_char(), None);
    }
}


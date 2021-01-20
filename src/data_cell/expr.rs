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
pub struct SourceSlice<'s> {
    src_content: &'s str,
    src_name: &'s str,
    start_offset: usize,
    end_offset: usize,
    start_line: u32,
    start_col: u32,
}
impl<'s> SourceSlice<'s> {
    pub fn new(content: &'s str, name: &'s str) -> Self {
        SourceSlice {
            src_content: content,
            src_name: name,
            start_offset: 0,
            end_offset: content.len(),
            start_line: 1,
            start_col: 1,
        }
    }
    pub fn as_str(&self) -> &'s str {
        &self.src_content[self.start_offset..self.end_offset]
    }
    pub fn parse_token<'a>(
        &self,
        _xc: &mut ExecutionContext<'a>
    ) -> Result<Token<'s, BasicTokenData<'a>>, ParseError<'a>> {
        Ok(Token {
            data: BasicTokenData::End,
            src: *self
        })
    }
    pub fn get_start_pos(&self) -> (&'s str, u32, u32) {
        (self.src_name, self.start_line, self.start_col)
    }
}

pub struct Token<'s, T> {
    pub data: T,
    pub src: SourceSlice<'s>
}

// type PrimaryToken<'a, 's> = Token<'s, PrimaryData<'a>>;
// 
// enum PrimaryData<'a> {
//     //BoolLiteral(bool),
//     //U64Literal(u64),
//     //I64Literal(i64),
//     //StringLiteral(String<'a>),
//     //BinLiteral(Vector<'a, u8>),
//     Identifier(String<'a>),
// }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_slice_as_str() {
        let mut src = SourceSlice::new("abcde", "<>");
        src.start_offset = 1;
        src.end_offset = 4;
        assert_eq!(src.as_str(), "bcd");
    }

    #[test]
    fn empty_source_slice_yields_end_basic_token() {
        let src = SourceSlice::new("", "-empty-");
        let mut xc = ExecutionContext::nop();
        assert_eq!(src.get_start_pos(), ("-empty-", 1, 1));
        let t = src.parse_token(&mut xc).unwrap();
        assert_eq!(t.data, BasicTokenData::End);
    }
}

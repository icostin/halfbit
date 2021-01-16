use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::UpperHex;
use core::fmt::Formatter;
use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;

use crate::mm::Vector;
use crate::mm::String;

pub trait DataCellOps: Debug + Display + UpperHex {
    fn type_name(&self) -> &'static str;
    fn to_text<T: FmtWrite>(&mut self, w: &mut T) -> FmtResult {
        write!(w, "{}", self)
    }
}

#[derive(Debug)]
pub enum DataCell<'a> {
    Nothing,
    Bool(bool),
    U64(u64),
    I64(i64),
    String(String<'a>),
    Identifier(String<'a>),
    ByteVector(Vector<'a, u8>),
    CellVector(Vector<'a, Self>),
    Record(Vector<'a, Self>, &'static [&'static str]),
}

impl<'a> DataCellOps for DataCell<'a> {
    fn type_name(&self) -> &'static str {
        match self {
            DataCell::Nothing => "nothing",
            DataCell::Bool(_) => "bool",
            DataCell::U64(_) => "uint64",
            DataCell::I64(_) => "int64",
            DataCell::String(_) => "string",
            DataCell::Identifier(_) => "identifier",
            DataCell::ByteVector(_) => "byte_vector",
            DataCell::CellVector(_) => "cell_vector",
            DataCell::Record(_, _) => "record",
        }
    }
}

impl Display for DataCell<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataCell::Nothing => {
                Display::fmt("", f)
            },
            DataCell::Bool(v) => {
                let s = if *v { "true" } else { "false" };
                Display::fmt(s, f)
            },
            DataCell::U64(v) => { Display::fmt(v, f) },
            DataCell::I64(v) => { Display::fmt(v, f) },
            DataCell::String(v) => { Debug::fmt(v, f) },
            DataCell::Identifier(v) => { Display::fmt(v, f) },
            DataCell::ByteVector(v) => {
                write!(f, "b\"")?;
                for &b in v.as_slice() {
                    if b == 0x22 || b == 0x5C {
                        write!(f, "\\{}", b as char)?;
                    } else if b >= 0x20_u8 && b <= 0x7E_u8 {
                        write!(f, "{}", b as char)?;
                    } else {
                        write!(f, "\\x{:02X}", b)?;
                    }
                }
                write!(f, "\"")
            },
            DataCell::CellVector(v) => {
                write!(f, "[")?;
                Display::fmt(v, f)?;
                write!(f, "]")
            },
            DataCell::Record(values, keys) => {
                if values.is_empty() {
                    return write!(f, "{{}}")
                }
                let mut key_iter = keys.iter();
                let mut sep = "{ ";
                for v in values.as_slice().iter() {
                    let k = key_iter.next().unwrap_or(&"_");
                    write!(f, "{}{}: ", sep, k)?;
                    sep = ", ";
                    Display::fmt(v, f)?;
                }
                write!(f, " }}")
            },
        }
    }
}

impl UpperHex for DataCell<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataCell::U64(v) => UpperHex::fmt(v, f),
            DataCell::I64(v) => UpperHex::fmt(v, f),
            _ => Display::fmt(self, f)
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;
    extern crate std;
    use std::string::String as StdString;
    use super::*;
    use crate::mm::Allocator;
    use crate::mm::NOP_ALLOCATOR;

    #[test]
    fn nothing_type_name() {
        assert_eq!(DataCell::Nothing.type_name(), "nothing");
    }

    #[test]
    fn bool_type_name() {
        assert_eq!(DataCell::Bool(true).type_name(), "bool");
    }

    #[test]
    fn u64_type_name() {
        assert_eq!(DataCell::U64(1).type_name(), "uint64");
    }

    #[test]
    fn i64_type_name() {
        assert_eq!(DataCell::I64(-1).type_name(), "int64");
    }

    #[test]
    fn string_type_name() {
        assert_eq!(DataCell::String(String::map_str("bla")).type_name(), "string");
    }

    #[test]
    fn identifier_type_name() {
        assert_eq!(DataCell::Identifier(String::map_str("bla")).type_name(), "identifier");
    }

    #[test]
    fn byte_vector_type_name() {
        assert_eq!(DataCell::ByteVector(Vector::new(NOP_ALLOCATOR.to_ref())).type_name(), "byte_vector");
    }

    #[test]
    fn cell_vector_type_name() {
        assert_eq!(DataCell::CellVector(Vector::new(NOP_ALLOCATOR.to_ref())).type_name(), "cell_vector");
    }

    #[test]
    fn record_type_name() {
        assert_eq!(DataCell::Record(Vector::new(NOP_ALLOCATOR.to_ref()), &[]).type_name(), "record");
    }

    #[test]
    fn nothing_fmt() {
        let mut s = StdString::new();
        write!(s, "{:3}", DataCell::Nothing).unwrap();
        assert_eq!(s, "   ");
    }

    #[test]
    fn bool_fmt() {
        {
            let mut s = StdString::new();
            write!(s, "{:<7}", DataCell::Bool(true)).unwrap();
            assert_eq!(s, "true   ");
        }
        {
            let mut s = StdString::new();
            write!(s, "{:>7}", DataCell::Bool(false)).unwrap();
            assert_eq!(s, "  false");
        }
        {
            let mut s = StdString::new();
            write!(s, "{:X}", DataCell::Bool(false)).unwrap();
            assert_eq!(s, "false");
        }
    }

    #[test]
    fn u64_hex_fmt() {
        let mut s = StdString::new();
        write!(s, "{:<20X}", DataCell::U64(0xABCD_1234_EF01_5678)).unwrap();
        assert_eq!(s, "ABCD1234EF015678    ");
    }

    #[test]
    fn u64_fmt() {
        let mut s = StdString::new();
        write!(s, "{:>5}", DataCell::U64(123)).unwrap();
        assert_eq!(s, "  123");
    }

    #[test]
    fn i64_fmt() {
        let mut s = StdString::new();
        write!(s, "{:>5}", DataCell::I64(-123)).unwrap();
        assert_eq!(s, " -123");

    }

    #[test]
    fn i64_hex_fmt() {
        let mut s = StdString::new();
        write!(s, "{:<5X}", DataCell::I64(0xABC)).unwrap();
        assert_eq!(s, "ABC  ");
    }

    #[test]
    fn string_fmt() {
        let mut s = StdString::new();
        write!(s, "{}", DataCell::String(String::map_str("abc\tdef\n"))).unwrap();
        assert_eq!(s, "\"abc\\tdef\\n\"");
    }

    #[test]
    fn identifier_fmt() {
        let mut s = StdString::new();
        write!(s, "{}", DataCell::Identifier(String::map_str("abc-def"))).unwrap();
        assert_eq!(s, "abc-def");
    }

    #[test]
    fn byte_vector_fmt() {
        let mut s = StdString::new();
        write!(s, "{}", DataCell::ByteVector(Vector::map_slice(b"abc-def\x00\x01\xFF\\\"."))).unwrap();
        assert_eq!(s, "b\"abc-def\\x00\\x01\\xFF\\\\\\\".\"");
        //assert_eq!(s, "[97, 98, 99, 45, 100, 101, 102, 0, 1, 255, 46]");
    }

    #[test]
    fn cell_vector_fmt() {
        let cells = [
            DataCell::Nothing,
            DataCell::Bool(true),
            DataCell::U64(0x123),
            DataCell::I64(-111),
            DataCell::String(String::map_str("hello")),
            DataCell::Identifier(String::map_str("body")),
            DataCell::ByteVector(Vector::map_slice(b"bin")),
            DataCell::CellVector(Vector::new(NOP_ALLOCATOR.to_ref())),
        ];
        let v = DataCell::CellVector(Vector::map_slice(&cells));
        let mut s = StdString::new();
        write!(s, "{}", v).unwrap();
        assert_eq!(s, "[, true, 291, -111, \"hello\", body, b\"bin\", []]");
    }

    #[test]
    fn empty_record_fmt() {
        let r = DataCell::Record(Vector::new(NOP_ALLOCATOR.to_ref()), &[]);
        let mut s = StdString::new();
        write!(s, "{}", r).unwrap();
        assert_eq!(s, "{}");
    }

    #[test]
    fn record_fmt() {
        let values = [
            DataCell::Nothing,
            DataCell::Bool(true),
            DataCell::U64(0x123),
            DataCell::I64(-111),
            DataCell::String(String::map_str("hello")),
            DataCell::Identifier(String::map_str("body")),
            DataCell::ByteVector(Vector::map_slice(b"bin")),
            DataCell::CellVector(Vector::new(NOP_ALLOCATOR.to_ref())),
        ];
        const KEYS: &'static [&'static str] = &[
            "bumper",
            "is_absurd",
            "absurdity_level",
            "highest_score",
            "end_greeting",
            "tag",
            "raw_data",
            //"shopping_list",
        ];
        let mut v = DataCell::Record(Vector::map_slice(&values), &KEYS);
        let mut s = StdString::new();
        v.to_text(&mut s).unwrap();
        assert_eq!(s, "{ bumper: , is_absurd: true, absurdity_level: 291, highest_score: -111, end_greeting: \"hello\", tag: body, raw_data: b\"bin\", _: [] }");
    }
}


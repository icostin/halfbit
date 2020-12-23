use core::fmt::Display;
use core::fmt::UpperHex;
use core::fmt::Formatter;

#[derive(Debug)]
pub enum DataCell {
    Nothing,
    Bool(bool),
    U64(u64),
    I64(i64),
}

impl DataCell {
    pub fn type_name(&self) -> &'static str {
        match self {
            DataCell::Nothing => "nothing",
            DataCell::Bool(_) => "bool",
            DataCell::U64(_) => "uint64",
            DataCell::I64(_) => "int64",
        }
    }
}

impl Display for DataCell {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataCell::Nothing => {
                Display::fmt("", f)
            },
            DataCell::Bool(v) => {
                let s = if *v { "true" } else { "false" };
                Display::fmt(s, f)
            },
            DataCell::U64(v) => {
                Display::fmt(v, f)
            },
            DataCell::I64(v) => {
                Display::fmt(v, f)
            },
        }
    }
}

impl UpperHex for DataCell {
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

}


extern crate num;

pub fn is_power_of_2<T> (n: T) -> bool
    where T: num::traits::Unsigned + num::traits::int::PrimInt {
    let zero: T = num::zero();
    let one: T = num::One::one();
    n != zero && (n & (n - one)) == zero
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_power_of_2_checks () {
        assert!(!is_power_of_2(0u32));
        assert!(is_power_of_2(1u8));
        assert!(is_power_of_2(2u16));
        assert!(!is_power_of_2(3u64));
        assert!(is_power_of_2(4usize));
    }
}

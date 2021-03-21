use crate::util::BiggerNum;
use std::{
    convert::TryInto,
    fmt::{Debug, Display},
    ops::{Add, Deref, Div, Mul, Sub},
};

#[repr(transparent)]
#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct Percent {
    value: u8,
}

impl Percent {
    pub fn from_value_in_range<N: BiggerNum>(start: N, stop: N, point: N) -> Percent
    where
        N::Next: Copy
            + PartialOrd
            + Display
            + From<u8>
            + From<N>
            + TryInto<u8, Error: std::fmt::Debug>
            + Add<Output = N::Next>
            + Sub<Output = N::Next>
            + Mul<Output = N::Next>
            + Div<Output = N::Next>,
    {
        let start: N::Next = N::Next::from(start);
        let stop: N::Next = N::Next::from(stop);
        let point: N::Next = N::Next::from(point);

        if start > point || stop < point {
            panic!(
                "Number out of bounds. Expression {} < {} < {} is incorrect.",
                start, point, stop
            );
        }

        let hundred: N::Next = N::Next::from(100);
        let length: N::Next = stop - start;
        let percent: N::Next = (hundred * (point - start)) / length;

        Percent {
            value: percent.try_into().unwrap(),
        }
    }

    pub fn point_at_range<N: BiggerNum>(self, start: N, stop: N) -> N
    where
        N::Next: Copy
            + From<u8>
            + From<N>
            + TryInto<N, Error: std::fmt::Debug>
            + Add<Output = N::Next>
            + Sub<Output = N::Next>
            + Mul<Output = N::Next>
            + Div<Output = N::Next>,
    {
        // FIXME start < stop.
        // FIXME ^^^^^^^^^^^^ start < stop or start <= stop ?
        let value = N::Next::from(self.value);
        let hundred = N::Next::from(100u8);
        let stop = N::Next::from(stop);
        let start = N::Next::from(start);

        ((value * (stop - start)) / hundred + start)
            .try_into()
            .unwrap()
    }
}

impl Deref for Percent {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.value)
    }
}

impl Debug for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}%", self.value)
    }
}

macro_rules! from_impl {
    ($($t:ty)*) => {
	$(
	    impl From<$t> for Percent {
		#[allow(unused_comparisons)]
		fn from(value: $t) -> Self {
		    if value > 100 {
			panic!("Value out of a percent value: {}", value);
		    }

		    Percent {
			value: value as u8
		    }
		}
	    }
	)*
    };
}

// Accept only unsigned values.
from_impl!(u8 u16 u32 u64 u128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_value_in_range_success_range_0_to_100() {
        assert_eq!(
            Percent::from(0u8),
            Percent::from_value_in_range(0i32, 100i32, 0i32)
        );
        assert_eq!(
            Percent::from(100u8),
            Percent::from_value_in_range(0i32, 100i32, 100i32)
        );
        assert_eq!(
            Percent::from(50u8),
            Percent::from_value_in_range(0i32, 100i32, 50i32)
        );
    }

    #[test]
    fn from_value_in_range_success_range_50_to_150() {
        assert_eq!(
            Percent::from(0u8),
            Percent::from_value_in_range(50i32, 150i32, 50i32)
        );
        assert_eq!(
            Percent::from(100u8),
            Percent::from_value_in_range(50i32, 150i32, 150i32)
        );
        assert_eq!(
            Percent::from(50u8),
            Percent::from_value_in_range(50i32, 150i32, 100i32)
        );
    }

    #[test]
    fn point_at_range_success_range_0_to_100() {
        assert_eq!(0, Percent::from(0u8).point_at_range(0i32, 100i32));
        assert_eq!(100, Percent::from(100u8).point_at_range(0i32, 100i32));
        assert_eq!(50, Percent::from(50u8).point_at_range(0i32, 100i32));
    }

    #[test]
    fn point_at_range_success_range_50_to_150() {
        assert_eq!(50, Percent::from(0u8).point_at_range(50i32, 150i32));
        assert_eq!(150, Percent::from(100u8).point_at_range(50i32, 150i32));
        assert_eq!(100, Percent::from(50u8).point_at_range(50i32, 150i32));
    }

    #[test]
    #[should_panic]
    fn from_kaboom_value_greater_100() {
        Percent::from(101u8);
    }
}

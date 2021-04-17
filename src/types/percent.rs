use crate::util::BiggerNum;
use bounded_nums::{AsBoundedU8, BoundariesError, BoundedU8};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
    ops::{Add, Deref, Div, Mul, Sub},
};

type InnerPercent = BoundedU8<0, 100>;

#[repr(transparent)]
#[derive(PartialEq, PartialOrd, Eq, Ord, Clone, Copy)]
pub struct Percent {
    value: InnerPercent,
}

impl Percent {
    pub fn from_bounded<N: AsBoundedU8<0, 100>>(value: N) -> Percent {
        Percent {
            value: value.as_bounded_u8(),
        }
    }

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

        // Unwrapping this is safe as long as start <= point <= stop is true.
        Percent {
            value: InnerPercent::from_u8(percent.try_into().unwrap()).unwrap(),
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
        let value = N::Next::from(self.value.value());
        let hundred = N::Next::from(100u8);
        let stop = N::Next::from(stop);
        let start = N::Next::from(start);

        ((value * (stop - start)) / hundred + start)
            .try_into()
            .unwrap()
    }
}

impl Deref for Percent {
    type Target = InnerPercent;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.value.value())
    }
}

impl Debug for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

macro_rules! try_from_impl {
    ($($t:ty)*) => {
	$(
	    impl TryFrom<$t> for Percent {
		type Error = BoundariesError;
		#[allow(unused_comparisons)]
		fn try_from(value: $t) -> Result<Self, Self::Error> {
		    u8::try_from(value)
			.map_err(|_| {
			    if value < 0 {
				BoundariesError::Underflow
			    } else {
				BoundariesError::Overflow
			    }
			})
			.and_then(|value| BoundedU8::<0, 100>::from_u8(value))
			.map(Percent::from_bounded)
		}
	    }
	)*
    };
}

try_from_impl!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_value_in_range_success_range_0_to_100() {
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<0>()),
            Percent::from_value_in_range(0i32, 100i32, 0i32)
        );
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<100>()),
            Percent::from_value_in_range(0i32, 100i32, 100i32)
        );
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<50>()),
            Percent::from_value_in_range(0i32, 100i32, 50i32)
        );
    }

    #[test]
    fn from_value_in_range_success_range_50_to_150() {
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<0>()),
            Percent::from_value_in_range(50i32, 150i32, 50i32)
        );
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<100>()),
            Percent::from_value_in_range(50i32, 150i32, 150i32)
        );
        assert_eq!(
            Percent::from_bounded(InnerPercent::from_const::<50>()),
            Percent::from_value_in_range(50i32, 150i32, 100i32)
        );
    }

    #[test]
    fn point_at_range_success_range_0_to_100() {
        assert_eq!(
            0,
            Percent::from_bounded(InnerPercent::from_const::<0>()).point_at_range(0i32, 100i32)
        );
        assert_eq!(
            100,
            Percent::from_bounded(InnerPercent::from_const::<100>()).point_at_range(0i32, 100i32)
        );
        assert_eq!(
            50,
            Percent::from_bounded(InnerPercent::from_const::<50>()).point_at_range(0i32, 100i32)
        );
    }

    #[test]
    fn point_at_range_success_range_50_to_150() {
        assert_eq!(
            50,
            Percent::from_bounded(InnerPercent::from_const::<0>()).point_at_range(50i32, 150i32)
        );
        assert_eq!(
            150,
            Percent::from_bounded(InnerPercent::from_const::<100>()).point_at_range(50i32, 150i32)
        );
        assert_eq!(
            100,
            Percent::from_bounded(InnerPercent::from_const::<50>()).point_at_range(50i32, 150i32)
        );
    }
}

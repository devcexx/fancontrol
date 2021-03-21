use crate::util::BiggerNum;
use std::{
    convert::TryInto,
    fmt::{Debug, Display},
    ops::{Add, Deref, Div, Mul, Sub},
};

#[repr(transparent)]
pub struct Percent {
    value: u8,
}

impl Percent {
    pub fn map_to_range<N: BiggerNum>(self, start: N, stop: N) -> N
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
        let value = N::Next::from(self.value);
        let one = N::Next::from(1u8);
        let hundred = N::Next::from(100u8);
        let stop = N::Next::from(stop);
        let start = N::Next::from(start);

        ((value * (stop - start + one)) / hundred + start)
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
		    if value < 0 || value > 100 {
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

from_impl!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);

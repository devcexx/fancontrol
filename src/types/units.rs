pub trait MeasureUnit {
    type Holder;
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Measure<U: MeasureUnit> {
    value: U::Holder,
}

impl<U: MeasureUnit> Measure<U> {
    pub fn new(value: U::Holder) -> Measure<U> {
        Self { value }
    }

    #[inline(always)]
    pub fn raw_value(self) -> U::Holder {
        self.value
    }
}

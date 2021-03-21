pub trait BiggerNum: Sized {
    type Next: From<Self>;
}

macro_rules! biggernum_impl {
    ($($t:ty => $next:ty),*) => {
	$(
	    impl BiggerNum for $t {
		type Next = $next;
	    }
	)*
    };
}

biggernum_impl!(
    u8   => u16,
    u16  => u32,
    u32  => u64,
    u64  => u128,
    u128 => u128,
    
    i8   => i16,
    i16  => i32,
    i32  => i64,
    i64  => i128,
    
    f32  => f64,
    f64  => f64);

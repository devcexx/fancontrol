#[macro_export]
macro_rules! driver_log_define {
    ($driver:literal, $prefix:ident) => {
        targeted_log::targeted_log!(std::concat!("[drivers::", $driver, " {}"), $prefix);
    };
}

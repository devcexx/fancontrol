// TODO Improve this (see card).

#[macro_export]
macro_rules! dev_debug {
    ($self:ident, $arg0:tt, $($arg:tt)*) => {
	log::debug!(std::concat!("[device/{}] ", $arg0), $self.name, $($arg)*)
    };
}

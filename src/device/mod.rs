use std::ffi::OsStr;
use udev::Device as UdevDevice;

mod dev;
pub mod drivers;
mod registry;

pub use dev::*;
pub use registry::driver_registry_find;

pub fn udev_find_with_tags<T: AsRef<OsStr>>(tags: Vec<T>) -> Option<UdevDevice> {
    let mut enumerator = udev::Enumerator::new().unwrap();
    tags.iter().for_each(|x| {
        enumerator.match_tag(x).unwrap();
    });

    let mut devices = enumerator.scan_devices().unwrap();

    devices.next()
}

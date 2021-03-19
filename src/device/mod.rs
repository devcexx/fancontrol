use std::ffi::OsStr;
use udev::Device as UdevDevice;

mod dev;
mod hwmon;
mod nct6775;

pub use dev::*;
pub use hwmon::*;
pub use nct6775::*;

pub fn udev_find_with_tags<T: AsRef<OsStr>>(tags: Vec<T>) -> Option<UdevDevice> {
    let mut enumerator = udev::Enumerator::new().unwrap();
    tags.iter().for_each(|x| {
        enumerator.match_tag(x).unwrap();
    });

    let mut devices = enumerator.scan_devices().unwrap();

    devices.next()
}

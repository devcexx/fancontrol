use std::ffi::OsStr;
use udev::Device as UdevDevice;

pub fn udev_find_with_tags<T: AsRef<OsStr>>(tags: Vec<T>) -> Option<UdevDevice> {
    let mut enumerator = udev::Enumerator::new().unwrap();
    tags.iter().for_each(|x| {
        // TODO Propagate error!
        enumerator.match_tag(x).unwrap();
    });

    let mut devices = enumerator.scan_devices().unwrap();

    devices.next()
}

pub fn udev_extract_tags(device: &UdevDevice) -> Option<Vec<&str>> {
    device
        .property_value("TAGS")
        .and_then(|tags| tags.to_str())
        .map(|tags| tags.split(':').filter(|&tag| !tag.is_empty()).collect())
}

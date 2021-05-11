use libc::{c_int, poll, pollfd, POLLIN};
use std::convert::TryInto;
use std::io::Result;
use std::os::unix::io::AsRawFd;
use std::{io::Error, time::Duration};
use udev::{Event, MonitorSocket};

#[derive(Copy, Clone)]
pub enum PollMode {
    NoWait,
    WaitInfinite,
    WaitTimeout(Duration),
}

impl PollMode {
    fn raw_timeout_millis(self) -> c_int {
        match self {
            PollMode::NoWait => 0,
            PollMode::WaitInfinite => -1,
            PollMode::WaitTimeout(duration) => duration.as_millis().try_into().unwrap(),
        }
    }
}

pub struct UdevPoller {
    monitor: MonitorSocket,
    pollfd: pollfd,
}

impl UdevPoller {
    pub fn poll_on(monitor: MonitorSocket) -> UdevPoller {
        let raw_fd = monitor.as_raw_fd();

        UdevPoller {
            monitor,
            pollfd: pollfd {
                fd: raw_fd,
                events: POLLIN,
                revents: 0,
            },
        }
    }

    fn do_poll(&mut self, raw_timeout: c_int) -> Result<Vec<Event>> {
        let errno = unsafe { poll((&mut self.pollfd) as *mut pollfd, 1, raw_timeout) };
        if errno < 0 {
            Err(Error::last_os_error())
        } else {
            let mut events = Vec::new();
            while let Some(event) = self.monitor.next() {
                events.push(event)
            }
            Ok(events)
        }
    }

    pub fn poll_events(&mut self, mode: PollMode) -> Result<Vec<Event>> {
        self.do_poll(mode.raw_timeout_millis())
    }
}

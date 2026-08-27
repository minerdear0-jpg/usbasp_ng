//! USB open helper for composite HIDUART diag EP2.

use anyhow::{bail, Context, Result};
use rusb::{Direction, GlobalContext, TransferType};

use crate::protocol::{IF_MONITOR, PID, VID};

pub struct CompositeHandle {
    pub handle: rusb::DeviceHandle<GlobalContext>,
    pub serial: String,
    /// Linux: `/dev/bus/usb/BBB/DDD`. Other OS: `usb:bus:addr`.
    pub path: String,
}

pub fn linux_usb_path(bus: u8, addr: u8) -> String {
    if cfg!(target_os = "linux") {
        format!("/dev/bus/usb/{bus:03}/{addr:03}")
    } else {
        format!("usb:{bus}:{addr}")
    }
}

pub fn open_composite(want_serial: &str) -> Result<CompositeHandle> {
    match try_open_composite(want_serial)? {
        Some(h) => Ok(h),
        None => bail!("no composite USBasp 16c0:05dc (diag) found"),
    }
}

/// `Ok(None)` = stick not on the bus (watch may wait). Claim/open errors propagate.
pub fn try_open_composite(want_serial: &str) -> Result<Option<CompositeHandle>> {
    for dev in rusb::devices().context("list USB")?.iter() {
        let desc = match dev.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        if desc.vendor_id() != VID || desc.product_id() != PID {
            continue;
        }
        let ver = desc.device_version();
        if ver.major() == 2 && ver.minor() == 3 {
            continue;
        }
        let path = linux_usb_path(dev.bus_number(), dev.address());
        let Ok(handle) = dev.open() else {
            continue;
        };
        let ser = handle
            .read_serial_number_string_ascii(&desc)
            .unwrap_or_default();
        if !want_serial.is_empty() && ser != want_serial {
            continue;
        }
        let _ = handle.detach_kernel_driver(IF_MONITOR);
        handle
            .claim_interface(IF_MONITOR)
            .with_context(|| format!("claim IF2 at {path}"))?;
        let config = dev.active_config_descriptor().context("config")?;
        let mut found = false;
        for iface in config.interfaces() {
            for id in iface.descriptors() {
                if id.interface_number() != IF_MONITOR {
                    continue;
                }
                for ep in id.endpoint_descriptors() {
                    if ep.address() == crate::protocol::EP2_IN
                        && ep.transfer_type() == TransferType::Interrupt
                        && ep.direction() == Direction::In
                    {
                        found = true;
                    }
                }
            }
        }
        if !found {
            bail!("no interrupt IN 0x82 on IF2 ({path})");
        }
        return Ok(Some(CompositeHandle {
            handle,
            serial: ser,
            path,
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::linux_usb_path;

    #[test]
    fn linux_bus_addr_path() {
        let p = linux_usb_path(1, 12);
        if cfg!(target_os = "linux") {
            assert_eq!(p, "/dev/bus/usb/001/012");
        } else {
            assert_eq!(p, "usb:1:12");
        }
    }
}

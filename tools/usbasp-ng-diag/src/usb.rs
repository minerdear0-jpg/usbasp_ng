//! USB open helper for composite HIDUART diag EP2.

use anyhow::{bail, Context, Result};
use rusb::{Direction, GlobalContext, TransferType};

use crate::protocol::{IF_MONITOR, PID, VID};

pub struct CompositeHandle {
    pub handle: rusb::DeviceHandle<GlobalContext>,
    pub serial: String,
}

pub fn open_composite(want_serial: &str) -> Result<CompositeHandle> {
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
            .context("claim IF2")?;
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
            bail!("no interrupt IN 0x82 on IF2");
        }
        return Ok(CompositeHandle {
            handle,
            serial: ser,
        });
    }
    bail!("no composite USBasp 16c0:05dc (diag) found");
}

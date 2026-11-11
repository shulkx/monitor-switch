//! Guess which input of a monitor this computer is plugged into (spec 5.3).

use crate::ddc::caps::{port_class, PortClass, USB_C};
use crate::model::ConnectionKind;

/// `inputs` is what the monitor lists for VCP 0x60 (empty when unknown).
/// A user override is applied by the caller, not here.
pub fn guess_local_input(connection: ConnectionKind, is_laptop: bool, inputs: &[u16]) -> Option<u16> {
    let lowest = |class: PortClass| inputs.iter().copied().filter(|&v| port_class(v) == class).min();
    let has_usb_c = inputs.contains(&USB_C);
    let unknown = inputs.is_empty();
    match connection {
        ConnectionKind::Hdmi => lowest(PortClass::Hdmi).or(if unknown { Some(0x11) } else { None }),
        ConnectionKind::DisplayPort => {
            if is_laptop && has_usb_c {
                return Some(USB_C);
            }
            if let Some(dp) = lowest(PortClass::Dp) {
                return Some(dp);
            }
            if has_usb_c {
                return Some(USB_C);
            }
            if unknown {
                Some(if is_laptop { USB_C } else { 0x0F })
            } else {
                None
            }
        }
        ConnectionKind::UsbTunnel => {
            if has_usb_c || unknown {
                Some(USB_C)
            } else {
                lowest(PortClass::Dp)
            }
        }
        ConnectionKind::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConnectionKind::*;

    const DELL: [u16; 3] = [0x0F, 0x11, 0x1B];

    #[test]
    fn desktop_on_displayport_uses_dp() {
        assert_eq!(guess_local_input(DisplayPort, false, &DELL), Some(0x0F));
    }

    #[test]
    fn laptop_on_displayport_uses_usb_c_when_the_monitor_has_it() {
        assert_eq!(guess_local_input(DisplayPort, true, &DELL), Some(0x1B));
    }

    #[test]
    fn laptop_on_displayport_without_usb_c_uses_dp() {
        assert_eq!(guess_local_input(DisplayPort, true, &[0x0F, 0x11]), Some(0x0F));
    }

    #[test]
    fn hdmi_uses_the_lowest_hdmi_value() {
        assert_eq!(guess_local_input(Hdmi, false, &[0x12, 0x0F, 0x11]), Some(0x11));
    }

    #[test]
    fn hdmi_without_hdmi_inputs_gives_none() {
        assert_eq!(guess_local_input(Hdmi, false, &[0x0F]), None);
    }

    #[test]
    fn usb_tunnel_prefers_usb_c() {
        assert_eq!(guess_local_input(UsbTunnel, false, &DELL), Some(0x1B));
    }

    #[test]
    fn unknown_connection_gives_none() {
        assert_eq!(guess_local_input(Other, false, &DELL), None);
    }

    #[test]
    fn without_capabilities_falls_back_to_standard_values() {
        assert_eq!(guess_local_input(DisplayPort, false, &[]), Some(0x0F));
        assert_eq!(guess_local_input(DisplayPort, true, &[]), Some(USB_C));
        assert_eq!(guess_local_input(Hdmi, false, &[]), Some(0x11));
    }
}

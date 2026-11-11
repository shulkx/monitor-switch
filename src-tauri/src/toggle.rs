//! Which input the "toggle" hotkey switches to (spec 5.2).

use crate::ddc::caps::USB_C;

/// If the monitor is on the first input of the pair, go to the second; otherwise go to the first.
pub fn toggle_target(current: Option<u16>, pair: [u16; 2]) -> u16 {
    if current == Some(pair[0]) {
        pair[1]
    } else {
        pair[0]
    }
}

/// The pair used when the user has not chosen one: this computer's input plus the first other
/// input that has a name, else USB-C when the monitor has it (the usual laptop port).
/// Never any other input: switching to an empty port blanks the screen, and a monitor that
/// then goes to sleep may only come back with its physical buttons.
pub fn default_pair(local: Option<u16>, named: &[u16], inputs: &[u16]) -> Option<[u16; 2]> {
    let local = local?;
    let other = named
        .iter()
        .copied()
        .find(|&v| v != local)
        .or_else(|| (local != USB_C && inputs.contains(&USB_C)).then_some(USB_C))?;
    Some([local, other])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_between_the_two_inputs() {
        assert_eq!(toggle_target(Some(0x0F), [0x0F, 0x1B]), 0x1B);
        assert_eq!(toggle_target(Some(0x1B), [0x0F, 0x1B]), 0x0F);
    }

    #[test]
    fn from_any_other_input_goes_to_the_first() {
        assert_eq!(toggle_target(Some(0x11), [0x0F, 0x1B]), 0x0F);
        assert_eq!(toggle_target(None, [0x0F, 0x1B]), 0x0F);
    }

    #[test]
    fn default_pair_prefers_a_named_input() {
        assert_eq!(default_pair(Some(0x0F), &[0x0F, 0x11], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x11]));
    }

    #[test]
    fn default_pair_falls_back_to_usb_c() {
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x1B]));
    }

    #[test]
    fn default_pair_never_guesses_another_input() {
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F, 0x11]), None);
        // On the laptop itself (local = USB-C) nothing is guessed either.
        assert_eq!(default_pair(Some(0x1B), &[], &[0x0F, 0x11, 0x1B]), None);
    }

    #[test]
    fn no_default_pair_without_a_local_input() {
        assert_eq!(default_pair(None, &[0x1B], &[0x0F, 0x1B]), None);
    }
}

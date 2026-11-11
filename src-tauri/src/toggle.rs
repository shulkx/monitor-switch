//! Which input the "toggle" hotkey switches to (spec 5.2).

/// If the monitor is on the first input of the pair, go to the second; otherwise go to the first.
pub fn toggle_target(current: Option<u16>, pair: [u16; 2]) -> u16 {
    if current == Some(pair[0]) {
        pair[1]
    } else {
        pair[0]
    }
}

/// The pair used when the user has not chosen one: this computer's input plus the
/// first other input that has a name, else the first other input the monitor lists.
pub fn default_pair(local: Option<u16>, named: &[u16], inputs: &[u16]) -> Option<[u16; 2]> {
    let local = local?;
    let other = named
        .iter()
        .copied()
        .find(|&v| v != local)
        .or_else(|| inputs.iter().copied().find(|&v| v != local))?;
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
        assert_eq!(default_pair(Some(0x0F), &[0x0F, 0x1B], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x1B]));
    }

    #[test]
    fn default_pair_falls_back_to_listed_inputs() {
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F, 0x11, 0x1B]), Some([0x0F, 0x11]));
    }

    #[test]
    fn no_default_pair_without_a_local_input_or_another_input() {
        assert_eq!(default_pair(None, &[0x1B], &[0x0F, 0x1B]), None);
        assert_eq!(default_pair(Some(0x0F), &[], &[0x0F]), None);
    }
}

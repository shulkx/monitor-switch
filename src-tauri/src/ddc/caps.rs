//! Parse the DDC/CI capabilities string a monitor reports, and name input values.

/// VCP feature code for "Input Select".
pub const VCP_INPUT_SELECT: u8 = 0x60;
/// Input value most vendors (Dell included) use for USB-C. Not part of MCCS.
pub const USB_C: u16 = 0x1B;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub model: Option<String>,
    /// Values the monitor lists for VCP 0x60, in the order it lists them.
    pub inputs: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortClass {
    Dp,
    Hdmi,
    UsbC,
    Other,
}

pub fn parse(raw: &str) -> Capabilities {
    let model = section(raw, "model")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let inputs = section(raw, "vcp")
        .map(|vcp| feature_values(vcp, "60"))
        .unwrap_or_default();
    Capabilities { model, inputs }
}

/// Short name shown in the UI for a VCP 0x60 value.
pub fn input_label(value: u16) -> String {
    match value {
        0x01 => "VGA".into(),
        0x02 => "VGA 2".into(),
        0x03 => "DVI".into(),
        0x04 => "DVI 2".into(),
        0x0F => "DP".into(),
        0x10 => "DP 2".into(),
        0x11 => "HDMI".into(),
        0x12 => "HDMI 2".into(),
        USB_C => "USB-C".into(),
        other => format!("0x{other:02X}"),
    }
}

pub fn port_class(value: u16) -> PortClass {
    match value {
        0x0F | 0x10 => PortClass::Dp,
        0x11 | 0x12 => PortClass::Hdmi,
        USB_C => PortClass::UsbC,
        _ => PortClass::Other,
    }
}

/// Text inside `name( ... )`, matching parentheses. `name` must be a whole word,
/// so "vcp" does not match "vcpname".
fn section<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    let lower = raw.to_ascii_lowercase();
    let needle = format!("{name}(");
    let mut from = 0;
    while let Some(pos) = lower[from..].find(&needle) {
        let at = from + pos;
        let starts_word = at == 0 || {
            let before = lower.as_bytes()[at - 1];
            !before.is_ascii_alphanumeric() && before != b'_'
        };
        if starts_word {
            return balanced(raw, at + name.len()).map(|(inner, _)| inner);
        }
        from = at + needle.len();
    }
    None
}

/// `open` is the index of a '('. Returns the text up to its matching ')' and the index after it.
fn balanced(s: &str, open: usize) -> Option<(&str, usize)> {
    let mut depth = 0usize;
    for (i, &b) in s.as_bytes().iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[open + 1..i], i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Inside a `vcp(...)` section, the hex values listed for feature `code`.
fn feature_values(vcp: &str, code: &str) -> Vec<u16> {
    let bytes = vcp.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_hexdigit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        let token = &vcp[start..i];
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'(' {
            let Some((inner, end)) = balanced(vcp, j) else {
                return Vec::new();
            };
            if token.eq_ignore_ascii_case(code) {
                return inner
                    .split_whitespace()
                    .filter_map(|t| u16::from_str_radix(t, 16).ok())
                    .collect();
            }
            i = end;
        }
    }
    Vec::new()
}

/// Capabilities string in the format Dell monitors report.
#[cfg(test)]
pub(crate) const DELL_STYLE_CAPS: &str = "(prot(monitor)type(LCD)model(U2723QE)cmds(01 02 03 07 0C E3 F3)vcp(02 04 05 08 10 12 14(01 04 05 06 08 09 0B 0C) 16 18 1A 52 60(0F 11 1B) AA(01 02 04) AC AE B2 B6 C6 C8 C9 D6(01 04 05) DC(00 03 05) DF E0 E1 E2(00 1D 01 02 04 0E 12 14 23 24 27) F0(00 0C 0F 10 11 31 32 34 35) F1 F2 FD)mswhql(1)asset_eep(40)mccs_ver(2.1))";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_and_inputs() {
        let caps = parse(DELL_STYLE_CAPS);
        assert_eq!(caps.model.as_deref(), Some("U2723QE"));
        assert_eq!(caps.inputs, vec![0x0F, 0x11, 0x1B]);
    }

    #[test]
    fn tolerates_case_and_spacing() {
        assert_eq!(parse("(VCP(10 60 ( 0f  11 ) 62))").inputs, vec![0x0F, 0x11]);
    }

    #[test]
    fn skips_value_lists_of_other_features() {
        assert_eq!(parse("(vcp(14(60 01) 60(12)))").inputs, vec![0x12]);
    }

    #[test]
    fn ignores_the_vcpname_section() {
        assert_eq!(parse("(vcpname(60(Input)) vcp(60(11)))").inputs, vec![0x11]);
    }

    #[test]
    fn missing_or_broken_strings_give_nothing() {
        assert_eq!(parse(""), Capabilities::default());
        assert!(parse("(vcp(10 12))").inputs.is_empty());
        assert!(parse("(vcp(60(0F 11").inputs.is_empty());
    }

    #[test]
    fn names_inputs() {
        assert_eq!(input_label(0x0F), "DP");
        assert_eq!(input_label(0x10), "DP 2");
        assert_eq!(input_label(0x11), "HDMI");
        assert_eq!(input_label(0x12), "HDMI 2");
        assert_eq!(input_label(USB_C), "USB-C");
        assert_eq!(input_label(0x90), "0x90");
    }

    #[test]
    fn classifies_inputs() {
        assert_eq!(port_class(0x10), PortClass::Dp);
        assert_eq!(port_class(0x11), PortClass::Hdmi);
        assert_eq!(port_class(USB_C), PortClass::UsbC);
        assert_eq!(port_class(0x01), PortClass::Other);
    }
}

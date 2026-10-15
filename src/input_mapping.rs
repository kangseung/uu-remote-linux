//! Translate UU's verified Windows-VK input path into X11 KeySyms.
//! A KeySym is not an X11 keycode: the input engine must resolve it against the
//! active X keyboard mapping (e.g. XKeysymToKeycode) before XTest injection.
//! Letters use the unshifted lowercase symbol; separate modifier events carry
//! Shift/Ctrl/Alt state. Unknown VKs are ignored instead of injecting raw values.

/// Map an official Windows virtual-key identifier to a base X11 KeySym.
/// The wire has no extended-key bit; VK_RETURN consequently means Return.
/// OEM punctuation follows the US base-symbol interpretation; other physical
/// layouts may need their own OEM policy. OEM_102 is deliberately unsupported.
pub fn windows_vk_to_keysym(vk: i32) -> Option<u64> {
    let keysym = match vk {
        0x30..=0x39 => vk as u64,                // 0..9
        0x41..=0x5a => (vk + 0x20) as u64,      // A..Z -> a..z
        0x60..=0x69 => 0xffb0 + (vk - 0x60) as u64, // KP_0..KP_9
        0x70..=0x87 => 0xffbe + (vk - 0x70) as u64, // F1..F24
        0x08 => 0xff08, // BackSpace
        0x09 => 0xff09, // Tab
        0x0c => 0xff0b, // Clear
        0x0d => 0xff0d, // Return
        0x10 | 0xa0 => 0xffe1, // Shift / Shift_L
        0xa1 => 0xffe2, // Shift_R
        0x11 | 0xa2 => 0xffe3, // Control / Control_L
        0xa3 => 0xffe4, // Control_R
        0x12 | 0xa4 => 0xffe9, // Alt / Alt_L
        0xa5 => 0xffea, // Alt_R
        0x13 => 0xff13, // Pause
        0x14 => 0xffe5, // Caps_Lock
        0x1b => 0xff1b, // Escape
        0x20 => 0x0020, // space
        0x21 => 0xff55, // Prior / Page_Up
        0x22 => 0xff56, // Next / Page_Down
        0x23 => 0xff57, // End
        0x24 => 0xff50, // Home
        0x25 => 0xff51, // Left
        0x26 => 0xff52, // Up
        0x27 => 0xff53, // Right
        0x28 => 0xff54, // Down
        0x2c => 0xff61, // Print
        0x2d => 0xff63, // Insert
        0x2e => 0xffff, // Delete
        0x5b => 0xffeb, // Super_L
        0x5c => 0xffec, // Super_R
        0x5d => 0xff67, // Menu
        0x6a => 0xffaa, // KP_Multiply
        0x6b => 0xffab, // KP_Add
        0x6c => 0xffac, // KP_Separator
        0x6d => 0xffad, // KP_Subtract
        0x6e => 0xffae, // KP_Decimal
        0x6f => 0xffaf, // KP_Divide
        0x90 => 0xff7f, // Num_Lock
        0x91 => 0xff14, // Scroll_Lock
        0xba => 0x003b, // OEM_1: semicolon
        0xbb => 0x003d, // OEM_PLUS: equal
        0xbc => 0x002c, // OEM_COMMA
        0xbd => 0x002d, // OEM_MINUS
        0xbe => 0x002e, // OEM_PERIOD
        0xbf => 0x002f, // OEM_2: slash
        0xc0 => 0x0060, // OEM_3: grave
        0xdb => 0x005b, // OEM_4: bracketleft
        0xdc => 0x005c, // OEM_5: backslash
        0xdd => 0x005d, // OEM_6: bracketright
        0xde => 0x0027, // OEM_7: apostrophe
        // OEM_102 depends on the physical keyboard/layout; do not guess.
        _ => return None,
    };
    Some(keysym)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_shift_and_left_use_named_keysyms() {
        assert_eq!(windows_vk_to_keysym(160), Some(0xffe1));
        assert_eq!(windows_vk_to_keysym(37), Some(0xff51));
        assert_ne!(windows_vk_to_keysym(160), Some(160));
    }

    #[test]
    fn letters_use_unshifted_symbols_and_digits_keep_ascii() {
        assert_eq!(windows_vk_to_keysym(65), Some(u64::from(b'a')));
        assert_eq!(windows_vk_to_keysym(90), Some(u64::from(b'z')));
        for digit in b'0'..=b'9' {
            assert_eq!(windows_vk_to_keysym(i32::from(digit)), Some(u64::from(digit)));
        }
    }

    #[test]
    fn modifiers_preserve_left_and_right_identity() {
        let variants = [
            (0x10, 0xffe1), (0xa0, 0xffe1), (0xa1, 0xffe2),
            (0x11, 0xffe3), (0xa2, 0xffe3), (0xa3, 0xffe4),
            (0x12, 0xffe9), (0xa4, 0xffe9), (0xa5, 0xffea),
            (0x5b, 0xffeb), (0x5c, 0xffec),
        ];
        for (vk, expected) in variants {
            assert_eq!(windows_vk_to_keysym(vk), Some(expected));
        }
    }

    #[test]
    fn function_and_keypad_ranges_have_correct_endpoints() {
        assert_eq!(windows_vk_to_keysym(112), Some(0xffbe));
        assert_eq!(windows_vk_to_keysym(135), Some(0xffd5));
        assert_eq!(windows_vk_to_keysym(96), Some(0xffb0));
        assert_eq!(windows_vk_to_keysym(105), Some(0xffb9));
        assert_eq!(windows_vk_to_keysym(106), Some(0xffaa));
        assert_eq!(windows_vk_to_keysym(111), Some(0xffaf));
    }

    #[test]
    fn navigation_and_oem_base_keys_are_not_raw_keycodes() {
        assert_eq!(windows_vk_to_keysym(46), Some(0xffff));
        assert_eq!(windows_vk_to_keysym(36), Some(0xff50));
        assert_eq!(windows_vk_to_keysym(186), Some(u64::from(b';')));
        assert_eq!(windows_vk_to_keysym(187), Some(u64::from(b'=')));
        assert_eq!(windows_vk_to_keysym(220), Some(u64::from(b'\\')));
        assert_eq!(windows_vk_to_keysym(222), Some(u64::from(b'\'')));
    }

    #[test]
    fn unknown_or_layout_specific_keys_are_ignored() {
        for vk in [-1, 0, 7, 21, 136, 226, 255, 256, i32::MAX] {
            assert_eq!(windows_vk_to_keysym(vk), None);
        }
    }
}

/// Printable ASCII (plus newline/tab) → (Windows VK, needs Shift) on a US layout,
/// for IME text_input injection. OEM VKs follow the same US base-symbol policy as
/// windows_vk_to_keysym; the result must round-trip through it.
pub fn ascii_to_windows_vk(c: u8) -> Option<(i32, bool)> {
    let pair = match c {
        b'0'..=b'9' => (i32::from(c), false),
        b'a'..=b'z' => (i32::from(c) - 0x20, false),
        b'A'..=b'Z' => (i32::from(c), true),
        b'\n' => (0x0d, false),
        b'\t' => (0x09, false),
        b' ' => (0x20, false),
        b')' => (0x30, true), b'!' => (0x31, true), b'@' => (0x32, true), b'#' => (0x33, true),
        b'$' => (0x34, true), b'%' => (0x35, true), b'^' => (0x36, true), b'&' => (0x37, true),
        b'*' => (0x38, true), b'(' => (0x39, true),
        b';' => (0xba, false), b'=' => (0xbb, false), b',' => (0xbc, false), b'-' => (0xbd, false),
        b'.' => (0xbe, false), b'/' => (0xbf, false), b'`' => (0xc0, false),
        b'[' => (0xdb, false), b'\\' => (0xdc, false), b']' => (0xdd, false), b'\'' => (0xde, false),
        b':' => (0xba, true), b'+' => (0xbb, true), b'<' => (0xbc, true), b'_' => (0xbd, true),
        b'>' => (0xbe, true), b'?' => (0xbf, true), b'~' => (0xc0, true),
        b'{' => (0xdb, true), b'|' => (0xdc, true), b'}' => (0xdd, true), b'"' => (0xde, true),
        _ => return None,
    };
    Some(pair)
}

#[cfg(test)]
mod text_tests {
    use super::*;
    #[test]
    fn every_printable_ascii_maps_and_round_trips_to_a_keysym() {
        for c in 0x20..=0x7e {
            let (vk, _) = ascii_to_windows_vk(c).unwrap();
            assert!(windows_vk_to_keysym(vk).is_some(), "char {c:#x} has no keysym");
        }
        assert_eq!(ascii_to_windows_vk(b'\n'), Some((0x0d, false)));
        assert_eq!(ascii_to_windows_vk(b'\t'), Some((0x09, false)));
        assert_eq!(ascii_to_windows_vk(0x7f), None);
        assert_eq!(ascii_to_windows_vk(0x80), None);
    }

    #[test]
    fn shifted_digit_row_and_letter_case_match_us_layout() {
        for (c, digit) in b")!@#$%^&*(".iter().zip(b'0'..=b'9') {
            assert_eq!(ascii_to_windows_vk(*c), Some((i32::from(digit), true)));
        }
        assert_eq!(ascii_to_windows_vk(b'a'), Some((0x41, false)));
        assert_eq!(ascii_to_windows_vk(b'Z'), Some((0x5a, true)));
        assert_eq!(ascii_to_windows_vk(b' '), Some((0x20, false)));
        assert_eq!(ascii_to_windows_vk(b'_'), Some((0xbd, true)));
        assert_eq!(ascii_to_windows_vk(b'/'), Some((0xbf, false)));
    }
}

/// Single-screen portal capture inside a multi-monitor desktop: the uinput absolute
/// pointer maps 0..=65535 onto the whole desktop bounding box, while the viewer
/// normalizes coordinates to the captured screen. Logical compositor coordinates
/// throughout (stream pixel size differs under display scaling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsoluteMapping {
    stream_x: i64, stream_y: i64, stream_w: i64, stream_h: i64,
    desktop_x: i64, desktop_y: i64, desktop_w: i64, desktop_h: i64,
}

impl AbsoluteMapping {
    /// Resolve the captured screen's logical rect against the desktop layout.
    /// Origin comes from the portal stream `position`; without it, the unique
    /// output matching the stream's logical size wins. Returns None (identity)
    /// when geometry is unknown, ambiguous, or the whole desktop is captured.
    pub fn resolve(position: Option<(i32, i32)>, stream_logical: (i32, i32), outputs: &[crate::output_layout::Output]) -> Option<Self> {
        if stream_logical.0 <= 0 || stream_logical.1 <= 0 || outputs.is_empty() { return None; }
        let min_x = outputs.iter().map(|o| o.0).min()?;
        let min_y = outputs.iter().map(|o| o.1).min()?;
        let max_x = outputs.iter().map(|o| o.0 + o.2).max()?;
        let max_y = outputs.iter().map(|o| o.1 + o.3).max()?;
        let desktop = (i64::from(max_x - min_x), i64::from(max_y - min_y));
        if desktop.0 <= 0 || desktop.1 <= 0 { return None; }
        let stream = (i64::from(stream_logical.0), i64::from(stream_logical.1));
        let origin = match position {
            Some(origin) => origin,
            None => {
                let matches: Vec<_> = outputs.iter().filter(|o| (o.2, o.3) == stream_logical).collect();
                if matches.len() == 1 { (matches[0].0, matches[0].1) }
                else if stream == desktop { (min_x, min_y) }
                else { return None; }
            }
        };
        let mapping = Self {
            stream_x: i64::from(origin.0), stream_y: i64::from(origin.1),
            stream_w: stream.0, stream_h: stream.1,
            desktop_x: i64::from(min_x), desktop_y: i64::from(min_y),
            desktop_w: desktop.0, desktop_h: desktop.1,
        };
        // Whole-desktop capture keeps the established direct scaling.
        (mapping.stream_x != mapping.desktop_x || mapping.stream_y != mapping.desktop_y
            || mapping.stream_w != mapping.desktop_w || mapping.stream_h != mapping.desktop_h)
            .then_some(mapping)
    }

    /// Stream pixel point → uinput absolute coordinates (0..=ABS_MAX over the desktop box).
    pub fn map(&self, x: i32, y: i32, pixel_w: u32, pixel_h: u32) -> (i32, i32) {
        let axis = |v: i32, pixels: u32, origin: i64, logical: i64, base: i64, total: i64| -> i32 {
            let point = origin as f64 + f64::from(v) * logical as f64 / f64::from(pixels.max(1));
            ((point - base as f64) * f64::from(crate::uinput::ABS_MAX) / total as f64).round()
                .clamp(0.0, f64::from(crate::uinput::ABS_MAX)) as i32
        };
        (axis(x, pixel_w, self.stream_x, self.stream_w, self.desktop_x, self.desktop_w),
         axis(y, pixel_h, self.stream_y, self.stream_h, self.desktop_y, self.desktop_h))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::AbsoluteMapping;

    #[test]
    fn identity_when_whole_desktop_is_captured() {
        let outputs = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        assert_eq!(AbsoluteMapping::resolve(Some((0, 0)), (3840, 1080), &outputs), None);
        assert_eq!(AbsoluteMapping::resolve(None, (3840, 1080), &outputs), None);
        assert_eq!(AbsoluteMapping::resolve(None, (1920, 1080), &[]), None);
        assert_eq!(AbsoluteMapping::resolve(None, (0, 1080), &outputs), None);
    }

    #[test]
    fn left_and_right_screens_of_two() {
        let outputs = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        let left = AbsoluteMapping::resolve(Some((0, 0)), (1920, 1080), &outputs).unwrap();
        let right = AbsoluteMapping::resolve(Some((1920, 0)), (1920, 1080), &outputs).unwrap();
        // 65535/3840 per logical pixel: left edge 0 / 32768, center 16384 / 49151.
        assert_eq!(left.map(960, 540, 1920, 1080), (16384, 32768));
        assert_eq!(right.map(960, 540, 1920, 1080), (49151, 32768));
        // The captured screen's left edge maps to its desktop offset, not to zero.
        assert_eq!(right.map(0, 540, 1920, 1080).0, 32768);
        assert_eq!(left.map(0, 540, 1920, 1080).0, 0);
    }

    #[test]
    fn matches_unique_output_by_logical_size_without_position() {
        let outputs = [(0, 0, 2048, 1152), (2048, 36, 1920, 1080)];
        let mapping = AbsoluteMapping::resolve(None, (1920, 1080), &outputs).unwrap();
        assert_eq!(mapping, AbsoluteMapping::resolve(Some((2048, 36)), (1920, 1080), &outputs).unwrap());
        // Same-size screens are ambiguous: fail soft to identity.
        let twin = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        assert_eq!(AbsoluteMapping::resolve(None, (1920, 1080), &twin), None);
    }

    #[test]
    fn fractional_scale_uses_logical_coordinates() {
        // 2560x1440 panel at 1.5x: logical 1707x960, stream pixels stay 2560x1440.
        let outputs = [(0, 0, 1707, 960), (1707, 0, 1920, 1080)];
        let mapping = AbsoluteMapping::resolve(Some((0, 0)), (1707, 960), &outputs).unwrap();
        let (u, v) = mapping.map(1280, 720, 2560, 1440); // center of the stream
        assert_eq!(u, (853.5_f64 * 65535.0 / 3627.0).round() as i32);
        assert_eq!(v, (480.0_f64 * 65535.0 / 1080.0).round() as i32);
        // Stream right edge reaches the screen's logical right edge, not the desktop's.
        assert_eq!(mapping.map(2560, 720, 2560, 1440).0, (1707.0_f64 * 65535.0 / 3627.0).round() as i32);
    }
}

/// Official Mac CONTROL absolute coordinates are normalized to the source screen.
pub fn normalized_pointer(x: f64, y: f64, width: u32, height: u32) -> Option<(i32, i32)> {
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32
        || !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) { return None; }
    Some(((x * (width - 1) as f64).round() as i32, (y * (height - 1) as f64).round() as i32))
}

#[cfg(test)]
mod pointer_tests {
    use super::normalized_pointer;
    #[test]
    fn maps_source_screen_edges_and_center() {
        assert_eq!(normalized_pointer(0.0, 0.0, 1920, 1080), Some((0,0)));
        assert_eq!(normalized_pointer(1.0, 1.0, 1920, 1080), Some((1919,1079)));
        assert_eq!(normalized_pointer(0.5, 0.5, 1920, 1080), Some((960,540)));
    }
    #[test]
    fn maps_observed_editor_titlebar_coordinate() {
        assert_eq!(normalized_pointer(0.6805736636245111, 0.15990730011587487,1920,1080),Some((1306,173)));
    }
    #[test]
    fn rejects_invalid_coordinate_or_screen_dimensions() {
        assert_eq!(normalized_pointer(f64::NAN,0.5,1920,1080),None);
        assert_eq!(normalized_pointer(0.5,f64::INFINITY,1920,1080),None);
        assert_eq!(normalized_pointer(-0.1,0.5,1920,1080),None);
        assert_eq!(normalized_pointer(0.5,1.1,1920,1080),None);
        assert_eq!(normalized_pointer(0.5,0.5,0,1080),None);
    }
}

/// Official Mac sends mouse button bitmasks, not X11 button ordinals.
pub fn mouse_button_to_x11(button: i32) -> Option<i32> {
    match button { 1 => Some(1), 2 => Some(3), 4 => Some(2), _ => None }
}
#[cfg(test)]
mod button_tests {
    #[test]
    fn maps_verified_button_bitmasks() {
        assert_eq!(super::mouse_button_to_x11(1),Some(1));
        assert_eq!(super::mouse_button_to_x11(2),Some(3));
        assert_eq!(super::mouse_button_to_x11(4),Some(2));
        assert_eq!(super::mouse_button_to_x11(3),None);
    }
}

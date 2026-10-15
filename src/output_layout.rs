//! Logical output layout via xdg-output (Wayland). Fail-soft: None keeps the
//! identity pointer mapping (X11, nested sessions, compositors without xdg-output).

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawOutput { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

/// One output: (x, y, width, height) in compositor logical coordinates.
pub type Output = (i32, i32, i32, i32);

#[cfg(all(target_os = "linux", any(feature = "wayland", feature = "portal")))]
mod imp {
    extern "C" {
        fn uu_output_layout(outputs: *mut super::RawOutput, capacity: i32) -> i32;
    }
    pub fn outputs() -> Option<Vec<super::Output>> {
        let mut list = vec![super::RawOutput { x: 0, y: 0, width: 0, height: 0 }; 32];
        let count = unsafe { uu_output_layout(list.as_mut_ptr(), list.len() as i32) };
        if count <= 0 { return None; }
        Some(list[..count as usize].iter().map(|o| (o.x, o.y, o.width, o.height)).collect())
    }
}

#[cfg(not(all(target_os = "linux", any(feature = "wayland", feature = "portal"))))]
mod imp {
    pub fn outputs() -> Option<Vec<super::Output>> { None }
}

pub fn outputs() -> Option<Vec<Output>> { imp::outputs() }

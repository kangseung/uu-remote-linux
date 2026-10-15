//! Select the host desktop, not the presence of an XWayland DISPLAY socket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopBackend { X11, Wayland, Portal, Kms }

/// `UU_DESKTOP_BACKEND=none`: headless host — no display is opened; terminal, file transfer and
/// port mapping stay available (servers without a monitor or logged-in session).
pub fn headless() -> bool { std::env::var("UU_DESKTOP_BACKEND").is_ok_and(|v| v == "none") }

pub fn detect() -> Result<DesktopBackend, String> {
    let backend=select(std::env::var("UU_DESKTOP_BACKEND").ok().as_deref(),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").ok().as_deref())?;
    let explicit=std::env::var("UU_DESKTOP_BACKEND").unwrap_or_else(|_|"auto".into());
    let desktop=std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    // systemd user services can start before Plasma imports the session environment into the
    // manager: XDG_SESSION_TYPE/XDG_CURRENT_DESKTOP are then frozen-missing for the process
    // and detection once fell to X11 (x11_init failed on every offer). detect() re-runs per
    // ScreenCapture::new, so probe the live compositor socket; an unknown Wayland desktop
    // prefers Portal (generic) over the wlroots-only native engine.
    let socket_live=wayland_socket_live();
    Ok(if explicit=="auto" && backend==DesktopBackend::Wayland
        && (desktop.split(':').any(|s|s.eq_ignore_ascii_case("gnome")||s.eq_ignore_ascii_case("kde"))
            || (desktop.is_empty() && socket_live)) { DesktopBackend::Portal } else { backend })
}

/// Live filesystem probe: unlike the frozen process environment, the compositor socket
/// appears as soon as the session is up. WAYLAND_DISPLAY may itself be missing; the
/// conventional default is wayland-0.
fn wayland_socket_live() -> bool {
    let Some(runtime)=std::env::var_os("XDG_RUNTIME_DIR") else { return false };
    let name=std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    std::path::Path::new(&runtime).join(name).exists()
}

fn select(explicit: Option<&str>, session: Option<&str>, socket: Option<&str>) -> Result<DesktopBackend, String> {
    select_with_probe(explicit, session, socket, wayland_socket_live())
}

fn select_with_probe(explicit: Option<&str>, session: Option<&str>, socket: Option<&str>, socket_live: bool) -> Result<DesktopBackend, String> {
    match explicit {
        Some("x11") => Ok(DesktopBackend::X11),
        Some("wayland") => Ok(DesktopBackend::Wayland),
        Some("kms") => Ok(DesktopBackend::Kms),
        Some("portal") => Ok(DesktopBackend::Portal),
        Some("none") => Ok(DesktopBackend::X11), // never opened, see headless()
        None | Some("auto") => {
            if session == Some("wayland") || socket.is_some_and(|s| !s.is_empty()) || socket_live {
                Ok(DesktopBackend::Wayland)
            } else { Ok(DesktopBackend::X11) }
        },
        Some(_) => Err("UU_DESKTOP_BACKEND must be auto, x11, wayland, portal, kms or none".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xwayland_does_not_override_wayland_session() {
        assert_eq!(select_with_probe(None,Some("wayland"),Some("wayland-0"),false),Ok(DesktopBackend::Wayland));
        assert_eq!(select_with_probe(None,None,Some("wayland-1"),false),Ok(DesktopBackend::Wayland));
        assert_eq!(select_with_probe(None,Some("x11"),None,false),Ok(DesktopBackend::X11));
        assert_eq!(select_with_probe(Some("x11"),Some("wayland"),Some("wayland-0"),true),Ok(DesktopBackend::X11));
        assert_eq!(select_with_probe(Some("kms"),Some("wayland"),Some("wayland-0"),true),Ok(DesktopBackend::Kms));
        assert!(select_with_probe(Some("typo"),None,None,false).is_err());
    }
    #[test]
    fn live_socket_probe_covers_missing_session_environment() {
        // systemd user services started before the session env import: all env vars absent,
        // but the compositor socket already exists on disk.
        assert_eq!(select_with_probe(None,None,None,true),Ok(DesktopBackend::Wayland));
        assert_eq!(select_with_probe(None,None,None,false),Ok(DesktopBackend::X11));
    }
}

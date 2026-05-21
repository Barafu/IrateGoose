use dbus::arg::RefArg;
use dbus::blocking::{BlockingSender, Connection};

/// Represents a concrete system color scheme preference detected via D-Bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedTheme {
    Dark,
    Light,
}

/// Detects the system color scheme preference via the Freedesktop portal D-Bus API.
///
/// Queries `org.freedesktop.portal.Settings.Read` for the `color-scheme` key
/// under the `org.freedesktop.appearance` namespace.
///
/// Returns:
/// - `Some(DetectedTheme::Dark)`  — dark mode preferred
/// - `Some(DetectedTheme::Light)` — light mode preferred
/// - `None`                       — could not determine (no portal, no preference, or error)
pub fn detect_system_theme() -> Option<DetectedTheme> {
    let conn = Connection::new_session().ok()?;

    let mut msg = dbus::Message::new_method_call(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
        "Read",
    )
    .ok()?;

    msg.append_all(("org.freedesktop.appearance", "color-scheme"));

    let reply = conn
        .send_with_reply_and_block(msg, std::time::Duration::from_secs(5))
        .ok()?;

    let mut iter = reply.iter_init();
    let value: Box<dyn RefArg> = iter.get()?;
    let n = value.as_u64()? as u32;

    match n {
        1 => Some(DetectedTheme::Dark),
        2 => Some(DetectedTheme::Light),
        _ => None,
    }
}

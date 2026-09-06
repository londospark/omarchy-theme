//! Iced adapter for omarchy-theme.
//!
//! Two halves:
//! * [`to_theme`] — maps the resolved Omarchy palette onto an `iced::Theme`
//!   (0.13 `Custom` theme; iced derives the extended palette/contrasts).
//! * [`subscription`] — an iced `Subscription` that delivers a
//!   [`ThemeChanged`]-convertible message whenever the desktop theme changes
//!   (push tier: debounced, swap-tolerant, deduplicated by iced's id system).
//!
//! ```no_run
//! use omarchy_theme_iced::{subscription, to_theme, ThemeChanged};
//!
//! #[derive(Clone, Debug)]
//! enum Msg { Omarchy(ThemeChanged) }
//! impl From<ThemeChanged> for Msg {
//!     fn from(v: ThemeChanged) -> Msg { Msg::Omarchy(v) }
//! }
//!
//! struct App { theme: iced::Theme }
//!
//! impl App {
//!     fn theme(&self) -> iced::Theme { self.theme.clone() }
//!     fn subscription(&self) -> iced::Subscription<Msg> {
//!         omarchy_theme_iced::subscription()
//!     }
//!     fn update(&mut self, msg: Msg) {
//!         if let Msg::Omarchy(_) = msg {
//!             if let Ok(t) = omarchy_theme::Theme::current() {
//!                 self.theme = to_theme(&t);
//!             }
//!         }
//!     }
//! }
//! ```

use std::thread;

use futures::StreamExt;

/// The event behind the subscription; apps convert it into their own
/// message via `From<ThemeChanged>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeChanged;

/// Map an Omarchy theme onto an `iced::Theme`. The palette mode picks the
/// base so generated contrast colors read correctly.
pub fn to_theme(t: &omarchy_theme::Theme) -> iced::Theme {
    use iced::theme::Palette;

    let p = &t.palette;
    let c = |key: &str| -> iced::Color {
        p.color(key)
            .map(|rgba| iced::Color::from(rgba.to_f32_array()))
            .unwrap_or(iced::Color::BLACK)
    };
    let name = format!("Omarchy {}", t.name.clone().unwrap_or_default());
    iced::Theme::custom(
        name,
        Palette {
            background: c("background"),
            text: c("foreground"),
            primary: c("accent"),
            success: c("green"),
            danger: c("red"),
        },
    )
}

/// A subscription delivering `Message: From<ThemeChanged>` whenever the live
/// theme changes. iced deduplicates it by type, so calling this from
/// `subscription()` every update is the intended usage.
pub fn subscription<Message>() -> iced::Subscription<Message>
where
    Message: Send + From<ThemeChanged> + 'static,
{
    iced::Subscription::run(deliver::<Message>)
}

fn deliver<Message: Send + From<ThemeChanged> + 'static>(
) -> futures::stream::BoxStream<'static, Message> {
    let (tx, rx) = futures::channel::mpsc::unbounded::<()>();
    if spawn_delivery(tx).is_err() {
        // No live theme/watchable state: an empty, immediately-ended stream.
        return futures::stream::empty().boxed();
    }
    futures::stream::unfold(rx, move |mut rx| async move {
        rx.next().await.map(|()| (Message::from(ThemeChanged), rx))
    })
    .boxed()
}

/// Raw inotify on the `current/` state dir (survives the atomic swap),
/// debounced, with content-signature confirmation so half-states never
/// fire. One thread per active subscription; the thread ends when the
/// consumer (iced) drops the stream.
fn spawn_delivery(tx: futures::channel::mpsc::UnboundedSender<()>) -> Result<(), omarchy_theme::Error> {
    let dir = omarchy_theme::current_state_dir()?;
    if !dir.is_dir() {
        return Err(omarchy_theme::Error::NoTheme);
    }
    thread::spawn(move || {
        use inotify::{Inotify, WatchMask};
        let mut inotify = match Inotify::init() {
            Ok(i) => i,
            Err(_) => return,
        };
        if inotify
            .watches()
            .add(&dir, WatchMask::MOVED_TO | WatchMask::CREATE | WatchMask::DELETE)
            .is_err()
        {
            return;
        }
        let mut current = omarchy_theme::Theme::current().ok();
        let mut buffer = [0u8; 1024];
        loop {
            match inotify.read_events_blocking(&mut buffer) {
                Ok(_) => {
                    // Debounce the swap burst, then confirm via signature.
                    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(150);
                    while std::time::Instant::now() < deadline {
                        let _ = inotify.read_events(&mut buffer);
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    match (current.as_ref(), omarchy_theme::Theme::current()) {
                        (Some(old), Ok(fresh)) if fresh.signature() != old.signature() => {
                            if tx.unbounded_send(()).is_err() {
                                return; // consumer dropped
                            }
                            current = Some(fresh);
                        }
                        (None, Ok(fresh)) => current = Some(fresh),
                        _ => {}
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return,
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// to_theme must not panic and must carry the theme slug — iced itself
    /// is exercised by the example.
    #[test]
    fn maps_live_theme() {
        let Ok(theme) = omarchy_theme::Theme::current() else { return };
        let t = to_theme(&theme);
        let name = format!("{:?}", t);
        assert!(name.contains("Omarchy"), "theme name: {name}");
        let palette = t.palette();
        let want = theme.palette.color("background").unwrap();
        assert_eq!(palette.background, iced::Color::from(want.to_f32_array()));
    }
}

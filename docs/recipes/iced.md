# Recipe: iced (Rust)

`omarchy-theme-iced` covers both halves iced needs: palette → `iced::Theme`,
and a `Subscription` so theme switches reach your `update()` like any other
message (retained-mode push tier).

```rust
use iced::{Element, Subscription, Task, Theme};
use omarchy_theme_iced::{subscription, to_theme, ThemeChanged};

#[derive(Clone, Debug)]
enum Msg { Omarchy(ThemeChanged) /* ...your messages */ }

impl From<ThemeChanged> for Msg {
    fn from(v: ThemeChanged) -> Msg { Msg::Omarchy(v) }
}

struct App { theme: Theme, /* ... */ }

impl App {
    fn new() -> (Self, Task<Msg>) {
        let t = omarchy_theme::Theme::current().ok();
        let name = t.as_ref().and_then(|t| t.name.clone()).unwrap_or_default();
        let theme = t.as_ref().map(to_theme).unwrap_or_else(Theme::CatppuccinMocha);
        (Self { theme, /* ... */ }, Task::none())
    }

    fn subscription(&self) -> Subscription<Msg> { subscription() }

    fn theme(&self) -> Theme { self.theme.clone() }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Omarchy(_) => {
                if let Ok(t) = omarchy_theme::Theme::current() {
                    self.theme = to_theme(&t);   // iced restyles everything
                }
                Task::none()
            }
            /* ... */
        }
    }
}
```

`to_theme` maps: `background→background`, `foreground→text`,
`accent→primary`, `green→success`, `red→danger`; iced derives its extended
palette (contrasts, hover tints) from those five automatically, so this is
the complete mapping. `name` in the returned theme reads
`Omarchy <slug>` — handy in theme pickers.

The subscription is id-deduplicated by iced (safe to return from every
`subscription()` call), debounced, and swap-tolerant; it goes silent when no
Omarchy state dir exists, so an app built with this still runs on vanilla
Linux/macOS/Windows.

Live demo: [`crates/omarchy-theme-iced/examples/iced_demo.rs`](../../crates/omarchy-theme-iced/examples/iced_demo.rs)
— `cargo run -p omarchy-theme-iced --example iced_demo`.

//! omarchy-theme-iced demo: a small counter app that live-follows the
//! Omarchy desktop theme. Change theme with `omarchy theme set <name>` and
//! watch it retint without restart.
//!
//!   cargo run -p omarchy-theme-iced --example iced_demo

use iced::widget::{button, column, container, text};
use iced::{Element, Task, Theme};

use omarchy_theme_iced::{subscription, to_theme, ThemeChanged};

fn title(_: &App) -> String {
    String::from("omarchy-theme x iced")
}

fn main() -> iced::Result {
    iced::application(title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .run_with(App::new)
}

#[derive(Clone, Debug)]
enum Msg {
    Increment,
    Omarchy(ThemeChanged),
}

impl From<ThemeChanged> for Msg {
    fn from(v: ThemeChanged) -> Msg {
        Msg::Omarchy(v)
    }
}

struct App {
    count: i32,
    theme_name: String,
    theme: Theme,
}

impl App {
    fn new() -> (Self, Task<Msg>) {
        let omarchy = omarchy_theme::Theme::current().ok();
        let theme_name = omarchy
            .as_ref()
            .and_then(|t| t.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let theme = match &omarchy {
            Some(t) => to_theme(t),
            None => Theme::CatppuccinMocha, // graceful off-desktop default
        };
        ((Self { count: 0, theme_name, theme }), Task::none())
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> iced::Subscription<Msg> {
        subscription()
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Omarchy(_) => {
                if let Ok(t) = omarchy_theme::Theme::current() {
                    self.theme_name = t.name.clone().unwrap_or_default();
                    self.theme = to_theme(&t);
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Msg> {
        container(
            column![
                text(format!("Omarchy: {}", self.theme_name)).size(24),
                text(format!("counter: {}", self.count)),
                button("increment").on_press(Msg::Increment),
                button("+1 (primary tint)").on_press(Msg::Increment),
            ]
            .spacing(16)
            .width(240),
        )
        .center(iced::Length::Fill)
        .into()
    }
}

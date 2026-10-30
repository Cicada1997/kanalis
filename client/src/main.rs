pub mod state;
pub mod ui;
pub mod net;
pub mod auth;
pub mod message_history;

use ui::{ update, view };
use net::{ connect };
use state::{ State };

use iced::Theme;

fn main() -> iced::Result {
    dotenv::dotenv().ok();

    iced::application(State::new, update, view)
        .subscription(|state: &State| {
            Subscription::batch([
                state
                    .current_token
                    .as_ref()
                    .map_or_else(Subscription::none, |token| connect(token.clone())),
                keypress_subscription(),
            ])
        })
        .theme(|_: &State| Theme::Dark)
        .centered()
        .run()
}

use iced::{
    Event, Subscription,
    event::{ self, Status },
    keyboard::{ self, Key, key::Named, Modifiers},
};

fn keypress_subscription() -> Subscription<ui::Message> {
    event::listen_with(|event, status, _window| match (event, status) {
        (
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Tab),
                modifiers, .. 
            }),
            Status::Ignored,
        ) => {
            match modifiers.shift() {
                true => Some(ui::Message::FocusPrevious),
                false => Some(ui::Message::FocusNext),
            }
        }

        (
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Character(c),
                modifiers: Modifiers::ALT, ..
            }),
            Status::Ignored,
        ) => {
            match c.as_str() {
                "j" => Some(ui::Message::ChannelNext),
                "k" => Some(ui::Message::ChannelPrevious),
                _ => { None }
            }
        }
        _ => None,
    })
}

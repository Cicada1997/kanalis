pub mod state;
pub mod protocol;
pub mod ui;
pub mod net;

use ui::{ update, view };
use net::{ connect };
use state::{ State };

use iced::Theme;

fn main() -> iced::Result {
    dotenv::dotenv().ok();

    iced::application(State::default, update, view)
        .subscription(|state: &State| {
            state.current_token.as_ref()
                .map_or_else(
                    iced::Subscription::none,
                    |token| connect(token.clone())
                )

        })
        .theme(|_: &State| Theme::Dark)
        .centered()
        .run()
}

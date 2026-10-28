pub use super::create_channel_modal::CreateChannelModal; 

use crate::state::{ self, State, AuthStatus };
use crate::ui::Message;
use crate::net::ConnEvent;

use regex::Regex;
use protocol::{ ChannelId, ClientPacket };
use iced::widget::{ text, text_input, button, column, row, scrollable, Row, container, image };
use iced::{ Fill, Length, Element, Color, Background, Alignment };

#[derive(Debug, Clone, Default)]
pub enum ChannelButton {
    #[default]
    Primary,
    Current,
}

pub fn render(state: &State) -> Element<'_, Message> {
    let mut channel_list = column![].spacing(5);
    
    for channel in &state.channels {
        let ch = button(text(&channel.name));

        let ch = match state.current_channel == channel.id {
            true => ch.style(|_, _| {
                let bg = Background::Color(Color::from_rgb(0.5, 0.5, 0.5));
                let mut style = button::Style::default();
                style.background = Some(bg);
                style
            }),
            false => ch.on_press(Message::SwitchChannel(channel.id))
        }.width(Fill);

        channel_list = channel_list.push(ch);
    }

    let create_ch_button = button(
        text("+")
            .width(Fill)
            .center()
            .size(20)
    ).padding(0)
        .on_press(Message::ShowCreateChannel)
        .width(Fill);

    channel_list = channel_list.push(create_ch_button);


    scrollable(
        channel_list
            .width(140)
            .spacing(10)
            .padding(10)
    ).into()
}


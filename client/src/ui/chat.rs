pub use super::create_channel_modal::CreateChannelModal; 

use crate::state::{ self, State, AuthStatus };
use crate::net::ConnEvent;
use crate::ui::Message;
use crate::state::ChatMessage;

use std::sync::LazyLock;
static IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<image:([^>]*)>").unwrap());

use regex::Regex;
use protocol::{ ChannelId, ClientPacket };
use iced::widget::{ text, text_input, button, column, row, scrollable, Row, container, image };
use iced::{ Fill, Length, Element, Color, Background, Alignment };

#[must_use]
pub fn server_view(state: &State) -> Element<'_, Message> {
    row![
        super::channel_bar::render(state),
        column![
            chat_history(state),
            chat_input(state),
        ].width(Length::FillPortion(1)).spacing(20),
    ]
    .spacing(20)
    .into()
}

fn message<'a>(state: &'a State, message: &'a ChatMessage) -> Element<'a, Message> {
    // Bilderna
    let images: Vec<Element<'a, Message>> = IMAGE_RE
        .captures_iter(&message.content)
        .filter_map(|caps| caps.get(1))
        // .filter_map(
        .map(|m| {
            // match state.image_cache.get(m.as_str()) {
            // Some(handle) => {
            state.image_cache.get(m.as_str())
                .and_then(|handle| {
                    Some(
                        image(handle.clone())
                            .width(Length::Fixed(200.0))
                            .border_radius(8)
                    )
                })
                .unwrap_or(
                    image("assets/loading_image.png")
                        .width(Length::Fixed(20.0))
                        .border_radius(8)
                )
                .into()
        })
        .collect();

    let clean_text = IMAGE_RE
        .replace_all(&message.content, "")
        .trim()
        .to_string();

    let timestamp = message.timestamp
        // .and_local_timezone(tz)
        // .unwrap()
        .format("%Y.%m.%d kl %H")
        .to_string();
    

    let header = row![
        text(&message.user.name)
            .size(15)
            .color(Color::from_rgb(0.5, 0.5, 1.0)),
        text(timestamp)
            .size(12)
            .color(Color::from_rgb(0.55, 0.55, 0.55)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(4);

    if !clean_text.is_empty() {
        body = body.push(text(clean_text).size(15));
    }

    if !images.is_empty() {
        body = body.push(Row::with_children(images).spacing(8));
    }

    container(body)
        .padding([8, 12])
        .width(Length::Fill)
        .into()
}

fn chat_history(state: &State) -> Element<'_, Message> {
    let mut history = column![].spacing(10);
    
    if let Some(messages) = state.messages.get(state.current_channel) {
        history = messages
            .iter()
            .map(|m| message(state, m))
            .collect();
    }

    scrollable(history.width(Length::FillPortion(1)))
        .height(Length::Fill)
        .auto_scroll(true)
        .anchor_bottom()
        .into()
}
 
fn chat_input(state: &State) -> Element<'_, Message> {
    text_input("Skriv något till n00bsen...", &state.current_message)
        .on_input(Message::UpdateMessage)
        .on_submit(Message::SendMessage)
        .into()
}


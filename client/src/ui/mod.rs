pub mod create_channel_modal;

pub use create_channel_modal::CreateChannelModal; 

use crate::state::{ self, State, AuthStatus };
use crate::net::ConnEvent;

use regex::Regex;
use protocol::{ ChannelId, ClientPacket };
use iced::widget::{ text, text_input, button, column, row, scrollable, Row, container, image };
use iced::{ Fill, Length, Element, Color, Background, Alignment };

#[derive(Clone)]
pub enum Message {
    SendMessage,
    UpdateMessage(String),

    SwitchChannel(ChannelId),

    ConnMessage(ConnEvent),

    SubmitLogin,
    UpdateLoginUsername(String),
    UpdateLoginPassword(String),

    ShowCreateChannel,
    ModalMessage(create_channel_modal::Message),
    
    ImageLoaded(String, Option<Vec<u8>>),
}

pub fn update(state: &mut State, event: Message) -> iced::Task<Message> {
    let mut tasks = vec![];
    
    match event {
        Message::ConnMessage(conn_event) => tasks.push(state.apply_conn_event(conn_event)),
        
        Message::SwitchChannel(id) => state.switch_channel(id),
        Message::UpdateMessage(msg) => state.current_message = msg,
        Message::SendMessage => state.send_current_message(),

        Message::UpdateLoginUsername(txt) => state.username_input_field = txt,
        Message::UpdateLoginPassword(txt) => state.password_input_field = txt,
        Message::SubmitLogin => state.submit_login(),

        Message::ShowCreateChannel => {
            state.create_channel_modal = Some(CreateChannelModal::default());
        }
        
        Message::ModalMessage(modal_msg) => {
            if let Some(modal) = &mut state.create_channel_modal {
                let action = modal.update(modal_msg);
                
                match action {
                    create_channel_modal::Action::Submit { name, private } => {
                        if let Some(ref sender) = state.sender {
                            let _ = sender.try_send(ClientPacket::CreateChannel { name, private });
                            state.create_channel_modal = None;
                        }
                    }
                    create_channel_modal::Action::Cancel => {
                        state.create_channel_modal = None;
                    }
                    create_channel_modal::Action::None => {}
                }
            }
        }

        Message::ImageLoaded(url, Some(bytes)) => {
            let handle = iced::widget::image::Handle::from_bytes(bytes);
            state.image_cache.insert(url, handle);
        }
        Message::ImageLoaded(_, None) => {}
    }

    iced::Task::batch(tasks)
}

#[must_use]
pub fn view(state: &State) -> Element<'_, Message> {
    if let Some(modal) = &state.create_channel_modal {
        modal.view().map(Message::ModalMessage)
    } else {
        match state.auth_status {
            AuthStatus::LoggedIn => server_view(state),
            AuthStatus::Waiting => waiting_view(state),
            AuthStatus::LoggedOut => login_view(state),
        }
    }
}

// --- KOMPONENTER ---

#[must_use]
pub fn waiting_view(_state: &State) -> Element<'_, Message> {
    text("Waiting to to be authenticated...")
        .width(Length::FillPortion(1))
        .height(Length::FillPortion(1))
        .center()
        .into()
}

#[must_use]
pub fn login_view(state: &State) -> Element<'_, Message> {
    column![
        text("Log in"),
        login_field(state)
    ]
        .padding(250)
        .into()
}

#[must_use]
pub fn server_view(state: &State) -> Element<'_, Message> {
    row![
        sidebar(state),
        column![
            chat_history(state),
            chat_input(state),
        ].width(Length::FillPortion(1)).spacing(20),
    ]
    .spacing(20)
    .into()
}

#[derive(Debug, Clone, Default)]
pub enum ChannelButton {
    #[default]
    Primary,
    Current,
}

fn sidebar(state: &State) -> Element<'_, Message> {
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

use std::sync::LazyLock;

static IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<image:([^>]*)>").unwrap());


fn message<'a>(state: &'a State, message: &'a state::Message) -> Element<'a, Message> {
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

fn login_field(state: &State) -> Element<'_, Message> {
    column![
        text_input("Användarnamn...", &state.username_input_field)
            .on_input(Message::UpdateLoginUsername),
        text_input("Lösenord...", &state.password_input_field)
            .on_input(Message::UpdateLoginPassword)
            .on_submit(Message::SubmitLogin),
        button("logga in")
            .on_press(Message::SubmitLogin)
    ]
        .width(Fill)
        .height(Fill)
        .into()
}

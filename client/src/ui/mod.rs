pub mod create_channel_modal;

pub use create_channel_modal::CreateChannelModal; // Fixed typo

use crate::state::{ self, State, AuthStatus };
use crate::protocol::{ ChannelId, ClientPacket };
use crate::net::ConnEvent;

use iced::widget::{ text, text_input, button, column, row, scrollable };
use iced::{ Fill, Length, Element, Color, Background, };

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
    // Add a wrapper for the modal's internal messages
    ModalMessage(create_channel_modal::Message),
}

pub fn update(state: &mut State, event: Message) {
    match event {
        Message::ConnMessage(conn_event) => state.apply_conn_event(conn_event),
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
    }
}

#[must_use]
pub fn view(state: &State) -> Element<'_, Message> {
    if let Some(modal) = &state.create_channel_modal {
        // Map the child's messages to the parent's message enum
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

fn message(message: &state::Message) -> Element<'_, Message> {
    column![
        row![
            text(&message.user.name).color(iced::Color::from_rgb(0.5, 0.5, 1.0)),
            text(format!("{:?}", message.timestamp)).size(12),
        ].spacing(10),
        text(&message.content),
    ].into()
}

fn chat_history(state: &State) -> Element<'_, Message> {
    let mut history = column![].spacing(10);
    
    if let Some(messages) = state.messages.get(state.current_channel) {
        history = messages
            .iter()
            .map(message)
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

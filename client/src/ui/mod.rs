pub mod create_channel_modal;
pub mod channel_bar;
pub mod chat;

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

        // TODO: reload with "not-found" image
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
            AuthStatus::LoggedIn => chat::server_view(state),
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

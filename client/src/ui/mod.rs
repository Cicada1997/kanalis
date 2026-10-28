pub mod create_channel_modal;
pub mod channel_bar;
pub mod chat;
pub mod auth;

pub use create_channel_modal::CreateChannelModal; 

use crate::state::{ State, AuthStatus };
use crate::net::ConnEvent;

use protocol::{ ChannelId, ClientPacket };

use iced::widget::text;
use iced::{ Task, Length, Element };

#[derive(Clone)]
pub enum Message {
    SendMessage,
    UpdateMessage(String),

    SwitchChannel(ChannelId),

    ConnMessage(ConnEvent),

    ShowCreateChannel,
    ModalMessage(create_channel_modal::Message),

    LoginMessage(auth::LoginMessage),
    LoginResult(Result<String, String>),
    
    ImageLoaded(String, Option<Vec<u8>>),
}

pub fn update(state: &mut State, event: Message) -> Task<Message> {
    match event {
        Message::ConnMessage(conn_event) => state.apply_conn_event(conn_event),
        
        Message::SwitchChannel(id) => {
            state.switch_channel(id);
            Task::none()
        }
        Message::UpdateMessage(msg) => {
            state.current_message = msg;
            Task::none()
        }
        Message::SendMessage => {
            state.send_current_message();
            Task::none()
        }

        Message::LoginMessage(login_msg) => {
            match state.auth.update(login_msg) {
                auth::Action::Run(task) => task.map(Message::from),
                auth::Action::Login { username, password } => {
                    state.auth_status = AuthStatus::Waiting;
                    Task::perform(
                        crate::auth::login(username, password),
                        Message::LoginResult
                    )
                }
                auth::Action::None => Task::none(),
            }
        }

        Message::LoginResult(Ok(token)) => {
            state.current_token = Some(token);
            state.auth_status = AuthStatus::LoggedIn;
            Task::none()
        }
         
        Message::LoginResult(Err(_reason)) => {
            state.auth_status = AuthStatus::LoggedOut;
            Task::none()
        }


        Message::ShowCreateChannel => {
            state.create_channel_modal = Some(CreateChannelModal::default());
            Task::none()
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
            Task::none()
        }

        Message::ImageLoaded(url, Some(bytes)) => {
            let handle = iced::widget::image::Handle::from_bytes(bytes);
            state.image_cache.insert(url, handle);
            Task::none()
        }

        // TODO: reload with "not-found" image
        Message::ImageLoaded(_, None) => Task::none()
    }
}

#[must_use]
pub fn view(state: &State) -> Element<'_, Message> {
    if let Some(modal) = &state.create_channel_modal {
        modal.view().map(Message::ModalMessage)
    } else {
        match state.auth_status {
            AuthStatus::LoggedIn => chat::server_view(state),
            AuthStatus::Waiting => waiting_view(state),
            AuthStatus::LoggedOut => state.auth.view(),
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

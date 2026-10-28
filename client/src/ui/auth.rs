use iced::widget::{button, center, column, operation, text, text_input};
use iced::{Element, Task};

use super::Message;

#[derive(Debug, Clone)]
pub enum LoginMessage {
    UsernameChanged(String),
    PasswordChanged(String),
    UsernameSubmitted,
    Submit,
}

impl From<LoginMessage> for Message {
    fn from(msg: LoginMessage) -> Self {
        Self::LoginMessage(msg)
    }
}

pub enum Action {
    None,
    Run(Task<LoginMessage>),
    Login { username: String, password: String },
}

#[derive(Default)]
pub struct AuthComponent {
    username: String,
    password: String,
}

impl AuthComponent {
    pub fn update(&mut self, message: LoginMessage) -> Action {
        match message {
            LoginMessage::UsernameChanged(s) => {
                self.username = s;
                Action::None
            }
            LoginMessage::PasswordChanged(s) => {
                self.password = s;
                Action::None
            }
            LoginMessage::UsernameSubmitted => Action::Run(operation::focus_next()),
            LoginMessage::Submit if self.can_submit() => Action::Login {
                username: self.username.clone(),
                password: self.password.clone(),
            },
            LoginMessage::Submit => Action::None,
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        let form: Element<'_, LoginMessage> = column![
            text("Logga in").size(24),
            text_input("Användarnamn", &self.username)
                .on_input(LoginMessage::UsernameChanged)
                .on_submit(LoginMessage::UsernameSubmitted),
            text_input("Lösenord", &self.password)
                .secure(true)
                .on_input(LoginMessage::PasswordChanged)
                .on_submit(LoginMessage::Submit),
            button("Logga in")
                .on_press_maybe(self.can_submit().then_some(LoginMessage::Submit)),
        ]
        .spacing(10)
        .max_width(320)
        .into();

        center(form.map(Message::from)).into()
    }

    fn can_submit(&self) -> bool {
        !self.username.is_empty() && !self.password.is_empty()
    }
}

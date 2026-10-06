use iced::widget::{button, column, container, text, text_input, row};
use iced::{Element, Length, alignment};

#[derive(Default)]
pub struct CreateChannelModal {
    channel_name: String,
    private: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ChannelNameChanged(String),
    Submit,
    Cancel,
}

pub enum Action {
    None,
    Submit {
        name: String,
        private: bool,
    },
    Cancel,
}

impl CreateChannelModal {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::ChannelNameChanged(name) => {
                self.channel_name = name;
                Action::None
            },
            Message::Submit => {
                Action::Submit {
                    name: self.channel_name.clone(),
                    private: self.private,
                }
            },
            Message::Cancel => Action::Cancel,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text("Create a new channel").size(24),
                text_input("Channel Name", &self.channel_name)
                    .on_input(Message::ChannelNameChanged)
                    .on_submit(Message::Submit),
                row![
                    button("Cancel").on_press(Message::Cancel),
                    button("Create").on_press(Message::Submit),
                ]
                .spacing(10)
                // Use align_y for Rows in Iced 0.12+
                .align_y(alignment::Vertical::Center) 
            ]
            .spacing(20)
        )
        .padding(30)
        .width(Length::Fixed(400.0))
        // Dropped the `.style(...)` call to fix the E0433 error
        .into()
    }
}

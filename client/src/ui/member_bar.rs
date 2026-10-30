use crate::state::State;
use crate::ui::Message;

use iced::widget::{ text, button, column, scrollable };
use iced::{ Fill, Element, Color, Background };

pub fn render(state: &State) -> Element<'_, Message> {
    let mut member_list = column![].spacing(5);
    
    for member in &state.members {
        member_list = member_list.push(
            button(text(&member.username))
                .width(Fill)
        );
    }

    scrollable(
        member_list
            .width(140)
            .spacing(10)
            .padding(10)
    ).into()
}


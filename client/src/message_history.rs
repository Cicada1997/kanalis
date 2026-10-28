use crate::state::{ ChatMessage };

use protocol::ChannelId;

use std::collections::{ HashMap };

 

struct ChannelMessages {
    messages: Vec<ChatMessage>,
    sorted: bool,
}

impl ChannelMessages {
    pub fn get(&self) -> &[ChatMessage] {
        return self.messages.as_slice();
    }

    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.sort();
    }

    #[allow(unused)]
    pub fn extend(&mut self, messages: &[ChatMessage]) {
        self.messages.extend(messages.iter().cloned());
        self.sort();
    }

    pub fn push_unchecked(&mut self, message: ChatMessage) {
        self.sorted = false;
        self.messages.push(message);
    }

    #[allow(unused)]
    pub fn try_sort(&mut self) {
        if !self.sorted {
            self.sort();
        }
    }

    fn sort(&mut self) {
        self.messages.sort_by(|m1, m2| m1.timestamp.cmp(&m2.timestamp));
        self.sorted = true;
    }
}

#[derive(Default)]
pub struct MessageHistory {
    channels: HashMap<ChannelId, ChannelMessages>
}

impl MessageHistory {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, channel_id: ChannelId) -> Option<&[ChatMessage]> {
        self.channels.get(&channel_id).map(|ch| ch.get())
    }

    pub fn push(&mut self, message: ChatMessage) {
        self.channels.entry(message.channel_id)
            .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true })
            .push(message);
    }

    pub fn push_unchecked(&mut self, message: ChatMessage) {
        if let Some(ch) = self.channels.get_mut(&message.channel_id) {
            ch.push_unchecked(message);
        } else {
            self.channels.insert(message.channel_id, ChannelMessages {
                messages: vec![message],
                sorted: true,
            });
        }
    }

    pub fn extend(&mut self, messages: &[ChatMessage]) {
        for message in messages {
            self.channels.entry(message.channel_id)
                .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true })
                .push(message.clone());
        }
    }

    pub fn extend_unchecked(&mut self, messages: &[ChatMessage], id: ChannelId) {
        let ch = self.channels.entry(id)
            .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true });
        
        ch.messages.extend(messages.iter().cloned());
        ch.sorted = false;
    }
}

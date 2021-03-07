use crate::imp::{core::*, prelude::*};

#[derive(Debug)]
pub(crate) struct Route {
    channel: ChannelOwner
}

impl Route {
    pub(crate) fn new(channel: ChannelOwner) -> Self { Self { channel } }
}

impl RemoteObject for Route {
    fn channel(&self) -> &ChannelOwner { &self.channel }
    fn channel_mut(&mut self) -> &mut ChannelOwner { &mut self.channel }
}

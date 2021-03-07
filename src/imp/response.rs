use crate::imp::{core::*, prelude::*};

#[derive(Debug)]
pub(crate) struct Response {
    channel: ChannelOwner
}

impl Response {
    pub(crate) fn try_new(ctx: &Context, channel: ChannelOwner) -> Result<Self, Error> {
        Ok(Self { channel })
    }
}

impl RemoteObject for Response {
    fn channel(&self) -> &ChannelOwner { &self.channel }
    fn channel_mut(&mut self) -> &mut ChannelOwner { &mut self.channel }
}

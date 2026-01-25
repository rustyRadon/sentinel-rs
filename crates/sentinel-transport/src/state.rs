use std::marker::PhantomData;
use crate::SentinelTransport;

pub struct Unauthenticated;
pub struct Authenticated { pub user_id: String }
pub struct Closing;

pub struct Connection<T: SentinelTransport, S> {
    transport: T,
    _state: PhantomData<S>,
}

impl<T: SentinelTransport> Connection<T, Unauthenticated> {
    pub fn new(transport: T) -> Self {
        Self { transport, _state: PhantomData }
    }

    pub fn into_authenticated(self, _user_id: String) -> Connection<T, Authenticated> {
        Connection {
            transport: self.transport,
            _state: PhantomData,
        }
    }
}

impl<T: SentinelTransport> Connection<T, Authenticated> {
    pub async fn send_data(&mut self, _data: &[u8]) -> Result<(), std::io::Error> {
        // implementation ........ hmm
        Ok(())
    }
}
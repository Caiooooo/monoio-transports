pub mod connector;
pub mod key;
pub mod pool;

use std::rc::Rc;

use http::HeaderMap;
use monoio::io::sink::SinkExt;
use monoio::io::stream::Stream;
use monoio_http::h1::payload::Payload;

use self::connector::Connector;
use crate::request::ClientRequest;

use self::{
    connector::{DefaultTcpConnector, DefaultTlsConnector},
    key::Key,
};

// TODO: ClientBuilder
pub struct ClientInner<C, #[cfg(feature = "tls")] CS> {
    cfg: ClientConfig,
    http_connector: C,
    #[cfg(feature = "tls")]
    https_connector: CS,
}

pub struct Client<
    C = DefaultTcpConnector<Key>,
    #[cfg(feature = "tls")] CS = DefaultTlsConnector<Key>,
> {
    #[cfg(feature = "tls")]
    shared: Rc<ClientInner<C, CS>>,
    #[cfg(not(feature = "tls"))]
    shared: Rc<ClientInner<C>>,
}

#[cfg(feature = "tls")]
impl<C, CS> Clone for Client<C, CS> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

#[cfg(not(feature = "tls"))]
impl<C> Clone for Client<C> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct ClientConfig {
    default_headers: Rc<HeaderMap>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let shared = Rc::new(ClientInner {
            cfg: ClientConfig::default(),
            http_connector: Default::default(),
            #[cfg(feature = "tls")]
            https_connector: Default::default(),
        });
        Self { shared }
    }

    // TODO: allow other connector impl.
    pub fn request<M, U>(&self, method: M, uri: U) -> ClientRequest
    where
        http::Method: TryFrom<M>,
        <http::Method as TryFrom<M>>::Error: Into<http::Error>,
        http::Uri: TryFrom<U>,
        <http::Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        let mut req = ClientRequest::new(self.clone()).method(method).uri(uri);
        for (key, value) in self.shared.cfg.default_headers.iter() {
            req = req.header(key, value);
        }
        req
    }

    // TODO: error handling
    pub async fn send(
        &self,
        request: http::Request<Payload>,
    ) -> Result<http::Response<Payload>, ()> {
        let uri = request.uri();
        let key = uri.try_into().unwrap();
        let mut codec = self.shared.http_connector.connect(key).await.unwrap();
        codec.send_and_flush(request).await.unwrap();
        // Note: the first unwrap is Option
        let resp = codec.next().await.unwrap().unwrap();
        Ok(resp)
    }
}

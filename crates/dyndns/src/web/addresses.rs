use color_eyre::eyre::{bail, Result, WrapErr};
use itertools::Itertools;
use std::net::{SocketAddr, ToSocketAddrs};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tailsome::IntoResult;
use tokio::net::TcpListener;
use tracing::info;

pub struct MultipleAddrsIncoming {
    pub listeners: Vec<TcpListener>,
}

impl Future for MultipleAddrsIncoming {
    type Output = std::io::Result<(tokio::net::TcpStream, std::net::SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for listener in &mut self.listeners {
            if let Poll::Ready(Ok(value)) = Pin::new(listener).poll_accept(cx) {
                return Poll::Ready(Ok(value));
            }
        }

        Poll::Pending
    }
}

pub fn socket_addresses_from_host_and_port(hostname: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let addrs = (hostname, port)
        .to_socket_addrs()
        .wrap_err("Failed to convert hostname:port to at least one listening address")?
        .filter(|addr| {
            // Filter out ipv6 link local addresses. Not robust, but better method is unstable.
            // Identify them by a non-zero scope id.
            // https://github.com/rust-lang/rust/issues/27709
            if let SocketAddr::V6(ipv6) = addr {
                if ipv6.scope_id() != 0 {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>();

    if addrs.is_empty() {
        bail!("Failed to resolve at least one listening address");
    };
    addrs.into_ok()
}

pub fn socket_acceptor_from_socket_addreses(addrs: &[SocketAddr]) -> Result<MultipleAddrsIncoming> {
    let addrs = addrs
        .iter()
        .map(|a| {
            let listener =
                std::net::TcpListener::bind(a).wrap_err("Failed to bind to address socket")?;
            listener
                .set_nonblocking(true)
                .wrap_err("Failed move tcp stream into non blocking mode")?;
            let listener =
                TcpListener::from_std(listener).wrap_err("Failed to create async TcpListener")?;
            Ok(listener)
        })
        .collect::<Result<Vec<_>>>()?;
    MultipleAddrsIncoming { listeners: addrs }.into_ok()
}

pub fn print_listener_addresses(addrs: &[SocketAddr]) {
    let addrs = addrs
        .iter()
        .map(|addr| format!("    http://{addr}"))
        .join("\n");
    info!("Web server listening on the following addresses:\n{addrs}");
}

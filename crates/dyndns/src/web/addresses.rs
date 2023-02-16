use color_eyre::eyre::{bail, Result, WrapErr};
use hyper::server::{accept::Accept, conn::AddrIncoming};
use itertools::Itertools;
use std::net::{SocketAddr, ToSocketAddrs};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tailsome::IntoResult;
use tracing::info;

pub struct MultipleAddrsIncoming {
    pub addrs: Vec<AddrIncoming>,
}

impl Accept for MultipleAddrsIncoming {
    type Conn = <AddrIncoming as Accept>::Conn;
    type Error = <AddrIncoming as Accept>::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        for addr in &mut self.addrs {
            if let Poll::Ready(Some(value)) = Pin::new(addr).poll_accept(cx) {
                return Poll::Ready(Some(value));
            }
        }

        Poll::Pending
    }
}

pub fn socket_addresses_from_host_and_port(hostname: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let addrs = (hostname, port)
        .to_socket_addrs()
        .wrap_err("Failed to convert hostname:port to at least one listening address")?
        .into_iter()
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
        .map(|a| AddrIncoming::bind(a).wrap_err("Failed to bind to address socket"))
        .collect::<Result<Vec<_>>>()?;
    MultipleAddrsIncoming { addrs }.into_ok()
}

pub fn print_listener_addresses(addrs: &[SocketAddr]) {
    let addrs = addrs
        .iter()
        .map(|addr| format!("    http://{addr}"))
        .join("\n");
    info!("Web server listening on the following addresses:\n{addrs}");
}

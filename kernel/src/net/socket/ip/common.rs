// SPDX-License-Identifier: MPL-2.0

use aster_bigtcp::{
    errors::BindError,
    iface::BindPortConfig,
    wire::{IpAddress, IpEndpoint, Ipv4Address, Ipv6Address},
};

use crate::{
    net::{
        iface::{Iface, iter_all_ifaces, loopback_iface, virtio_iface},
        socket::util::check_port_privilege,
    },
    prelude::*,
};

fn get_iface_to_bind(ip_addr: &IpAddress) -> Option<Arc<Iface>> {
    match *ip_addr {
        IpAddress::Ipv4(ipv4_addr) => iter_all_ifaces()
            .find(|iface| {
                iface
                    .ipv4_cidr()
                    .is_some_and(|cidr| cidr.address() == ipv4_addr)
            })
            .map(Clone::clone),
        IpAddress::Ipv6(ipv6_addr) => iter_all_ifaces()
            .find(|iface| {
                iface
                    .ipv6_cidr()
                    .is_some_and(|cidr| cidr.address() == ipv6_addr)
            })
            .map(Clone::clone),
    }
}

fn is_unspecified(ip_addr: &IpAddress) -> bool {
    matches!(
        *ip_addr,
        IpAddress::Ipv4(addr) if addr == Ipv4Address::UNSPECIFIED
    ) || matches!(
        *ip_addr,
        IpAddress::Ipv6(addr) if addr == Ipv6Address::UNSPECIFIED
    )
}

/// Get a suitable iface to deal with sendto/connect request if the socket is not bound to an iface.
/// If the remote address is the same as that of some iface, we will use the iface.
/// Otherwise, we will use a default interface.
fn get_ephemeral_iface(remote_ip_addr: &IpAddress) -> Arc<Iface> {
    match remote_ip_addr {
        IpAddress::Ipv4(remote_ipv4_addr) => {
            if let Some(iface) = iter_all_ifaces().find(|iface| {
                iface
                    .ipv4_cidr()
                    .is_some_and(|cidr| cidr.address() == *remote_ipv4_addr)
            }) {
                return iface.clone();
            }

            // FIXME: Instead of hardcoding the rules here, we should choose the
            // default interface according to the routing table.
            if let Some(virtio_iface) = virtio_iface() {
                virtio_iface.clone()
            } else {
                loopback_iface().clone()
            }
        }
        IpAddress::Ipv6(remote_ipv6_addr) => {
            if let Some(iface) = iter_all_ifaces().find(|iface| {
                iface
                    .ipv6_cidr()
                    .is_some_and(|cidr| cidr.address() == *remote_ipv6_addr)
            }) {
                return iface.clone();
            }

            // Fall back to an interface with an IPv6 address.
            // Prefer virtio over loopback for external traffic.
            if let Some(virtio_iface) = virtio_iface()
                && virtio_iface.ipv6_cidr().is_some()
            {
                return virtio_iface.clone();
            }

            loopback_iface().clone()
        }
    }
}

pub(super) fn resolve_bind_iface_and_config(
    endpoint: &IpEndpoint,
    can_reuse: bool,
) -> Result<(Arc<Iface>, BindPortConfig)> {
    check_port_privilege(endpoint.port)?;

    // BigTCP currently binds a socket to one concrete interface. Treat Linux's
    // wildcard addresses as selecting the default interface for that address
    // family, and materialize the interface address internally so outbound
    // packets never carry an unspecified source address.
    let (iface, bind_endpoint) = if is_unspecified(&endpoint.addr) {
        let iface = get_ephemeral_iface(&endpoint.addr);
        let bind_addr = match endpoint.addr {
            IpAddress::Ipv4(_) => iface
                .ipv4_cidr()
                .map(|cidr| IpAddress::Ipv4(cidr.address())),
            IpAddress::Ipv6(_) => iface
                .ipv6_cidr()
                .map(|cidr| IpAddress::Ipv6(cidr.address())),
        }
        .ok_or_else(|| {
            Error::with_message(
                Errno::EADDRNOTAVAIL,
                "no interface has an address for the specified family",
            )
        })?;
        (iface, IpEndpoint::new(bind_addr, endpoint.port))
    } else {
        let iface = get_iface_to_bind(&endpoint.addr).ok_or_else(|| {
            Error::with_message(
                Errno::EADDRNOTAVAIL,
                "the address is not available from the local machine",
            )
        })?;
        (iface, *endpoint)
    };

    let bind_port_config = BindPortConfig::new(bind_endpoint, can_reuse);

    Ok((iface, bind_port_config))
}

impl From<BindError> for Error {
    fn from(value: BindError) -> Self {
        match value {
            BindError::Exhausted => {
                Error::with_message(Errno::EAGAIN, "no ephemeral port is available")
            }
            BindError::InUse => {
                Error::with_message(Errno::EADDRINUSE, "the address is already in use")
            }
        }
    }
}

pub(super) fn get_ephemeral_endpoint(remote_endpoint: &IpEndpoint) -> Option<IpEndpoint> {
    let iface = get_ephemeral_iface(&remote_endpoint.addr);
    match remote_endpoint.addr {
        IpAddress::Ipv4(_) => {
            let ipv4_cidr = iface.ipv4_cidr()?;
            Some(IpEndpoint::new(IpAddress::Ipv4(ipv4_cidr.address()), 0))
        }
        IpAddress::Ipv6(_) => {
            let ipv6_cidr = iface.ipv6_cidr()?;
            Some(IpEndpoint::new(IpAddress::Ipv6(ipv6_cidr.address()), 0))
        }
    }
}

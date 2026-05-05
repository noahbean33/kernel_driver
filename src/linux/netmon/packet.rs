// SPDX-License-Identifier: GPL-2.0

//! Packet parsing and protocol identification.
//!
//! This module provides structures and functions for extracting protocol
//! information from network packets captured via netfilter hooks.

use crate::skbuff::SkBuff;

/// IP protocol numbers.
const IPPROTO_TCP: u8 = 6;
const IPPROTO_UDP: u8 = 17;
const IPPROTO_ICMP: u8 = 1;

/// EtherType for IPv4.
const ETH_P_IP: u16 = 0x0800;

/// Minimum IPv4 header length (no options).
const IPV4_HEADER_MIN_LEN: usize = 20;

/// Network protocol type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Other(u8),
}

impl Protocol {
    /// Create a Protocol from the raw IP protocol number.
    fn from_ip_proto(proto: u8) -> Self {
        match proto {
            IPPROTO_TCP => Protocol::Tcp,
            IPPROTO_UDP => Protocol::Udp,
            IPPROTO_ICMP => Protocol::Icmp,
            other => Protocol::Other(other),
        }
    }
}

/// Parsed packet information.
#[derive(Debug)]
pub(crate) struct PacketInfo {
    /// Network protocol (TCP, UDP, ICMP, etc.).
    pub protocol: Protocol,
    /// Source IPv4 address (network byte order converted to host).
    pub src_ip: u32,
    /// Destination IPv4 address (network byte order converted to host).
    pub dst_ip: u32,
    /// Source port (0 for protocols without ports like ICMP).
    pub src_port: u16,
    /// Destination port (0 for protocols without ports like ICMP).
    pub dst_port: u16,
}

/// IPv4 header representation (simplified).
#[repr(C, packed)]
struct Ipv4Header {
    /// Version (4 bits) + IHL (4 bits).
    version_ihl: u8,
    /// Type of Service.
    tos: u8,
    /// Total length.
    tot_len: u16,
    /// Identification.
    id: u16,
    /// Fragment offset + flags.
    frag_off: u16,
    /// Time to live.
    ttl: u8,
    /// Protocol.
    protocol: u8,
    /// Header checksum.
    check: u16,
    /// Source address.
    saddr: u32,
    /// Destination address.
    daddr: u32,
}

impl Ipv4Header {
    /// Get the Internet Header Length (in bytes).
    fn ihl(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }
}

/// TCP header representation (first 4 bytes for ports).
#[repr(C, packed)]
struct TcpHeader {
    /// Source port.
    src_port: u16,
    /// Destination port.
    dst_port: u16,
}

/// UDP header representation (first 4 bytes for ports).
#[repr(C, packed)]
struct UdpHeader {
    /// Source port.
    src_port: u16,
    /// Destination port.
    dst_port: u16,
}

impl PacketInfo {
    /// Extract packet information from an sk_buff.
    ///
    /// Returns `None` if the packet is not IPv4 or if the data is too short.
    pub(crate) fn from_skb(skb: &SkBuff) -> Option<Self> {
        let data = skb.data();

        // Check minimum length for an IPv4 header
        if data.len() < IPV4_HEADER_MIN_LEN {
            return None;
        }

        // Parse the IPv4 header
        // SAFETY: We've verified the data length is sufficient.
        let ip_header = unsafe { &*(data.as_ptr() as *const Ipv4Header) };

        // Verify it's IPv4 (version field should be 4)
        let version = (ip_header.version_ihl >> 4) & 0x0F;
        if version != 4 {
            return None;
        }

        let ihl = ip_header.ihl();
        if ihl < IPV4_HEADER_MIN_LEN || data.len() < ihl {
            return None;
        }

        let protocol = Protocol::from_ip_proto(ip_header.protocol);
        let src_ip = u32::from_be(ip_header.saddr);
        let dst_ip = u32::from_be(ip_header.daddr);

        // Extract port information based on protocol
        let (src_port, dst_port) = match protocol {
            Protocol::Tcp => {
                if data.len() < ihl + 4 {
                    return None;
                }
                // SAFETY: We've verified the data length is sufficient.
                let tcp = unsafe { &*(data.as_ptr().add(ihl) as *const TcpHeader) };
                (u16::from_be(tcp.src_port), u16::from_be(tcp.dst_port))
            }
            Protocol::Udp => {
                if data.len() < ihl + 4 {
                    return None;
                }
                // SAFETY: We've verified the data length is sufficient.
                let udp = unsafe { &*(data.as_ptr().add(ihl) as *const UdpHeader) };
                (u16::from_be(udp.src_port), u16::from_be(udp.dst_port))
            }
            _ => (0, 0),
        };

        Some(PacketInfo {
            protocol,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
        })
    }
}

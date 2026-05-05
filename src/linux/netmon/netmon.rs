// SPDX-License-Identifier: GPL-2.0

//! Rust Network Monitoring Kernel Module
//!
//! This module registers a netfilter hook to monitor incoming and outgoing
//! network packets. It supports filtering by protocol (TCP/UDP), IPv4 address,
//! and port number.

use kernel::prelude::*;
use core::pin::Pin;

mod netfilter;
mod skbuff;
mod packet;

use netfilter::{HookResponse, NetFilterHookOps, NfHookOps, ProtocolFamily, HookNumber};
use skbuff::SkBuff;
use packet::{PacketInfo, Protocol};

module! {
    type: NetMon,
    name: "netmon",
    author: "author",
    description: "Network monitoring module written in Rust",
    license: "GPL",
}

/// Configuration for packet filtering.
struct FilterConfig {
    /// Filter by protocol (None = accept all).
    protocol: Option<Protocol>,
    /// Filter by source IP (None = accept all).
    src_ip: Option<u32>,
    /// Filter by destination IP (None = accept all).
    dst_ip: Option<u32>,
    /// Filter by source port (None = accept all).
    src_port: Option<u16>,
    /// Filter by destination port (None = accept all).
    dst_port: Option<u16>,
}

impl FilterConfig {
    /// Create a default config that accepts all packets.
    fn new() -> Self {
        Self {
            protocol: None,
            src_ip: None,
            dst_ip: None,
            src_port: None,
            dst_port: None,
        }
    }

    /// Check if a packet matches the filter criteria.
    fn matches(&self, info: &PacketInfo) -> bool {
        if let Some(proto) = &self.protocol {
            if info.protocol != *proto {
                return false;
            }
        }
        if let Some(src_ip) = self.src_ip {
            if info.src_ip != src_ip {
                return false;
            }
        }
        if let Some(dst_ip) = self.dst_ip {
            if info.dst_ip != dst_ip {
                return false;
            }
        }
        if let Some(src_port) = self.src_port {
            if info.src_port != src_port {
                return false;
            }
        }
        if let Some(dst_port) = self.dst_port {
            if info.dst_port != dst_port {
                return false;
            }
        }
        true
    }
}

/// Structure representing the network monitor kernel module.
struct NetMon {
    /// Netfilter hook operations.
    _nfho: Pin<Box<NetFilterHookOps>>,
}

/// The netfilter hook callback function.
///
/// This function is called for every packet that passes through the registered
/// hook point. It extracts packet information, applies filters, and logs
/// matching packets.
fn hook_func(skb: &SkBuff) -> HookResponse {
    if let Some(info) = PacketInfo::from_skb(skb) {
        let config = FilterConfig::new();
        if config.matches(&info) {
            log_packet(&info, skb);
        }
    }
    HookResponse::Accept
}

/// Log packet information to the kernel log.
fn log_packet(info: &PacketInfo, skb: &SkBuff) {
    let proto_str = match info.protocol {
        Protocol::Tcp => "Tcp",
        Protocol::Udp => "Udp",
        Protocol::Icmp => "Icmp",
        Protocol::Other(n) => {
            pr_info!(
                "Protocol({}): {}.{}.{}.{}:{} -> {}.{}.{}.{}:{}\n",
                n,
                (info.src_ip >> 24) & 0xFF,
                (info.src_ip >> 16) & 0xFF,
                (info.src_ip >> 8) & 0xFF,
                info.src_ip & 0xFF,
                info.src_port,
                (info.dst_ip >> 24) & 0xFF,
                (info.dst_ip >> 16) & 0xFF,
                (info.dst_ip >> 8) & 0xFF,
                info.dst_ip & 0xFF,
                info.dst_port
            );
            print_hex_dump(skb);
            return;
        }
    };

    pr_info!(
        "{}: {}.{}.{}.{}:{} -> {}.{}.{}.{}:{}\n",
        proto_str,
        (info.src_ip >> 24) & 0xFF,
        (info.src_ip >> 16) & 0xFF,
        (info.src_ip >> 8) & 0xFF,
        info.src_ip & 0xFF,
        info.src_port,
        (info.dst_ip >> 24) & 0xFF,
        (info.dst_ip >> 16) & 0xFF,
        (info.dst_ip >> 8) & 0xFF,
        info.dst_ip & 0xFF,
        info.dst_port
    );

    print_hex_dump(skb);
}

/// Print a hex dump of the packet data.
fn print_hex_dump(skb: &SkBuff) {
    pr_info!("Packet hex dump:\n");
    let data = skb.data();
    let len = skb.len().min(64); // Limit dump to 64 bytes

    let mut offset: usize = 0;
    while offset < len {
        let end = (offset + 16).min(len);
        let chunk = &data[offset..end];

        // Format: "OFFSET  XX XX XX XX ..."
        let mut hex_str = [0u8; 48];
        let mut pos = 0;
        for byte in chunk {
            if pos > 0 {
                hex_str[pos] = b' ';
                pos += 1;
            }
            hex_str[pos] = to_hex_digit(byte >> 4);
            hex_str[pos + 1] = to_hex_digit(byte & 0x0F);
            pos += 2;
        }

        pr_info!(
            "{:06X}\t{}\n",
            offset,
            // SAFETY: hex_str contains only valid ASCII hex characters
            unsafe { core::str::from_utf8_unchecked(&hex_str[..pos]) }
        );
        offset += 16;
    }
}

/// Convert a nibble (0-15) to its ASCII hex character.
#[inline]
fn to_hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'A' + (nibble - 10),
    }
}

impl kernel::Module for NetMon {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust Network Monitor (init)\n");

        let nfho = NetFilterHookOps::new(
            hook_func,
            ProtocolFamily::Inet,
            HookNumber::PreRouting,
            i32::MIN, // Highest priority
        )?;

        // Register the hook with the kernel
        let nfho = nfho.register()?;

        Ok(Self { _nfho: nfho })
    }
}

impl Drop for NetMon {
    fn drop(&mut self) {
        pr_info!("Rust Network Monitor (exit)\n");
        // Hook is automatically unregistered when _nfho is dropped.
    }
}

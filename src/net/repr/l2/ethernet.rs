use core::fmt::{Display, Formatter, Result};

pub struct EthernetAddress(pub [u8; 6]);

impl Display for EthernetAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_fmt(format_args!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        ))
    }
}

#[repr(u16)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86dd,
}

pub struct EthernetHeader {
    pub source_address: EthernetAddress,
    pub destination_address: EthernetAddress,
    pub ether_type: EtherType,
}

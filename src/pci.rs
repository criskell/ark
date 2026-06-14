use core::slice;

use crate::{arch::io, pci::EtherType::Arp};

fn read_configuration_register_long(bus: u8, device: u8, function: u8, offset: u32) -> u32 {
    let address = 0x80000000
        | (bus as u32) << 16
        | (device as u32) << 11
        | (function as u32) << 8
        | offset & 0xFFFC;

    unsafe {
        io::outportl(0xCF8, address);
        io::inportl(0xCFC)
    }
}

fn write_configuration_register_long(bus: u8, device: u8, function: u8, offset: u32, value: u32) {
    let address = 0x80000000
        | (bus as u32) << 16
        | (device as u32) << 11
        | (function as u32) << 8
        | offset & 0xFFFC;

    unsafe {
        io::outportl(0xCF8, address);
        io::outportl(0xCFC, value);
    }
}

fn get_vendor_and_device_id(bus: u8, device: u8, function: u8) -> (u16, u16) {
    let register = read_configuration_register_long(bus, device, function, 0);
    let vendor_id = register & 0xFFFF;
    let device_id = register >> 16;

    return (vendor_id as u16, device_id as u16);
}

fn get_header_type(bus: u8, device: u8, function: u8) -> u8 {
    let register = read_configuration_register_long(bus, device, function, 0xC);

    ((register >> 16) & 0xFF) as u8
}

pub fn visit_buses() {
    let header_type = get_header_type(0, 0, 0);

    if header_type & 0x80 == 0 {
        visit_bus(0);
    } else {
        for function in 0..8 {
            let (vendor_id, _) = get_vendor_and_device_id(0, 0, function);

            if vendor_id == 0xFFFF {
                break;
            }

            visit_bus(function);
        }
    }
}

fn visit_bus(bus: u8) {
    for device in 0..32 {
        let (vendor_id, _) = get_vendor_and_device_id(bus, device, 0);

        if vendor_id == 0xFFFF {
            continue;
        }

        visit_function(bus, device, 0);

        let header_type = get_header_type(bus, device, 0);

        if (header_type & 0x80) != 0 {
            for function in 1..8 {
                let (vendor_id, _) = get_vendor_and_device_id(bus, device, function);

                if vendor_id != 0xFFFF {
                    visit_function(bus, device, function);
                }
            }
        }
    }
}

fn visit_function(bus: u8, device: u8, function: u8) {
    let class_register = read_configuration_register_long(bus, device, function, 0x8) >> 16;
    let base_class = class_register >> 8;
    let subclass = class_register & 0xFF;

    if base_class == 0x6 && subclass == 0x4 {
        let secondary_bus_number =
            read_configuration_register_long(bus, device, function, 0x18) >> 8 & 0xFF;

        visit_bus(secondary_bus_number as u8);
    }

    if base_class == 0x02 && subclass == 0x00 {
        let mmio_address =
            read_configuration_register_long(bus, device, function, 0x10) & 0xFFFFFFF0;
        let mac_memory_address = mmio_address + 0x5400;

        unsafe {
            let mmio_ptr = mmio_address as *mut u32;

            // Locate MAC address
            let ral0 = (mac_memory_address as *const u32)
                .read_volatile()
                .to_le_bytes();
            let mut rah0 = ((mac_memory_address + 4) as *const u32)
                .read_volatile()
                .to_le_bytes();

            println!("MAC address:");
            println!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                ral0[0], ral0[1], ral0[2], ral0[3], rah0[0], rah0[1]
            );

            // Enable bus mastering
            let command_with_bus_mastering =
                (read_configuration_register_long(bus, device, function, 0x04) & 0xFFFF) | (1 << 2);

            write_configuration_register_long(
                bus,
                device,
                function,
                0x04,
                command_with_bus_mastering,
            );

            // Reset board
            let ctrl = mmio_ptr.read_volatile();

            // Enable CTRL.RST
            mmio_ptr.write_volatile(ctrl | (1 << 26));

            // Wait the bit disabled
            while mmio_ptr.read_volatile() & 1 << 26 != 0 {}

            let ctrl = mmio_ptr.read_volatile();

            // Enable SLU (bit 6), ASDE (Auto-Speed Detection Enable - bit 5) and reset PHY_RST bit.
            mmio_ptr.write_volatile((ctrl | (1 << 6) | (1 << 5)) & !(1 << 31));

            (mac_memory_address as *mut u32).write_volatile(u32::from_le_bytes(ral0));

            // Address valid
            rah0[3] |= 1 << 7;
            ((mac_memory_address + 4) as *mut u32).write_volatile(u32::from_le_bytes(rah0));

            // Create ring
            for i in 0..32 {
                RECEIVE_RING.0[i].buffer_address = RECEIVE_RING_BUFFERS.0[i].as_ptr() as u64;
            }

            let receive_ring_address = &RECEIVE_RING as *const _ as u64;

            (mmio_ptr.byte_add(0x2800)).write_volatile(receive_ring_address as u32); // RDBAL
            (mmio_ptr.byte_add(0x2804)).write_volatile((receive_ring_address >> 32) as u32); // RDBAH
            (mmio_ptr.byte_add(0x2808)).write_volatile(32 * 16); // RDLEN
            (mmio_ptr.byte_add(0x2810)).write_volatile(0); // RDH
            (mmio_ptr.byte_add(0x2818)).write_volatile(31); // RDT

            // RCTL - Enable reception
            let rctl = (1 << 1) // Enable bit - bit 1
                | (1 << 15) // Broadcast Accept Mode - Bit 15
                | (1 << 26); // SECRC (Strip Ethernet CRC) - Bit 26

            (mmio_ptr.add(0x0100 / 4)).write_volatile(rctl);

            // Enable transmission
            for i in 0..32 {
                TRANSMIT_RING.0[i].buffer_address = TRANSMIT_RING_BUFFERS.0[i].as_ptr() as u64;
            }

            let transmit_ring_address = &TRANSMIT_RING as *const _ as u64;

            (mmio_ptr.byte_add(0x3800)).write_volatile(transmit_ring_address as u32); // TDBAL
            (mmio_ptr.byte_add(0x3804)).write_volatile((transmit_ring_address >> 32) as u32); // TDBAH
            (mmio_ptr.byte_add(0x3808)).write_volatile(32 * 16); // TDLEN
            (mmio_ptr.byte_add(0x3810)).write_volatile(0); // TDH
            (mmio_ptr.byte_add(0x3818)).write_volatile(0); // TDT

            let tctl = (1 << 1) // Enable bit - bit 1
                | (1 << 3) // Pad short packets - bit 3
                | (0x10 << 4) // Collision Threshold - bit 4..11
                | (0x40 << 12); // Collision Distance - bit 12..21

            (mmio_ptr.byte_add(0x0400)).write_volatile(tctl);
            (mmio_ptr.byte_add(0x0408)).write_volatile(0x0060200A); // Transmit Inter Packet Gap

            let mut receive_current = 0;
            let mut transmit_current = 0;
            let mut sent_arp_reply = false;

            loop {
                let descriptor = &mut RECEIVE_RING.0[receive_current];

                if core::ptr::addr_of!(descriptor.status).read_volatile() & 1 == 0 {
                    continue;
                }

                let length = descriptor.length;

                println!("length = {length}");

                let ptr = descriptor.buffer_address as *const u8;

                let slice = slice::from_raw_parts(ptr, 6);
                let mut destination_address = [0; 6];
                destination_address.copy_from_slice(slice);

                let slice = slice::from_raw_parts(ptr.byte_add(6), 6);
                let mut source_address = [0; 6];
                source_address.copy_from_slice(slice);

                let slice = slice::from_raw_parts(ptr.byte_add(12), 2);
                let ether_type = EtherType::from(u16::from_be_bytes(slice.try_into().unwrap()));

                let ethernet_frame = EthernetPacket {
                    ether_type,
                    destination_address,
                    source_address,
                };

                let payload = slice::from_raw_parts(ptr.byte_add(14), (length - 14) as usize);

                if !sent_arp_reply {
                    let arp_packet = ArpPacket {
                        hardware_type: u16::from_be_bytes(payload[0..2].try_into().unwrap()), // Ethernet = 1
                        protocol_type: u16::from_be_bytes(payload[2..4].try_into().unwrap()), // Ipv4 = 0x0800
                        hardware_length: payload[4], // MAC Address = 6
                        protocol_length: payload[5], // IPv4 Address = 4
                        opcode: u16::from_be_bytes(payload[6..8].try_into().unwrap()), // 1 = Request, 2 = Reply
                        sender_hardware_address: payload[8..14].try_into().unwrap(),
                        sender_protocol_address: payload[14..18].try_into().unwrap(),
                        target_hardware_address: payload[18..24].try_into().unwrap(),
                        target_protocol_address: payload[24..28].try_into().unwrap(),
                    };

                    println!(
                        "ethertype: {:?}, destination_address: {:?}, payload: {:?}",
                        ethernet_frame.ether_type, ethernet_frame.destination_address, payload
                    );

                    println!("arp_packet = {:?}", arp_packet);

                    let reply_arp_packet = ArpPacket {
                        hardware_type: 1,      // Ethernet = 1
                        protocol_type: 0x0800, // Ipv4 = 0x0800
                        hardware_length: 6,    // MAC Address = 6
                        protocol_length: 4,    // IPv4 Address = 4
                        opcode: 0x2,           // 1 = Request, 2 = Reply
                        sender_hardware_address: slice::from_raw_parts(
                            mac_memory_address as *const u8,
                            6,
                        )
                        .try_into()
                        .unwrap(),
                        sender_protocol_address: [192, 168, 100, 2], // google.com IPv4 address
                        target_hardware_address: arp_packet.sender_hardware_address,
                        target_protocol_address: arp_packet.sender_protocol_address,
                    };

                    let reply_ethernet_packet = EthernetPacket {
                        destination_address: arp_packet.sender_hardware_address,
                        source_address: slice::from_raw_parts(mac_memory_address as *const u8, 6)
                            .try_into()
                            .unwrap(),
                        ether_type: EtherType::Arp,
                    };

                    let transmit_descriptor = &mut TRANSMIT_RING.0[transmit_current];

                    transmit_descriptor.length = 42; // THE ANSWER FOR EVERYTHING IN THE UNIVERSE!
                    transmit_descriptor.command = 1 /* EOP (End Of Packet) */ | 1 << 1 /* IFCS (Insert Frame Check Sequence) */ | 1 << 3 /* RS (Report Status) */;

                    let buffer_address = transmit_descriptor.buffer_address as *mut u8;
                    let buffer = slice::from_raw_parts_mut(buffer_address, 42);

                    buffer[0..6].copy_from_slice(&reply_ethernet_packet.destination_address);
                    buffer[6..12].copy_from_slice(&reply_ethernet_packet.source_address);
                    buffer[12..14]
                        .copy_from_slice(&(reply_ethernet_packet.ether_type as u16).to_be_bytes());
                    buffer[14..16].copy_from_slice(&reply_arp_packet.hardware_type.to_be_bytes());
                    buffer[16..18].copy_from_slice(&reply_arp_packet.protocol_type.to_be_bytes());
                    buffer[18] = reply_arp_packet.hardware_length;
                    buffer[19] = reply_arp_packet.protocol_length;
                    buffer[20..22].copy_from_slice(&reply_arp_packet.opcode.to_be_bytes());
                    buffer[22..28].copy_from_slice(&reply_arp_packet.sender_hardware_address);
                    buffer[28..32].copy_from_slice(&reply_arp_packet.sender_protocol_address);
                    buffer[32..38].copy_from_slice(&reply_arp_packet.target_hardware_address);
                    buffer[38..42].copy_from_slice(&reply_arp_packet.target_protocol_address);

                    println!("{:?}", buffer);

                    transmit_current = (transmit_current + 1) % 32;

                    // Write TDT
                    (mmio_ptr.byte_add(0x3818)).write_volatile(transmit_current as u32);

                    // sent_arp_reply = true;
                }

                descriptor.length = 0;
                descriptor.status = 0;

                // Write RDT
                (mmio_ptr.byte_add(0x2818)).write_volatile(receive_current as u32);

                receive_current = (receive_current + 1) % 32;
            }
        }
    }
}

#[derive(Debug)]
pub enum EtherType {
    IpV4 = 0x8000,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
}

impl From<u16> for EtherType {
    fn from(value: u16) -> Self {
        match value {
            0x8000 => EtherType::IpV4,
            0x0806 => EtherType::Arp,
            0x86DD => EtherType::Ipv6,
            _ => panic!("Unknown ether type: {value}"),
        }
    }
}

pub struct EthernetPacket {
    pub destination_address: [u8; 6],
    pub source_address: [u8; 6],
    pub ether_type: EtherType,
}

#[derive(Debug)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_length: u8,
    pub protocol_length: u8,
    pub opcode: u16,
    pub sender_hardware_address: [u8; 6],
    pub sender_protocol_address: [u8; 4],
    pub target_hardware_address: [u8; 6],
    pub target_protocol_address: [u8; 4],
}

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct NetworkReceiveDescriptor {
    buffer_address: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

// Allocate ring + buffers
#[repr(C, align(16))]
struct RingBuffers([[u8; 2048]; 32]);

#[repr(align(16))]
struct ReceiveRing([NetworkReceiveDescriptor; 32]);

static mut RECEIVE_RING: ReceiveRing = ReceiveRing(
    [NetworkReceiveDescriptor {
        buffer_address: 0,
        length: 0,
        checksum: 0,
        errors: 0,
        special: 0,
        status: 0,
    }; 32],
);

static RECEIVE_RING_BUFFERS: RingBuffers = RingBuffers([[0u8; 2048]; 32]);

#[repr(align(16))]
struct TransmitRing([NetworkTransmitDescriptor; 32]);

static mut TRANSMIT_RING: TransmitRing = TransmitRing(
    [NetworkTransmitDescriptor {
        buffer_address: 0,
        length: 0,
        checksum_offset: 0,
        command: 0,
        status: 0,
        checksum_start: 0,
        special: 0,
    }; 32],
);

#[repr(C)]
#[derive(Copy, Clone)]
struct NetworkTransmitDescriptor {
    buffer_address: u64,
    length: u16,
    checksum_offset: u8,
    command: u8,
    status: u8,
    checksum_start: u8,
    special: u8,
}

static TRANSMIT_RING_BUFFERS: RingBuffers = RingBuffers([[0u8; 2048]; 32]);

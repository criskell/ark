use crate::arch::io;

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
        let (vendor_id, device_id) = get_vendor_and_device_id(bus, device, 0);

        if vendor_id == 0xFFFF {
            continue;
        }

        println!(
            "vendor_id = {vendor_id:x}, device_id = {device_id:x}, bus = {bus}, device = {device}, function = 0"
        );

        visit_function(bus, device, 0);

        let header_type = get_header_type(bus, device, 0);

        if (header_type & 0x80) != 0 {
            for function in 1..8 {
                let (vendor_id, device_id) = get_vendor_and_device_id(bus, device, function);

                if vendor_id != 0xFFFF {
                    println!(
                        "vendor_id = {vendor_id:x}, device_id = {device_id:x}, bus = {bus}, device = {device}, function = {function}"
                    );

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

    println!("base_class = {base_class}, subclass = {subclass}");

    if base_class == 0x6 && subclass == 0x4 {
        let secondary_bus_number =
            read_configuration_register_long(bus, device, function, 0x18) >> 8 & 0xFF;

        visit_bus(secondary_bus_number as u8);
    }

    if base_class == 0x02 && subclass == 0x00 {
        let mac_memory_address =
            (read_configuration_register_long(bus, device, function, 0x10) & 0xFFFFFFF0) + 0x5400;

        unsafe {
            let ral0 = (*(mac_memory_address as *const u32)).to_le_bytes();
            let rah0 = (*((mac_memory_address + 6) as *const u16)).to_le_bytes();

            println!("MAC address:");
            println!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                ral0[0], ral0[1], ral0[2], ral0[3], rah0[0], rah0[1]
            );
        }
    }
}

use crate::arch::io;

const CONFIG_ADDRESS_PORT: u16 = 0xCF8;
const CONFIG_DATA_PORT: u16 = 0xCFC;

const BRIDGE_DEVICE_CLASS: u8 = 0x6;
const PCI_TO_PCI_BRIDGE_SUBCLASS: u8 = 0x4;

pub struct Device {
    pub is_multi_function: bool,
    pub header_type: u8,
}

pub fn read_configuration_register_long(
    bus: u8,
    device_number: u8,
    function: u8,
    offset: u32,
) -> u32 {
    let address =
        0x80000000 | (bus << 16 | device_number << 11 | function << 8) as u32 | offset & 0xFFFC;

    unsafe {
        io::outportl(CONFIG_ADDRESS_PORT, address);
        io::inportl(CONFIG_DATA_PORT)
    }
}

pub fn visit_bus(bus: u8) {
    for device in 0..32 {
        visit_device(bus, device, 0);
    }
}

fn visit_device(bus: u8, device: u8, function: u8) {
    let vendorId = getVendorId(bus, device, function);

    // Device does not exist.
    if (vendorId == 0xFFFF) {
        return;
    }

    visit_pci_bridge(bus, device, function);

    headerType = getHeaderType(bus, device, function);

    // Device is multi function.
    if ((headerType & 0x80) != 0) {
        for function in 1..8 {
            if (getVendorId(bus, device, function) != 0xFFFF) {
                visit_pci_bridge(bus, device, function);
            }
        }
    }
}

fn visit_pci_bridge(bus: u8, device: u8, function: u8) {
    let pci_base_class = getBaseClass(bus, device, function);
    let pci_sub_class = getSubClass(bus, device, function);

    if (pci_base_class == BRIDGE_DEVICE_CLASS && pci_sub_class == PCI_TO_PCI_BRIDGE_SUBCLASS) {
        let secondary_bus_number = getSecondaryBusNumber(bus, device, function);
        visit_bus(secondary_bus_number);
    }
}

fn visit_buses() {
    let mut bus;
    let header_type = getHeaderType(0, 0, 0);

    if ((header_type & 0x80) == 0) {
        // Single PCI host controller
        // Responsável pelo bus 0
        visit_bus(0);
    } else {
        // temos controllers para cada função
        for function in 0..8 {
            if (getVendorId(0, 0, function) == 0xFFFF) {
                break;
            }

            // é o controller responsável pela função function

            bus = function;
            visit_bus(bus);
        }
    }
}

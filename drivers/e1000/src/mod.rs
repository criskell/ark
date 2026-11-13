fn pci_enable() {}

pub fn init() {
    pci::register_driver(PciDriver {
        matcher: PciMatcher {
            identities: [[VENDOR_ID, DEVICE_ID]],

            baseClass: 0x02,
            subclass: 00,
        },

        enable: pci_enable,
    });
}

pub mod util;

#[repr(transparent)]
pub struct PhysicalAddr(u64);

#[repr(transparent)]
pub struct VirtualAddr(u64);

#[repr(transparent)]
pub struct OffsetMapping(pub u64);

impl OffsetMapping {
    pub fn virtual_to_physical(&self, virtual_addr: u64) -> u64 {
        self.0 + virtual_addr
    }
}

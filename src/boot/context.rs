pub struct MemoryRegion {
    pub start_address: usize,
    pub size: usize,
}

pub struct KernelBootContext {
    pub memory_map: &[MemoryRegion],
}

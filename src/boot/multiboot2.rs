const MULTIBOOT2_END_TAG_TYPE: u32 = 0;
const MULTIBOOT2_MEM_BASICINFO_TAG_TYPE: u32 = 4;

struct Multiboot2Info {
    total_size: u32,
    reserved: u32,
}

struct Multiboot2Tag {
    tag_type: u32,
    size: u32,
}

struct Multiboot2TagBasicMemInfo {
    base: Multiboot2Tag,
    mem_lower: u32,
    mem_upper: u32,
}

#[no_mangle]
pub extern "C" fn multiboot2_kernel_enter(multiboot2_magic: u32, multiboot2_info_addr: u32) {
    if multiboot2_magic != MULTIBOOT2_MAGIC {
        panic!("Invalid multiboot2 magic.");
    }

    if multiboot2_info_addr & 7 != 0 {
        panic!("Multiboot2 info address misaligned.");
    }

    let info_ptr = multiboot2_info_addr as *const Multiboot2Info;

    let mut current_tag_ptr = unsafe { info_ptr.byte_add(core::mem::size_of::<Multiboot2Info>()) as *const Multiboot2Tag };

    loop {
        let current_tag = unsafe { &*current_tag_ptr };

        if current_tag.tag_type == MULTIBOOT2_MEM_BASICINFO_TAG_TYPE {
            break;
        }

        match current_tag.tag_type {
            MULTIBOOT2_MEM_BASICINFO_TAG_TYPE => {
                let mem_tag = unsafe { &*(current_tag_ptr as *const Multiboot2TagBasicMemInfo) };

                mem_tag.mem_upper;
                mem_tag.mem_lower;
            }

            _ => {}
        }

        current_tag_ptr = unsafe { current_tag_ptr.byte_add(current_tag.size as usize) };
    }
}

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;
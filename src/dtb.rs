use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;
use fdt::Fdt;
use uefi::boot::{self, AllocateType, MemoryType, PAGE_SIZE, image_handle};
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode};
use uefi::proto::unsafe_protocol;
use uefi::{CStr16, CString16, Result, Status, StatusExt, cstr16, guid};

const DIRECTORY: &CStr16 = cstr16!(r"\dtbloader\dtbs\qcom");
const DEVICE_TREE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
const FIXUP_SLACK: usize = 16 * 1024;

#[unsafe_protocol("e617d64c-fe08-46da-f4dc-bbd5870c7300")]
#[repr(C)]
struct DtFixup {
    revision: u64,
    fixup: unsafe extern "efiapi" fn(*mut Self, *mut c_void, *mut usize, u32) -> Status,
}

pub struct DeviceTree {
    pub model: String,
    path: CString16,
}

fn read(path: &CStr16) -> Result<Vec<u8>> {
    let mut file_system = boot::get_image_file_system(image_handle())?;
    let mut root = file_system.open_volume()?;
    read_from(&mut root, path)
}

fn read_from(root: &mut Directory, path: &CStr16) -> Result<Vec<u8>> {
    let mut file = root
        .open(path, FileMode::Read, FileAttribute::READ_ONLY)?
        .into_regular_file()
        .ok_or(Status::UNSUPPORTED)?;
    let size = usize::try_from(file.get_boxed_info::<FileInfo>()?.file_size())
        .map_err(|_| Status::BAD_BUFFER_SIZE)?;
    let mut data = vec![0; size];
    let read = file.read(&mut data)?;
    data.truncate(read);
    Ok(data)
}

pub fn available() -> Result<Vec<DeviceTree>> {
    let mut file_system = boot::get_image_file_system(image_handle())?;
    let mut root = file_system.open_volume()?;
    let mut directory = root
        .open(DIRECTORY, FileMode::Read, FileAttribute::READ_ONLY)?
        .into_directory()
        .ok_or(Status::UNSUPPORTED)?;
    let mut trees = Vec::new();

    while let Some(entry) = directory.read_entry_boxed()? {
        if entry.is_directory() {
            continue;
        }
        let name = entry.file_name().to_string();
        if !name.ends_with(".dtb") {
            continue;
        }
        let path = CString16::try_from(format!(r"\dtbloader\dtbs\qcom\{name}").as_str())
            .map_err(|_| Status::INVALID_PARAMETER)?;
        let data = read_from(&mut root, &path)?;
        if let Ok(tree) = Fdt::new(&data) {
            trees.push(DeviceTree {
                model: tree.root().model().to_string(),
                path,
            });
        }
    }

    trees.sort_by(|left, right| left.model.cmp(&right.model));
    Ok(trees)
}

pub fn install(tree: &DeviceTree) -> Result {
    let data = read(&tree.path)?;
    let size = Fdt::new(&data)
        .map_err(|_| Status::LOAD_ERROR)?
        .total_size();
    let pages = (size + FIXUP_SLACK).div_ceil(PAGE_SIZE);
    let allocation = boot::allocate_pages(AllocateType::AnyPages, MemoryType::ACPI_RECLAIM, pages)?;

    unsafe {
        ptr::write_bytes(allocation.as_ptr(), 0, pages * PAGE_SIZE);
        ptr::copy_nonoverlapping(data.as_ptr(), allocation.as_ptr(), size);
    }

    let result = apply_fixups(allocation.as_ptr().cast(), pages * PAGE_SIZE).and_then(|_| unsafe {
        boot::install_configuration_table(&DEVICE_TREE, allocation.as_ptr().cast())
    });
    if result.is_err() {
        unsafe { boot::free_pages(allocation, pages)? };
    }
    result
}

fn apply_fixups(tree: *mut c_void, capacity: usize) -> Result {
    let Ok(handle) = boot::get_handle_for_protocol::<DtFixup>() else {
        return Ok(());
    };
    let mut protocol = boot::open_protocol_exclusive::<DtFixup>(handle)?;
    let mut size = capacity;
    unsafe { (protocol.fixup)(&mut *protocol, tree, &mut size, 1) }.to_result()
}

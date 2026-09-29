#![no_main]
#![no_std]

extern crate alloc;

mod ui;

use alloc::vec::Vec;
use core::time::Duration;
use uefi::boot::{self, LoadImageSource, image_handle};
use uefi::prelude::*;
use uefi::proto::BootPolicy;
use uefi::proto::device_path::{DevicePath, build};
use uefi::proto::loaded_image::LoadedImage;
use uefi::runtime::{self, VariableAttributes, VariableVendor};
use uefi::{CStr16, Result, Status, cstr16, guid};

use ui::Choice;

const DTB_LOADER: &CStr16 = cstr16!(r"\EFI\BOOT\drivers_aa64\adtbloaderaa64.efi");
const SYSTEMD_BOOT: &CStr16 = cstr16!(r"\EFI\systemd\systemd-bootaa64.efi");
const LOADER_VENDOR: VariableVendor = VariableVendor(guid!("4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"));
const TIMEOUT_ONESHOT: &CStr16 = cstr16!("LoaderConfigTimeoutOneShot");

fn load(path: &CStr16) -> Result<Handle> {
    let image = boot::open_protocol_exclusive::<LoadedImage>(image_handle())?;
    let device = image.device().ok_or(Status::NOT_FOUND)?;
    drop(image);

    let device_path = boot::open_protocol_exclusive::<DevicePath>(device)?;
    let mut buffer = Vec::new();
    let mut builder = build::DevicePathBuilder::with_vec(&mut buffer);
    for node in device_path.node_iter() {
        builder = builder.push(&node).map_err(|_| Status::OUT_OF_RESOURCES)?;
    }
    let path = builder
        .push(&build::media::FilePath { path_name: path })
        .and_then(|builder| builder.finalize())
        .map_err(|_| Status::OUT_OF_RESOURCES)?;

    boot::load_image(
        image_handle(),
        LoadImageSource::FromDevicePath {
            device_path: path,
            boot_policy: BootPolicy::ExactMatch,
        },
    )
}

fn run() -> Result {
    if let Ok(driver) = load(DTB_LOADER) {
        let _ = boot::start_image(driver);
    }

    if matches!(ui::menu(), Choice::Advanced) {
        let attributes = VariableAttributes::NON_VOLATILE
            | VariableAttributes::BOOTSERVICE_ACCESS
            | VariableAttributes::RUNTIME_ACCESS;
        runtime::set_variable(TIMEOUT_ONESHOT, &LOADER_VENDOR, attributes, b"0\0\0\0")?;
    }

    ui::clear();
    boot::start_image(load(SYSTEMD_BOOT)?)
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    match run() {
        Ok(()) => Status::SUCCESS,
        Err(error) => {
            uefi::println!("Armada Boot failed: {:?}", error.status());
            boot::stall(Duration::from_secs(5));
            error.status()
        }
    }
}

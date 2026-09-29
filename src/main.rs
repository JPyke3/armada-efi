#![no_main]
#![no_std]

extern crate alloc;

mod dtb;
mod ui;

use alloc::vec::Vec;
use core::time::Duration;
use uefi::boot::{self, LoadImageSource, image_handle};
use uefi::prelude::*;
use uefi::proto::BootPolicy;
use uefi::proto::device_path::{DevicePath, build};
use uefi::proto::loaded_image::LoadedImage;
use uefi::{CStr16, Result, Status, cstr16};

use ui::Choice;

const DTB_LOADER: &CStr16 = cstr16!(r"\EFI\BOOT\drivers_aa64\adtbloaderaa64.efi");
const SYSTEMD_BOOT: &CStr16 = cstr16!(r"\EFI\systemd\systemd-bootaa64.efi");

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

    let mut timed = true;
    loop {
        match ui::menu(timed) {
            Choice::Armada => break,
            Choice::Advanced => {
                let trees = dtb::available()?;
                let models: Vec<_> = trees.iter().map(|tree| tree.model.as_str()).collect();
                if let Some(selected) = ui::device_menu(&models) {
                    dtb::install(&trees[selected])?;
                    break;
                }
                timed = false;
            }
        }
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

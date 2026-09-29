<p align="center">
  <a href="https://armadaos.dev/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset=".github/assets/armada-mark-white.svg">
      <img src=".github/assets/armada-mark-black.svg" alt="Armada" width="112">
    </picture>
  </a>
</p>

<h1 align="center">Armada EFI</h1>

<p align="center"><strong>Graphical UEFI boot manager and Device Tree selector for Armada OS</strong></p>

<p align="center">
  <a href="https://github.com/armada-os/armada-efi/actions/workflows/ci.yml"><img alt="Build status" src="https://github.com/armada-os/armada-efi/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://armadaos.dev/"><img alt="Documentation" src="https://img.shields.io/badge/docs-armadaos.dev-18181a?style=flat"></a>
  <a href="https://discord.gg/HdmdSxTD5S"><img alt="Discord community" src="https://img.shields.io/badge/chat-Discord-5865F2?style=flat&amp;logo=discord&amp;logoColor=white"></a>
</p>

<p align="center">
  <a href="https://armadaos.dev/"><strong>Armada OS</strong></a>
  ·
  <a href="https://armadaos.dev/devices/supported-devices/">Supported devices</a>
  ·
  <a href="https://armadaos.dev/">Documentation</a>
  ·
  <a href="https://discord.gg/HdmdSxTD5S">Discord</a>
</p>

## Overview

**armada-efi** (built as `armada-boot.efi`) is a lightweight `no_std` Rust UEFI boot application for ARM64 gaming handhelds running [Armada OS](https://github.com/armada-os/armada). It acts as the primary UEFI bootloader (`BOOTAA64.EFI`), providing a graphical boot splash, handheld button navigation, and optional Device Tree overrides before handing execution over to `systemd-boot`.

## Features

- **Graphical Boot Interface**: Rendered via UEFI Graphics Output Protocol (GOP) with Armada branding, display rotation detection, and text console fallback.
- **Handheld Controls**: Navigate the boot menu using device volume buttons (`VOL+`/`VOL-`) and `POWER`, or standard keyboard arrows and `Enter`.
- **Timed Boot**: Displays a 3-second countdown for unattended startup, automatically pausing countdown if user input is received.
- **Bootc Rollback**: Offers the previous deployment when available and selects it for one boot through `systemd-boot`.
- **Device Tree Selection**: Browse and select supported Qualcomm Device Trees (`.dtb`) grouped by manufacturer, applying `DtFixup` protocol fixups dynamically.
- **Clean Chainloading**: Loads the `adtbloader` driver to initialize device trees, then chainloads `systemd-boot`.

## Armada Bootchain

On UEFI-based ARM64 handhelds, `armada-efi` sits between the platform firmware and `systemd-boot`:

```text
Firmware (UEFI)
   └─► \EFI\BOOT\BOOTAA64.EFI (armada-boot.efi)
         ├─► Loads \EFI\BOOT\drivers_aa64\adtbloaderaa64.efi
         ├─► Renders boot menu / applies DTB overrides
         └─► Chainloads \EFI\systemd\systemd-bootaa64.efi
               └─► Linux Kernel (with ostree deployment)
```

1. **UEFI Firmware** launches `armada-boot.efi` as the default boot executable (`\EFI\BOOT\BOOTAA64.EFI`).
2. **Armada EFI** starts `adtbloader` to support hardware fixups, displays the splash menu, and allows manual device tree selection if needed.
3. **systemd-boot** is chainloaded to evaluate Boot Loader Specification (BLS) entries and boot the active Armada OS ostree deployment.

## Building

Requires a Rust toolchain with the `aarch64-unknown-uefi` target:

```console
$ rustup target add aarch64-unknown-uefi
$ cargo build --release --target aarch64-unknown-uefi
```

The compiled binary will be placed at `target/aarch64-unknown-uefi/release/armada-boot.efi`.

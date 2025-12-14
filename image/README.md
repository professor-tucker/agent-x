# ph-agent Live Image — Local build instructions

This document explains how to build the bootable ISO locally (Linux) and provides a safe, repeatable container-based method.

Prerequisites (preferred):
- A Linux build host (Ubuntu/Debian recommended) with `sudo` access.
- Or Docker Desktop on Windows with Linux containers enabled.
- Enough disk space (several GB) and internet access.

Quick local build (on Linux):
1. Build the Rust agent binary:

```bash
cd agent
cargo build --release
cd ../image
```

2. Install required packages:

```bash
sudo apt-get update
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin locales \
  syslinux-utils mtools dosfstools
```

3. Run the image builder (this script uses `sudo` internally):

```bash
sudo bash build-image.sh
```

Containerized build (recommended when you don't want to install packages locally):
- The repository root will be mounted into a disposable Ubuntu container and the `build-image.sh` will run there.
- The container must be run privileged to allow `debootstrap`/chroot and some operations used by the script.

Example Docker command (run from Windows PowerShell):

```powershell
docker run --rm --privileged \
  -v "C:/Users/TY-FOON/Documents/vectorsyntax-compliance:/work" \
  -w /work/agent-x/image ubuntu:24.04 \
  bash -lc "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin syslinux-utils mtools dosfstools && sed -i 's/sudo //g' build-image.sh && bash build-image.sh"
```

Safety notes and warnings:
- The build process runs chroot/debootstrap and writes files under the working directory; it requires root or privileged container access.
- Do not run the produced ISO on production hardware without testing it in a VM first.
- The image will auto-start `ph-agent` which contacts your controller and may run workloads; review and set network/agent configuration before booting.
- Running the container with `--privileged` grants wide host-level permissions to the container — only run this command on machines you trust.

If you want, I can run the containerized build now (it will take several minutes). The build will create `ph-agent-live-*.iso` under `agent-x/image` and in CI the ISO will be uploaded as an artifact.
# ph-agent Live Image

This folder contains a helper script to build a bootable Live ISO that automatically runs the `ph-agent` binary.

Important notes
- The build script uses `debootstrap` to create a minimal Debian filesystem. Run it on a Linux builder (WSL2 or a Linux VM recommended).
- You must compile the agent first: `cd ../agent && cargo build --release`. The script expects `../agent/target/release/ph-agent`.
- The script requires root privileges and several build packages. On Debian/Ubuntu install:

```bash
sudo apt update
sudo apt install -y debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin mtools
```

How to build

```bash
cd pooled-hypervisor/image
sudo ./build-image.sh
```

The generated ISO will be `ph-agent-live-<release>.iso`. Test in a VM (QEMU/VirtualBox) before using on real hardware.

Dual-boot considerations
- Booting this image from USB will run the live environment; it does not modify the host disk unless you add installers.
- For safe dual-boot usage: boot from USB and run agent temporarily, or adapt the image to provide an installer that writes a partition.

Security and deployment
- The live image is intended for ephemeral agent execution. For BYOD scenarios, require explicit user consent and provide opt-in controls.
- For production, add secure bootstrap (signed images, attestations, mTLS certificates) and audit logging.

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

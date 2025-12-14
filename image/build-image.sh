#!/usr/bin/env bash
set -euo pipefail

# Build a minimal Debian/Ubuntu-based live ISO that auto-starts the ph-agent.
# Requirements (run on a Linux build host with sudo):
# sudo apt install -y debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin schroot
# Run this script from the pooled-hypervisor/image directory.

RELEASE=${RELEASE:-bookworm}
ARCH=${ARCH:-amd64}
CHROOT_DIR="chroot-root"
IMAGE_DIR="iso-root"
IMAGE_NAME="ph-agent-live-${RELEASE}.iso"

echo "Building live image for ${RELEASE} (${ARCH})"

sudo rm -rf "$CHROOT_DIR" "$IMAGE_DIR" "$IMAGE_NAME"
mkdir -p "$CHROOT_DIR"

echo "Bootstrapping base system..."
sudo debootstrap --arch=$ARCH --variant=minbase $RELEASE "$CHROOT_DIR" http://deb.debian.org/debian/

echo "Mounting proc/sys/dev for chroot..."
sudo mount --bind /dev $CHROOT_DIR/dev
sudo mount --bind /run $CHROOT_DIR/run || true
sudo mount -t proc /proc $CHROOT_DIR/proc
sudo mount -t sysfs /sys $CHROOT_DIR/sys

cat <<'EOF' | sudo chroot $CHROOT_DIR /bin/bash -s
set -e
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  systemd-sysv dbus ca-certificates curl iproute2 iputils-ping \
  containerd runc squashfs-tools initramfs-tools grub-pc-bin grub-efi-amd64-bin \
  locales
locale-gen en_US.UTF-8
apt-get clean
rm -rf /var/lib/apt/lists/*
EOF

echo "Copying ph-agent binary into image (build it first)
Place the compiled agent at ../agent/target/release/ph-agent"
if [ ! -f ../agent/target/release/ph-agent ]; then
  echo "Error: compiled agent not found at ../agent/target/release/ph-agent"
  echo "Build the agent with: (cd ../agent && cargo build --release)"
  exit 1
fi

sudo mkdir -p $CHROOT_DIR/usr/local/bin
sudo cp ../agent/target/release/ph-agent $CHROOT_DIR/usr/local/bin/
sudo chmod +x $CHROOT_DIR/usr/local/bin/ph-agent

echo "Installing ph-agent systemd service..."
sudo mkdir -p $CHROOT_DIR/etc/systemd/system
sudo tee $CHROOT_DIR/etc/systemd/system/ph-agent.service > /dev/null <<'SERVICE'
[Unit]
Description=ph-agent (pooled-hypervisor agent)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/ph-agent
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
SERVICE

sudo chroot $CHROOT_DIR /bin/bash -lc "systemctl enable ph-agent.service"

echo "Preparing ISO root..."
rm -rf $IMAGE_DIR
mkdir -p $IMAGE_DIR/live

echo "Creating squashfs filesystem..."
sudo mksquashfs $CHROOT_DIR $IMAGE_DIR/live/filesystem.squashfs -comp xz -e boot

echo "Copying kernel/initrd from chroot (if available)..."
KERNEL=$(ls $CHROOT_DIR/boot/vmlinuz-* 2>/dev/null | head -n1 || true)
INITRD=$(ls $CHROOT_DIR/boot/initrd.img-* 2>/dev/null | head -n1 || true)
if [ -n "$KERNEL" ] && [ -n "$INITRD" ]; then
  sudo cp "$KERNEL" $IMAGE_DIR/vmlinuz
  sudo cp "$INITRD" $IMAGE_DIR/initrd.img
fi

cat > $IMAGE_DIR/README.txt <<'TXT'
ph-agent live image
Boot this image to run the pooled-hypervisor agent on a host. The agent will start automatically.
TXT

echo "Generating ISO with GRUB..."
mkdir -p iso-tmp/boot/grub
cat > iso-tmp/boot/grub/grub.cfg <<'GRUB'
set default=0
set timeout=5
menuentry "ph-agent live" {
    linux /vmlinuz boot=live
    initrd /initrd.img
}
GRUB

cp -r $IMAGE_DIR/* iso-tmp/ || true

sudo grub-mkrescue -o "$IMAGE_NAME" iso-tmp || {
  echo "grub-mkrescue failed; you may need additional packages or run this on a Linux host with grub tools installed."
}

echo "Cleaning up mounts..."
sudo umount $CHROOT_DIR/proc || true
sudo umount $CHROOT_DIR/sys || true
sudo umount $CHROOT_DIR/dev || true
sudo umount $CHROOT_DIR/run || true

echo "ISO created: $IMAGE_NAME"
echo "Test it in a VM before using on real hardware."

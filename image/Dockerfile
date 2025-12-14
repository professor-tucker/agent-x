FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl apt-transport-https gnupg2 software-properties-common \
    debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin \
    syslinux-utils mtools dosfstools build-essential git sudo \
    docker.io \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /work

COPY .. /work

ENV CONTAINER_BUILD=1

# Ensure build script is executable
RUN chmod +x /work/agent-x/image/build-image.sh

CMD ["/bin/bash"]

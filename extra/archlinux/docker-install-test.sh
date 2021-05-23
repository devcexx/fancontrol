#!/bin/bash

# This file only is a helper for testing that the PKGBUILD file works
# in a fresh ArchLinux environment without any deps installed.
docker run -it --rm -e "LET_ME_INTERACT=$LET_ME_INTERACT" -v "$(pwd)/../../:/host:ro" archlinux bash -c '
       pacman --noconfirm -Sy fakeroot binutils sudo && \
       useradd -m builder && \
       echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers && \
       mkdir -p /build && \
       cp -r /host/{extra,src,Cargo.*,*.rs} /build && \
       chown -R builder:builder /build && \
       cd /build/extra/archlinux && \
       su builder -c "makepkg --noconfirm -si" && \
       (test "x$LET_ME_INTERACT" "==" "x1" && bash || true)'

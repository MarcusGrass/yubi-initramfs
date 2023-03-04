#!/bin/sh
# init generator, should probably be done by root, but whatever
set -ex
DIR=yubi-initramfs-generated
if [ -d $DIR ];
then
  mkdir -p "$DIR"
else
  rm -r "$DIR"
fi

set -e
# Create some dirs that we're going to need
# Cryptsetup needs 'run' to exist
mkdir -p "$DIR"/{bin,dev,lib64,mnt/root,proc,run,sbin,sys}
# The binary is 'statically' linked because rust creates an ELF that needs the linux linker even though it's not
# really needed. So we copy the linker into it
cp /lib64/ld-linux-x86-64.so.2 "$DIR"/lib64

# We also need a statically linked busybox
cp $(which busybox) "$DIR"/bin/busybox

# We need cryptsetup for Luks decrypt
cp $(which cryptsetup) "$DIR"/sbin/cryptsetup
# We need to proxy blkid into sbin
echo "#!/bin/busybox sh
blkid" > "$DIR"/sbin/blkid && chmod +x "$DIR"/sbin/blkid
BINARY=target/x86_64-unknown-linux-gnu/lto/yubi-initramfs
# Build the file without any target cpu
# RUSTFLAGS='-C panic=abort -C link-arg=-nostartfiles' cargo b -p yubi-initramfs --target x86_64-unknown-linux-gnu --profile lto
# Name the binary the init default (which is just init)
cp "$BINARY" "$DIR"/init

# Just execute the binary by proxy
cp --archive /dev/{null,console,tty} "$DIR"/dev

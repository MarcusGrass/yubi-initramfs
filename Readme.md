# Initramfs generator/binary/runner
WIP, the name comes from yubikey initramfs, the initial
idea was to use a yubikey to emit a secret that decrypts 
disks on boot.  

Now I'm thinking that it might be better to encrypt the secrets directly 
into the initramfs and decrypt that (maybe using a Yubikey) with a bootloader.  

The yubikey parts of that still needs revising, figuring out how to talk to a 
USB-device using the Linux APIs were difficult enough, and that's with quick feedback, 
going through that with EFI might be beyond reason, even for me.  

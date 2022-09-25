.PHONY: all clean run install love

RUST_ARCH	= x86_64-unknown-uefi
TARGET		= target/$(RUST_ARCH)/release/*.efi
OVMF		= var/ovmfx64.fd

MNT			= ./mnt
EFI_BOOT	= $(MNT)/EFI/BOOT

all: $(TARGET)

clean:
	-rm -rf target

$(TARGET): **/src/** lib/**/src/**
	cargo build --release --target x86_64-unknown-uefi --target i686-unknown-uefi --target aarch64-unknown-uefi

$(EFI_BOOT):
	mkdir -p $(EFI_BOOT)

install: $(TARGET) $(EFI_BOOT)
	cp target/$(RUST_ARCH)/release/*.efi $(EFI_BOOT)

run: install $(OVMF)
	qemu-system-x86_64 -bios $(OVMF) -drive format=raw,file=fat:rw:$(MNT) -monitor stdio

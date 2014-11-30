all: os.iso

run: os.iso
	@qemu-system-x86_64 -hda os.iso

clean:
	@rm -f os.iso isofiles/boot/os.bin bin/loader.o

os.iso: isofiles/boot/os.bin
	@grub-mkrescue -o os.iso isofiles

lib_rustos = $(shell ls target | grep librustos | head -1)

isofiles/boot/os.bin: rustos src/x86_64/linker.ld bin/loader.o bin/handlers.o 
	@ld -T src/x86_64/linker.ld bin/loader.o bin/handlers.o target/$(lib_rustos) -o isofiles/boot/os.bin

bin/loader.o: src/x86_64/loader.asm
	@nasm -felf64 src/x86_64/loader.asm -o bin/loader.o

bin/handlers.o: src/x86_64/handlers.asm
	@nasm -felf64 src/x86_64/handlers.asm -o bin/handlers.o

rustos:
	@cargo build	
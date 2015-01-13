all: os.iso

run: os.iso
	@qemu-system-x86_64 -hda os.iso

clean:
	@rm -f os.iso isofiles/boot/os.bin bin/loader.o

os.iso: isofiles/boot/os.bin
	@grub-mkrescue -o os.iso isofiles

lib_rustos = $(shell ls target | grep librustos | head -1)

isofiles/boot/os.bin: rustos arch/x86_64/assembly/linker.ld bin/loader.o bin/handlers.o 
	@ld -T arch/x86_64/assembly/linker.ld bin/loader.o bin/handlers.o target/$(lib_rustos) -o isofiles/boot/os.bin

bin/loader.o: arch/x86_64/assembly/loader.asm
	@nasm -felf64 arch/x86_64/assembly/loader.asm -o bin/loader.o

bin/handlers.o: arch/x86_64/assembly/handlers.asm
	@nasm -felf64 arch/x86_64/assembly/handlers.asm -o bin/handlers.o

rustos:
	@cargo build	
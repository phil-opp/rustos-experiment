all: os.iso

run: os.iso
	@qemu-system-x86_64 -hda os.iso

clean:
	@rm -f os.iso isofiles/boot/os.bin bin/loader.o

os.iso: isofiles/boot/os.bin
	@grub-mkrescue isofiles -o os.iso

isofiles/boot/os.bin: rustos src/x86_64/linker.ld bin/loader.o bin/handlers.o 
	@ld -T src/x86_64/linker.ld bin/loader.o bin/handlers.o target/librustos5-ecf1c983669218b9.a -o isofiles/boot/os.bin

bin/loader.o: src/x86_64/loader.asm
	@nasm -felf64 src/x86_64/loader.asm -o bin/loader.o

bin/handlers.o: src/x86_64/handlers.asm
	@nasm -felf64 src/x86_64/handlers.asm -o bin/handlers.o

rustos:
	cargo build	
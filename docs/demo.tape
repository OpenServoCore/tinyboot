Output docs/demo.gif

Set Shell "bash"
Set Width 800
Set Height 400
Set FontSize 14
Set Padding 20
Set Theme "Dracula"

Type "size examples/ch32/system-flash/target/riscv32ec-unknown-none-elf/release/boot"
Enter
Sleep 3s

Type "echo 'Bootloader: 1874 bytes — fits in 1920-byte system flash, all 16KB free for app'"
Enter
Sleep 3s

Ctrl+L
Sleep 500ms

Type "tinyboot info --port /dev/ttyACM0"
Enter
Sleep 2s

Ctrl+L
Sleep 500ms

Type "tinyboot flash --reset --port /dev/ttyACM0 examples/ch32/system-flash/target/riscv32ec-unknown-none-elf/release/app"
Enter
Sleep 30s

Ctrl+L
Sleep 500ms

Type "tinyboot info --port /dev/ttyACM0"
Enter
Sleep 3s

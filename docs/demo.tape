Output docs/demo.gif

Set Shell "bash"
Set Width 1200
Set Height 600
Set FontSize 14
Set Padding 20
Set Theme "Dracula"

Type "rust-size examples/ch32/system-flash/target/riscv32ec-unknown-none-elf/release/boot"
Enter
Sleep 3s

Type "echo '1920 bytes — fits exactly in system flash, all 16KB free for app'"
Enter
Sleep 3s

Ctrl+L
Sleep 500ms

Type "tinyboot info"
Enter
Sleep 2s

Ctrl+L
Sleep 500ms

Type "tinyboot flash --reset examples/ch32/system-flash/target/riscv32ec-unknown-none-elf/release/app"
Enter
Sleep 5s

Ctrl+L
Sleep 500ms

Type "tinyboot info"
Enter
Sleep 3s

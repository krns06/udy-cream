use emulator::Rv64SGEmulator;

mod emulator;

fn main() {
    let mut rv64sg_emulator = Rv64SGEmulator::load_from_filename(
        0,
        4096,
        1024 * 1024 * 4,
        "rv64-tests/share/riscv-tests/isa/rv64uf-p-recoding.bin",
    )
    .unwrap();
    rv64sg_emulator.exec_program(0x4c);
}

#[cfg(test)]
mod tests {
    use crate::emulator::Rv64SGEmulator;

    const TEST_DIR: &str = "rv64-tests/share/riscv-tests/isa/";

    fn test_exec_program(filename: &str, end_point: u64, addrres: usize, value: u64) {
        let mut rv64sg_emulator = Rv64SGEmulator::load_from_filename(
            0,
            4096,
            1024 * 1024 * 4,
            &format!("{}{}", TEST_DIR, filename),
        )
        .unwrap();

        rv64sg_emulator.exec_program(end_point);
        assert!(rv64sg_emulator.load_memory_64bit(addrres).unwrap() == value);
    }

    #[test]
    fn rv64ui_p_all() {
        test_exec_program("rv64ui-p-add.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-addi.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-addiw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-and.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-andi.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-auipc.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-beq.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-bge.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-bgeu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-blt.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-bltu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-bne.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-fence_i.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-jal.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-jalr.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-jalr.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lb.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lbu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-ld.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lh.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lhu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lui.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-lwu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-ma_data.bin", 0x4c, 0x2000, 1);
        test_exec_program("rv64ui-p-or.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-ori.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sb.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sd.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sh.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-simple.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sll.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-slli.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-slliw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sllw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-slt.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-slti.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sltiu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sltu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sra.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-srai.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sraiw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sraw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-srl.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-srli.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-srliw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-srlw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sub.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-subw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-sw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-xor.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ui-p-xori.bin", 0x4c, 0x1000, 1);
    }

    #[test]
    fn rv64um_p_all() {
        test_exec_program("rv64um-p-div.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-divu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-divuw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-divw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-mul.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-mulh.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-mulhsu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-mulhu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-mulw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-rem.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-remu.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-remuw.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64um-p-remw.bin", 0x4c, 0x1000, 1);
    }

    #[test]
    fn rv64ud_p_all() {
        test_exec_program("rv64ud-p-fadd.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fclass.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fcmp.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fcvt.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fcvt_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fdiv.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fmadd.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-fmin.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-ldst.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-move.bin", 0x4c, 0x2000, 1);
        test_exec_program("rv64ud-p-recoding.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ud-p-structural.bin", 0x4c, 0x1000, 1);
    }

    #[test]
    fn rv64uf_p_all() {
        test_exec_program("rv64uf-p-fadd.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fclass.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fcmp.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fcvt.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fcvt_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fmadd.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-fmin.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-ldst.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-move.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64uf-p-recoding.bin", 0x4c, 0x1000, 1);
    }

    #[test]
    fn rv64ua_p_all() {
        test_exec_program("rv64ua-p-amoadd_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoadd_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoand_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoand_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomax_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomax_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomaxu_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomaxu_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomin_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amomin_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amominu_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amominu_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoor_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoor_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoswap_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoswap_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoxor_d.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-amoxor_w.bin", 0x4c, 0x1000, 1);
        test_exec_program("rv64ua-p-lrsc.bin", 0x4c, 0x1000, 1);
    }
}

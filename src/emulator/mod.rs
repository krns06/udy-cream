mod emulator_tests;
mod helpers;

use std::{
    fs::File,
    io::{self, Read},
};

use softfloat_wrapper::{ExceptionFlags, Float, F32, F64};

use self::helpers::{
    extend_sign_128bit, extend_sign_12bit, extend_sign_13bit, extend_sign_16bit, extend_sign_21bit,
    extend_sign_32bit, extend_sign_8bit, extend_sign_n, extract_csr, extract_funct3,
    extract_funct7, extract_imm_11_0, extract_imm_31_12, extract_offset_11_0,
    extract_offset_11_5_4_0, extract_offset_12_10_5_4_1_11, extract_rd, extract_rm, extract_rs1,
    extract_rs2, extract_rs3, extract_shamt, extract_zimm, is_nan_boxing, nan_boxing, rm_to_swrm,
    swef_to_fflags, truncate_top_16bit, truncate_top_32bit,
};

pub struct Rv64SGEmulator {
    memory: Vec<u8>,
    preserved_memory: Option<(usize, usize)>,
    registers: [u64; 32],
    f_registers: [u64; 32],
    csrs: [u64; 4096],
    pc: u64,
    mode: MachineMode,
}

impl Rv64SGEmulator {
    pub fn load_from_filename(
        entry: u64,
        sp: u64,
        memsz: usize,
        filename: &str,
    ) -> io::Result<Self> {
        let mut file_obj = File::open(filename)?;
        let mut buf = vec![0; memsz];

        file_obj.read(&mut buf)?;
        let mut rv64sg_emulator = Rv64SGEmulator {
            memory: buf,
            preserved_memory: None,
            registers: [0; 32],
            f_registers: [0; 32],
            csrs: [0; 4096],
            mode: MachineMode::M,
            pc: entry,
        };

        rv64sg_emulator.registers[2] = sp;

        Ok(rv64sg_emulator)
    }
}

fn print_not_implement(what: String) {
    println!("Error: not implemented\n{}", what);
}

impl Rv64SGEmulator {
    fn fetch_instraction(&self) -> Vec<u8> {
        let mut instruction = vec![0; 4];
        instruction.clone_from_slice(&self.memory[self.pc as usize..self.pc as usize + 4]);

        instruction
    }

    // どの命令を実行するか判定し実行する関数
    // 将来的には最初にC拡張か判定し次に他の命令か判定する。
    // 例外が発生した場合、すぐにNoneを返す。
    // またこの関数内で例外が発生した場合（不正な命令等）set_exception_causeに理由を引数にしてすぐに返す。
    // printlnとかはデバッグが終わったら消す。
    fn decode_and_exec(&mut self, instruction: Vec<u8>) -> Option<()> {
        match instruction[0] & 0x7f {
            0x3 => match extract_funct3(&instruction) {
                0 => self.lb(&instruction),
                1 => self.lh(&instruction),
                2 => self.lw(&instruction),
                3 => self.ld(&instruction),
                4 => self.lbu(&instruction),
                5 => self.lhu(&instruction),
                6 => self.lwu(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x3, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x7 => match extract_funct3(&instruction) {
                2 => self.f_lw(&instruction),
                3 => self.f_ld(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x7, funct3));
                    self.set_exception_cause(2)
                }
            },
            0xf => match extract_funct3(&instruction) {
                0 => self.fence(&instruction),
                1 => self.fence_i(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0xf, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x13 => match extract_funct3(&instruction) {
                0 => self.addi(&instruction),
                1 => match instruction[3] >> 2 {
                    0 => self.slli(&instruction),
                    b_26_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 26-31bit: {:x}",
                            0x13, 1, b_26_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                2 => self.slti(&instruction),
                3 => self.sltiu(&instruction),
                4 => self.xori(&instruction),
                5 => match instruction[3] >> 2 {
                    0 => self.srli(&instruction),
                    0x10 => self.srai(&instruction),
                    b_26_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 26-31bit: {:x}",
                            0x13, 5, b_26_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                6 => self.ori(&instruction),
                7 => self.andi(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x13, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x17 => self.auipc(&instruction),
            0x1b => match extract_funct3(&instruction) {
                0 => self.addiw(&instruction),
                1 => match instruction[3] >> 2 {
                    0 => self.slliw(&instruction),
                    b_26_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 26-31bit: {:x}",
                            0x1b, 1, b_26_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                5 => match instruction[3] >> 2 {
                    0 => self.srliw(&instruction),
                    0x10 => self.sraiw(&instruction),
                    b_26_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 26-31bit: {:x}",
                            0x1b, 5, b_26_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x1b, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x23 => match extract_funct3(&instruction) {
                0 => self.sb(&instruction),
                1 => self.sh(&instruction),
                2 => self.sw(&instruction),
                3 => self.sd(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x23, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x27 => match extract_funct3(&instruction) {
                2 => self.f_sw(&instruction),
                3 => self.f_sd(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x27, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x2f => match extract_funct3(&instruction) {
                2 => match instruction[3] >> 3 {
                    0 => self.a_moadd_w(&instruction),
                    0x1 => self.a_moswap_w(&instruction),
                    0x2 => self.a_lr_w(&instruction),
                    0x3 => self.a_sc_w(&instruction),
                    0x4 => self.a_moxor_w(&instruction),
                    0x8 => self.a_moor_w(&instruction),
                    0xc => self.a_moand_w(&instruction),
                    0x10 => self.a_momin_w(&instruction),
                    0x14 => self.a_momax_w(&instruction),
                    0x18 => self.a_mominu_w(&instruction),
                    0x1c => self.a_momaxu_w(&instruction),
                    b_27_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 27-31bit: {:x}",
                            0x2f, 2, b_27_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                3 => match instruction[3] >> 3 {
                    0 => self.a_moadd_d(&instruction),
                    0x1 => self.a_moswap_d(&instruction),
                    0x4 => self.a_moxor_d(&instruction),
                    0x8 => self.a_moor_d(&instruction),
                    0xc => self.a_moand_d(&instruction),
                    0x10 => self.a_momin_d(&instruction),
                    0x14 => self.a_momax_d(&instruction),
                    0x18 => self.a_mominu_d(&instruction),
                    0x1c => self.a_momaxu_d(&instruction),
                    b_27_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 27-31bit: {:x}",
                            0x2f, 3, b_27_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x2f, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x33 => match extract_funct3(&instruction) {
                0 => match instruction[3] >> 1 {
                    0 => self.add(&instruction),
                    1 => self.mul(&instruction),
                    0x20 => self.sub(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 0, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                1 => match instruction[3] >> 1 {
                    0 => self.sll(&instruction),
                    1 => self.mulh(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 1, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                2 => match instruction[3] >> 1 {
                    0 => self.slt(&instruction),
                    1 => self.mulhsu(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 2, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                3 => match instruction[3] >> 1 {
                    0 => self.sltu(&instruction),
                    1 => self.mulhu(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 3, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                4 => match instruction[3] >> 1 {
                    0 => self.xor(&instruction),
                    1 => self.div(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 4, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                5 => match instruction[3] >> 1 {
                    0 => self.srl(&instruction),
                    1 => self.divu(&instruction),
                    0x20 => self.sra(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 5, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                6 => match instruction[3] >> 1 {
                    0 => self.or(&instruction),
                    1 => self.rem(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 6, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                7 => match instruction[3] >> 1 {
                    0 => self.and(&instruction),
                    1 => self.remu(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x33, 7, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x33, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x37 => self.lui(&instruction),
            0x3b => match extract_funct3(&instruction) {
                0 => match instruction[3] >> 1 {
                    0 => self.addw(&instruction),
                    1 => self.mulw(&instruction),
                    0x20 => self.subw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 0, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                1 => match instruction[3] >> 1 {
                    0 => self.sllw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 1, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                4 => match instruction[3] >> 1 {
                    1 => self.divw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 4, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                5 => match instruction[3] >> 1 {
                    0 => self.srlw(&instruction),
                    1 => self.divuw(&instruction),
                    0x20 => self.sraw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 5, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                6 => match instruction[3] >> 1 {
                    1 => self.remw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 6, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                7 => match instruction[3] >> 1 {
                    1 => self.remuw(&instruction),
                    b_25_31 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} 25-31bit: {:x}",
                            0x3b, 7, b_25_31
                        ));
                        self.set_exception_cause(2)
                    }
                },
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x3b, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x43 => match (instruction[3] & 0x6) >> 1 {
                0 => self.f_madd_s(&instruction),
                1 => self.f_madd_d(&instruction),
                b_25_26 => {
                    print_not_implement(format!("op: {:x} 25-26bit: {:x}", 0x43, b_25_26));
                    self.set_exception_cause(2)
                }
            },
            0x47 => match (instruction[3] & 0x6) >> 1 {
                0 => self.f_msub_s(&instruction),
                1 => self.f_msub_d(&instruction),
                b_25_26 => {
                    print_not_implement(format!("op: {:x} 25-26bit: {:x}", 0x47, b_25_26));
                    self.set_exception_cause(2)
                }
            },
            0x4b => match (instruction[3] & 0x6) >> 1 {
                0 => self.f_nmsub_s(&instruction),
                1 => self.f_nmsub_d(&instruction),
                b_25_26 => {
                    print_not_implement(format!("op: {:x} 25-26bit: {:x}", 0x4b, b_25_26));
                    self.set_exception_cause(2)
                }
            },
            0x4f => match (instruction[3] & 0x6) >> 1 {
                0 => self.f_nmadd_s(&instruction),
                1 => self.f_nmadd_d(&instruction),
                b_25_26 => {
                    print_not_implement(format!("op: {:x} 25-26bit: {:x}", 0x4f, b_25_26));
                    self.set_exception_cause(2)
                }
            },
            0x53 => match extract_funct7(&instruction) {
                0 => self.f_add_s(&instruction),
                1 => self.f_add_d(&instruction),
                4 => self.f_sub_s(&instruction),
                5 => self.f_sub_d(&instruction),
                8 => self.f_mul_s(&instruction),
                9 => self.f_mul_d(&instruction),
                0xd => self.f_div_d(&instruction),
                0x10 => match extract_funct3(&instruction) {
                    0 => self.f_sgnj_s(&instruction),
                    1 => self.f_sgnjn_s(&instruction),
                    2 => self.f_sgnjx_s(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x10
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x11 => match extract_funct3(&instruction) {
                    0 => self.f_sgnj_d(&instruction),
                    1 => self.f_sgnjn_d(&instruction),
                    2 => self.f_sgnjx_d(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x11
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x14 => match extract_funct3(&instruction) {
                    0 => self.f_min_s(&instruction),
                    1 => self.f_max_s(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x14
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x15 => match extract_funct3(&instruction) {
                    0 => self.f_min_d(&instruction),
                    1 => self.f_max_d(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x15
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x20 => match extract_rs2(&instruction) {
                    1 => self.f_cvt_s_d(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, 0x20, rs2
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x21 => match extract_rs2(&instruction) {
                    0 => self.f_cvt_d_s(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, 0x21, rs2
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x2d => match extract_rs2(&instruction) {
                    0 => self.f_sqrt_d(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, 0x2d, rs2
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x50 => match extract_funct3(&instruction) {
                    0 => self.f_le_s(&instruction),
                    1 => self.f_lt_s(&instruction),
                    2 => self.f_eq_s(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x50
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x51 => match extract_funct3(&instruction) {
                    0 => self.f_le_d(&instruction),
                    1 => self.f_lt_d(&instruction),
                    2 => self.f_eq_d(&instruction),
                    funct3 => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} funct7: {:x}",
                            0x53, funct3, 0x51
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x60 => match extract_rs2(&instruction) {
                    0 => self.f_cvt_w_s(&instruction),
                    1 => self.f_cvt_wu_s(&instruction),
                    2 => self.f_cvt_l_s(&instruction),
                    3 => self.f_cvt_lu_s(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, rs2, 0x60
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x61 => match extract_rs2(&instruction) {
                    0 => self.f_cvt_w_d(&instruction),
                    1 => self.f_cvt_wu_d(&instruction),
                    2 => self.f_cvt_l_d(&instruction),
                    3 => self.f_cvt_lu_d(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, rs2, 0x61
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x68 => match extract_rs2(&instruction) {
                    0 => self.f_cvt_s_w(&instruction),
                    1 => self.f_cvt_s_wu(&instruction),
                    2 => self.f_cvt_s_l(&instruction),
                    3 => self.f_cvt_s_lu(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, rs2, 0x68
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x69 => match extract_rs2(&instruction) {
                    0 => self.f_cvt_d_w(&instruction),
                    1 => self.f_cvt_d_wu(&instruction),
                    2 => self.f_cvt_d_l(&instruction),
                    3 => self.f_cvt_d_lu(&instruction),
                    rs2 => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct7: {:x}",
                            0x53, rs2, 0x69
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x70 => match (extract_rs2(&instruction), extract_funct3(&instruction)) {
                    (0, 0) => self.f_mv_x_w(&instruction),
                    (0, 1) => self.f_class_s(&instruction),
                    (rs2, funct3) => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct3: {:x} funct7: {:x}",
                            0x53, rs2, funct3, 0x70
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x71 => match (extract_rs2(&instruction), extract_funct3(&instruction)) {
                    (0, 0) => self.f_mv_x_d(&instruction),
                    (0, 1) => self.f_class_d(&instruction),
                    (rs2, funct3) => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct3: {:x} funct7: {:x}",
                            0x53, rs2, funct3, 0x71
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x78 => match (extract_rs2(&instruction), extract_funct3(&instruction)) {
                    (0, 0) => self.f_mv_w_x(&instruction),
                    (rs2, funct3) => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct3: {:x} funct7: {:x}",
                            0x53, rs2, funct3, 0x78
                        ));
                        self.set_exception_cause(2)
                    }
                },
                0x79 => match (extract_rs2(&instruction), extract_funct3(&instruction)) {
                    (0, 0) => self.f_mv_d_x(&instruction),
                    (rs2, funct3) => {
                        print_not_implement(format!(
                            "op: {:x} rs2: {:x} funct3: {:x} funct7: {:x}",
                            0x53, rs2, funct3, 0x79
                        ));
                        self.set_exception_cause(2)
                    }
                },
                funct7 => {
                    print_not_implement(format!("op: {:x} funct7: {:x}", 0x53, funct7));
                    self.set_exception_cause(2)
                }
            },
            0x63 => match extract_funct3(&instruction) {
                0 => self.beq(&instruction),
                1 => self.bne(&instruction),
                4 => self.blt(&instruction),
                5 => self.bge(&instruction),
                6 => self.bltu(&instruction),
                7 => self.bgeu(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x63, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x67 => match extract_funct3(&instruction) {
                0 => self.jalr(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x67, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x73 => match extract_funct3(&instruction) {
                0 => match (
                    instruction[0],
                    instruction[1],
                    instruction[2],
                    instruction[3],
                ) {
                    (0x73, 0, 0, 0) => self.ecall(&instruction),
                    (0x73, 0, 0x20, 0x30) => self.mret(&instruction),
                    inst => {
                        print_not_implement(format!(
                            "op: {:x} funct3: {:x} inst: {:?}",
                            0x73, 0, inst
                        ));
                        self.set_exception_cause(2)
                    }
                },
                1 => self.csrrw(&instruction),
                2 => self.csrrs(&instruction),
                5 => self.csrrwi(&instruction),
                7 => self.csrrci(&instruction),
                funct3 => {
                    print_not_implement(format!("op: {:x} funct3: {:x}", 0x73, funct3));
                    self.set_exception_cause(2)
                }
            },
            0x6f => self.jal(&instruction),
            op => {
                print_not_implement(format!("op: {:x}", op));
                self.set_exception_cause(2)
            }
        }
    }

    // address + sizeがメモリの大きさを超えるか判定する関数
    // 超えた場合にtrue 超えなかった場合はfalseを返す。
    fn is_over_memory(&mut self, address: usize, size: usize) -> bool {
        if address + size >= self.memory.len() {
            true
        } else {
            false
        }
    }

    fn is_exit(&self, end_point: u64) -> bool {
        if self.pc == end_point {
            true
        } else {
            false
        }
    }

    pub fn exec_program(&mut self, end_point: u64) {
        self.initialize_csrs();
        loop {
            println!("pc: {:x}", self.pc);
            let instruction = self.fetch_instraction();

            match self.decode_and_exec(instruction) {
                Some(_) => {}
                None => self.call_exception(),
            }

            if self.is_exit(end_point) {
                println!("0x1000: {:x}", self.memory[0x1000]);
                return;
            }
        }
    }
}

// Rv64i
impl Rv64SGEmulator {
    // pcの値を変更する関数
    // この関数を使うことでメモリの外側に出たときの原因を自動的に設定できる。
    fn progress_pc(&mut self, pc: u64) -> Option<()> {
        self.pc = pc;
        if self.is_over_memory(self.pc as usize, 4) {
            self.set_exception_cause(2)
        } else {
            Some(())
        }
    }

    fn load_memory_8bit(&mut self, offset: usize) -> Option<u64> {
        if self.is_over_memory(offset, 1) {
            self.set_exception_cause(5)?;
        }

        Some(self.memory[offset] as u64)
    }

    fn load_memory_16bit(&mut self, offset: usize) -> Option<u64> {
        if self.is_over_memory(offset, 2) {
            self.set_exception_cause(5)?;
        }

        Some((self.memory[offset] as u64) + ((self.memory[offset + 1] as u64) << 8))
    }

    fn load_memory_32bit(&mut self, offset: usize) -> Option<u64> {
        if self.is_over_memory(offset, 4) {
            self.set_exception_cause(5)?;
        }

        Some(
            (self.memory[offset] as u64)
                + ((self.memory[offset + 1] as u64) << 8)
                + ((self.memory[offset + 2] as u64) << 16)
                + ((self.memory[offset + 3] as u64) << 24),
        )
    }

    fn load_memory_64bit(&mut self, offset: usize) -> Option<u64> {
        if self.is_over_memory(offset, 8) {
            self.set_exception_cause(5)?;
        }

        Some(
            (self.memory[offset] as u64)
                + ((self.memory[offset + 1] as u64) << 8)
                + ((self.memory[offset + 2] as u64) << 16)
                + ((self.memory[offset + 3] as u64) << 24)
                + ((self.memory[offset + 4] as u64) << 32)
                + ((self.memory[offset + 5] as u64) << 40)
                + ((self.memory[offset + 6] as u64) << 48)
                + ((self.memory[offset + 7] as u64) << 56),
        )
    }

    fn save_memory_8bit(&mut self, offset: usize, value: u64) -> Option<()> {
        if self.is_over_memory(offset, 1) {
            self.set_exception_cause(7)?;
        }

        self.memory[offset] = value as u8;
        Some(())
    }

    fn save_memory_16bit(&mut self, offset: usize, value: u64) -> Option<()> {
        if self.is_over_memory(offset, 2) {
            self.set_exception_cause(7)?;
        }

        self.memory[offset] = value as u8;
        self.memory[offset + 1] = (value >> 8) as u8;
        Some(())
    }

    fn save_memory_32bit(&mut self, offset: usize, value: u64) -> Option<()> {
        if self.is_over_memory(offset, 4) {
            self.set_exception_cause(7)?;
        }

        self.memory[offset] = value as u8;
        self.memory[offset + 1] = (value >> 8) as u8;
        self.memory[offset + 2] = (value >> 16) as u8;
        self.memory[offset + 3] = (value >> 24) as u8;
        Some(())
    }

    fn save_memory_64bit(&mut self, offset: usize, value: u64) -> Option<()> {
        if self.is_over_memory(offset, 8) {
            self.set_exception_cause(7)?;
        }

        self.memory[offset] = value as u8;
        self.memory[offset + 1] = (value >> 8) as u8;
        self.memory[offset + 2] = (value >> 16) as u8;
        self.memory[offset + 3] = (value >> 24) as u8;
        self.memory[offset + 4] = (value >> 32) as u8;
        self.memory[offset + 5] = (value >> 40) as u8;
        self.memory[offset + 6] = (value >> 48) as u8;
        self.memory[offset + 7] = (value >> 56) as u8;
        Some(())
    }
}

impl Rv64SGEmulator {
    fn lb(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] = extend_sign_8bit(
                self.load_memory_8bit(self.registers[rs1].wrapping_add(offset) as usize)?,
            );
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn lh(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] = extend_sign_16bit(
                self.load_memory_16bit(self.registers[rs1].wrapping_add(offset) as usize)?,
            );
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn lw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(
                self.load_memory_32bit(self.registers[rs1].wrapping_add(offset) as usize)?,
            );
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn ld(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] =
                self.load_memory_64bit(self.registers[rs1].wrapping_add(offset) as usize)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn lbu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] =
                self.load_memory_8bit(self.registers[rs1].wrapping_add(offset) as usize)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn lhu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_imm_11_0(instruction));

        if rd != 0 {
            self.registers[rd] =
                self.load_memory_16bit(self.registers[rs1].wrapping_add(offset) as usize)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn lwu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        if rd != 0 {
            self.registers[rd] =
                self.load_memory_32bit(self.registers[rs1].wrapping_add(offset) as usize)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn fence(&mut self, _: &Vec<u8>) -> Option<()> {
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn fence_i(&mut self, _: &Vec<u8>) -> Option<()> {
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn addi(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(&instruction));

        if rd != 0 {
            self.registers[rd] = self.registers[rs1].wrapping_add(imm);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn srli(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] >> shamt;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn srai(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_n(self.registers[rs1] >> shamt, 63 - shamt);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn xor(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] ^ self.registers[rs2];
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn srl(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] >> (self.registers[rs2] & 0x3f);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sra(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let shift = self.registers[rs2] & 0x1f;
            self.registers[rd] = extend_sign_n(self.registers[rs1] >> shift, 63 - shift);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn or(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] | self.registers[rs2];
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn slli(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] << shamt;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn slti(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(instruction));
        let flag = self.registers[rs1].wrapping_sub(imm);

        if rd != 0 {
            if (flag >> 63) == 1 {
                self.registers[rd] = 1;
            } else {
                self.registers[rd] = 0;
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sltiu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(instruction));

        if rd != 0 {
            if self.registers[rs1] < imm {
                self.registers[rd] = 1;
            } else {
                self.registers[rd] = 0;
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn xori(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(instruction));

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] ^ imm;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn andi(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(instruction));

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] & imm;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn ori(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extract_imm_11_0(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] | extend_sign_12bit(imm);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn auipc(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let imm = extend_sign_32bit(extract_imm_31_12(instruction));

        if rd != 0 {
            self.registers[rd] = self.pc.wrapping_add(imm);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn addiw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let imm = extend_sign_12bit(extract_imm_11_0(instruction));

        if rd != 0 {
            self.registers[rd] =
                extend_sign_32bit(truncate_top_32bit(self.registers[rs1].wrapping_add(imm)));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn slliw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] =
                extend_sign_32bit(truncate_top_32bit(self.registers[rs1] << shamt));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn srliw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] =
                extend_sign_32bit(truncate_top_32bit(self.registers[rs1]) >> shamt);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sraiw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let shamt = extract_shamt(instruction);

        if rd != 0 {
            self.registers[rd] =
                extend_sign_n(truncate_top_32bit(self.registers[rs1]) >> shamt, 31 - shamt);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn addw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(truncate_top_32bit(
                self.registers[rs1].wrapping_add(self.registers[rs2]),
            ));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn subw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(truncate_top_32bit(
                self.registers[rs1].wrapping_sub(self.registers[rs2]),
            ));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sllw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(truncate_top_32bit(
                self.registers[rs1] << (self.registers[rs2] & 0x1f),
            ));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn srlw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(
                truncate_top_32bit(self.registers[rs1]) >> (self.registers[rs2] & 0x1f),
            );
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sraw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let shift = self.registers[rs2] & 0x1f;
            self.registers[rd] =
                extend_sign_n(truncate_top_32bit(self.registers[rs1]) >> shift, 31 - shift);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sb(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_8bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            self.registers[rs2],
        )?;
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sh(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_16bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            truncate_top_16bit(self.registers[rs2]),
        )?;
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_32bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            self.registers[rs2],
        )?;
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sd(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_64bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            self.registers[rs2],
        )?;
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn add(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1].wrapping_add(self.registers[rs2]);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sub(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1].wrapping_sub(self.registers[rs2]);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sll(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] << (self.registers[rs2] & 0x3f);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn slt(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let flag = self.registers[rs1].wrapping_sub(self.registers[rs2]);

        if rd != 0 {
            if (flag >> 63) == 1 {
                self.registers[rd] = 1;
            } else {
                self.registers[rd] = 0;
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn sltu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            if self.registers[rs1] < self.registers[rs2] {
                self.registers[rd] = 1;
            } else {
                self.registers[rd] = 0;
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn and(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1] & self.registers[rs2];
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn bge(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));
        let flag = self.registers[rs1].wrapping_sub(self.registers[rs2]);

        if flag == 0 || (flag >> 63) == 0 {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn bltu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));

        if self.registers[rs1] < self.registers[rs2] {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn bgeu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));

        if self.registers[rs1] >= self.registers[rs2] {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn jalr(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        let t = self.pc.wrapping_add(4);
        self.progress_pc(self.registers[rs1].wrapping_add(offset) & !1)?;
        if rd != 0 {
            self.registers[rd] = t;
        }

        Some(())
    }

    fn lui(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let imm = extend_sign_32bit(extract_imm_31_12(instruction));

        if rd != 0 {
            self.registers[rd] = imm;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn beq(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));

        if self.registers[rs1] == self.registers[rs2] {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn bne(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));

        if self.registers[rs1] != self.registers[rs2] {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn blt(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_13bit(extract_offset_12_10_5_4_1_11(instruction));
        let flag = self.registers[rs1].wrapping_sub(self.registers[rs2]);

        if (flag >> 63) == 1 {
            self.progress_pc(self.pc.wrapping_add(offset))
        } else {
            self.progress_pc(self.pc.wrapping_add(4))
        }
    }

    fn ecall(&mut self, _: &Vec<u8>) -> Option<()> {
        let cause = match self.mode {
            MachineMode::U => 8,
            MachineMode::S => 9,
            MachineMode::M => 11,
        };

        self.set_exception_cause(cause)
    }

    fn mret(&mut self, _: &Vec<u8>) -> Option<()> {
        let pc = self.read_csr(M_EPC)?;
        let mut mstatus = self.read_csr(M_STATUS)?;
        mstatus = (mstatus & 0xfffffffffffffff7) | ((mstatus & 0x80) >> 4);
        mstatus = mstatus | 0x80;
        let mode = (mstatus & 0x1800) >> 11;
        mstatus = (mstatus & 0xffffffffffffe7ff) | ((MachineMode::U as u64) << 11);
        self.write_csr(M_STATUS, mstatus)?;
        self.mode = MachineMode::from_u64(mode).unwrap();
        self.progress_pc(pc)
    }

    fn csrrw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rv_csr = extract_csr(instruction);

        let t = self.read_csr(rv_csr)?;
        self.write_csr(rv_csr, self.registers[rs1])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn csrrs(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rv_csr = extract_csr(instruction);

        let t = self.read_csr(rv_csr)?;
        self.write_csr(rv_csr, t | self.registers[rs1])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn csrrwi(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let zimm = extract_zimm(instruction);
        let rv_csr = extract_csr(instruction);

        if rd != 0 {
            self.registers[rd] = self.read_csr(rv_csr)?;
        }

        self.write_csr(rv_csr, zimm)?;
        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn csrrci(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let zimm = extract_zimm(instruction);
        let rv_csr = extract_csr(instruction);

        let t = self.read_csr(rv_csr)?;
        self.write_csr(rv_csr, t & (!zimm))?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn jal(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(&instruction);
        let mut offset = (((instruction[3] as u64) & 0x80) << (20 - 8))
            + (((instruction[2] as u64) & 0x0f) << (16 - 1))
            + (((instruction[1] as u64) & 0xf0) << (12 - 5))
            + (((instruction[2] as u64) & 0x10) << (11 - 5))
            + ((((instruction[3]) as u64) & 0x7f) << 3)
            + ((((instruction[2]) as u64) & 0xe0) >> 5);
        offset = extend_sign_21bit(offset << 1);

        if rd != 0 {
            self.registers[rd] = self.pc.wrapping_add(4);
        }

        self.progress_pc(self.pc.wrapping_add(offset))
    }
}

//Rv64m
impl Rv64SGEmulator {
    fn mul(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = self.registers[rs1].wrapping_mul(self.registers[rs2]);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn mulh(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = (extend_sign_128bit(self.registers[rs1])
                .wrapping_mul(extend_sign_128bit(self.registers[rs2]))
                >> 64) as u64;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn mulhsu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = (extend_sign_128bit(self.registers[rs1])
                .wrapping_mul(self.registers[rs2] as u128)
                >> 64) as u64;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn mulhu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = ((self.registers[rs1] as u128)
                .wrapping_mul(self.registers[rs2] as u128)
                >> 64) as u64;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn div(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if self.registers[rs2] != 0 {
                (self.registers[rs1] as i64).wrapping_div(self.registers[rs2] as i64) as u64
            } else {
                u64::MAX
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn divu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if self.registers[rs2] != 0 {
                self.registers[rs1].wrapping_div(self.registers[rs2])
            } else {
                u64::MAX
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn rem(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if self.registers[rs2] != 0 {
                (self.registers[rs1] as i64).wrapping_rem(self.registers[rs2] as i64) as u64
            } else {
                self.registers[rs1]
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn remu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if self.registers[rs2] != 0 {
                self.registers[rs1].wrapping_rem(self.registers[rs2])
            } else {
                self.registers[rs1]
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn mulw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(truncate_top_32bit(
                self.registers[rs1].wrapping_mul(self.registers[rs2]),
            ));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn divw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if truncate_top_32bit(self.registers[rs2]) != 0 {
                (self.registers[rs1] as i32).wrapping_div(self.registers[rs2] as i32) as u64
            } else {
                u64::MAX
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn divuw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if truncate_top_32bit(self.registers[rs2]) != 0 {
                extend_sign_32bit(
                    truncate_top_32bit(self.registers[rs1])
                        .wrapping_div(truncate_top_32bit(self.registers[rs2])),
                )
            } else {
                u64::MAX
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn remw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if truncate_top_32bit(self.registers[rs2]) != 0 {
                (truncate_top_32bit(self.registers[rs1]) as i32)
                    .wrapping_rem(truncate_top_32bit(self.registers[rs2]) as i32)
                    as u64
            } else {
                self.registers[rs1]
            }
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn remuw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            self.registers[rd] = if truncate_top_32bit(self.registers[rs2]) != 0 {
                extend_sign_32bit(
                    truncate_top_32bit(self.registers[rs1])
                        .wrapping_rem(truncate_top_32bit(self.registers[rs2])),
                )
            } else {
                self.registers[rs1]
            };
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }
}

//Rv64f + d
impl Rv64SGEmulator {
    fn f_lw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        self.f_registers[rd] =
            nan_boxing(self.load_memory_32bit(self.registers[rs1].wrapping_add(offset) as usize)?);

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_ld(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let offset = extend_sign_12bit(extract_offset_11_0(instruction));

        self.f_registers[rd] =
            self.load_memory_64bit(self.registers[rs1].wrapping_add(offset) as usize)?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sw(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_32bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            self.f_registers[rs2],
        );

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sd(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let offset = extend_sign_12bit(extract_offset_11_5_4_0(instruction));

        self.save_memory_64bit(
            self.registers[rs1].wrapping_add(offset) as usize,
            self.f_registers[rs2],
        )?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_madd_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .fused_mul_add(
                F64::from_bits(self.f_registers[rs2]),
                F64::from_bits(self.f_registers[rs3]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_madd_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs3_value = if is_nan_boxing(self.f_registers[rs3]) {
            F32::from_bits(self.f_registers[rs3] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] = nan_boxing(
            rs1_value
                .fused_mul_add(rs2_value, rs3_value, rm_to_swrm(rm).unwrap())
                .to_bits() as u64,
        );
        flag.get();
        let fflagsg = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflagsg | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_msub_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs3_value = if is_nan_boxing(self.f_registers[rs3]) {
            F32::from_bits(self.f_registers[rs3] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] = nan_boxing(
            rs1_value
                .fused_mul_add(rs2_value, rs3_value.neg(), rm_to_swrm(rm).unwrap())
                .to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_msub_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .fused_mul_add(
                F64::from_bits(self.f_registers[rs2]),
                F64::from_bits(self.f_registers[rs3]).neg(),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_nmsub_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs3_value = if is_nan_boxing(self.f_registers[rs3]) {
            F32::from_bits(self.f_registers[rs3] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] = nan_boxing(
            rs1_value
                .neg()
                .fused_mul_add(rs2_value, rs3_value, rm_to_swrm(rm).unwrap())
                .to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_nmsub_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .neg()
            .fused_mul_add(
                F64::from_bits(self.f_registers[rs2]),
                F64::from_bits(self.f_registers[rs3]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_nmadd_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs3_value = if is_nan_boxing(self.f_registers[rs3]) {
            F32::from_bits(self.f_registers[rs3] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] = nan_boxing(
            rs1_value
                .neg()
                .fused_mul_add(rs2_value, rs3_value.neg(), rm_to_swrm(rm).unwrap())
                .to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_nmadd_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);
        let rs3 = extract_rs3(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .neg()
            .fused_mul_add(
                F64::from_bits(self.f_registers[rs2]),
                F64::from_bits(self.f_registers[rs3]).neg(),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_add_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] =
            nan_boxing(rs1_value.add(rs2_value, rm_to_swrm(rm).unwrap()).to_bits() as u64);
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_add_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .add(
                F64::from_bits(self.f_registers[rs2]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sub_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] =
            nan_boxing(rs1_value.sub(rs2_value, rm_to_swrm(rm).unwrap()).to_bits() as u64);
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sub_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .sub(
                F64::from_bits(self.f_registers[rs2]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mul_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        self.f_registers[rd] =
            nan_boxing(rs1_value.mul(rs2_value, rm_to_swrm(rm).unwrap()).to_bits() as u64);
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mul_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .mul(
                F64::from_bits(self.f_registers[rs2]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_div_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .div(
                F64::from_bits(self.f_registers[rs2]),
                rm_to_swrm(rm).unwrap(),
            )
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnj_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut rs1_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs1]) {
            self.f_registers[rs1] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        let rs2_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs2]) {
            self.f_registers[rs2] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        rs1_value.set_sign(rs2_value.sign());

        self.f_registers[rd] = nan_boxing(rs1_value.to_bits() as u64);

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnjn_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut rs1_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs1]) {
            self.f_registers[rs1] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        let rs2_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs2]) {
            self.f_registers[rs2] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        rs1_value.set_sign(rs2_value.neg().sign());

        self.f_registers[rd] = nan_boxing(rs1_value.to_bits() as u64);

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnjx_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut rs1_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs1]) {
            self.f_registers[rs1] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        let rs2_value = F32::from_bits(if is_nan_boxing(self.f_registers[rs2]) {
            self.f_registers[rs2] as u32
        } else {
            F32::quiet_nan().to_bits()
        });
        rs1_value.set_sign(rs2_value.sign() ^ rs1_value.sign());

        self.f_registers[rd] = nan_boxing(rs1_value.to_bits() as u64);

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnj_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut value = F64::from_bits(self.f_registers[rs1]);
        value.set_sign(F64::from_bits(self.f_registers[rs2]).sign());
        self.f_registers[rd] = value.to_bits();

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnjn_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut value = F64::from_bits(self.f_registers[rs1]);
        value.set_sign(F64::from_bits(self.f_registers[rs2]).neg().sign());
        self.f_registers[rd] = value.to_bits();

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sgnjx_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut rs1_value = F64::from_bits(self.f_registers[rs1]);
        let rs2_value = F64::from_bits(self.f_registers[rs2]);
        rs1_value.set_sign(rs1_value.sign() ^ rs2_value.sign());
        self.f_registers[rd] = rs1_value.to_bits();

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_min_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = 0;
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };

        if rs1_value.is_signaling_nan() || rs2_value.is_signaling_nan() {
            flag |= 0x10;
        }
        self.f_registers[rd] = nan_boxing(if rs1_value.is_nan() && rs2_value.is_nan() {
            F32::quiet_nan().to_bits() as u64
        } else if rs1_value.is_nan() {
            rs2_value.to_bits() as u64
        } else if rs2_value.is_nan() {
            rs1_value.to_bits() as u64
        } else if (rs1_value.is_negative_zero() && rs2_value.is_positive_zero())
            || (rs1_value.is_positive_zero() && rs2_value.is_negative_zero())
        {
            F32::negative_zero().to_bits() as u64
        } else if rs1_value.lt(rs2_value) {
            rs1_value.to_bits() as u64
        } else {
            rs2_value.to_bits() as u64
        });

        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | flag)?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_max_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = 0;
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };
        let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
            F32::from_bits(self.f_registers[rs2] as u32)
        } else {
            F32::quiet_nan()
        };
        if rs1_value.is_signaling_nan() || rs2_value.is_signaling_nan() {
            flag |= 0x10;
        }
        self.f_registers[rd] = nan_boxing(if rs1_value.is_nan() && rs2_value.is_nan() {
            F32::quiet_nan().to_bits() as u64
        } else if rs1_value.is_nan() {
            rs2_value.to_bits() as u64
        } else if rs2_value.is_nan() {
            rs1_value.to_bits() as u64
        } else if (rs1_value.is_negative_zero() && rs2_value.is_positive_zero())
            || (rs1_value.is_positive_zero() && rs2_value.is_negative_zero())
        {
            F32::positive_zero().to_bits() as u64
        } else if rs1_value.lt(rs2_value) {
            rs2_value.to_bits() as u64
        } else {
            rs1_value.to_bits() as u64
        });

        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | flag)?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_min_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = 0;
        let rs1_value = F64::from_bits(self.f_registers[rs1]);
        let rs2_value = F64::from_bits(self.f_registers[rs2]);
        if rs1_value.is_signaling_nan() || rs2_value.is_signaling_nan() {
            flag |= 0x10;
        }

        self.f_registers[rd] = if rs1_value.is_nan() && rs2_value.is_nan() {
            F64::quiet_nan().to_bits()
        } else if rs1_value.is_nan() {
            rs2_value.to_bits()
        } else if rs2_value.is_nan() {
            rs1_value.to_bits()
        } else if (rs1_value.is_negative_zero() && rs2_value.is_positive_zero())
            || (rs1_value.is_positive_zero() && rs1_value.is_negative_zero())
        {
            F64::negative_zero().to_bits()
        } else if rs1_value.lt(rs2_value) {
            rs1_value.to_bits()
        } else {
            rs2_value.to_bits()
        };

        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | flag)?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_max_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let mut flag = 0;
        let rs1_value = F64::from_bits(self.f_registers[rs1]);
        let rs2_value = F64::from_bits(self.f_registers[rs2]);
        if rs1_value.is_signaling_nan() || rs2_value.is_signaling_nan() {
            flag |= 0x10;
        }

        self.f_registers[rd] = if rs1_value.is_nan() && rs2_value.is_nan() {
            F64::quiet_nan().to_bits()
        } else if rs1_value.is_nan() {
            rs2_value.to_bits()
        } else if rs2_value.is_nan() {
            rs1_value.to_bits()
        } else if (rs1_value.is_negative_zero() && rs2_value.is_positive_zero())
            || (rs1_value.is_positive_zero() && rs2_value.is_positive_zero())
        {
            F64::positive_zero().to_bits()
        } else if rs1_value.lt(rs2_value) {
            rs2_value.to_bits()
        } else {
            rs1_value.to_bits()
        };
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | flag)?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_s_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = nan_boxing(
            F64::from_bits(self.f_registers[rs1])
                .to_f32(rm_to_swrm(rm).unwrap())
                .to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_d_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F32::from_bits(self.f_registers[rs1] as u32)
            .to_f64(rm_to_swrm(rm).unwrap())
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_sqrt_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = F64::from_bits(self.f_registers[rs1])
            .sqrt(rm_to_swrm(rm).unwrap())
            .to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_eq_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = 0;
            let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
                F32::from_bits(self.f_registers[rs1] as u32)
            } else {
                F32::quiet_nan()
            };
            let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
                F32::from_bits(self.f_registers[rs2] as u32)
            } else {
                F32::quiet_nan()
            };
            if rs1_value.is_signaling_nan() || rs2_value.is_signaling_nan() {
                flag |= 0x10;
            }
            self.registers[rd] = if rs1_value.is_nan() || rs2_value.is_nan() {
                0
            } else if rs1_value.eq(rs2_value) {
                1
            } else {
                0
            };
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | flag)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_le_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = 0;
            let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
                F32::from_bits(self.f_registers[rs1] as u32)
            } else {
                F32::quiet_nan()
            };
            let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
                F32::from_bits(self.f_registers[rs2] as u32)
            } else {
                F32::quiet_nan()
            };
            self.registers[rd] = if rs1_value.is_nan() || rs2_value.is_nan() {
                flag |= 0x10;
                0
            } else if rs1_value.le(rs2_value) {
                1
            } else {
                0
            };
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | flag)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_lt_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = 0;
            let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
                F32::from_bits(self.f_registers[rs1] as u32)
            } else {
                F32::quiet_nan()
            };
            let rs2_value = if is_nan_boxing(self.f_registers[rs2]) {
                F32::from_bits(self.f_registers[rs2] as u32)
            } else {
                F32::quiet_nan()
            };
            self.registers[rd] = if rs1_value.is_nan() || rs2_value.is_nan() {
                flag |= 0x10;
                0
            } else if rs1_value.lt(rs2_value) {
                1
            } else {
                0
            };
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | flag)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_le_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = 0;
            let rs1_value = F64::from_bits(self.f_registers[rs1]);
            let rs2_value = F64::from_bits(self.f_registers[rs2]);
            self.registers[rd] = if rs1_value.is_nan() || rs2_value.is_nan() {
                flag |= 0x10;
                0
            } else if rs1_value.le(rs2_value) {
                1
            } else {
                0
            };
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | flag)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_lt_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = 0;
            let rs1_value = F64::from_bits(self.f_registers[rs1]);
            let rs2_value = F64::from_bits(self.f_registers[rs2]);
            self.registers[rd] = if rs1_value.is_nan() | rs2_value.is_nan() {
                flag |= 0x10;
                0
            } else if rs1_value.lt(rs2_value) {
                1
            } else {
                0
            };
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | flag)?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_eq_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        if rd != 0 {
            let mut flag = ExceptionFlags::default();
            flag.set();
            if F64::from_bits(self.f_registers[rs1]).eq(F64::from_bits(self.f_registers[rs2])) {
                self.registers[rd] = 1;
            } else {
                self.registers[rd] = 0;
            }
            flag.get();
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_w_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.registers[rd] = F32::from_bits(self.f_registers[rs1] as u32)
            .to_i32(rm_to_swrm(rm).unwrap(), true) as u64;
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_wu_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.registers[rd] = extend_sign_32bit(
            F32::from_bits(self.f_registers[rs1] as u32).to_u32(rm_to_swrm(rm).unwrap(), true)
                as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_l_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.registers[rd] = F32::from_bits(self.f_registers[rs1] as u32)
            .to_i64(rm_to_swrm(rm).unwrap(), true) as u64;
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_lu_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.registers[rd] =
            F32::from_bits(self.f_registers[rs1] as u32).to_u64(rm_to_swrm(rm).unwrap(), true);
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_w_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            let mut flag = ExceptionFlags::default();
            flag.set();
            self.registers[rd] =
                F64::from_bits(self.f_registers[rs1]).to_i32(rm_to_swrm(rm).unwrap(), true) as u64;
            flag.get();
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_wu_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            let mut flag = ExceptionFlags::default();
            flag.set();
            self.registers[rd] = extend_sign_32bit(
                F64::from_bits(self.f_registers[rs1]).to_u32(rm_to_swrm(rm).unwrap(), true) as u64,
            );
            flag.get();
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_l_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            let mut flag = ExceptionFlags::default();
            flag.set();
            self.registers[rd] =
                F64::from_bits(self.f_registers[rs1]).to_i64(rm_to_swrm(rm).unwrap(), true) as u64;
            flag.get();
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_lu_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            let mut flag = ExceptionFlags::default();
            flag.set();
            self.registers[rd] =
                F64::from_bits(self.f_registers[rs1]).to_u64(rm_to_swrm(rm).unwrap(), true);
            flag.get();
            let fflags = self.read_csr(FFLAGS)?;
            self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_s_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = nan_boxing(
            F32::from_i32(self.registers[rs1] as i32, rm_to_swrm(rm).unwrap()).to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_s_wu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = nan_boxing(
            F32::from_u32(self.registers[rs1] as u32, rm_to_swrm(rm).unwrap()).to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_s_l(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.get();
        self.f_registers[rd] = nan_boxing(
            F32::from_i64(self.registers[rs1] as i64, rm_to_swrm(rm).unwrap()).to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_s_lu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] = nan_boxing(
            F32::from_u64(self.registers[rs1], rm_to_swrm(rm).unwrap()).to_bits() as u64,
        );
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_d_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] =
            F64::from_i32(self.registers[rs1] as i32, rm_to_swrm(rm).unwrap()).to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_d_wu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] =
            F64::from_u32(self.registers[rs1] as u32, rm_to_swrm(rm).unwrap()).to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_d_l(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] =
            F64::from_i64(self.registers[rs1] as i64, rm_to_swrm(rm).unwrap()).to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_cvt_d_lu(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rm = extract_rm(instruction, self.read_csr(FRM)?);
        let rs1 = extract_rs1(instruction);

        let mut flag = ExceptionFlags::default();
        flag.set();
        self.f_registers[rd] =
            F64::from_u64(self.registers[rs1], rm_to_swrm(rm).unwrap()).to_bits();
        flag.get();
        let fflags = self.read_csr(FFLAGS)?;
        self.write_csr(FFLAGS, fflags | swef_to_fflags(flag))?;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mv_x_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(truncate_top_32bit(self.f_registers[rs1]));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_class_s(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        let mut class = 0;
        let rs1_value = if is_nan_boxing(self.f_registers[rs1]) {
            F32::from_bits(self.f_registers[rs1] as u32)
        } else {
            F32::quiet_nan()
        };

        if rs1_value.is_negative_infinity() {
            class |= 0x1;
        }

        if rs1_value.is_negative_normal() {
            class |= 0x2;
        }

        if rs1_value.is_negative_subnormal() {
            class |= 0x4;
        }

        if rs1_value.is_negative_zero() {
            class |= 0x8;
        }

        if rs1_value.is_positive_zero() {
            class |= 0x10;
        }

        if rs1_value.is_positive_subnormal() {
            class |= 0x20;
        }

        if rs1_value.is_positive_normal() {
            class |= 0x40;
        }

        if rs1_value.is_positive_infinity() {
            class |= 0x80;
        }

        if rs1_value.is_signaling_nan() {
            class |= 0x100;
        } else if rs1_value.is_nan() {
            class |= 0x200;
        }

        if rd != 0 {
            self.registers[rd] = class;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mv_x_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            self.registers[rd] = self.f_registers[rs1];
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_class_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        let mut class = 0;
        let rs1_value = F64::from_bits(self.f_registers[rs1]);
        if rs1_value.is_negative_infinity() {
            class |= 0x1;
        }

        if rs1_value.is_negative_normal() {
            class |= 0x2;
        }

        if rs1_value.is_negative_subnormal() {
            class |= 0x4;
        }

        if rs1_value.is_negative_zero() {
            class |= 0x8;
        }

        if rs1_value.is_positive_zero() {
            class |= 0x10;
        }

        if rs1_value.is_positive_subnormal() {
            class |= 0x20;
        }

        if rs1_value.is_positive_normal() {
            class |= 0x40;
        }

        if rs1_value.is_positive_infinity() {
            class |= 0x80;
        }

        if rs1_value.is_signaling_nan() {
            class |= 0x100;
        } else if rs1_value.is_nan() {
            class |= 0x200;
        }

        if rd != 0 {
            self.registers[rd] = class;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mv_w_x(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        self.f_registers[rd] = nan_boxing(self.registers[rs1]);

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn f_mv_d_x(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        self.f_registers[rd] = self.registers[rs1];

        self.progress_pc(self.pc.wrapping_add(4))
    }
}

// Rv64a
impl Rv64SGEmulator {
    fn a_moadd_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;
        self.save_memory_32bit(
            self.registers[rs1] as usize,
            t.wrapping_add(truncate_top_32bit(self.registers[rs2])),
        )?;

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moswap_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;
        self.save_memory_32bit(
            self.registers[rs1] as usize,
            truncate_top_32bit(self.registers[rs2]),
        )?;

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_lr_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);

        if rd != 0 {
            self.registers[rd] =
                extend_sign_32bit(self.load_memory_32bit(self.registers[rs1] as usize)?);
            self.preserved_memory = Some((
                self.registers[rs1] as usize,
                self.registers[rs1] as usize + 4,
            ));
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_sc_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let flag = if let Some(preserved_memory) = self.preserved_memory {
            if preserved_memory.0 <= self.registers[rs1] as usize
                && preserved_memory.1 >= self.registers[rs1] as usize + 4
            {
                true
            } else {
                false
            }
        } else {
            false
        };

        if flag {
            self.save_memory_32bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            if flag {
                self.registers[rd] = 0;
            } else {
                self.registers[rd] = 1;
            }
        }
        self.preserved_memory = None;

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moxor_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;
        self.save_memory_32bit(
            self.registers[rs1] as usize,
            t ^ truncate_top_32bit(self.registers[rs2]),
        )?;

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moor_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;
        self.save_memory_32bit(
            self.registers[rs1] as usize,
            t | truncate_top_32bit(self.registers[rs2]),
        )?;

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moand_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;
        self.save_memory_32bit(
            self.registers[rs1] as usize,
            t & truncate_top_32bit(self.registers[rs2]),
        )?;

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momin_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;

        if truncate_top_32bit(self.registers[rs2]) as i32 > t as i32 {
            self.save_memory_32bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_32bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momax_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;

        if t as i32 > truncate_top_32bit(self.registers[rs2]) as i32 {
            self.save_memory_32bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_32bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_mominu_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;

        if truncate_top_32bit(self.registers[rs2]) > t {
            self.save_memory_32bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_32bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momaxu_w(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_32bit(self.registers[rs1] as usize)?;

        if t > truncate_top_32bit(self.registers[rs2]) {
            self.save_memory_32bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_32bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = extend_sign_32bit(t);
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moadd_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        self.save_memory_64bit(
            self.registers[rs1] as usize,
            t.wrapping_add(self.registers[rs2]),
        )?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moswap_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        self.save_memory_64bit(self.registers[rs1] as usize, self.registers[rs2])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moxor_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        self.save_memory_64bit(self.registers[rs1] as usize, t ^ self.registers[rs2])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moor_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        self.save_memory_64bit(self.registers[rs1] as usize, t | self.registers[rs2])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_moand_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        self.save_memory_64bit(self.registers[rs1] as usize, t & self.registers[rs2])?;

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momin_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        if self.registers[rs2] as i64 > t as i64 {
            self.save_memory_64bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_64bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momax_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        if t as i64 > self.registers[rs2] as i64 {
            self.save_memory_64bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_64bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_mominu_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        if self.registers[rs2] > t {
            self.save_memory_64bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_64bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }

    fn a_momaxu_d(&mut self, instruction: &Vec<u8>) -> Option<()> {
        let rd = extract_rd(instruction);
        let rs1 = extract_rs1(instruction);
        let rs2 = extract_rs2(instruction);

        let t = self.load_memory_64bit(self.registers[rs1] as usize)?;
        if t > self.registers[rs2] {
            self.save_memory_64bit(self.registers[rs1] as usize, t)?;
        } else {
            self.save_memory_64bit(self.registers[rs1] as usize, self.registers[rs2])?;
        }

        if rd != 0 {
            self.registers[rd] = t;
        }

        self.progress_pc(self.pc.wrapping_add(4))
    }
}

// CSR系
#[derive(PartialEq, Clone, Copy)]
pub enum MachineMode {
    U = 0,
    S = 1,
    M = 3,
}

impl MachineMode {
    pub fn from_u64(mode: u64) -> Option<Self> {
        match mode {
            0 => Some(Self::U),
            1 => Some(Self::S),
            3 => Some(Self::M),
            _ => None,
        }
    }

    pub fn to_usize(&self) -> usize {
        match self {
            Self::U => 0,
            Self::S => 1,
            Self::M => 3,
        }
    }
}

pub const FFLAGS: usize = 0x001;
pub const FRM: usize = 0x002;
pub const FCSR: usize = 0x003;

pub const M_STATUS: usize = 0x300;
pub const M_EDELEG: usize = 0x302;
pub const M_IDELEG: usize = 0x303;
pub const M_TVEC: usize = 0x305;
pub const M_EPC: usize = 0x341;
pub const M_CAUSE: usize = 0x342;
pub const M_HARTID: usize = 0xf14;

pub struct CsrStatus {
    readable: bool,
    writreable: bool,
}

impl CsrStatus {
    fn from_usize(mode: &MachineMode, rv_csr: usize) -> Option<Self> {
        let mut res = CsrStatus {
            readable: false,
            writreable: false,
        };
        if mode.to_usize() < (rv_csr & 0x300) >> 8 {
            return None;
        }

        match rv_csr >> 10 {
            3 => res.readable = true,
            1 | 0 => {
                res.readable = true;
                res.writreable = true;
            }
            _ => return None,
        }

        Some(res)
    }
}

impl Rv64SGEmulator {
    fn initialize_csrs(&mut self) {
        self.csrs[M_HARTID] = 0;
    }

    fn read_csr(&mut self, rv_csr: usize) -> Option<u64> {
        let csr_status = CsrStatus::from_usize(&self.mode, rv_csr).unwrap();

        if !csr_status.readable && rv_csr >= self.csrs.len() {
            self.set_exception_cause(2)?;
        }

        match rv_csr {
            FRM => Some((self.csrs[FCSR] & 0xe0) >> 5),
            FFLAGS => Some(self.csrs[FCSR] & 0x1f),
            rv_csr => Some(self.csrs[rv_csr]),
        }
    }

    fn write_csr(&mut self, rv_csr: usize, value: u64) -> Option<()> {
        let csr_status = CsrStatus::from_usize(&self.mode, rv_csr).unwrap();

        if !csr_status.writreable && rv_csr >= self.csrs.len() {
            self.set_exception_cause(2)?;
        }

        match rv_csr {
            FCSR => {
                self.csrs[FCSR] = value & 0xff;
                Some(())
            }
            FRM => {
                self.csrs[FCSR] = (self.csrs[FCSR] & 0x1f) + ((value & 0x7) << 5);
                Some(())
            }
            FFLAGS => {
                self.csrs[FCSR] = (self.csrs[FCSR] & 0xe0) + (value & 0x1f);
                Some(())
            }
            M_STATUS => {
                self.csrs[rv_csr] = value & 0x8000003f007fffea;
                Some(())
            }
            M_EDELEG => {
                self.csrs[M_EDELEG] = value & 0xffff0000ff00bbff;
                Some(())
            }
            M_TVEC => {
                if !((value & 0x3) > 1) {
                    self.csrs[M_TVEC] = value;
                }

                Some(())
            }
            M_EPC => {
                self.csrs[M_EPC] = value & 0xfffffffffffffffe;
                Some(())
            }
            rv_csr => {
                self.csrs[rv_csr] = value;
                Some(())
            }
        }
    }

    // 命令の中で例外が起こったときに呼ばれる関数
    // これはモードに関係なく実行することができるが命令の正規の実行時には呼ばない。
    // Noneを常時返す。
    fn set_exception_cause(&mut self, cause: u64) -> Option<()> {
        self.csrs[M_CAUSE] = cause;
        None
    }

    fn call_exception(&mut self) {
        let current_mode = self.mode;
        let mcause = self.csrs[M_CAUSE];

        if mcause >> 63 == 0 {
            if !(self.mode == MachineMode::M) && self.csrs[M_EDELEG] == mcause {
                self.mode = MachineMode::S;
            } else {
                self.mode = MachineMode::M;
            }
        } else {
        }

        match self.mode {
            MachineMode::M => {
                self.write_csr(M_EPC, self.pc).unwrap();

                let mtvec = self.read_csr(M_TVEC).unwrap();
                let mut mstatus = self.read_csr(M_STATUS).unwrap() & 0xffffffffffffe6ff;
                mstatus = (mstatus & 0xffffffffffffff77) | ((mstatus & 0x8) << 4);
                mstatus = mstatus
                    | ((current_mode as u64) << 11)
                    | (((((current_mode.to_usize() + 1) & 0x2) >> 1) as u64) << 8);

                self.write_csr(M_STATUS, mstatus).unwrap();

                if mtvec & 0x3 == 1 && mcause >> 63 == 1 {
                    self.progress_pc(
                        (mtvec & 0xfffffffffffffffc) + 4 * (mcause & 0x7fffffffffffffff),
                    )
                    .unwrap();
                } else {
                    self.progress_pc(mtvec & 0xfffffffffffffffc);
                }
            }
            _ => {}
        }
    }
}

use softfloat_wrapper::{ExceptionFlags, RoundingMode};

// Rv64i &
pub fn extract_rd(instruction: &Vec<u8>) -> usize {
    (((instruction[1] & 0xf) << 1) + ((instruction[0] & 0x80) >> 7)) as usize
}

pub fn extract_rs1(instruction: &Vec<u8>) -> usize {
    (((instruction[2] & 0x0f) << 1) + (instruction[1] >> 7)) as usize
}

pub fn extract_rs2(instruction: &Vec<u8>) -> usize {
    (((instruction[2] & 0xf0) >> 4) + ((instruction[3] & 0x1) << 4)) as usize
}

pub fn extract_funct3(instruction: &Vec<u8>) -> usize {
    ((instruction[1] & 0x70) >> 4) as usize
}

pub fn extract_csr(instruction: &Vec<u8>) -> usize {
    extract_imm_11_0(instruction) as usize
}

pub fn extract_imm_11_0(instruction: &Vec<u8>) -> u64 {
    ((instruction[3] as u64 & 0xff) << 4) + ((instruction[2] as u64 & 0xf0) >> 4)
}

pub fn extract_imm_31_12(instruction: &Vec<u8>) -> u64 {
    ((instruction[3] as u64) << 24)
        + ((instruction[2] as u64) << 16)
        + (((instruction[1] as u64) & 0xf0) << (12 - 4))
}

pub fn extract_offset_11_0(instruction: &Vec<u8>) -> u64 {
    extract_imm_11_0(instruction)
}

pub fn extract_offset_11_5_4_0(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x0f) << 1)
        + (((instruction[0] as u64) & 0x80) >> 7)
        + (((instruction[3] as u64) & 0xfe) << 4)
}

pub fn extract_offset_12_10_5_4_1_11(instruction: &Vec<u8>) -> u64 {
    (((instruction[3] as u64) & 0x80) << 5)
        + (((instruction[0] as u64) & 0x80) << 4)
        + (((instruction[3] as u64) & 0x7e) << 4)
        + (((instruction[1] as u64) & 0x0f) << 1)
}

pub fn extract_zimm(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x80) >> 7) + (((instruction[2] as u64) & 0x0f) << 1)
}

pub fn extract_shamt(instruction: &Vec<u8>) -> u64 {
    (((instruction[3] as u64) & 0x3) << 4) + (((instruction[2] as u64) & 0xf0) >> 4)
}

pub fn extend_sign_8bit(value: u64) -> u64 {
    (value + 0x7fffffffffffff80) ^ 0x7fffffffffffff80
}

pub fn extend_sign_12bit(value: u64) -> u64 {
    (value + 0x7ffffffffffff800) ^ 0x7ffffffffffff800
}

pub fn extend_sign_13bit(value: u64) -> u64 {
    (value + 0x7FFFFFFFFFFFF000) ^ 0x7FFFFFFFFFFFF000
}

pub fn extend_sign_16bit(value: u64) -> u64 {
    (value + 0x7FFFFFFFFFFF8000) ^ 0x7fffffffffff8000
}

pub fn extend_sign_21bit(value: u64) -> u64 {
    (value + 0x7FFFFFFFFFF00000) ^ 0x7FFFFFFFFFF00000
}

pub fn extend_sign_32bit(value: u64) -> u64 {
    (value + 0x7FFFFFFF80000000) ^ 0x7FFFFFFF80000000
}

pub fn extend_sign_n(value: u64, shift: u64) -> u64 {
    let mask = (0xffffffffffffffff << shift) ^ 0x8000000000000000;

    (value + mask) ^ mask
}

pub fn truncate_top_32bit(value: u64) -> u64 {
    value & 0xffffffff
}

pub fn truncate_top_16bit(value: u64) -> u64 {
    value & 0xffff
}

// Rv64m
pub fn extend_sign_128bit(value: u64) -> u128 {
    (value as u128 + 0x7fffffffffffffff8000000000000000) ^ 0x7fffffffffffffff8000000000000000
}

// Rv64 f + d
pub fn extract_funct7(instruction: &Vec<u8>) -> usize {
    (instruction[3] as usize) >> 1
}

pub fn extract_rm(instruction: &Vec<u8>, frm: u64) -> usize {
    let rm = extract_funct3(instruction);

    if rm == 7 {
        frm as usize
    } else {
        rm
    }
}

pub fn rm_to_swrm(rm: usize) -> Option<RoundingMode> {
    match rm {
        0 => Some(RoundingMode::TiesToEven),
        1 => Some(RoundingMode::TowardZero),
        2 => Some(RoundingMode::TowardNegative),
        3 => Some(RoundingMode::TowardPositive),
        4 => Some(RoundingMode::TiesToAway),
        _ => None,
    }
}

pub fn swef_to_fflags(swef: ExceptionFlags) -> u64 {
    let mut fflags = 0;

    if swef.is_inexact() {
        fflags |= 0b1;
    }

    if swef.is_underflow() {
        fflags |= 0b10;
    }

    if swef.is_overflow() {
        fflags |= 0b100;
    }

    if swef.is_infinite() {
        fflags |= 0b1000;
    }

    if swef.is_invalid() {
        fflags |= 0b10000;
    }

    fflags
}

pub fn nan_boxing(value: u64) -> u64 {
    value | 0xffffffff00000000
}

pub fn extract_rs3(instruction: &Vec<u8>) -> usize {
    ((instruction[3] & 0xf8) >> 3) as usize
}

pub fn is_nan_boxing(value: u64) -> bool {
    if (value >> 32) == 0xffffffff {
        true
    } else {
        false
    }
}

// Rv64c
pub fn c_extract_2_4_rd(instruction: &Vec<u8>) -> usize {
    ((instruction[0] & 0x1c) >> 2) as usize
}

pub fn c_extract_7_9_rd(instruction: &Vec<u8>) -> usize {
    c_extract_7_9_rs1(instruction)
}

pub fn c_extract_7_9_rs1(instruction: &Vec<u8>) -> usize {
    (((instruction[1] & 0x3) << 1) + (instruction[0] >> 7)) as usize
}

pub fn c_extract_7_11_rs1(instruction: &Vec<u8>) -> usize {
    ((instruction[1] as usize & 0xf) << 1) + (instruction[0] as usize >> 7)
}

pub fn c_extract_2_4_rs2(instruction: &Vec<u8>) -> usize {
    c_extract_2_4_rd(instruction)
}

pub fn c_extract_2_6_rs2(instruction: &Vec<u8>) -> usize {
    ((instruction[0] & 0x7c) >> 2) as usize
}

pub fn c_extract_uimm_5_2_7_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x1) << 7)
        + (((instruction[0] as u64) & 0x80) >> 1)
        + (((instruction[1] as u64) & 0x1e) << 1)
}

pub fn c_extract_uimm_5_3_2_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[0] as u64) & 0x20) << 1)
        + (((instruction[1] as u64) & 0x1c) << 1)
        + (((instruction[0] as u64) & 0x40) >> 4)
}

pub fn c_extract_uimm_5_3_8_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x3) << 7)
        + (((instruction[0] as u64) & 0x80) >> 1)
        + (((instruction[1] as u64) & 0x1c) << 1)
}

pub fn c_extract_uimm_5_4_3_8_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[0] as u64) & 0x1c) << 4)
        + (((instruction[1] as u64) & 0x10) << 1)
        + (((instruction[0] as u64) & 0x60) >> 2)
}

pub fn c_extract_uimm_5_4_0(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 1) + (((instruction[0] as u64) & 0x7c) >> 2)
}

pub fn c_extract_uimm_5_3_7_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[0] as u64) & 0x60) << 2) + (((instruction[1] as u64) & 0x1c) << 2)
}

pub fn c_extract_uimm_5_4_2_7_6(instruction: &Vec<u8>) -> u64 {
    (((instruction[0] as u64) & 0xc) << 4)
        + (((instruction[1] as u64) & 0x10) << 1)
        + (((instruction[0] as u64) & 0x70) >> 2)
}

pub fn c_extract_uimm_5_4_9_6_2_3(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x7) << 7)
        + (((instruction[0] as u64) & 0x80) >> 1)
        + (((instruction[1] as u64) & 0x18) << 1)
        + (((instruction[0] as u64) & 0x20) >> 2)
        + (((instruction[0] as u64) & 0x40) >> 4)
}

pub fn c_extract_imm_5_4_0(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 1) + (((instruction[0] as u64) & 0x7c) >> 2)
}

pub fn c_extract_imm_17_16_12(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 13) + (((instruction[0] as u64) & 0x7c) << 10)
}

pub fn c_extract_imm_9_4_5_8_7_5(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 5)
        + (((instruction[0] as u64) & 0x18) << 4)
        + (((instruction[0] as u64) & 0x20) << 1)
        + (((instruction[0] as u64) & 0x4) << 3)
        + (((instruction[0] as u64) & 0x40) >> 2)
}

pub fn c_extract_offset_8_4_3_7_6_2_1_5(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 4)
        + (((instruction[0] as u64) & 0x60) << 1)
        + (((instruction[0] as u64) & 0x4) << 3)
        + (((instruction[1] as u64) & 0xc) << 2)
        + (((instruction[0] as u64) & 0x18) >> 2)
}

pub fn c_extract_offset_11_4_9_8_10_6_7_3_1_5(instruction: &Vec<u8>) -> u64 {
    (((instruction[1] as u64) & 0x10) << 7)
        + (((instruction[1] as u64) & 0x1) << 10)
        + (((instruction[1] as u64) & 0x6) << 7)
        + (((instruction[0] as u64) & 0x40) << 1)
        + (((instruction[0] as u64) & 0x80) >> 1)
        + (((instruction[0] as u64) & 0x4) << 3)
        + (((instruction[1] as u64) & 0x8) << 1)
        + (((instruction[0] as u64) & 0x18) >> 2)
}

pub fn extend_sign_6bit(value: u64) -> u64 {
    (value + 0x7fffffffffffffe0) ^ 0x7fffffffffffffe0
}

pub fn extend_sign_9bit(value: u64) -> u64 {
    (value + 0x7ffffffffffffe00) ^ 0x7ffffffffffffe00
}

pub fn extend_sign_10bit(value: u64) -> u64 {
    (value + 0x7ffffffffffffe00) ^ 0x7ffffffffffffe00
}

pub fn extend_sign_18bit(value: u64) -> u64 {
    (value + 0x7fffffffffff2000) ^ 0x7fffffffffff2000
}

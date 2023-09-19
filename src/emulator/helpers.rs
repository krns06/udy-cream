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

pub fn extend_sign_21bit (value: u64) -> u64 {
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

pub fn truncate_top_8bit(value: u64) -> u64 {
    value & 0xff
}

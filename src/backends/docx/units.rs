pub const DXA_INCH: i32 = 1440; // standard measuring unit in Word

pub fn inches(i: f32) -> u32 {
    (DXA_INCH as f32 * i) as u32
}

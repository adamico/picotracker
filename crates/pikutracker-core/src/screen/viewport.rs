use super::frame::{LOGICAL_HEIGHT, LOGICAL_WIDTH};

pub fn integer_scale(window_width: u32, window_height: u32) -> u32 {
    (window_width / LOGICAL_WIDTH)
        .min(window_height / LOGICAL_HEIGHT)
        .max(1)
}

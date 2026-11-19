//! How the logical screen scales to fit a window.

use super::frame::{LOGICAL_HEIGHT, LOGICAL_WIDTH};

/// The largest whole-number scale at which the logical screen fits, and never less than ×1.
pub fn integer_scale(window_width: u32, window_height: u32) -> u32 {
    (window_width / LOGICAL_WIDTH)
        .min(window_height / LOGICAL_HEIGHT)
        .max(1)
}

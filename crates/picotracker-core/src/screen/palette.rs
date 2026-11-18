//! The 16-colour PICO-8 palette.

/// One of the 16 PICO-8 colours, named as PICO-8's manual numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    DarkBlue = 1,
    DarkPurple = 2,
    DarkGreen = 3,
    Brown = 4,
    DarkGrey = 5,
    LightGrey = 6,
    White = 7,
    Red = 8,
    Orange = 9,
    Yellow = 10,
    Green = 11,
    Blue = 12,
    Lavender = 13,
    Pink = 14,
    LightPeach = 15,
}

impl Colour {
    /// Every colour, in PICO-8's order.
    pub const ALL: [Colour; 16] = [
        Colour::Black,
        Colour::DarkBlue,
        Colour::DarkPurple,
        Colour::DarkGreen,
        Colour::Brown,
        Colour::DarkGrey,
        Colour::LightGrey,
        Colour::White,
        Colour::Red,
        Colour::Orange,
        Colour::Yellow,
        Colour::Green,
        Colour::Blue,
        Colour::Lavender,
        Colour::Pink,
        Colour::LightPeach,
    ];

    /// The colour as opaque RGBA bytes.
    pub const fn rgba(self) -> [u8; 4] {
        match self {
            Colour::Black => [0x00, 0x00, 0x00, 0xff],
            Colour::DarkBlue => [0x1d, 0x2b, 0x53, 0xff],
            Colour::DarkPurple => [0x7e, 0x25, 0x53, 0xff],
            Colour::DarkGreen => [0x00, 0x87, 0x51, 0xff],
            Colour::Brown => [0xab, 0x52, 0x36, 0xff],
            Colour::DarkGrey => [0x5f, 0x57, 0x4f, 0xff],
            Colour::LightGrey => [0xc2, 0xc3, 0xc7, 0xff],
            Colour::White => [0xff, 0xf1, 0xe8, 0xff],
            Colour::Red => [0xff, 0x00, 0x4d, 0xff],
            Colour::Orange => [0xff, 0xa3, 0x00, 0xff],
            Colour::Yellow => [0xff, 0xec, 0x27, 0xff],
            Colour::Green => [0x00, 0xe4, 0x36, 0xff],
            Colour::Blue => [0x29, 0xad, 0xff, 0xff],
            Colour::Lavender => [0x83, 0x76, 0x9c, 0xff],
            Colour::Pink => [0xff, 0x77, 0xa8, 0xff],
            Colour::LightPeach => [0xff, 0xcc, 0xaa, 0xff],
        }
    }
}

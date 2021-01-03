#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ColorRGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorRGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_u32(u: u32) -> Self {
        Self::new((u >> 16) as u8, (u >> 8) as u8, u as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u32() {
        assert_eq!(ColorRGB::from_u32(0x0180FF), ColorRGB::new(1, 128, 255));
    }
}

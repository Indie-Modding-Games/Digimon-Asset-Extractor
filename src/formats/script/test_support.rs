//! The script builder the tests use: a header of sections, then code byte by byte.

pub struct ScriptBuilder {
    bytes: Vec<u8>,
}

impl ScriptBuilder {
    /// A script whose code starts after its list of sections.
    pub fn new(sections: &[(u16, u16)]) -> Self {
        let mut builder = ScriptBuilder { bytes: Vec::new() };
        builder.u16(2 + (4 * sections.len() as u16) + 2);
        for (id, offset) in sections {
            builder.u16(*id);
            builder.u16(*offset);
        }
        builder.u16(0xFFFF);
        builder
    }

    pub fn u8(&mut self, values: &[u8]) -> &mut Self {
        self.bytes.extend_from_slice(values);
        self
    }

    pub fn u16(&mut self, value: u16) -> &mut Self {
        self.u8(&value.to_le_bytes())
    }

    pub fn u32(&mut self, value: u32) -> &mut Self {
        self.u16(value as u16);
        self.u16((value >> 16) as u16)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

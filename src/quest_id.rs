/// Compact representation of a BetterQuesting quest identifier.
use serde::{Deserialize, Serialize};
///
/// Historically, BetterQuesting uses two 32-bit integers (high/low) to form a 64-bit id.
/// This type stores only a single `u64`, and provides helpers to extract or construct with high/low parts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Copy)]
pub struct QuestId(u64);

impl QuestId {
    /// New from combined u64
    pub fn from_u64(id: u64) -> Self {
        QuestId(id)
    }

    /// New from explicit signed high/low
    pub fn from_parts(high: i32, low: i32) -> Self {
        let hi = high as i64 as u64;
        let lo = low as u32 as u64;
        QuestId((hi << 32) | lo)
    }

    /// The combined value as u64.
    pub fn as_u64(self) -> u64 {
        self.0
    }
    /// The high part as signed i32.
    pub fn high_part(self) -> i32 {
        (self.0 >> 32) as u32 as i32
    }
    /// The low part as signed i32.
    pub fn low_part(self) -> i32 {
        (self.0 & 0xFFFF_FFFF) as u32 as i32
    }
    /// The high part as unsigned u32.
    pub fn high_u32(self) -> u32 {
        (self.0 >> 32) as u32
    }
    /// The low part as unsigned u32.
    pub fn low_u32(self) -> u32 {
        self.0 as u32
    }
}

#[cfg(test)]
mod tests {
    use super::QuestId;

    #[test]
    fn questid_roundtrip_zero() {
        let qid = QuestId::from_parts(0, 0);
        assert_eq!(qid.as_u64(), 0);
        let qid2 = QuestId::from_u64(0);
        assert_eq!(qid, qid2);
        assert_eq!(qid2.high_part(), 0);
        assert_eq!(qid2.low_part(), 0);
        assert_eq!(qid2.high_u32(), 0);
        assert_eq!(qid2.low_u32(), 0);
    }

    #[test]
    fn questid_roundtrip_all_ones() {
        let qid = QuestId::from_parts(-1, -1);
        let u = qid.as_u64();
        let qid2 = QuestId::from_u64(u);
        assert_eq!(qid, qid2);
        assert_eq!(qid.high_part(), -1);
        assert_eq!(qid.low_part(), -1);
        assert_eq!(qid.high_u32(), 0xFFFF_FFFF);
        assert_eq!(qid.low_u32(), 0xFFFF_FFFF);
    }

    #[test]
    fn questid_extreme_high_low() {
        let hi = i32::MAX;
        let lo = i32::MIN;
        let qid = QuestId::from_parts(hi, lo);
        let u = qid.as_u64();
        let qid2 = QuestId::from_u64(u);
        assert_eq!(qid, qid2);
        assert_eq!(qid.high_part(), i32::MAX);
        assert_eq!(qid.low_part(), i32::MIN);
        assert_eq!(qid.high_u32(), i32::MAX as u32);
        assert_eq!(qid.low_u32(), i32::MIN as u32);
    }

    #[test]
    fn questid_unsigned_roundtrip() {
        let qid = QuestId::from_parts(0x12345678, 0x9ABCDEF0u32 as i32);
        let u = qid.as_u64();
        let qid2 = QuestId::from_u64(u);
        assert_eq!(qid2.high_u32(), 0x12345678);
        assert_eq!(qid2.low_u32(), 0x9ABCDEF0);
    }
}

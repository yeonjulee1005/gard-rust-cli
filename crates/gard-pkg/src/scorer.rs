use gard_core::{TierResult, Verdict};

pub fn aggregate(tier1: &TierResult, tier2: &TierResult, tier3: &TierResult) -> Verdict {
    if tier1.is_blocking() || tier2.is_blocking() || tier3.is_blocking() {
        return Verdict::Block;
    }
    if tier1.is_flagged() || tier2.is_flagged() || tier3.is_flagged() {
        return Verdict::Warn;
    }
    Verdict::Pass
}

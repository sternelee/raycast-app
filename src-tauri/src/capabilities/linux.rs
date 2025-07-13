use anyhow::Result;
use caps::{CapSet, Capability};

pub fn grant_capability() -> Result<()> {
    caps::raise(None, CapSet::Effective, Capability::CAP_DAC_OVERRIDE)?;
    Ok(())
}

pub fn revoke_capability() -> Result<()> {
    caps::clear(None, CapSet::Effective)?;
    caps::clear(None, CapSet::Permitted)?;
    Ok(())
}

pub fn can_use_capability() -> bool {
    caps::has_cap(None, CapSet::Permitted, Capability::CAP_DAC_OVERRIDE).unwrap_or(false)
}

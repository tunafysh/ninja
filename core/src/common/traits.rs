 
use crate::common::types::InstallStage;
use anyhow::Result;

pub trait Reporter {
    fn stage(&self, stage: InstallStage) -> Result<()>;
    fn progress(&self, percent: u8) -> Result<()>;
}

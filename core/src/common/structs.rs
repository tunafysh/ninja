use crate::common::traits::Reporter;
use anyhow::Result;

pub struct NoopReporter {}

impl Reporter for NoopReporter {
    fn progress(&self, _progress: u8) -> Result<()> {
        Ok(())
    }

    fn stage(&self, _stage: super::types::InstallStage) -> Result<()> {
        Ok(())
    }
}

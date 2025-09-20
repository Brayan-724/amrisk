use std::ops::ControlFlow;

use miette::Diagnostic;
use thiserror::Error;

use crate::generator::{AnalyzeImmInvalid, AnalyzeInsNotExist, AnalyzeOffsetInvalid, AnalyzeRegInvalid};
use crate::nodes::AnalyzeWhileConditionError;

pub type AnalyzeResult = ControlFlow<(), ()>;

#[derive(Default)]
pub struct AnalyzeSummary {
    errors: Vec<AnalyzeError>,
}

impl AnalyzeSummary {
    pub fn error(&mut self, error: impl Into<AnalyzeError>) {
        self.errors.push(error.into())
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn report_on(self, source: String) {
        for error in self.errors {
            eprintln!(
                "{:?}",
                miette::Report::new(error).with_source_code(source.clone())
            );
        }
    }
}

pub trait Analyze {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult;

    fn analyzed(&self) -> AnalyzeSummary {
        let mut summary = AnalyzeSummary::default();

        _ = self.analyze(&mut summary);

        summary
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum AnalyzeError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    InsNotExist(#[from] AnalyzeInsNotExist),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ImmInvalid(#[from] AnalyzeImmInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    OffsetInvalid(#[from] AnalyzeOffsetInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    RegInvalid(#[from] AnalyzeRegInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    WhileConditionError(#[from] AnalyzeWhileConditionError),
}

impl<T: Analyze> Analyze for Vec<T> {
    fn analyze(&self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        for v in self {
            v.analyze(summary)?;
        }

        ControlFlow::Continue(())
    }
}

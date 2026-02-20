use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::ControlFlow;

use miette::Diagnostic;
use thiserror::Error;

use crate::{generator, nodes};

pub type AnalyzeResult = ControlFlow<(), ()>;

#[derive(Default)]
pub struct AnalyzeSummary {
    errors: Vec<AnalyzeError>,
    stores: HashMap<TypeId, Box<dyn Any>>,
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

    pub fn store<T: AnalyzeStore>(&mut self) -> &mut T::Store {
        let type_id = TypeId::of::<T>();

        self.stores
            .entry(type_id)
            .or_insert_with(|| Box::from(T::Store::default()))
            .downcast_mut()
            .expect("`TypeId` ensures `Any` type safety")
    }
}

pub trait AnalyzeStore: 'static + Sized {
    type Store: Default;

    fn store(summary: &mut AnalyzeSummary) -> &mut Self::Store {
        summary.store::<Self>()
    }
}

pub trait Analyze {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult;

    fn analyzed(&mut self) -> AnalyzeSummary {
        let mut summary = AnalyzeSummary::default();

        _ = self.analyze(&mut summary);

        summary
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum AnalyzeError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    InsNotExist(#[from] generator::AnalyzeInsNotExist),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ImmInvalid(#[from] generator::AnalyzeImmInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    MissingSemicolon(#[from] nodes::AnalyzeMissingSemicolon),
    #[error(transparent)]
    #[diagnostic(transparent)]
    OffsetInvalid(#[from] generator::AnalyzeOffsetInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    RegInvalid(#[from] generator::AnalyzeRegInvalid),
    #[error(transparent)]
    #[diagnostic(transparent)]
    RegNotFound(#[from] generator::AnalyzeRegNotFound),
    #[error(transparent)]
    #[diagnostic(transparent)]
    VariableNotExistsError(#[from] nodes::AnalyzeVariableNotExistsError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    WhileConditionError(#[from] nodes::AnalyzeWhileConditionError),
}

impl<T: Analyze> Analyze for Vec<T> {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult {
        for v in self {
            v.analyze(summary)?;
        }

        ControlFlow::Continue(())
    }
}

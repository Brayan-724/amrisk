use std::any::TypeId;
use std::mem;
use std::ops::ControlFlow;

use indexmap::IndexSet;
use miette::Diagnostic;
use thiserror::Error;

use crate::shared_store::{StoreContainer, StoresContainer};
use crate::{generator, nodes};

/// whether if re-analyze or not
pub type AnalyzeResult = ControlFlow<bool, ()>;

#[derive(Default)]
pub struct AnalyzeSummary {
    errors: IndexSet<(AnalyzeError, TypeId)>,
    stores: StoresContainer,
}

impl AnalyzeSummary {
    pub fn clear_stores(&mut self) -> StoresContainer {
        mem::replace(&mut self.stores, StoresContainer::default())
    }

    pub fn error(&mut self, error: impl Into<AnalyzeError>) {
        self.errors.insert((error.into(), TypeId::of::<()>()));
    }

    pub fn error_marked<M: 'static + Sized>(&mut self, error: impl Into<AnalyzeError>) {
        self.errors.insert((error.into(), TypeId::of::<M>()));
    }

    // Return the number of removed errors
    pub fn remove_marked<M: 'static + Sized>(&mut self) -> usize {
        let len = self.errors.len();

        self.errors
            .retain(|(_, marker)| *marker != TypeId::of::<M>());

        len - self.errors.len()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn report_on(self, source: String) {
        for (error, _) in self.errors {
            eprintln!(
                "{:?}",
                miette::Report::new(error).with_source_code(source.clone())
            );
        }
    }
}

impl StoreContainer for AnalyzeSummary {
    fn container(&mut self) -> &mut StoresContainer {
        &mut self.stores
    }
}

pub trait Analyze {
    fn analyze(&mut self, summary: &mut AnalyzeSummary) -> AnalyzeResult;

    fn analyzed(&mut self) -> AnalyzeSummary {
        let mut summary = AnalyzeSummary::default();

        while let AnalyzeResult::Break(true) = self.analyze(&mut summary) {}

        summary
    }
}

#[derive(Hash, Debug, Error, Diagnostic, PartialEq, Eq)]
pub enum AnalyzeError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    FunctionExistsError(#[from] nodes::AnalyzeFunctionExistsError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    FunctionMismatchArgsCountError(#[from] nodes::AnalyzeFunctionMismatchArgsCountError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    FunctionNotExistsError(#[from] nodes::AnalyzeFunctionNotExistsError),
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

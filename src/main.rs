pub use crate as amrisk;

use miette::GraphicalReportHandler;

use crate::analysis::Analyze;
use crate::generator::Generate;
use crate::parser::ParseError;
use crate::pretty::PrettyPrint as _;

mod analysis;
mod generator;
mod nodes;
mod parser;
mod pretty;
mod shared_store;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();

    let src = include_str!("../examples/basic.rsk");

    let Ok(mut ast) = parser::parse(src).map_err(ParseError::report) else {
        return;
    };

    println!("{}", ast.pretty_printed(src));

    let mut summary = ast.analyzed();
    let has_errors = summary.has_errors();

    let mut store = summary.clear_store();
    store.clear_local();

    summary.report_on(src.into());

    if has_errors {
        return;
    }

    let buf = ast.generated(store);

    println!("{buf:#}");
}

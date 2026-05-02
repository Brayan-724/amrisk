use miette::GraphicalReportHandler;

use amrisk::analysis::Analyze;
use amrisk::generator::Generate;
use amrisk::parser::{self, ParseError};
use amrisk::pretty::PrettyPrint as _;

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

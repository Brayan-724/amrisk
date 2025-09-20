use miette::GraphicalReportHandler;

use crate::analysis::Analyze;
use crate::parser::{ParseError, PrettyPrint};

mod analysis;
mod generator;
mod nodes;
mod parser;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();

    let src = include_str!("../examples/basic.rsk");

    let Ok(ast) = parser::parse(src).map_err(ParseError::report) else {
        return;
    };

    println!("{}", ast.pretty_printed(src));

    let summary = ast.analyzed();
    // let has_errors = summary.has_errors();

    summary.report_on(src.into());
}

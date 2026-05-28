use ml_lua::parser::parse;
use skim::Parsed;
use std::sync::Arc;

fn main() {
    let source = "let x = 2";
    let report: miette::Report = match parse(Arc::from("test"), source) {
        Parsed::Ok(v) => {
            dbg!(v);
            return;
        }
        Parsed::Err(err) => err.into(),
        Parsed::Fatal(err) => err.into(),
        Parsed::Recover(err, v) => {
            dbg!(v);
            err.into()
        }
    };
    println!("{:?}", report.with_source_code(source));
}

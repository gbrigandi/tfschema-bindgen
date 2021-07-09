//! # Terraform Provider Schema to Serde code generator
//!
//! '''bash
//! cargo run --bin tfbindgen -- --help
//! '''

use std::path::PathBuf;
use structopt::StructOpt;
use tfschema_bindgen::binding::{
    export_schema_to_registry, generate_serde, read_tf_schema_from_file,
};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Terraform schema to Serde transformer",
    about = "Generate code for Serde containers from Terraform provider schema"
)]
struct Options {
    /// Path to the JSON-encoded terraform schema.
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let options = Options::from_args();
    let schema_deserialized = options.input.as_ref().map(|input| read_tf_schema_from_file(input).unwrap());
    let registry = export_schema_to_registry(&schema_deserialized.as_ref().unwrap())
        .expect("Error exporting terraform provider schema to serde-reflection");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    generate_serde("default", &mut out, &registry).expect("Error generating serde model")
}

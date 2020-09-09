//! This crate aims to compile schemas extracted from Terraform into Serde type definitions.
//!
//! ## Quick Start
//!
//! A Terraform schema is required for generating Rust types responsible of deserialization and serialization.   
//! It can either be exported from your Terraform plan or manually generated.
//! We'll take the latter approach, therefore defining a reference schema with just one provider type having one attribute:
//!
//! ```json
//!{
//!    "provider_schemas": {
//!        "test_provider": {
//!            "provider": {
//!                "version": 0,
//!                "block": {
//!                    "attributes": {
//!                        "base_url": {
//!                            "type": "string",
//!                            "description": "The url.",
//!                            "optional": true
//!                        }
//!                    }
//!                }
//!            }
//!        }
//!    },
//!    "format_version": "0.1"
//!}
//! ```
//!
//! In addition to a Rust library, this crate provides a binary tool `tfbindgen` to process Terraform schemas
//! saved on disk.
//! Outside of this repository, you may install the tool with:
//!
//! ```bash
//! cargo install tfschema-bindgen
//! ```
//!
//! Then use `$HOME/.cargo/bin/tfbindgen`.
//!
//! We're going to use this tool assuming that we're inside the repository.
//!
//! The following command will generate Serde bindings from the previous definitions, outputting those to `test.rs` module:
//!
//! ```bash
//! cargo run --bin tfbindgen -- test.json > test.rs
//! ```
//!
//! The following is a Rust example snippet comprising the previously generated bindings and a main function building on these in order
//! deserialize a configuration descriptor adhering to our Terraform schema:
//!
//! ```
//! #![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
//! use std::collections::BTreeMap as Map;
//! use serde::{Serialize, Deserialize};
//! use serde_bytes::ByteBuf as Bytes;
//!
//! #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
//! pub struct config {
//!    pub datasource: Option<Vec<datasource_root>>,
//!    pub provider: Option<Vec<provider_root>>,
//!    pub resource: Option<Vec<resource_root>>,
//! }
//!
//! #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
//! pub enum datasource_root {
//! }
//!
//! #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
//! pub enum provider_root {
//!    test_provider(Box<Vec<test_provider_details>>),
//! }
//!
//! #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
//! pub enum resource_root {
//! }
//!
//! #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
//! pub struct test_provider_details {
//!    pub base_url: Option<String>,
//! }
//!
//! const TF_JSON_CONFIG: &str = r#"{
//!    "provider": [
//!      {
//!        "test_provider": [
//!          {
//!            "base_url": "https://acme.com/foo"
//!          }
//!        ]
//!      }
//!    ]
//! }"#;
//!
//! fn main() -> Result<(), std::io::Error> {
//!    let res: config = serde_json::from_str(TF_JSON_CONFIG).unwrap();
//!
//!    assert_eq!(res.provider.as_ref().map(|x| x.is_empty()), Some(false));
//!    assert_eq!(
//!        res.provider.as_ref().map(|x| x.get(0).is_none()),
//!        Some(false)
//!    );
//!    let prv = res
//!        .provider
//!        .as_ref()
//!        .and_then(|x| x.get(0))
//!        .and_then(|x| match x {
//!            provider_root::test_provider(p) => p.get(0),
//!        });
//!    assert_eq!(prv.is_none(), false);
//!    assert_eq!(
//!        prv.and_then(|x| x.base_url.to_owned()),
//!        Some("https://acme.com/foo".to_owned())
//!    );
//!    print!("success!\n");
//!    Ok(())
//! }
//! ```
//! ## Quickstart Example
//!
//! In addition to a Rust library and generation tool, this crate provides the above example which
//! can be executed using the following command:
//!
//! ```bash
//! cargo run --example quickstart
//! ```
//!
//! ## Consuming third-party Terraform schemas
//!
//! In order to operate on Terraform configuration descriptors of third-party providers, Rust bindings have to be generated using the
//! provided schema descriptor in the JSON format.
//!   
//! Firstly, create a minimal Terraform plan referring declaring the target provider. The following is an example for enabling
//! the Amazon Web Services (AWS) Terraform provider:
//!
//! ```code
//! provider "aws" {
//!  version = ">= 2.31.0, < 3.0"
//!}
//! ```
//!
//! Initialize the Terraform plan so that the provider is installed in the local environment:
//!
//! ```bash
//! terraform init
//! ```
//!
//! Secondly, extract the schema for the providers defined in the Terraform plan, AWS in this case:
//!
//! ```bash
//! terraform providers schema -json > aws-provider-schema.json
//! ```
//!
//! Finally, generate the Rust (de)serialization types for the given provider using the following command (assuming you are inside the repository):
//!
//! ```bash
//! cargo run --bin tfbindgen -- aws-provider-schema.json > aws_provider_schema.rs
//! ```
//!
//! In order do (de)serialize provider's configuration, import the generated module in your application.
//!

// registry creation
pub mod binding;

// code generator
pub mod emit;

// configuraiton support for code generation
pub mod config;

/// Utility functions to help testing code generators.
#[doc(hidden)]
pub mod test_utils;

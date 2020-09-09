# tfschema-bindgen

[![tfschena-bindgen on crates.io](https://img.shields.io/crates/v/tfschema-bindgen)](https://crates.io/crates/tfschema-bindgen)
[![Documentation (latest release)](https://docs.rs/tfschema-bindgen/badge.svg)](https://docs.rs/tfschema-bindgen/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](../LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](../LICENSE-MIT)

This crate aims to compile schemas extracted from Terraform into Serde type definitions.

### Quick Start

A Terraform schema is required for generating the Rust types responsible of deserialization and serialization from Rust.
It can either be exported from your Terraform plan or manually generated.
We'll take the latter approach, defining an reference schema with just one provider type, exposing just one attribute :

```json
{
  "provider_schemas" : {
   "test_provider" : {
      "provider" : {
         "version" : 0,
         "block" : {
            "attributes" : {
               "base_url" : {
                  "type" : "string",
                  "description" : "The url.",
                  "optional" : true
               }
           }
       }
   },
"format_version" : "0.1"
}
```

In addition to a Rust library, this crate provides a binary tool `tf-schemabindgen` to process Terraform schemas
saved on disk.
Outside of this repository, you may install the tool with:

```bash
cargo install tf-schemabindgen
```

Then use `$HOME/.cargo/bin/tfschema-bindgen`.

We're going to use this tool assuming that we're inside the repository.

The following command will generate Rust class definitions from the previous definitions written in the 'test.json' file and write them into `test.rs`:

```bash
cargo run --bin tfschema-bindgen -- test.json > test.rs
```

This is how the generated Rust definitions followed by how these can be consumed for parsing a Terraform configuration descriptor :

```rust
#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_bytes::ByteBuf as Bytes;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct config {
   pub datasource: Option<Vec<datasource_root>>,
   pub provider: Option<Vec<provider_root>>,
   pub resource: Option<Vec<resource_root>>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum datasource_root {
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum provider_root {
   test_provider(Box<Vec<test_provider_details>>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum resource_root {
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_provider_details {
   pub base_url: Option<String>,
}

const TF_JSON_CONFIG: &str = r#"{
   "provider": [
     {
       "test_provider": [
         {
           "base_url": "https://acme.com/foo"
         }
       ]
     }
   ]
}"#;

fn main() -> Result<(), std::io::Error> {
   let res: config = serde_json::from_str(TF_JSON_CONFIG).unwrap();

   assert_eq!(res.provider.as_ref().map(|x| x.is_empty()), Some(false));
   assert_eq!(
       res.provider.as_ref().map(|x| x.get(0).is_none()),
       Some(false)
   );
   let prv = res
       .provider
       .as_ref()
       .and_then(|x| x.get(0))
       .and_then(|x| match x {
           provider_root::test_provider(p) => p.get(0),
       });
   assert_eq!(prv.is_none(), false);
   assert_eq!(
       prv.and_then(|x| x.base_url.to_owned()),
       Some("https://acme.com/foo".to_owned())
   );
   print!("success!\n");
   Ok(())
}
```
### Quickstart Example

In addition to a Rust library and generation tool, this crate provides the above example which
can be executed using the following command :

```bash
cargo run --example quickstart
```


## License

This project is available under the terms of either the [Apache 2.0 license](../LICENSE-APACHE) or the [MIT
license](../LICENSE-MIT).

<!--
README.md is generated from README.tpl by cargo readme. To regenerate:

cargo install cargo-readme
cargo readme > README.md
-->

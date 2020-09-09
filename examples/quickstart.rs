/*--- GENERATED START ---*/
#![allow(
    unused_imports,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf as Bytes;
use std::collections::BTreeMap as Map;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct config {
    pub datasource: Option<Vec<datasource_root>>,
    pub provider: Option<Vec<provider_root>>,
    pub resource: Option<Vec<resource_root>>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum datasource_root {}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum provider_root {
    test_provider(Box<Vec<test_provider_details>>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum resource_root {}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_provider_details {
    pub base_url: Option<String>,
}

/*--- GENERATED END ---*/

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

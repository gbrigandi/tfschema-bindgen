use crate::config::CodeGeneratorConfig;
use crate::emit::{CodeGenerator, Registry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_reflection::{ContainerFormat, Format, Named, VariantFormat};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

pub const RESERVED_WORDS: [&str; 32] = [
    "as",
    "break",
    "pub const",
    "continue",
    "else",
    "enum",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "mut",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "const",
    "box",
];

#[derive(Serialize, Deserialize)]
enum Void {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TerraformSchemaExport {
    provider_schemas: BTreeMap<String, Schema>,
    format_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schema {
    provider: Provider,
    data_source_schemas: Option<BTreeMap<String, SchemaItem>>,
    resource_schemas: Option<BTreeMap<String, SchemaItem>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Provider {
    version: i64,
    block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaItem {
    version: i64,
    block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    attributes: Option<BTreeMap<String, Attribute>>,
    block_types: Option<BTreeMap<String, NestedBlock>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StringKind {
    Plain,
    Markdown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    r#type: AttributeType,
    description: Option<String>,
    required: Option<bool>,
    optional: Option<bool>,
    computed: Option<bool>,
    sensitive: Option<bool>,
    description_kind: Option<StringKind>,
    deprecated: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NestedBlock {
    block: Block,
    nesting_mode: Option<String>,
    min_items: Option<u8>,
    max_items: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttributeType(Value);

pub fn generate_serde(
    config: &str,
    out: &mut dyn Write,
    registry: &Registry,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = CodeGeneratorConfig::new(config.to_string());

    CodeGenerator::new(&config).output(out, &registry)
}

pub fn export_schema_to_registry(
    schema: &TerraformSchemaExport,
) -> std::result::Result<Registry, Box<dyn std::error::Error>> {
    let mut r = Registry::new();
    let mut roots = BTreeMap::new();
    roots.insert("provider", Vec::<&str>::new());
    roots.insert("resource", Vec::<&str>::new());
    roots.insert("datasource", Vec::<&str>::new());

    for (pn, pv) in &schema.provider_schemas {
        let ps = &pv.provider;
        export_block(None, &pn, &ps.block, &mut r)?;
        if let Some(provider) = roots.get_mut("provider") {
            provider.push(pn);
        }

        if let Some(rss) = &pv.resource_schemas {
            for (n, i) in rss {
                let b = &i.block;
                export_block(Some("resource".to_owned()), &n, &b, &mut r)?;
                if let Some(resources) = roots.get_mut("resource") {
                    resources.push(n);
                }
            }
        }

        if let Some(dss) = &pv.data_source_schemas {
            for (n, i) in dss {
                let b = &i.block;
                export_block(Some("data_source".to_owned()), &n, &b, &mut r)?;
                if let Some(resources) = roots.get_mut("datasource") {
                    resources.push(n);
                }
            }
        }

        export_roots(&roots, &mut r)?;
        generate_config(&roots, &mut r)?;
    }
    Ok(r)
}

fn generate_config(
    roots: &BTreeMap<&str, Vec<&str>>,
    reg: &mut Registry,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut target_attrs = Vec::new();

    for root_name in roots.keys() {
        target_attrs.push(Named {
            name: root_name.to_string(),
            value: Format::Option(Box::new(Format::Seq(Box::new(Format::TypeName(format!(
                "{}_root",
                root_name
            )))))),
        });
    }
    reg.insert(
        (None, "config".to_string()),
        ContainerFormat::Struct(target_attrs),
    );
    Ok(())
}

fn export_roots(
    roots: &BTreeMap<&str, Vec<&str>>,
    reg: &mut Registry,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    for (root_name, root_members) in roots {
        let mut enumz = BTreeMap::new();
        for (pos, member) in root_members.iter().enumerate() {
            let mut variant_type_name = format!("Vec<Map<String, Vec<{}_details>>>", member);

            if root_name.to_string().eq("provider") {
                variant_type_name = format!("Vec<{}_details>", member);
            }

            enumz.insert(
                pos as u32,
                Named {
                    name: member.to_string(),
                    value: VariantFormat::NewType(Box::new(Format::TypeName(variant_type_name))),
                },
            );
        }
        reg.insert(
            (None, format!("{}_root", root_name.to_owned())),
            ContainerFormat::Enum(enumz),
        );
    }
    Ok(())
}

fn export_attributes(
    attrs: &BTreeMap<String, Attribute>,
) -> std::result::Result<Option<ContainerFormat>, Box<dyn std::error::Error>> {
    let mut target_attrs = Vec::new();
    for (an, at) in attrs {
        let an = RESERVED_WORDS
            .iter()
            .find(|w| an == &w.to_string())
            .map(|w| format!("r#{}", w))
            .unwrap_or_else(|| an.to_string());

        let f = match &at.r#type {
            AttributeType(Value::String(t)) if t == "string" => Format::Str,
            AttributeType(Value::String(t)) if t == "bool" => Format::Bool,
            AttributeType(Value::String(t)) if t == "number" => Format::I64,
            AttributeType(Value::String(t)) if t == "set" || t == "list" => {
                Format::Seq(Box::new(Format::Str))
            }
            AttributeType(Value::String(t)) if t == "map" => Format::Map {
                key: Box::new(Format::Str),
                value: Box::new(Format::Str),
            },
            AttributeType(Value::String(t)) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unknown type {}", t),
                )))
            }
            AttributeType(Value::Array(t))
                if t.first().unwrap() == "set" || t.first().unwrap() == "list" =>
            {
                Format::Seq(Box::new(Format::Str))
            }
            /* TODO: It will assume a map of strings even if the specified type is of a different kind (e.g. map of object) */
            AttributeType(Value::Array(t)) if t.first().unwrap() == "map" => Format::Map {
                key: Box::new(Format::Str),
                value: Box::new(Format::Str),
            },
            unknown => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Type {:?} not supported", unknown),
                )))
            }
        };
        let attr_fmt = match (at.optional, at.computed) {
            (Some(opt), _) if opt => Format::Option(Box::new(f.clone())),
            (_, Some(cmp)) if cmp => Format::Option(Box::new(f.clone())),
            _ => f.clone(),
        };

        target_attrs.push(Named {
            name: an,
            value: attr_fmt,
        });
    }
    if !target_attrs.is_empty() {
        Ok(Some(ContainerFormat::Struct(target_attrs)))
    } else {
        Ok(None)
    }
}

fn export_block(
    namespace: Option<String>,
    name: &str,
    blk: &Block,
    reg: &mut Registry,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cf1 = export_attributes(&blk.attributes.as_ref().unwrap())?;
    //reg.insert((None, format!("{}_details", name)), cf1.clone().unwrap());
    if let Some(bt) = &blk.block_types {
        let mut inner_block_types = Vec::new();
        for (block_type_name, nested_block) in bt {
            if let Some(attrs) = &nested_block.block.attributes {
                let nested_cf = export_attributes(attrs)?;
                let block_type_ns = namespace
                    .clone()
                    .map(|v| format!("{}_{}_block_type", name, v));
                let block_type_fqn = namespace
                    .clone()
                    .map(|v| format!("{}_{}_block_type_{}", name, v, block_type_name.to_owned()));
                reg.insert(
                    (block_type_ns.clone(), block_type_name.to_owned()),
                    nested_cf.unwrap(),
                );
                inner_block_types.push((block_type_name, block_type_fqn.unwrap()));
            }
        }

        let cf2 = match cf1 {
            Some(ContainerFormat::Struct(mut attrs)) => {
                for (_, (n, fqn)) in inner_block_types.iter().enumerate() {
                    attrs.push(Named {
                        name: n.to_string(),
                        value: Format::Option(Box::new(Format::Seq(Box::new(Format::TypeName(
                            fqn.to_string(),
                        ))))),
                    });
                }
                Some(ContainerFormat::Struct(attrs))
            }

            _ => cf1,
        };
        reg.insert((None, format!("{}_details", name)), cf2.unwrap());
    } else {
        reg.insert((None, format!("{}_details", name)), cf1.clone().unwrap());
    }

    Ok(())
}

pub fn read_tf_schema_from_file<P: AsRef<Path>>(
    path: P,
) -> std::result::Result<TerraformSchemaExport, Box<dyn std::error::Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path).expect("input file must be readable");
    let reader = BufReader::new(file);
    // Read the JSON contents of the file as an instance of `User`.
    let d: TerraformSchemaExport = serde_json::from_reader(reader)?;

    // Return the `Diagram`.
    Ok(d)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{config, datasource_root, provider_root, resource_root};
    use std::fs::File;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn test_deserialize_example_tf_schema() {
        let tf_schema = read_tf_schema_from_file("./tests/fixtures/test-provider-schema.json");

        assert!(tf_schema.is_ok());
        let test_schema = tf_schema
            .as_ref()
            .unwrap()
            .provider_schemas
            .get("test_provider");

        assert_eq!(tf_schema.as_ref().unwrap().provider_schemas.len(), 1);
        assert!(test_schema.is_some());
        assert_eq!(
            test_schema
                .unwrap()
                .data_source_schemas
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            test_schema.map(|x| x.resource_schemas.is_none()),
            Some(false)
        );
    }

    #[test]
    fn test_generate_registry_from_schema() {
        let tf_schema = read_tf_schema_from_file("./tests/fixtures/test-provider-schema.json");
        let registry = export_schema_to_registry(&tf_schema.as_ref().unwrap());

        assert!(registry.is_ok());
        assert_eq!(registry.unwrap().len(), 10);
    }

    #[test]
    fn test_generate_serde_model_from_registry() {
        let tf_schema = read_tf_schema_from_file("./tests/fixtures/test-provider-schema.json");
        let registry = export_schema_to_registry(&tf_schema.as_ref().unwrap());
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
    name = "testing"
    version = "0.1.0"
    edition = "2018"
    
    [dependencies]
    serde = { version = "1.0", features = ["derive"] }
    serde_bytes = "0.11"
    
    [workspace]
    "#,
        )
        .unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let source_path = dir.path().join("src/lib.rs");
        let mut source = File::create(&source_path).unwrap();
        generate_serde("test", &mut source, &registry.unwrap()).unwrap();
        // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
        let target_dir = std::env::current_dir().unwrap().join("../target");
        let status = Command::new("cargo")
            .current_dir(dir.path())
            .arg("build")
            .arg("--target-dir")
            .arg(target_dir)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_unmarshall_provider() {
        let res: config =
            serde_json::from_str(include_str!("../tests/fixtures/provider_test.json")).unwrap();
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
            prv.map(|x| x.api_token.to_owned()),
            Some("ABC12345".to_owned())
        );
    }

    #[test]
    fn test_unmarshall_resource() {
        let res: config =
            serde_json::from_str(include_str!("../tests/fixtures/resource_test.json")).unwrap();
        assert_eq!(res.resource.as_ref().map(|x| x.is_empty()), Some(false));
        assert_eq!(
            res.resource.as_ref().map(|x| x.get(0).is_none()),
            Some(false)
        );
        let res_a = res
            .resource
            .as_ref()
            .and_then(|x| x.get(0))
            .and_then(|x| match x {
                resource_root::test_resource_a(r1) => r1.get(0),
                _ => None,
            })
            .and_then(|x| x.get("test"))
            .and_then(|x| x.first());
        assert_eq!(res_a.is_none(), false);
        assert_eq!(
            res_a.map(|x| x.name.to_owned()),
            Some("test_resource_a".to_owned())
        );
    }

    #[test]
    fn test_unmarshall_datasource() {
        let res: config =
            serde_json::from_str(include_str!("../tests/fixtures/datasource_test.json")).unwrap();
        assert_eq!(res.datasource.as_ref().map(|x| x.is_empty()), Some(false));
        assert_eq!(
            res.datasource.as_ref().map(|x| x.get(0).is_none()),
            Some(false)
        );
        let res_a = res
            .datasource
            .as_ref()
            .and_then(|x| x.get(0))
            .and_then(|x| match x {
                datasource_root::test_data_source_b(ds1) => ds1.get(0),
                _ => None,
            })
            .and_then(|x| x.get("test"))
            .and_then(|x| x.first());
        assert_eq!(res_a.is_none(), false);
        assert_eq!(
            res_a.map(|x| x.name.to_owned()),
            Some("test_datasource_b".to_owned())
        );
    }

    #[test]
    fn test_unmarshall_block_type() {
        let res: config =
            serde_json::from_str(include_str!("../tests/fixtures/block_type_test.json")).unwrap();
        assert_eq!(res.datasource.as_ref().map(|x| x.is_empty()), Some(false));
        assert_eq!(
            res.datasource.as_ref().map(|x| x.get(0).is_none()),
            Some(false)
        );
        let res_a = res
            .datasource
            .as_ref()
            .and_then(|x| x.get(0))
            .and_then(|x| match x {
                datasource_root::test_data_source_a(ds1) => ds1.get(0),
                _ => None,
            })
            .and_then(|x| x.get("test"))
            .and_then(|x| x.first());
        assert_eq!(res_a.is_none(), false);
        assert_eq!(
            res_a.map(|x| x.name.to_owned()),
            Some("test_datasource_a".to_owned())
        );
        assert_eq!(res_a.map(|x| x.datasource_a_type.is_none()), Some(false));
        assert_eq!(
            res_a.and_then(|x| x.datasource_a_type.as_ref().map(|x| x.is_empty())),
            Some(false)
        );
        assert_eq!(
            res_a.and_then(|x| x
                .datasource_a_type
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .filter_type
                .to_owned()),
            Some("REGEX".to_owned())
        );
    }
}
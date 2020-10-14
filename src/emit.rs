// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

//!
//! Stripped down version of serde reflection's code generator for Rust.
//! Main changes are around supporting qualified names for Regitry entries as well
//! as well as customing stubs generation.
//!
use crate::config::CodeGeneratorConfig;
use serde_generate::indent::{IndentConfig, IndentedWriter};
use serde_reflection::{ContainerFormat, Format, Named, VariantFormat};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{Result, Write};

/// A map of container formats indexed by a qualified name
pub type QualifiedName = (Option<String>, String);
pub type Registry = BTreeMap<QualifiedName, ContainerFormat>;

/// Main configuration object for code-generation in Rust.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
    /// Which derive macros should be added (independently from serialization).
    derive_macros: Vec<String>,
    /// Additional block of text added before each new container definition.
    custom_derive_block: Option<String>,
    /// Whether definitions and fields should be marked as `pub`.
    track_visibility: bool,
}

/// Shared state for the code generation of a Rust source file.
struct RustEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Track which definitions have a known size. (Used to add `Box` types.)
    known_sizes: Cow<'a, HashSet<&'a str>>,
    /// Current namespace (e.g. vec!["my_package", "my_module", "MyClass"])
    current_namespace: Vec<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a Rust code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        Self {
            config,
            derive_macros: vec!["Clone", "Debug", "PartialEq", "PartialOrd"]
                .into_iter()
                .map(String::from)
                .collect(),
            custom_derive_block: None,
            track_visibility: true,
        }
    }

    /// Which derive macros should be added (independently from serialization).
    pub fn with_derive_macros(mut self, derive_macros: Vec<String>) -> Self {
        self.derive_macros = derive_macros;
        self
    }

    /// Additional block of text added after `derive_macros` (if any), before each new
    /// container definition.
    pub fn with_custom_derive_block(mut self, custom_derive_block: Option<String>) -> Self {
        self.custom_derive_block = custom_derive_block;
        self
    }

    /// Whether definitions and fields should be marked as `pub`.
    pub fn with_track_visibility(mut self, track_visibility: bool) -> Self {
        self.track_visibility = track_visibility;
        self
    }

    /// Write container definitions in Rust.
    pub fn output(
        &self,
        out: &mut dyn Write,
        registry: &Registry,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let external_names: BTreeSet<String> = self
            .config
            .external_definitions
            .values()
            .cloned()
            .flatten()
            .collect();

        let known_sizes = external_names
            .iter()
            .map(<String as std::ops::Deref>::deref)
            .collect::<HashSet<_>>();

        let current_namespace = self
            .config
            .module_name
            .split('.')
            .map(String::from)
            .collect();
        let mut emitter = RustEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(4)),
            generator: self,
            known_sizes: Cow::Owned(known_sizes),
            current_namespace,
        };

        emitter.output_preamble()?;
        for ((ns, name), format) in registry {
            emitter.output_container(ns, name, format)?;
            emitter.known_sizes.to_mut().insert(name);
        }
        Ok(())
    }
}

impl<'a, T> RustEmitter<'a, T>
where
    T: std::io::Write,
{
    fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(doc) = self.generator.config.comments.get(&path) {
            let text = textwrap::indent(doc, "/// ").replace("\n\n", "\n///\n");
            write!(self.out, "\n{}", text)?;
        }
        Ok(())
    }

    fn output_preamble(&mut self) -> Result<()> {
        let external_names = self
            .generator
            .config
            .external_definitions
            .values()
            .cloned()
            .flatten()
            .collect::<HashSet<_>>();
        writeln!(self.out, "#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]")?;
        if !external_names.contains("Map") {
            writeln!(self.out, "use std::collections::BTreeMap as Map;")?;
        }
        writeln!(self.out, "use serde::{{Serialize, Deserialize}};")?;
        if !external_names.contains("Bytes") {
            writeln!(self.out, "use serde_bytes::ByteBuf as Bytes;")?;
        }
        for (module, definitions) in &self.generator.config.external_definitions {
            // Skip the empty module name.
            if !module.is_empty() {
                writeln!(
                    self.out,
                    "use {}::{{{}}};",
                    module,
                    definitions.to_vec().join(", "),
                )?;
            }
        }
        writeln!(self.out)?;
        Ok(())
    }

    fn output_field_annotation(&mut self, format: &Format) -> std::io::Result<()> {
        use Format::*;
        match format {
            Str => writeln!(
                self.out,
                "#[serde(skip_serializing_if = \"String::is_empty\")]"
            )?,
            Option(_) => writeln!(
                self.out,
                "#[serde(skip_serializing_if = \"Option::is_none\")]"
            )?,
            _ => (),
        }

        Ok(())
    }

    fn quote_type(format: &Format, known_sizes: Option<&HashSet<&str>>) -> String {
        use Format::*;
        match format {
            TypeName(x) => {
                if let Some(set) = known_sizes {
                    if !set.contains(x.as_str()) {
                        return format!("Box<{}>", x);
                    }
                }
                x.to_string()
            }
            Unit => "()".into(),
            Bool => "bool".into(),
            I8 => "i8".into(),
            I16 => "i16".into(),
            I32 => "i32".into(),
            I64 => "i64".into(),
            I128 => "i128".into(),
            U8 => "u8".into(),
            U16 => "u16".into(),
            U32 => "u32".into(),
            U64 => "u64".into(),
            U128 => "u128".into(),
            F32 => "f32".into(),
            F64 => "f64".into(),
            Char => "char".into(),
            Str => "String".into(),
            Bytes => "Bytes".into(),

            Option(format) => format!("Option<{}>", Self::quote_type(format, known_sizes)),
            Seq(format) => format!("Vec<{}>", Self::quote_type(format, None)),
            Map { key, value } => format!(
                "Map<{}, {}>",
                Self::quote_type(key, None),
                Self::quote_type(value, None)
            ),
            Tuple(formats) => format!("({})", Self::quote_types(formats, known_sizes)),
            TupleArray { content, size } => {
                format!("[{}; {}]", Self::quote_type(content, known_sizes), *size)
            }

            Variable(_) => panic!("unexpected value"),
        }
    }

    fn quote_types(formats: &[Format], known_sizes: Option<&HashSet<&str>>) -> String {
        formats
            .iter()
            .map(|x| Self::quote_type(x, known_sizes))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn output_fields(&mut self, base: &[&str], fields: &[Named<Format>]) -> Result<()> {
        // Do not add 'pub' within variants.
        let prefix = if base.len() <= 1 && self.generator.track_visibility {
            "pub "
        } else {
            ""
        };
        for field in fields {
            self.output_comment(&field.name)?;
            self.output_field_annotation(&field.value)?;
            writeln!(
                self.out,
                "{}{}: {},",
                prefix,
                field.name,
                Self::quote_type(&field.value, Some(&self.known_sizes)),
            )?;
        }
        Ok(())
    }

    fn output_variant(&mut self, base: &str, name: &str, variant: &VariantFormat) -> Result<()> {
        self.output_comment(name)?;
        use VariantFormat::*;
        match variant {
            Unit => writeln!(self.out, "{},", name),
            NewType(format) => writeln!(
                self.out,
                "{}({}),",
                name,
                Self::quote_type(format, Some(&self.known_sizes))
            ),
            Tuple(formats) => writeln!(
                self.out,
                "{}({}),",
                name,
                Self::quote_types(formats, Some(&self.known_sizes))
            ),
            Struct(fields) => {
                writeln!(self.out, "{} {{", name)?;
                self.current_namespace.push(name.to_string());
                self.out.indent();
                self.output_fields(&[base, name], fields)?;
                self.out.unindent();
                self.current_namespace.pop();
                writeln!(self.out, "}},")
            }
            Variable(_) => panic!("incorrect value"),
        }
    }

    fn output_variants(
        &mut self,
        base: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        for (expected_index, (index, variant)) in variants.iter().enumerate() {
            assert_eq!(*index, expected_index as u32);
            self.output_variant(base, &variant.name, &variant.value)?;
        }
        Ok(())
    }

    fn output_container(
        &mut self,
        namespace: &Option<String>,
        name: &str,
        format: &ContainerFormat,
    ) -> Result<()> {
        self.output_comment(name)?;
        let mut derive_macros = self.generator.derive_macros.clone();
        derive_macros.push("Serialize".to_string());
        derive_macros.push("Deserialize".to_string());
        let mut prefix = String::new();
        if !derive_macros.is_empty() {
            prefix.push_str(&format!("#[derive({})]\n", derive_macros.join(", ")));
        }
        if let Some(text) = &self.generator.custom_derive_block {
            prefix.push_str(text);
            prefix.push_str("\n");
        }

        use ContainerFormat::*;
        match format {
            UnitStruct => writeln!(self.out, "{}struct {};\n", prefix, name),
            NewTypeStruct(format) => writeln!(
                self.out,
                "{}struct {}({}{});\n",
                prefix,
                name,
                if self.generator.track_visibility {
                    "pub "
                } else {
                    ""
                },
                Self::quote_type(format, Some(&self.known_sizes))
            ),
            TupleStruct(formats) => writeln!(
                self.out,
                "{}struct {}({});\n",
                prefix,
                name,
                Self::quote_types(formats, Some(&self.known_sizes))
            ),
            Struct(fields) => {
                let mut struct_name = name.to_string();
                prefix.clear();
                derive_macros.push("Default".to_string());
                prefix.push_str(&format!("#[derive({})]\n", derive_macros.join(", ")));

                if let Some(ns) = namespace {
                    prefix.push_str(&format!("#[serde(rename = \"{}\")]\n", name));
                    struct_name = format!("{}_{}", ns, name)
                }

                if self.generator.track_visibility {
                    prefix.push_str("pub ");
                }

                writeln!(self.out, "{}struct {} {{", prefix, struct_name)?;
                self.current_namespace.push(name.to_string());
                self.out.indent();
                self.output_fields(&[name], fields)?;
                self.out.unindent();
                self.current_namespace.pop();
                writeln!(self.out, "}}\n")
            }
            Enum(variants) => {
                if self.generator.track_visibility {
                    prefix.push_str("pub ");
                }

                writeln!(self.out, "{}enum {} {{", prefix, name)?;
                self.current_namespace.push(name.to_string());
                self.out.indent();
                self.output_variants(name, variants)?;
                self.out.unindent();
                self.current_namespace.pop();
                writeln!(self.out, "}}\n")
            }
        }
    }
}

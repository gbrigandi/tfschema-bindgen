// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

/// Code generation options meant to be supported by all languages.
#[derive(Clone, Debug)]
pub struct CodeGeneratorConfig {
    pub(crate) module_name: String,
    pub(crate) external_definitions: ExternalDefinitions,
    pub(crate) comments: DocComments,
}

/// Track types definitions provided by external modules.
pub type ExternalDefinitions =
    std::collections::BTreeMap</* module */ String, /* type names */ Vec<String>>;

/// Track documentation to be attached to particular definitions.
pub type DocComments =
    std::collections::BTreeMap</* qualified name */ Vec<String>, /* comment */ String>;

impl CodeGeneratorConfig {
    /// Default config for the given module name.
    pub fn new(module_name: String) -> Self {
        Self {
            module_name,
            external_definitions: BTreeMap::new(),
            comments: BTreeMap::new(),
        }
    }

    /// Container names provided by external modules.
    pub fn with_external_definitions(mut self, external_definitions: ExternalDefinitions) -> Self {
        self.external_definitions = external_definitions;
        self
    }

    /// Comments attached to particular entity.
    pub fn with_comments(mut self, mut comments: DocComments) -> Self {
        // Make sure comments end with a (single) newline.
        for comment in comments.values_mut() {
            *comment = format!("{}\n", comment.trim());
        }
        self.comments = comments;
        self
    }
}

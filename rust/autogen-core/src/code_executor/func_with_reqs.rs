use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Alias {
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportFromModule {
    pub module: String,
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Import {
    Simple(String),
    Aliased(Alias),
    From(Box<ImportFromModule>),
}

impl fmt::Display for Import {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Import::Simple(name) => write!(f, "import {}", name),
            Import::Aliased(alias) => write!(f, "import {} as {}", alias.name, alias.alias),
            Import::From(from_module) => {
                let imports_str = from_module
                    .imports
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "from {} import {}", from_module.module, imports_str)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionWithRequirements {
    pub func: Cow<'static, str>,
    pub python_packages: Vec<String>,
    pub global_imports: Vec<Import>,
}

impl FunctionWithRequirements {
    pub fn new(
        func: impl Into<Cow<'static, str>>,
        python_packages: Vec<String>,
        global_imports: Vec<Import>,
    ) -> Self {
        Self {
            func: func.into(),
            python_packages,
            global_imports,
        }
    }
}

pub fn to_code(func: &FunctionWithRequirements) -> &str {
    &func.func
}

pub fn import_to_str(im: &Import) -> String {
    im.to_string()
}

pub fn build_python_functions_file(funcs: &[FunctionWithRequirements]) -> String {
    let mut global_imports = std::collections::HashSet::new();
    for func in funcs {
        for im in &func.global_imports {
            global_imports.insert(im.clone());
        }
    }

    let mut content = global_imports
        .iter()
        .map(import_to_str)
        .collect::<Vec<_>>()
        .join("\n");
    content.push_str("\n\n");

    for func in funcs {
        content.push_str(to_code(func));
        content.push_str("\n\n");
    }

    content
}
//! Snippet, Layer, Context, Requirement, UsageEvent, Template y configuración.

pub mod body;
pub mod names;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    User,
    Project,
}

impl Layer {
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::User => "user",
            Layer::Project => "project",
        }
    }

    pub fn parse(s: &str) -> Option<Layer> {
        match s {
            "user" => Some(Layer::User),
            "project" => Some(Layer::Project),
            _ => None,
        }
    }
}

/// Un snippet tal como está en su fichero. La clave es el id estable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snippet {
    pub key: String,
    pub prefixes: Vec<String>,
    pub description: String,
    pub body: String,
    pub requires: Vec<String>,
    pub context: Option<Vec<String>>,
}

/// Identidad de un snippet para el registro de usos.
/// `scope` es la raíz del proyecto en la capa de proyecto y vacío en la de usuario.
/// `language` es el del fichero de snippets: "rust", "all"...
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SnippetId {
    pub layer: Layer,
    pub scope: String,
    pub language: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageEvent {
    pub id: SnippetId,
    pub project_root: String,
    /// Segundos Unix.
    pub at: i64,
}

/// Contenedor sintáctico del cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Context {
    Known(String),
    /// El árbol tiene errores que impiden decidirlo.
    Unknown,
}

/// Un nombre importado, descompuesto de una sentencia de import.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    /// Nombre que queda disponible en el fichero.
    pub name: String,
    /// La sentencia que importa solo este nombre.
    pub single: String,
    /// Ruta relativa al fichero ("./db", "super::x", "from .db").
    pub relative: bool,
    /// Texto de la sentencia completa de la que sale.
    pub statement: String,
    /// Posición de esa sentencia, para agrupar.
    pub start: usize,
}

/// Imports visibles donde está el cursor y dónde añadir los nuevos.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportScope {
    pub imports: Vec<Requirement>,
    /// Texto de cada sentencia de import, para los lenguajes que no se descomponen.
    pub statements: Vec<String>,
    pub insert_at: usize,
    pub newline_before: bool,
    pub indent: String,
    pub has_imports: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Literal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

/// Un nodo con su huella estructural: nombres y literales no cuentan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub hash: u64,
    /// Filas de 0 en adelante, ambas incluidas.
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    /// Se muestra al usuario.
    Warning(String),
    /// Solo va al log.
    Log(String),
}

/// `.molde/molde.toml`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Prefijo numérico de `sortText`: más bajo, más arriba.
    pub sort_priority: u32,
    /// Palabras que la generalización nunca convierte en placeholder.
    pub common_words: Vec<String>,
    /// Días sin uso para ser candidato a borrar.
    pub unused_days: i64,
    /// Por encima de este tamaño no se analiza con Treesitter.
    pub max_file_size_mb: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sort_priority: 50,
            common_words: Vec::new(),
            unused_days: 60,
            max_file_size_mb: 2,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}

/// `.molde/templates/<nombre>/template.toml`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub source: String,
    pub source_name: String,
    pub target: String,
    /// Rutas dentro de `source` que no se copian.
    #[serde(default)]
    pub exceptions: Vec<String>,
    #[serde(default)]
    pub post: Vec<String>,
    #[serde(default)]
    pub inject: Vec<Inject>,
    /// Fichero que se abre tras generar, relativo a `source`.
    #[serde(default)]
    pub main: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inject {
    pub file: String,
    pub marker: String,
    pub line: String,
}

impl Template {
    pub fn parse(text: &str) -> Result<Template, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}

//! Lo que el núcleo necesita del exterior: SnippetRepository, UsageStore, SyntaxAnalyzer, Clock, FileSystem y Shell.

use crate::domain::*;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Fecha de modificación y tamaño de un fichero.
pub type Stamp = (SystemTime, u64);

#[derive(Debug, Clone, PartialEq)]
pub struct SnippetFileError {
    pub line: usize,
    pub message: String,
}

pub trait SnippetRepository: Send {
    /// None si el fichero no existe.
    fn load(&self, path: &Path) -> Result<Option<Vec<Snippet>>, SnippetFileError>;
    /// Añade la entrada al final del fichero, creándolo si hace falta. Devuelve su línea, desde 0.
    fn append(&self, path: &Path, snippet: &Snippet) -> Result<u32, String>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UsageStats {
    pub first_seen: Option<i64>,
    pub uses: u64,
    pub last_use: Option<i64>,
}

pub trait UsageStore: Send {
    fn record(&mut self, event: &UsageEvent) -> Result<(), String>;
    /// Guarda `at` como fecha de alta de los snippets que aún no la tienen.
    fn register(&mut self, ids: &[SnippetId], at: i64) -> Result<(), String>;
    /// Una fila por id, en el mismo orden.
    fn stats(&self, ids: &[SnippetId]) -> Result<Vec<UsageStats>, String>;
}

pub trait SyntaxAnalyzer: Send {
    fn supports(&self, language: &str) -> bool;
    fn language_for_path(&self, path: &Path) -> Option<String>;
    /// Otros ficheros de snippets que ve un lenguaje: los TSX también ven los de TypeScript.
    fn extra_snippet_languages(&self, language: &str) -> Vec<String>;
    fn container(&self, language: &str, text: &str, offset: usize) -> Context;
    fn imports(&self, language: &str, text: &str, offset: usize) -> ImportScope;
    /// Descompone una sentencia de import. Vacío si el lenguaje no sabe descomponerla.
    fn parse_requirements(
        &self,
        language: &str,
        statement: &str,
    ) -> Result<Vec<Requirement>, String>;
    fn tokens(&self, language: &str, text: &str) -> Vec<Token>;
    fn fragments(&self, language: &str, text: &str, min_lines: usize) -> Vec<Fragment>;
}

pub trait Clock: Send {
    /// Segundos Unix.
    fn now(&self) -> i64;
}

pub trait FileSystem: Send {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;
    /// Crea los directorios que falten.
    fn write(&self, path: &Path, content: &[u8]) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn stamp(&self, path: &Path) -> Option<Stamp>;
    /// Ficheros bajo `dir`, ordenados, respetando .gitignore y sin ocultos.
    fn walk(&self, dir: &Path) -> Vec<PathBuf>;
}

pub trait Shell: Send {
    /// Ejecuta con `sh -c` en `cwd`. Devuelve si ha ido bien y la salida combinada.
    fn run(&self, command: &str, cwd: &Path) -> (bool, String);
}

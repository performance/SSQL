use pyo3::prelude::*;
use pyo3::create_exception;
use ssql_compiler::compiler::{Compiler as RustCompiler, CompilerConfig};
use std::sync::{Arc, Mutex};

// Custom Exception for SSQL Compilation Errors
create_exception!(ssql, CompileError, pyo3::exceptions::PyValueError);

/// Result object for safe compilation
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct CompilationResult {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub sql: Option<String>,
    #[pyo3(get)]
    pub error: Option<String>,
}

/// The main SSQL Compiler class exposed to Python
#[pyclass]
pub struct Compiler {
    // We use Arc<Mutex> to allow safe sharing of mutable compiler state (e.g., alias counters)
    // across Python threads if necessary, and to satisfy PyO3's requirements for shared references.
    inner: Arc<Mutex<RustCompiler>>,
}

#[pymethods]
impl Compiler {
    /// Initialize the compiler with native SSQL schema DDL (`... इति आख्या ... रच्यताम्`)
    /// and an explicit target dialect (`"sqlite"` or `"postgres"`).
    #[new]
    #[pyo3(signature = (schema_ddl, dialect, verbose=false, dump_ast=false, dump_resolved=false, dump_symtable=false))]
    pub fn new(
        schema_ddl: String,
        dialect: String,
        verbose: bool,
        dump_ast: bool,
        dump_resolved: bool,
        dump_symtable: bool
    ) -> PyResult<Self> {
        let config = CompilerConfig {
            dialect,
            verbose,
            dump_ast,
            dump_resolved,
            dump_symtable,
            ..CompilerConfig::default()
        };

        let mut compiler = RustCompiler::new(config)
             .map_err(|e| PyErr::new::<CompileError, _>(format!("Failed to initialize compiler: {}", e)))?;

        // Same schema-loading entry point the CLI uses for a discovered
        // schema.सं (src/compiler.rs::load_schema_from_ssql_string) -
        // tokenizes/parses the DDL text and registers it into the schema.
        compiler.load_schema_from_ssql_string(&schema_ddl)
            .map_err(|e| PyErr::new::<CompileError, _>(format!("Failed to load schema: {:#}", e)))?;

        Ok(Compiler {
            inner: Arc::new(Mutex::new(compiler)),
        })
    }

    /// Compile SSQL source string to SQL
    /// Raises ssql.CompileError on failure
    pub fn compile(&self, source: String) -> PyResult<String> {
        let mut compiler = self.inner.lock().map_err(|e| {
             PyErr::new::<CompileError, _>(format!("Compiler lock poisoned: {}", e))
        })?;
        
        // Each call sees the schema as loaded, never CTEs or DDL left behind
        // by an earlier call; errors carry their full cause chain.
        compiler.compile_isolated(&source)
            .map_err(|e| PyErr::new::<CompileError, _>(format!("{:#}", e)))
    }

    /// Compile SSQL source string to SQL safely
    /// Returns CompilationResult object instead of raising exception
    pub fn compile_safe(&self, source: String) -> PyResult<CompilationResult> {
        let mut compiler = match self.inner.lock() {
            Ok(c) => c,
            Err(e) => return Ok(CompilationResult {
                success: false,
                sql: None,
                error: Some(format!("Compiler lock poisoned: {}", e)),
            }),
        };

        match compiler.compile_isolated(&source) {
            Ok(sql) => Ok(CompilationResult {
                success: true,
                sql: Some(sql),
                error: None,
            }),
            Err(e) => Ok(CompilationResult {
                success: false,
                sql: None,
                error: Some(format!("{:#}", e)),
            }),
        }
    }
}

#[pymodule]
fn ssql(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Compiler>()?;
    m.add_class::<CompilationResult>()?;
    m.add("CompileError", m.py().get_type::<CompileError>())?;
    Ok(())
}

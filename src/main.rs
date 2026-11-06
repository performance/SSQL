//! SSQL Compiler CLI
//!
//! Usage: ssql_compiler <input.सं> [--output <output.sql>] [--dialect <sqlite|postgres>]

use anyhow::{Context, Result};
use clap::Parser;
use ssql_compiler::SQLDialect;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Input SSQL file (.सं)
    #[arg(value_name = "INPUT")]
    input: Option<PathBuf>,

    /// Output SQL file (default: same name with .sql extension)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// SQL dialect (sqlite or postgres)
    #[arg(short, long, default_value = "sqlite")]
    dialect: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dump the Abstract Syntax Tree (AST) to stdout
    #[arg(long)]
    dump_ast: bool,

    /// Dump the Resolved AST to stdout
    #[arg(long)]
    dump_resolved: bool,

    /// Dump the Symbol Table analysis to stdout
    #[arg(long)]
    dump_symtable: bool,

    /// Reverse-transpile a standard SQL file (.sql) into SSQL (.सं)
    #[arg(long, value_name = "SQL_FILE")]
    from_sql: Option<PathBuf>,

    /// Optional SSQL schema file (schema.सं) to resolve table/column stems and genders during --from-sql
    #[arg(long, value_name = "SCHEMA_FILE")]
    schema: Option<PathBuf>,

    /// Format the input SSQL file (.सं) in-place using canonical SSQL formatting
    #[arg(long)]
    fmt: bool,

    /// Run SSQL style and linguistic purity linter on the input file (.सं)
    #[arg(long)]
    lint: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Compile SSQL file
    Compile {
        /// Input SSQL file
        input: PathBuf,
    },
    /// Print every case form of a stem, as the morphology engine declines
    /// it (`decline नामन् नपुंसकलिङ्गम् एकवचनम्`): the forms a declaration of
    /// that stem, gender and number accepts.
    Decline {
        stem: String,
        gender: String,
        number: String,
    },
    /// Print every fixed SSQL word (keywords, spelled words, built-in
    /// function names) as a JSON array; the editors' word lists come from it.
    Vocabulary,
    /// Print the lexer's tokens of a file as JSON (`[{"text", "position"}]`,
    /// `position` the byte offset error messages report).
    Tokens {
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    
    let args = Args::parse();

    if let Some(sql_path) = &args.from_sql {
        let ssql_out = ssql_compiler::sql_importer::import_sql_file(
            sql_path,
            args.schema.as_deref(),
        )?;
        if let Some(out_path) = &args.output {
            fs::write(out_path, &ssql_out)
                .context(format!("Failed to write SSQL output to {}", out_path.display()))?;
            println!("✅ SSQL written to: {}", out_path.display());
        } else {
            println!("{}", ssql_out);
        }
        return Ok(());
    }

    if args.fmt {
        let input = args
            .input
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--fmt requires an input .सं file"))?;
        ssql_compiler::formatter::format_ssql_file(input, true)?;
        println!("✅ Formatted: {}", input.display());
        return Ok(());
    }

    if args.lint {
        let input = args
            .input
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--lint requires an input .सं file"))?;
        let diags = ssql_compiler::linter::lint_ssql_file(input)?;
        if diags.is_empty() {
            println!("✅ No style or purity lint issues in {}", input.display());
            return Ok(());
        } else {
            for d in &diags {
                eprintln!("⚠️  {}", d);
            }
            std::process::exit(1);
        }
    }

    // Handle Subcommands
    if let Some(cmd) = &args.command {
        match cmd {
            Commands::Compile { input } => {
                 return run_compiler(input.clone(), &args);
            }
            Commands::Decline { stem, gender, number } => {
                use ssql_compiler::types::{Linga, Vachana, Vibhakti};
                let gender = match gender.as_str() {
                    "पुंलिङ्गम्" => Linga::Masculine,
                    "स्त्रीलिङ्गम्" => Linga::Feminine,
                    "नपुंसकलिङ्गम्" => Linga::Neuter,
                    other => anyhow::bail!("unknown gender `{}` (पुंलिङ्गम् / स्त्रीलिङ्गम् / नपुंसकलिङ्गम्)", other),
                };
                let number = match number.as_str() {
                    "एकवचनम्" => Vachana::Eka,
                    "द्विवचनम्" => Vachana::Dvi,
                    "बहुवचनम्" => Vachana::Bahu,
                    other => anyhow::bail!("unknown number `{}` (एकवचनम् / द्विवचनम् / बहुवचनम्)", other),
                };
                let table = ssql_compiler::morphology::MorphologyEngine::new()?.decline_with_gender(stem, gender, number)?;
                let order = [
                    (Vibhakti::Prathama, "prathama"), (Vibhakti::Dvitiya, "dvitiya"), (Vibhakti::Tritiya, "tritiya"),
                    (Vibhakti::Chaturthi, "chaturthi"), (Vibhakti::Panchami, "panchami"), (Vibhakti::Shashthi, "shashthi"),
                    (Vibhakti::Saptami, "saptami"),
                ];
                let forms: Vec<String> = order.iter()
                    .filter_map(|(v, name)| table.forms.get(v).map(|f| format!("\"{}\": \"{}\"", name, f)))
                    .collect();
                println!("{{{}}}", forms.join(", "));
                return Ok(());
            }
            Commands::Vocabulary => {
                println!("{}", serde_json::Value::from(ssql_compiler::vocabulary()));
                return Ok(());
            }
            Commands::Tokens { input } => {
                let source = std::fs::read_to_string(input)?;
                let tokens: Vec<serde_json::Value> = ssql_compiler::lexer::tokenize(&source)?
                    .into_iter()
                    .map(|t| serde_json::json!({"text": t.text, "position": t.position}))
                    .collect();
                println!("{}", serde_json::Value::Array(tokens));
                return Ok(());
            }
        }
    }

    // Handle Top-level Input (Legacy/Default usage)
    if let Some(input) = &args.input {
        return run_compiler(input.clone(), &args);
    } else {
         // No input provided, print help
         use clap::CommandFactory;
         Args::command().print_help()?;
         return Ok(());
    }
}

fn run_compiler(input: PathBuf, args: &Args) -> Result<()> {
    if args.verbose {
        println!("📖 Reading SSQL file: {}", input.display());
    }

    // Determine output file
    let output_path = args.output.clone().unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("sql");
        path
    });

    // Parse dialect
    let dialect = match args.dialect.to_lowercase().as_str() {
        "sqlite" => SQLDialect::SQLite,
        "postgres" | "postgresql" => SQLDialect::PostgreSQL,
        _ => {
            eprintln!("❌ Invalid dialect: {}. Use 'sqlite' or 'postgres'", args.dialect);
            std::process::exit(1);
        }
    };

    if args.verbose {
        println!("🎯 Target dialect: {:?}", dialect);
    }

    use ssql_compiler::compiler::CompilerConfig;
    
    let config = CompilerConfig {
        dialect: args.dialect.clone(),
        verbose: args.verbose,
        dump_ast: args.dump_ast,
        dump_resolved: args.dump_resolved,
        dump_symtable: args.dump_symtable,
    };
    
    match ssql_compiler::compile_ssql_with_dumps(&input, config) {
        Ok((sql, dumps)) => {
            for dump in &dumps {
                println!("{}", dump);
            }
             if args.verbose {
                println!("✅ Compilation successful!");
                println!("\n{}", "=".repeat(60));
                println!("Generated SQL:");
                println!("{}", "=".repeat(60));
                println!("{}", sql);
                println!("{}", "=".repeat(60));
            }

            // Write output
            fs::write(&output_path, &sql)
                .context(format!("Failed to write output to {}", output_path.display()))?;

            println!("✅ SQL written to: {}", output_path.display());
            Ok(())
        },
        Err(e) => {
            let message = format!("{:#}", e);
            let message = if message.contains("Found schema file") {
                message
            } else {
                fs::read_to_string(&input)
                    .map(|source| ssql_compiler::compiler::offsets_to_line_col(&message, &source))
                    .unwrap_or(message)
            };
            eprintln!("❌ Compilation failed:");
            eprintln!("{}", message);
            std::process::exit(1);
        }
    }
}

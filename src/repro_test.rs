
use crate::ssql_parser::{Token, parse};
use crate::types::Vachana;

    #[test]
    fn test_repro_01() {
        // Tokems from debug output
        // [Panchami, Prathama, Number, Ityasmat, Adhikam, Asti, Iti, Niyamena, Prathama, Cha, Prathama, Cha, FetchVerb, Purnadanda]
        let tokens = vec![
            Token::Panchami { text: "पुस्तकेभ्यः".into(), vachana: Vachana::Bahu, pratipadika: "पुस्तक".into() },
            Token::Prathama { text: "प्रकाशनवर्षम्".into(), vachana: Vachana::Eka, pratipadika: "प्रकाशनवर्ष".into() },
            Token::Number("२०१०".into()),
            Token::Ityasmat("इत्यस्मात्".to_string()),
            Token::Adhikam("अधिकम्".to_string()),
            Token::Asti,
            Token::Iti,
            Token::Niyamena,
            Token::Prathama { text: "शीर्षकम्".into(), vachana: Vachana::Eka, pratipadika: "शीर्षक".into() },
            Token::Cha,
            Token::Prathama { text: "प्रकाशनवर्षम्".into(), vachana: Vachana::Eka, pratipadika: "प्रकाशनवर्ष".into() },
            Token::Cha,
            Token::FetchVerb("आह्रियताम्".to_string()),
            Token::Purnadanda,
        ];
        
        let result = parse(tokens);
        if let Err(e) = &result {
             eprintln!("Repro parse failed: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_repro_41_cte() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        
        let ssql = r#"
        विक्रयणवस्तुभ्यः
            विक्रयणैः सह
                विक्रयणवस्तुनः विक्रयणसङ्ख्या विक्रयणस्य सङ्ख्या इत्यनया समा अस्ति
                इति मेलनेन
            परिमाणस्य योगः
                यत्र विक्रयणस्य विक्रयणदिनाङ्कस्य अनुसारम् आरोहक्रमेण
            चलद्योगः इति आख्या
            आह्रियताम् ।
        "#;

        let lexer_tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched_tokens = annotator.annotate(lexer_tokens, &load_university_bookstore_schema()).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        
        println!("DEBUG Tokens length: {}", parser_tokens.len());
        for (i, tok) in parser_tokens.iter().enumerate() {
            println!("  Token {}: {:?}", i, tok);
        }
        
        let result = parse(parser_tokens);
        match result {
            Ok(ast) => println!("Success AST: {:?}", ast),
            Err(errors) => {
                for (i, err) in errors.iter().enumerate() {
                    println!("Error {}: {:?}", i, err);
                }
                panic!("Parse failed");
            }
        }
    }

    #[test]
    fn test_retired_kriya_and_sankulam() {
        // क्रिया bound a word to a SQL name; every built-in is called by its
        // own name, so the binding is retired (expert review J1). सङ्कुलम्
        // (IMPORT) was parsed and thrown away; SSQL has no modules.
        use crate::lexer::tokenize;

        let binding = tokenize(r#"वर्गमूलम् इति क्रिया "SQRT" इति आख्या रच्यताम् ।"#);
        let err = binding.expect_err("a क्रिया binding must be rejected").to_string();
        assert!(err.contains("a function is not bound to a SQL name"), "{}", err);
        let import = tokenize("गणितम् इति सङ्कुलम् आश्रित्य । पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।");
        let err = import.expect_err("an import must be rejected").to_string();
        assert!(err.contains("SSQL has no modules"), "{}", err);
    }

    #[test]
    fn test_repro_field_access_expression() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;

        let ssql = "विक्रयणस्य विक्रयणदिनाङ्कः";
        let lexer_tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched_tokens = annotator.annotate(lexer_tokens, &crate::schema::Schema::default()).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        use chumsky::Parser;
        let result = crate::ssql_parser::parse_expression()
            .then_ignore(chumsky::prelude::end())
            .parse(&parser_tokens[..])
            .into_result();
        match result {
            Ok(ast) => println!("Success AST: {:?}", ast),
            Err(errors) => {
                for err in errors {
                    println!("Error: {:?}", err);
                }
                panic!("Parse failed");
            }
        }
    }

    #[test]
    fn test_repro_window_spec() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        
        
        let schema = load_university_bookstore_schema();
        
        let ssql = r#"
        विक्रयाधिकारिभ्यः
            श्रेणी
                यत्र आपणसङ्ख्यायाः आधारेण विभाजनम्
                नाम्नः अनुसारम् अवरोहक्रमेण
        च सर्वम् च आह्रियेताम्।
        "#;
        
        let lexer_tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched_tokens = annotator.annotate(lexer_tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        
        println!("Tokens for test_window_function:");
        for (i, tok) in parser_tokens.iter().enumerate() {
            println!("  Token {}: {:?}", i, tok);
        }
        
        let ast = parse(parser_tokens).unwrap();
        println!("AST: {:?}", ast);
        
        let _config = crate::compiler::CompilerConfig {
            dialect: "sqlite".to_string(),
            verbose: true,
            dump_ast: false,
            dump_resolved: false,
            dump_symtable: false,
        };
        
        let adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
        let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &schema, vec![]);
        
        // Find statement
        match &ast {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                if let crate::ast::Statement::Dql(dql) = &statements[0] {
                    let sql = codegen.generate_dql_raw(dql).unwrap();
                    println!("Generated SQL: {}", sql);
                }
            }
            _ => panic!("Expected QueryBatch"),
        }
    }

    #[test]
    fn test_repro_window_column() {
        // Load schema from native DDL
        let schema = load_university_bookstore_schema();
        
        let col_res_text = schema.resolve_column("आपणसङ्ख्या", Some("विक्रयाधिकारिन्"));
        println!("Resolve by text: {:?}", col_res_text);
        
        let col_res_stem = schema.resolve_column("आपणसङ्ख्या", Some("विक्रयाधिकारिन्"));
        println!("Resolve by stem: {:?}", col_res_stem);
        
        // Let's also check the fields of विक्रयाधिकारिन्
        if let Some(table) = schema.tables.get("विक्रयाधिकारिन्") {
            println!("Table keys:");
            for k in table.fields.keys() {
                println!("  k: {:?} (bytes: {:?})", k, k.as_bytes());
            }
        } else {
            println!("Table विक्रयाधिकारिन् not found!");
            for k in schema.tables.keys() {
                println!("  Table k: {:?}", k);
            }
        }
        
        let input_text = "आपणसङ्ख्या";
        println!("input_text bytes: {:?}", input_text.as_bytes());
    }

    #[test]
    fn test_repro_window_frame() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        
        
        let schema = load_university_bookstore_schema();

        let test_cases = vec![
            (
                r#"
                पुस्तकेभ्यः
                    मूल्यस्य माध्यम्
                        यत्र मूल्यस्य अनुसारम् आरोहक्रमेण
                        पङ्क्तयः ३ पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम्
                आह्रियताम्।
                "#,
                "ROWS BETWEEN 3 PRECEDING AND CURRENT ROW"
            ),
            (
                r#"
                पुस्तकेभ्यः
                    मूल्यस्य माध्यम्
                        यत्र मूल्यस्य अनुसारम् आरोहक्रमेण
                        पङ्क्तयः ३ पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम् वर्जयित्वा वर्तमानपङ्क्तिम्
                आह्रियताम्।
                "#,
                "ROWS BETWEEN 3 PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW"
            ),
            (
                r#"
                पुस्तकेभ्यः
                    मूल्यस्य माध्यम्
                        यत्र मूल्यस्य अनुसारम् आरोहक्रमेण
                        व्याप्तिः अपरिमितपूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम्
                आह्रियताम्।
                "#,
                "RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"
            ),
        ];

        for (ssql, expected_sql_snippet) in test_cases {
            let lexer_tokens = tokenize(ssql).unwrap();
            let mut annotator = crate::annotator::Annotator::new().unwrap();
            let enriched_tokens = annotator.annotate(lexer_tokens, &schema).unwrap();
            let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
            
            let ast = parse(parser_tokens).unwrap();
            let adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
            let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &schema, vec![]);
            
            match &ast {
                crate::ast::AstNode::QueryBatch { statements, .. } => {
                    if let crate::ast::Statement::Dql(dql) = &statements[0] {
                        let sql = codegen.generate_dql_raw(dql).unwrap();
                        println!("Generated SQL: {}", sql);
                        assert!(sql.contains(expected_sql_snippet), "Expected SQL to contain '{}', but got '{}'", expected_sql_snippet, sql);
                    }
                }
                _ => panic!("Expected QueryBatch"),
            }
        }
    }

    #[test]
    fn test_repro_insert_select() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        
        
        let schema = load_university_bookstore_schema();

        // One form each (expert review F3, Q7): `<col> <value>` pairs, and
        // INSERT … SELECT as `<table> [<cols>] यत् <query> इति <verb>`.
        let test_cases = vec![
            (
                "प्रकाशकेषु नाम यत् लेखकेभ्यः नाम आह्रियताम् इति निवेश्यताम्।",
                "INSERT INTO publishers (name) SELECT t1.name FROM authors t1;"
            ),
            (
                "प्रकाशकेषु यत् लेखकेभ्यः नाम आह्रियताम् इति निवेश्यताम्।",
                "INSERT INTO publishers SELECT t1.name FROM authors t1;"
            ),
            (
                "प्रकाशकेषु नाम च सङ्ख्या च यत् लेखकेभ्यः नाम च सङ्ख्या च आह्रियेताम् इति निवेश्येताम्।",
                "INSERT INTO publishers (name, id) SELECT t1.name, t1.id FROM authors t1;"
            ),
            (
                "प्रकाशकेषु नाम \"गीता\" च सङ्ख्या १० च निवेश्येताम्।",
                "INSERT INTO publishers (name, id) VALUES ('गीता', 10);"
            ),
        ];

        // The former alternatives are no longer accepted.
        for retired in [
            "प्रकाशकेषु लेखकेभ्यः नाम आह्रियताम् निवेश्यताम्।",
            "प्रकाशकेषु नाम्नि \"गीता\" च सङ्ख्यायाम् १० च निवेश्येताम्।",
            "प्रकाशकेषु \"गीता\" नाम्नि च १० सङ्ख्यायाम् च निवेश्येताम्।",
            "प्रकाशकेषु \"गीता\" नाम इति च १० सङ्ख्या इति च निवेश्येताम्।",
        ] {
            let mut annotator = crate::annotator::Annotator::new().unwrap();
            let enriched = annotator.annotate(tokenize(retired).unwrap(), &schema).unwrap();
            assert!(parse(convert_vibhakti_tokens(enriched)).is_err(), "should be rejected: {}", retired);
        }

        for (ssql, expected_sql) in test_cases {
            let lexer_tokens = tokenize(ssql).unwrap();
            let mut annotator = crate::annotator::Annotator::new().unwrap();
            let enriched_tokens = annotator.annotate(lexer_tokens, &schema).unwrap();
            let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
            
            let ast = parse(parser_tokens).unwrap();
            let adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
            let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &schema, vec![]);
            
            match &ast {
                crate::ast::AstNode::QueryBatch { statements, .. } => {
                    if let crate::ast::Statement::Dml(dml) = &statements[0] {
                        let sql = codegen.generate_dml(dml).unwrap();
                        println!("Generated SQL: {}", sql);
                        
                        let norm_sql = sql.replace("\n", " ").replace("\r", " ").replace("\t", " ")
                            .split_whitespace().collect::<Vec<&str>>().join(" ");
                        let norm_expected = expected_sql.replace("\n", " ").replace("\r", " ").replace("\t", " ")
                            .split_whitespace().collect::<Vec<&str>>().join(" ");
                        
                        assert_eq!(norm_sql, norm_expected);
                    } else {
                        panic!("Expected DML statement");
                    }
                }
                _ => panic!("Expected QueryBatch"),
            }
        }
    }

    #[test]
    fn test_repro_create_view() {
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;
        
        
        let schema = load_university_bookstore_schema();
        let test_cases = vec![
            // Case 1: Parenthesized view
            (
                r#"
                अधिकविक्रीतपुस्तकानि इति दृश्यम् "top_selling_books" इति आख्या अधिकविक्रीतपुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्
                    पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम्
                इति दृश्यम्।
                "#,
                "CREATE VIEW top_selling_books AS SELECT t1.title, t1.price FROM books t1;"
            ),
            // Case 2: Unparenthesized view
            (
                r#"
                अधिकविक्रीतपुस्तकानि इति दृश्यम् "top_selling_books" इति आख्या अधिकविक्रीतपुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्
                    पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियताम्
                इति दृश्यम्।
                "#,
                "CREATE VIEW top_selling_books AS SELECT t1.title, t1.price FROM books t1;"
            ),
        ];

        for (ssql, expected_sql) in test_cases {
            let lexer_tokens = tokenize(ssql).unwrap();
            let mut annotator = crate::annotator::Annotator::new().unwrap();
            let enriched_tokens = annotator.annotate(lexer_tokens, &schema).unwrap();
            let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
            
            let ast = parse(parser_tokens).unwrap();
            let adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
            let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &schema, vec![]);
            
            match &ast {
                crate::ast::AstNode::QueryBatch { statements, .. } => {
                    if let crate::ast::Statement::Ddl(ddl) = &statements[0] {
                        let sql = codegen.generate_ddl(ddl).unwrap();
                        println!("Generated SQL: {}", sql);
                        
                        let norm_sql = sql.replace("\n", " ").replace("\r", " ").replace("\t", " ")
                            .split_whitespace().collect::<Vec<&str>>().join(" ");
                        let norm_expected = expected_sql.replace("\n", " ").replace("\r", " ").replace("\t", " ")
                            .split_whitespace().collect::<Vec<&str>>().join(" ");
                        
                        assert_eq!(norm_sql, norm_expected);
                    } else {
                        panic!("Expected DDL statement");
                    }
                }
                _ => panic!("Expected QueryBatch"),
            }
        }

        // Case 3: Semantic warning check on view query body
        let ssql_warning = r#"
        अधिकविक्रीतपुस्तकानि इति दृश्यम् "top_selling_books" इति आख्या अधिकविक्रीतपुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्
            पुस्तकात् शीर्षकम् च मूल्यम् च आह्रियताम्
        इति दृश्यम्।
        "#;
        let lexer_tokens = tokenize(ssql_warning).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched_tokens = annotator.annotate(lexer_tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        let ast = parse(parser_tokens).unwrap();

        let mut validator = crate::validator::Validator::new(&schema);
        let _ = validator.validate(&ast);
        
        println!("Warnings: {:?}", validator.warnings);
        assert!(!validator.warnings.is_empty());
        assert!(validator.warnings[0].contains("FROM source table must be in the Ablative Plural form"));
    }

    #[test]
    fn test_repro_recursive_cte() {
        use crate::compiler::{Compiler, CompilerConfig};
        
        let ssql_sqlite = r#"
        प्रवाहः इति आख्या प्रवाह इति पुंलिङ्गम् एकवचनम् आवर्तिका सञ्ज्ञा
            विक्रयाधिकारिभ्यः प्रबन्धकसङ्ख्या रिक्ता अस्ति इति नियमेन सङ्ख्या च नाम च आह्रियेताम् च
            विक्रयाधिकारिभ्यः प्रवाहेण सह विक्रयाधिकारिणः प्रबन्धकसङ्ख्या प्रवाहस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
                विक्रयाधिकारिणः सङ्ख्या च विक्रयाधिकारिणः नाम च आह्रियेताम् च
            इत्यनयोः सङ्गमः सङ्ख्या चक्रनिवारणेन ।

        प्रवाहात्
        सङ्ख्या च नाम च
        आह्रियेताम् ।
        "#;
        
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let scratch = BookstoreScratch::new("temp_recursive", id);
        let temp_path = scratch.file("query.sam");
        std::fs::write(&temp_path, ssql_sqlite).unwrap();
        
        // SQLite
        let mut compiler_sqlite = Compiler::new(CompilerConfig {
            dialect: "sqlite".to_string(),
            ..Default::default()
        }).unwrap();
        let sql_sqlite = compiler_sqlite.compile(&temp_path).unwrap();
        println!("Compiled SQLite:\n{}", sql_sqlite);
        assert!(sql_sqlite.contains("WITH RECURSIVE"));
        assert!(sql_sqlite.contains("UNION ALL"));
        assert!(sql_sqlite.contains("CHAR(31)"));
        assert!(sql_sqlite.contains("instr(t3.path, CHAR(31) ||"));
        
        // Postgres
        let mut compiler_postgres = Compiler::new(CompilerConfig {
            dialect: "postgres".to_string(),
            ..Default::default()
        }).unwrap();
        let sql_postgres = compiler_postgres.compile(&temp_path).unwrap();
        println!("Compiled PostgreSQL:\n{}", sql_postgres);
        assert!(sql_postgres.contains("WITH RECURSIVE"));
        assert!(sql_postgres.contains("UNION ALL"));
        // The same path guard as SQLite: the CYCLE clause kept the row that
        // closes the cycle (final review, PostgreSQL).
        assert!(sql_postgres.contains("chr(31)"));
        assert!(sql_postgres.contains("strpos(t3.path, chr(31) ||"));
        assert!(!sql_postgres.contains("CYCLE"));

    }

    #[test]
    fn test_repro_tcl() {
        use crate::compiler::{Compiler, CompilerConfig};
        
        let ssql = r#"
        व्यवहारः प्रारभ्यताम् ।
        "sp1" इति सङ्ग्रहबिन्दुः स्थाप्यताम् ।
        "prathama" इति सङ्ग्रहबिन्दुः स्थाप्यताम् ।
        "prathama" इति सङ्ग्रहबिन्दुम् प्रति व्यवहारः प्रत्याह्रियताम् ।
        व्यवहारः प्रत्याह्रियताम् ।
        व्यवहारः स्थिरीक्रियताम् ।
        "#;
        
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let scratch = BookstoreScratch::new("temp_tcl", id);
        let temp_path = scratch.file("query.sam");
        std::fs::write(&temp_path, ssql).unwrap();
        
        let mut compiler = Compiler::new(CompilerConfig {
            dialect: "sqlite".to_string(),
            ..Default::default()
        }).unwrap();
        let sql = compiler.compile(&temp_path).unwrap();
        println!("Compiled SQL:\n{}", sql);
        
        assert!(sql.contains("BEGIN;"));
        assert!(sql.contains("SAVEPOINT sp1;"));
        assert!(sql.contains("SAVEPOINT prathama;"));
        assert!(sql.contains("ROLLBACK TO SAVEPOINT prathama;"));
        assert!(sql.contains("ROLLBACK;"));
        assert!(sql.contains("COMMIT;"));
        
    }

    #[test]
    fn test_repro_ddl_and_setop_dialect_codegen() {
        
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;
        
        let schema = load_university_bookstore_schema();

        // 1. Test CREATE TABLE with Autoincrement
        // "पुस्तकानि इति पटलम् ... रच्यताम्"
        // Let's create an AST directly or parse SSQL. 
        // In SSQL:
        let ssql_create = r#"
        पुस्तकानि इति पटलम् "books_temp" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम्
            प्राथमिकः क्रमिकः क्रमाङ्कः इति पूर्णाङ्कः इति आख्या क्रमाङ्क इति पुंलिङ्गम् एकवचनम् च
            शीर्षकम् इति वाक्यम् इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च
        रच्यताम्।
        "#;

        let tokens = tokenize(ssql_create).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();

        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else {
            panic!("Expected QueryBatch");
        };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else {
            panic!("Expected DDL statement");
        };

        // SQLite codegen
        let sqlite_adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
        let mut sqlite_codegen = crate::ast_codegen::AstCodeGen::new(&sqlite_adaptor, &schema, vec![]);
        let sql_sqlite = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("SQLite DDL: {}", sql_sqlite);
        assert!(sql_sqlite.contains("CREATE TABLE books_temp"));
        assert!(sql_sqlite.contains("क्रमाङ्क INTEGER PRIMARY KEY AUTOINCREMENT"));

        // PostgreSQL codegen
        let pg_adaptor = crate::codegen::postgres::PostgresAdaptor::new();
        let mut pg_codegen = crate::ast_codegen::AstCodeGen::new(&pg_adaptor, &schema, vec![]);
        let sql_pg = pg_codegen.generate_ddl(ddl).unwrap();
        println!("Postgres DDL: {}", sql_pg);
        assert!(sql_pg.contains("CREATE TABLE books_temp"));
        assert!(sql_pg.contains("क्रमाङ्क BIGSERIAL PRIMARY KEY"));
        assert!(!sql_pg.contains("AUTOINCREMENT"));
    }

    #[test]
    fn test_repro_advanced_ddl_codegen() {
        
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;
        
        let schema = load_university_bookstore_schema();
        let mut annotator = crate::annotator::Annotator::new().unwrap();

        // A. Index creation
        // Single Column
        let ssql_idx1 = r#"पुस्तकानाम् शीर्षकस्य आधारे "idx_books_title" इति सूची रच्यताम्।"#;
        let tokens = tokenize(ssql_idx1).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };
        
        let sqlite_adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
        let mut sqlite_codegen = crate::ast_codegen::AstCodeGen::new(&sqlite_adaptor, &schema, vec![]);
        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Index 1 SQLite: {}", sql);
        assert_eq!(sql.trim(), "CREATE INDEX idx_books_title ON books (title);");

        let pg_adaptor = crate::codegen::postgres::PostgresAdaptor::new();
        let mut pg_codegen = crate::ast_codegen::AstCodeGen::new(&pg_adaptor, &schema, vec![]);
        let sql = pg_codegen.generate_ddl(ddl).unwrap();
        println!("Index 1 Postgres: {}", sql);
        assert_eq!(sql.trim(), "CREATE INDEX idx_books_title ON books (title);");

        // Multi Column Unique
        let ssql_idx2 = r#"पुस्तकानाम् शीर्षकम् आरोहः च मूल्यम् अवरोहः च इत्यनयोः आधारे "idx_multi" इति अद्वितीया सूची रच्यताम्।"#;
        let tokens = tokenize(ssql_idx2).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };
        
        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Index 2 SQLite: {}", sql);
        assert_eq!(sql.trim(), "CREATE UNIQUE INDEX idx_multi ON books (title ASC, price DESC);");

        // B. Drop Index
        let ssql_drop_idx = r#"विद्यते चेत् "idx_books_title" इति सूची लुप्यताम्।"#;
        let tokens = tokenize(ssql_drop_idx).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Drop Index SQLite: {}", sql);
        assert_eq!(sql.trim(), "DROP INDEX IF EXISTS idx_books_title;");

        // C. Alter Table ADD COLUMN
        let ssql_alter_add = r#"पुस्तकानि इति पटले "rating" इति पूर्णाङ्कः इति आख्या "rating" इति नपुंसकलिङ्गम् एकवचनम् योज्यताम्।"#;
        let tokens = tokenize(ssql_alter_add).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Alter Add SQLite: {}", sql);
        assert_eq!(sql.trim(), "ALTER TABLE books ADD COLUMN rating INTEGER;");

        // D. Alter Table DROP COLUMN
        let ssql_alter_drop = r#"पुस्तकानि इति पटले "rating" इति लुप्यताम्।"#;
        let tokens = tokenize(ssql_alter_drop).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Alter Drop SQLite: {}", sql);
        assert_eq!(sql.trim(), "ALTER TABLE books DROP COLUMN rating;");

        // E. Alter Table RENAME COLUMN
        // तस्मात्+अभिधीयताम् ("from that called X, let it be called Y")
        // deliberately replaces the old नाम-based marker: genitive+नाम was
        // token-for-token identical to an ordinary field-access noun
        // phrase used everywhere else in the grammar.
        let ssql_alter_rename_col = r#"पुस्तकानि इति पटले "rating" इति तस्मात् "book_rating" इति अभिधीयताम्।"#;
        let tokens = tokenize(ssql_alter_rename_col).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Alter Rename Col SQLite: {}", sql);
        assert_eq!(sql.trim(), "ALTER TABLE books RENAME COLUMN rating TO book_rating;");

        // F. Alter Table RENAME TABLE
        let ssql_alter_rename_table = r#"पुस्तकानि इति तस्मात् "all_books" इति अभिधीयताम्।"#;
        let tokens = tokenize(ssql_alter_rename_table).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Alter Rename Table SQLite: {}", sql);
        assert_eq!(sql.trim(), "ALTER TABLE books RENAME TO all_books;");
    }

    #[test]
    fn test_repro_rename_column_named_naama_is_unambiguous() {
        // Verify that नाम can be used as an ordinary column name/reference,
        // including as the column being renamed by RENAME COLUMN via तस्मात्/अभिधीयताम्.
        
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;

        let schema = load_university_bookstore_schema();
        let mut annotator = crate::annotator::Annotator::new().unwrap();

        let ssql = r#"लेखकाः इति पटले नाम इति तस्मात् "author_name" इति अभिधीयताम्।"#;
        let tokens = tokenize(ssql).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };

        let sqlite_adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
        let mut sqlite_codegen = crate::ast_codegen::AstCodeGen::new(&sqlite_adaptor, &schema, vec![]);
        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        println!("Rename नाम column SQLite: {}", sql);
        assert_eq!(sql.trim(), "ALTER TABLE authors RENAME COLUMN name TO author_name;");
    }

    // --- ALTER TABLE column-identity robustness: pure Samskrutam vs आख्या ---
    //
    // Every column can be referenced two ways: by its native Sanskrit name
    // (pratipadika/declined form) or, if one was assigned via column_def's
    // optional आख्या clause, by that SQL-level alias directly. These tests
    // exercise ADD/DROP/RENAME COLUMN under both forms.

    fn load_university_bookstore_schema() -> crate::schema::Schema {
        use crate::lexer::tokenize;
        use crate::ssql_parser::{convert_vibhakti_tokens, parse};
        use crate::ast::{AstNode, Statement};

        let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/university_bookstore/SAMSQL/schema.सं");
        let src = std::fs::read_to_string(schema_path).unwrap();
        let tokens = tokenize(&src).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &crate::schema::Schema::default()).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        drop(annotator);
        let ast = parse(parser_tokens).unwrap();
        let statements = match ast {
            AstNode::QueryBatch { statements, .. } => statements,
            AstNode::Statement(s) => vec![s],
        };
        let mut ddl_stmts = vec![];
        for stmt in statements {
            if let Statement::Ddl(d) = stmt {
                ddl_stmts.push(d);
            }
        }
        let mut schema = crate::schema::Schema::default();
        schema.update_from_ddl(&ddl_stmts, "sqlite").unwrap();
        schema
    }

    fn compile_ddl_with_warnings(ssql: &str, schema: &crate::schema::Schema) -> (String, Vec<String>) {
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;

        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let tokens = tokenize(ssql).unwrap();
        let enriched = annotator.annotate(tokens, schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();

        let mut validator = crate::validator::Validator::new(schema);
        validator.validate(&ast).unwrap();

        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch"); };
        let crate::ast::Statement::Ddl(ddl) = &statements[0] else { panic!("Expected DDL"); };
        let sqlite_adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
        let mut sqlite_codegen = crate::ast_codegen::AstCodeGen::new(&sqlite_adaptor, schema, vec![]);
        let sql = sqlite_codegen.generate_ddl(ddl).unwrap();
        (sql.trim().to_string(), validator.warnings.clone())
    }

    #[test]
    fn test_repro_add_column_pure_samskrutam() {
        // No sql_name string: the new column's own stem flows straight into
        // the generated SQL as its column name. इति आख्या + explicit
        // stem/gender/vachana is still mandatory: the stem is never
        // guessed from the declared name's surface form.
        let schema = load_university_bookstore_schema();
        let (sql, warnings) = compile_ddl_with_warnings(
            r#"पुस्तकानि इति पटले टिप्पणी इति वाक्यम् इति आख्या टिप्पणी इति स्त्रीलिङ्गम् एकवचनम् योज्यताम्।"#,
            &schema,
        );
        assert_eq!(sql, "ALTER TABLE books ADD COLUMN टिप्पणी TEXT;");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn test_repro_add_column_with_aakhya() {
        // Explicit आख्या clause assigns the SQL-level name at creation time.
        let schema = load_university_bookstore_schema();
        let (sql, warnings) = compile_ddl_with_warnings(
            r#"पुस्तकानि इति पटले टिप्पणी इति वाक्यम् "notes" इति आख्या टिप्पणी इति स्त्रीलिङ्गम् एकवचनम् योज्यताम्।"#,
            &schema,
        );
        assert_eq!(sql, "ALTER TABLE books ADD COLUMN notes TEXT;");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn test_repro_drop_column_pure_samskrutam() {
        // Drop the authors table's existing नाम column by its native
        // Sanskrit name - no English anywhere in the query text.
        let schema = load_university_bookstore_schema();
        let (sql, warnings) = compile_ddl_with_warnings(
            r#"लेखकाः इति पटले नाम इति लुप्यताम्।"#,
            &schema,
        );
        assert_eq!(sql, "ALTER TABLE authors DROP COLUMN name;");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn test_repro_drop_column_by_aakhya() {
        // Drop column by its declared SQL आख्या ("name").
        let schema = load_university_bookstore_schema();
        let (sql, warnings) = compile_ddl_with_warnings(
            r#"लेखकाः इति पटले "name" इति लुप्यताम्।"#,
            &schema,
        );
        assert_eq!(sql, "ALTER TABLE authors DROP COLUMN name;");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn test_repro_rename_column_by_aakhya() {
        // Rename column by its declared SQL आख्या ("name").
        let schema = load_university_bookstore_schema();
        let (sql, warnings) = compile_ddl_with_warnings(
            r#"लेखकाः इति पटले "name" इति तस्मात् "full_name" इति अभिधीयताम्।"#,
            &schema,
        );
        assert_eq!(sql, "ALTER TABLE authors RENAME COLUMN name TO full_name;");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    // --- JOIN types: LEFT (वाममेलनम्, pre-existing), RIGHT (दक्षिणमेलनम्),
    // FULL OUTER (पूर्णमेलनेन), and CROSS (सर्वमेलनेन) ---
    //
    // RIGHT/FULL/CROSS were declared in the AST (ast::JoinType) and even had
    // partial codegen support, but were never wired up in the lexer/parser -
    // docs/reference/keywords.md documented them as if they worked, but no
    // query could actually produce one. These tests compile the same join
    // condition (sales_reps LEFT/RIGHT/FULL/CROSS JOIN sales) through the
    // real setu_v4.toml schema for both dialects, so they catch not just
    // parser regressions but codegen/dialect-adaptor drift too.

    fn compile_dql(ssql: &str, dialect: &str) -> String {
        let config = crate::compiler::CompilerConfig {
            dialect: dialect.to_string(),
            ..Default::default()
        };
        let mut compiler = crate::compiler::Compiler::new(config).unwrap();
        compiler.schema = load_university_bookstore_schema();
        compiler.compile_string(ssql).unwrap()
    }

    const JOIN_SOURCE_QUERY_TEMPLATE: &str = r#"विक्रयाधिकारिभ्यः
        विक्रयणैः सह
            विक्रयाधिकारिणः सङ्ख्या विक्रयणस्य विक्रयाधिकारिसङ्ख्या इत्यनया समा अस्ति
            इति {JOIN_MARKER}
        विक्रयाधिकारिणः नाम
        आह्रियताम्।"#;

    fn join_query(marker: &str) -> String {
        JOIN_SOURCE_QUERY_TEMPLATE.replace("{JOIN_MARKER}", marker)
    }

    #[test]
    fn test_repro_left_join_both_dialects() {
        // Sanity baseline: वाममेलनम् predates the RIGHT/FULL/CROSS additions -
        // confirm it's still unaffected by them.
        let ssql = join_query("वाममेलनेन");
        let sqlite = compile_dql(&ssql, "sqlite");
        assert!(sqlite.contains("LEFT JOIN sales"), "sqlite: {}", sqlite);
        let postgres = compile_dql(&ssql, "postgres");
        assert!(postgres.contains("LEFT JOIN sales"), "postgres: {}", postgres);
    }

    #[test]
    fn test_repro_right_join_both_dialects() {
        let ssql = join_query("दक्षिणमेलनेन");
        let sqlite = compile_dql(&ssql, "sqlite");
        assert!(sqlite.contains("RIGHT JOIN sales"), "sqlite: {}", sqlite);
        let postgres = compile_dql(&ssql, "postgres");
        assert!(postgres.contains("RIGHT JOIN sales"), "postgres: {}", postgres);
    }

    #[test]
    fn test_repro_full_outer_join_both_dialects() {
        let ssql = join_query("पूर्णमेलनेन");
        let sqlite = compile_dql(&ssql, "sqlite");
        assert!(sqlite.contains("FULL OUTER JOIN sales"), "sqlite: {}", sqlite);
        let postgres = compile_dql(&ssql, "postgres");
        assert!(postgres.contains("FULL OUTER JOIN sales"), "postgres: {}", postgres);
    }

    #[test]
    fn test_repro_cross_join_both_dialects_no_on_clause() {
        // CROSS JOIN is an unconditional Cartesian product: there is no
        // condition to quote, so it is `<table> सह सर्वमेलनेन`.
        let ssql = "विक्रयाधिकारिभ्यः विक्रयणैः सह सर्वमेलनेन विक्रयाधिकारिणः नाम आह्रियताम्।".to_string();
        let sqlite = compile_dql(&ssql, "sqlite");
        assert!(sqlite.contains("CROSS JOIN sales"), "sqlite: {}", sqlite);
        assert!(!sqlite.contains("CROSS JOIN sales ON"), "sqlite should have no ON clause: {}", sqlite);
        let postgres = compile_dql(&ssql, "postgres");
        assert!(postgres.contains("CROSS JOIN sales"), "postgres: {}", postgres);
        assert!(!postgres.contains("CROSS JOIN sales ON"), "postgres should have no ON clause: {}", postgres);
    }

    #[test]
    fn test_repro_cross_join_rejects_a_condition() {
        // A condition on a CROSS JOIN would be silently ignored, so it is
        // not accepted; and an inner join always states its condition.
        for ssql in [
            "विक्रयाधिकारिभ्यः विक्रयणैः सह विक्रयाधिकारिणः सङ्ख्या विक्रयणस्य विक्रयाधिकारिसङ्ख्या इत्यनया समा अस्ति इति सर्वमेलनेन विक्रयाधिकारिणः नाम आह्रियताम्।",
            "विक्रयाधिकारिभ्यः विक्रयणैः सह इति मेलनेन विक्रयाधिकारिणः नाम आह्रियताम्।",
        ] {
            let mut compiler = crate::compiler::Compiler::new(crate::compiler::CompilerConfig { dialect: "sqlite".to_string(), ..Default::default() }).unwrap();
            compiler.schema = load_university_bookstore_schema();
            assert!(compiler.compile_string(ssql).is_err(), "should be rejected: {}", ssql);
        }
    }

    #[test]
    fn test_repro_subtraction() {
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::lexer::tokenize;
        use crate::ssql_parser::parse;
        use crate::schema::Schema;

        let schema_ssql = r#"
            वित्तपत्राणि इति पटलम् "finance_data" इति आख्या वित्तपत्र इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सकलविक्रयः इति वास्तविकः "gross_sales" इति आख्या सकलविक्रय इति पुंलिङ्गम् एकवचनम् च
                कर्तनम् इति वास्तविकः "deductions" इति आख्या कर्तन इति नपुंसकलिङ्गम् एकवचनम् च
            इति पटलम् ।
        "#;
        let query_ssql = r#"शुद्धायसूत्रम् इति आख्या शुद्धायसूत्र इति नपुंसकलिङ्गम् एकवचनम् सूत्रम् सकलविक्रयात् कर्तनम् विशोध्य अवशिष्टम् ।"#;
        
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let schema_tokens = tokenize(schema_ssql).unwrap();
        let schema_enriched = annotator.annotate(schema_tokens, &crate::schema::Schema::default()).unwrap();
        let schema_parser_tokens = convert_vibhakti_tokens(schema_enriched);
        let schema_ast = parse(schema_parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = schema_ast else { panic!("Expected QueryBatch"); };
        let mut ddl_statements = vec![];
        for stmt in statements {
            if let crate::ast::Statement::Ddl(ddl) = stmt {
                ddl_statements.push(ddl);
            }
        }
        let schema = Schema::from_ddl(&ddl_statements, "sqlite").unwrap();
        
        // 1. Verify 'विशोध्य'
        let query_tokens = tokenize(query_ssql).unwrap();
        let query_enriched = annotator.annotate(query_tokens, &schema).unwrap();
        let query_parser_tokens = convert_vibhakti_tokens(query_enriched);
        let _ast = parse(query_parser_tokens).unwrap();
        
        // 2-3. Subtraction has one form, विशोध्य (Līlāvatī-style शोधन): the
        // former synonyms शोधयित्वा and अपनीय are rejected with that hint.
        for old in ["शोधयित्वा", "अपनीय"] {
            let ssql = format!("शुद्धायसूत्रम् इति आख्या शुद्धायसूत्र इति नपुंसकलिङ्गम् एकवचनम् सूत्रम् सकलविक्रयात् कर्तनम् {} अवशिष्टम् ।", old);
            let err = tokenize(&ssql).unwrap_err().to_string();
            assert!(err.contains("Use `विशोध्य`"), "{}: {}", old, err);
        }

        // 4. Verify 'क्षणः' and 'क्षणम्' time units
        let ssql_kshana = r#"वित्तपत्रात् वित्तपत्रस्य समयस्य क्षणः च वित्तपत्रस्य समयस्य क्षणम् च आह्रियेताम् ।"#;
        let tokens = tokenize(ssql_kshana).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let _ = parse(parser_tokens).unwrap();
    }

    #[test]
    fn test_batch4_proposal_queries() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::schema::Schema;

        // DDL schemas for Projects 16-20
        let schema_ssql = r#"
            प्रभावकाः इति पटलम् "influencers" इति आख्या प्रभावक इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "influencer_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
                दिनाङ्कः इति दिनाङ्कः "post_date" इति आख्या पोस्टदिनाङ्क इति पुंलिङ्गम् एकवचनम् च
                प्रतिक्रिया इति पूर्णाङ्कः "likes" इति आख्या प्रतिक्रिया इति स्त्रीलिङ्गम् एकवचनम् च
            इति पटलम् ।

            सम्पर्काः इति पटलम् "clicks" इति आख्या सम्पर्क इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "click_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
                सङ्केतः इति वाक्यम् "session_id" इति आख्या सङ्केत इति पुंलिङ्गम् एकवचनम् च
                समयः इति पूर्णाङ्कः "click_time" इति आख्या क्लिकसमय इति पुंलिङ्गम् एकवचनम् च
                पुटम् इति वाक्यम् "page_name" इति आख्या पुट इति नपुंसकलिङ्गम् एकवचनम् च
            इति पटलम् ।

            प्रवेशाः इति पटलम् "admissions" इति आख्या प्रवेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "admission_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
                रोगी इति पूर्णाङ्कः "patient_id" इति आख्या रोगिन् इति पुंलिङ्गम् एकवचनम् च
                प्रवेशदिनम् इति पूर्णाङ्कः "admission_date" इति आख्या प्रवेशदिन इति नपुंसकलिङ्गम् एकवचनम् च
                निर्गमदिनम् इति पूर्णाङ्कः "discharge_date" इति आख्या निर्गमदिन इति नपुंसकलिङ्गम् एकवचनम् च
            इति पटलम् ।

            कर्मचारिणः इति पटलम् "employees" इति आख्या कर्मचारिन् इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "employee_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
                नाम इति वाक्यम् "employee_name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
                प्रबन्धकसङ्ख्या इति पूर्णाङ्कः "manager_id" इति आख्या प्रबन्धकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
            इति पटलम् ।

            दुर्घटनाः इति पटलम् "incidents" इति आख्या दुर्घटना इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "incident_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
                विवरणम् इति वाक्यम् "description" इति आख्या विवरण इति नपुंसकलिङ्गम् एकवचनम् च
                गम्भीरता इति वाक्यम् "severity" इति आख्या गम्भीरता इति स्त्रीलिङ्गम् एकवचनम् च
            इति पटलम् ।
        "#;

        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let schema_tokens = tokenize(schema_ssql).unwrap();
        let schema_enriched = annotator.annotate(schema_tokens, &crate::schema::Schema::default()).unwrap();
        let schema_parser_tokens = convert_vibhakti_tokens(schema_enriched);
        let schema_ast = parse(schema_parser_tokens).unwrap();
        let crate::ast::AstNode::QueryBatch { statements, .. } = schema_ast else { panic!("Expected QueryBatch"); };
        let mut ddl_statements = vec![];
        for stmt in statements {
            if let crate::ast::Statement::Ddl(ddl) = stmt {
                ddl_statements.push(ddl);
            }
        }
        let schema = Schema::from_ddl(&ddl_statements, "sqlite").unwrap();

        // Project 16 Elegant
        let q16 = r#"प्रभावकेभ्यः दिनाङ्कः च प्रतिक्रिया च प्रतिक्रियाणाम् माध्यम् यत्र दिनाङ्कस्य अनुसारम् आरोहक्रमेण पङ्क्तयः २ पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम् "moving_avg" इति आख्या च आह्रियन्ताम् ।"#;
        let tokens = tokenize(q16).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast16 = parse(parser_tokens).unwrap();
        println!("Q16 AST: {:?}", ast16);

        // Native subtraction with विशोध्य/अवशिष्टम् and a LAG subtrahend
        let q17 = r#"सम्पर्केभ्यः सङ्केतः च पुटम् च समयः च समयात् पूर्वपङ्क्तेः समयम् यत्र सङ्केतस्य आधारेण विभाजनम् समयस्य अनुसारम् आरोहक्रमेण विशोध्य अवशिष्टम् "time_diff" इति आख्या च आह्रियन्ताम् ।"#;
        let tokens = tokenize(q17).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast17 = parse(parser_tokens);
        println!("Q17 parse status: {:?}", ast17);
        assert!(ast17.is_ok());

        // Native addition with यत्...इति-grouped list-sum inside recursive CTE
        let q19 = r#"
        प्रवाहः इति आख्या प्रवाह इति पुंलिङ्गम् एकवचनम् आवर्तिका सञ्ज्ञा कर्मचारिभ्यः प्रबन्धकसङ्ख्या रिक्ता अस्ति इति नियमेन सङ्ख्या च नाम च १ स्तरः इति आख्या च आह्रियन्ताम् च कर्मचारिभ्यः प्रवाहेण सह कर्मचारिणः प्रबन्धकसङ्ख्या प्रवाहस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन कर्मचारिणः सङ्ख्या च कर्मचारिणः नाम च प्रवाहस्य स्तरः च १ च इत्यनयोः योगः स्तरः इति आख्या च आह्रियन्ताम् च इत्यनयोः सङ्गमः सङ्ख्या चक्रनिवारणेन ।
        प्रवाहात् सङ्ख्या च नाम च स्तरः च आह्रियन्ताम् ।
        "#;
        let tokens = tokenize(q19).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast19 = parse(parser_tokens);
        println!("Q19 parse status: {:?}", ast19);
        assert!(ast19.is_ok());

        // Project 20 Elegant
        let q20 = r#"दुर्घटनाभ्यः सङ्ख्या च विवरणम् च गम्भीरता च विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः "error_code" इति आख्या च आह्रियन्ताम् ।"#;
        let tokens = tokenize(q20).unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast20 = parse(parser_tokens);
        println!("Q20 parse status: {:?}", ast20);
        assert!(ast20.is_ok());
    }

    #[test]
    fn test_standalone_karaka_instr_and_numeric_substr_and_reject_postfix_func() {
        let ssql_instr = r#"दुर्घटनाभ्यः सङ्ख्या च विवरणे "ERR" इत्यस्य स्थानम् "err_pos" इति आख्या च आह्रियेताम् ।"#;
        let sqlite_instr = compile_safety_incident_dql(ssql_instr, "sqlite");
        assert!(
            sqlite_instr.contains("instr(t1.description, 'ERR') AS err_pos"),
            "unexpected sqlite INSTR SQL: {}",
            sqlite_instr
        );

        let ssql_num_substr = r#"दुर्घटनाभ्यः सङ्ख्या च विवरणे १ स्थानात् आरभ्य ३ वर्णानाम् खण्डः "prefix" इति आख्या च आह्रियेताम् ।"#;
        let sqlite_substr = compile_safety_incident_dql(ssql_num_substr, "sqlite");
        assert!(
            sqlite_substr.contains("substr(t1.description, 1, 3) AS prefix"),
            "unexpected sqlite SUBSTR SQL: {}",
            sqlite_substr
        );

        // Retired ungrammatical parenthesized postfix syntax must be rejected:
        let config = crate::compiler::CompilerConfig {
            dialect: "sqlite".to_string(),
            ..Default::default()
        };
        let mut compiler = crate::compiler::Compiler::new(config).unwrap();
        compiler.schema = load_safety_incident_logs_schema();
        let legacy = r#"दुर्घटनाभ्यः विवरणम् "ERR" स्थानम् "err_pos" इति आख्या आह्रियताम् ।"#;
        assert!(
            compiler.compile_string(legacy).is_err(),
            "Expected retired parenthesized postfix_func syntax to be rejected"
        );
    }

    // --- Locative-source chained SUBSTR/INSTR: "विवरणे \"ERR\" इत्यस्य
    // स्थानात् आरभ्य ६ वर्णानाम् खण्डः" ---
    //
    // Replaces the ungrammatical bare-nominative postfix form above
    // (q20/q1_elegant.सं/q1_parens.सं) with proper case-marked Sanskrit: a
    // locative source named once (विवरणे), a genitive-of-quoted-literal
    // search target ("ERR" इत्यस्य), the reserved ablative स्थानात् and
    // genitive-plural वर्णानाम् keywords (Token::Sthaanaat/Token::Varnaanaam
    // - deliberately dedicated keyword tokens, not generic vibhakti-tagged
    // vocabulary, so स्थान/वर्ण can never collide with a user-declared
    // column/table stem - see reserved_words.rs).

    fn load_safety_incident_logs_schema() -> crate::schema::Schema {
        use crate::lexer::tokenize;
        use crate::ssql_parser::{convert_vibhakti_tokens, parse};
        use crate::ast::{AstNode, Statement};

        let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/20_safety_incident_logs/schema.सं");
        let src = std::fs::read_to_string(schema_path).unwrap();
        let tokens = tokenize(&src).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &crate::schema::Schema::default()).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        drop(annotator);
        let ast = parse(parser_tokens).unwrap();
        let statements = match ast {
            AstNode::QueryBatch { statements, .. } => statements,
            AstNode::Statement(s) => vec![s],
        };
        let mut ddl_stmts = vec![];
        for stmt in statements {
            if let Statement::Ddl(d) = stmt {
                ddl_stmts.push(d);
            }
        }
        let mut schema = crate::schema::Schema::default();
        schema.update_from_ddl(&ddl_stmts, "sqlite").unwrap();
        schema
    }

    #[test]
    fn scratch_verify_locative_substr_tokenization() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;

        let schema = load_safety_incident_logs_schema();
        let ssql = r#"विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः"#;
        let tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);

        for (i, t) in parser_tokens.iter().enumerate() {
            println!("{i}: {t:?}");
        }

        assert!(matches!(parser_tokens[0], Token::Saptami { .. }), "token 0 should be Saptami (विवरणे): {:?}", parser_tokens[0]);
        assert!(matches!(parser_tokens[1], Token::StringLit(ref s) if s == "ERR"), "token 1 should be StringLit(ERR): {:?}", parser_tokens[1]);
        assert!(matches!(parser_tokens[2], Token::Ityasya(_)), "token 2 should be Ityasya: {:?}", parser_tokens[2]);
        assert!(matches!(parser_tokens[3], Token::Sthaanaat), "token 3 should be Sthaanaat: {:?}", parser_tokens[3]);
        assert!(matches!(parser_tokens[4], Token::Aarabhya), "token 4 should be Aarabhya: {:?}", parser_tokens[4]);
        assert!(matches!(parser_tokens[5], Token::Number(ref n) if n == "६"), "token 5 should be Number(६): {:?}", parser_tokens[5]);
        assert!(matches!(parser_tokens[6], Token::Varnaanaam), "token 6 should be Varnaanaam: {:?}", parser_tokens[6]);
        assert!(matches!(parser_tokens[7], Token::Khandah), "token 7 should be Khandah: {:?}", parser_tokens[7]);
        assert_eq!(parser_tokens.len(), 8, "unexpected extra/missing tokens: {:?}", parser_tokens);
    }

    #[test]
    fn test_locative_substr_ast_shape() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;
        use crate::ast::{Expression, Literal};

        let schema = load_safety_incident_logs_schema();
        let ssql = r#"दुर्घटनाभ्यः सङ्ख्या च विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः "error_code" इति आख्या च आह्रियेताम् ।"#;
        let tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &schema).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);
        let ast = parse(parser_tokens).unwrap();

        let crate::ast::AstNode::QueryBatch { statements, .. } = &ast else { panic!("Expected QueryBatch") };
        let crate::ast::Statement::Dql(dql) = &statements[0] else { panic!("Expected Dql") };
        let col = &dql.select.columns[1];
        assert_eq!(col.alias.as_deref(), Some("error_code"));

        let Expression::FunctionCall { name: outer_name, args: outer_args, .. } = &col.expr else {
            panic!("Expected outer FunctionCall, got {:?}", col.expr);
        };
        assert_eq!(outer_name, "खण्डः");
        assert_eq!(outer_args.len(), 3);

        let Expression::Identifier(outer_source) = &outer_args[0] else { panic!("Expected Identifier as arg 0") };
        assert_eq!(outer_source.pratipadika, "विवरण");

        // The position is NULL, not 0, where the text is absent.
        let Expression::FunctionCall { name: guard_name, args: guard_args, .. } = &outer_args[1] else {
            panic!("Expected nested FunctionCall as arg 1, got {:?}", outer_args[1]);
        };
        assert_eq!(guard_name, "अनावृत्तम्");
        let Expression::FunctionCall { name: inner_name, args: inner_args, .. } = &guard_args[0] else {
            panic!("Expected nested FunctionCall as guard arg 0, got {:?}", guard_args[0]);
        };
        assert_eq!(inner_name, "स्थानम्");
        assert_eq!(inner_args.len(), 2);
        let Expression::Identifier(inner_source) = &inner_args[0] else { panic!("Expected Identifier as inner arg 0") };
        assert_eq!(inner_source.pratipadika, "विवरण");
        assert!(matches!(&inner_args[1], Expression::Literal(Literal::String(s)) if s == "ERR"));

        assert!(matches!(&outer_args[2], Expression::Literal(Literal::Number(n)) if n == "६"));
    }

    fn compile_safety_incident_dql(ssql: &str, dialect: &str) -> String {
        let config = crate::compiler::CompilerConfig {
            dialect: dialect.to_string(),
            ..Default::default()
        };
        let mut compiler = crate::compiler::Compiler::new(config).unwrap();
        compiler.schema = load_safety_incident_logs_schema();
        compiler.compile_string(ssql).unwrap()
    }

    #[test]
    fn test_locative_substr_codegen_both_dialects() {
        let ssql = r#"दुर्घटनाभ्यः सङ्ख्या च विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः "error_code" इति आख्या च आह्रियेताम् ।"#;

        let sqlite_sql = compile_safety_incident_dql(ssql, "sqlite");
        assert!(
            sqlite_sql.contains("substr(t1.description, nullif(instr(t1.description, 'ERR'), 0), 6)"),
            "unexpected sqlite SQL: {}", sqlite_sql
        );

        let postgres_sql = compile_safety_incident_dql(ssql, "postgres");
        assert!(
            postgres_sql.contains("substr(t1.description, nullif(strpos(t1.description, 'ERR'), 0), 6)"),
            "unexpected postgres SQL: {}", postgres_sql
        );
    }

    // --- अद्यतनदिनाङ्कः/अद्यतनसमयः (CURRENT_DATE/
    // CURRENT_TIME) reachable as zero-arg builtins, mirroring अद्यतनक्षणः's
    // own already-working pattern exactly. ---

    #[test]
    fn test_current_date_time_tokenization() {
        use crate::lexer::tokenize;
        use crate::ssql_parser::convert_vibhakti_tokens;

        let ssql = r#"अद्यतनदिनाङ्कः च अद्यतनसमयः"#;
        let tokens = tokenize(ssql).unwrap();
        let mut annotator = crate::annotator::Annotator::new().unwrap();
        let enriched = annotator.annotate(tokens, &crate::schema::Schema::default()).unwrap();
        let parser_tokens = convert_vibhakti_tokens(enriched);

        assert!(matches!(parser_tokens[0], Token::AdyatanaDinankah), "token 0 should be AdyatanaDinankah: {:?}", parser_tokens[0]);
        assert!(matches!(parser_tokens[1], Token::Cha), "token 1 should be Cha: {:?}", parser_tokens[1]);
        assert!(matches!(parser_tokens[2], Token::AdyatanaSamayah), "token 2 should be AdyatanaSamayah: {:?}", parser_tokens[2]);
    }

    #[test]
    fn test_current_date_time_codegen_both_dialects() {
        let ssql = r#"पुस्तकेभ्यः शीर्षकम् च अद्यतनदिनाङ्कः च अद्यतनसमयः च आह्रियन्ताम् ।"#;

        let sqlite_sql = compile_dql(ssql, "sqlite");
        assert!(sqlite_sql.contains("date('now')"), "unexpected sqlite SQL (date): {}", sqlite_sql);
        assert!(sqlite_sql.contains("time('now')"), "unexpected sqlite SQL (time): {}", sqlite_sql);

        let postgres_sql = compile_dql(ssql, "postgres");
        // UTC, as SQLite's date('now') / time('now') are.
        assert!(postgres_sql.contains("CAST(now() AT TIME ZONE 'UTC' AS DATE)"), "unexpected postgres SQL (date): {}", postgres_sql);
        assert!(postgres_sql.contains("CAST(date_trunc('second', CAST(now() AT TIME ZONE 'UTC' AS TIME)) AS TIME)"), "unexpected postgres SQL (time): {}", postgres_sql);
        assert!(!postgres_sql.contains("CURRENT_TIME("), "CURRENT_TIME must not be called with parens in postgres: {}", postgres_sql);
    }

    #[test]
    fn test_pure_udf_shashthi_and_pradaya_calls() {
        let ssql = r#"
द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः दीयते
    मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

न्यूनीकृतम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च मूल्यह्रासः इति वास्तविकः च दीयेते
    मूल्यात् मूल्यह्रासम् विशोध्य अवशिष्टम् इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः शीर्षकम् च
    मूल्यस्य द्विगुणितम् "doubled_genitive" इति आख्या च
    मूल्यम् प्रदाय द्विगुणितम् "doubled_pradaya" इति आख्या च
    मूल्यम् च १० च प्रदाय न्यूनीकृतम् "discounted_pos" इति आख्या च
    न्यूनीकृतम् प्रदाय मूल्यह्रासः इत्यस्य मूल्यम् १० इति च मूल्यम् इत्यस्य मूल्यम् मूल्यम् इति "discounted_named" इति आख्या च
आह्रियन्ताम् ।
"#;
        let sql = compile_dql(ssql, "sqlite");
        assert!(
            sql.contains("((t1.price) * 2) AS doubled_genitive"),
            "expected Shashthi 1-arg UDF expansion in SQL: {}",
            sql
        );
        assert!(
            sql.contains("((t1.price) * 2) AS doubled_pradaya"),
            "expected Pradaya 1-arg UDF expansion in SQL: {}",
            sql
        );
        assert!(
            sql.contains("((t1.price) - (10)) AS discounted_pos"),
            "expected positional 2-arg UDF expansion in SQL: {}",
            sql
        );
        assert!(
            sql.contains("((t1.price) - (10)) AS discounted_named"),
            "expected named-arg (reordered) 2-arg UDF expansion in SQL: {}",
            sql
        );
    }

    #[test]
    fn test_pure_udf_param_verb_number_mismatch_rejected() {
        let ssql = r#"
न्यूनीकृतम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च मूल्यह्रासः इति वास्तविकः च दीयते
    मूल्यात् मूल्यह्रासम् विशोध्य अवशिष्टम् इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।
पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।
"#;
        let config = crate::compiler::CompilerConfig {
            dialect: "sqlite".to_string(),
            ..Default::default()
        };
        let mut compiler = crate::compiler::Compiler::new(config).unwrap();
        compiler.schema = load_university_bookstore_schema();
        let err = compiler
            .compile_string(ssql)
            .expect_err("2-param UDF with singular दीयते should fail strict validation");
        let msg = err.to_string();
        assert!(
            msg.contains("UDF parameter verb number mismatch"),
            "expected UDF parameter verb number mismatch warning, got: {}",
            msg
        );
    }

    #[test]
    fn test_sql_to_ssql_roundtrip_select_join_group_having() {
        let schema = load_university_bookstore_schema();
        let sql_input = r#"
            SELECT b.title, a.name, b.price
            FROM books b
            INNER JOIN authors a ON b.author_id = a.id
            WHERE b.price > 25 AND b.publication_year >= 2015
            ORDER BY b.price DESC
            LIMIT 10;
        "#;
        let ssql = crate::sql_importer::import_sql_to_ssql(sql_input, &schema)
            .expect("SQL -> SSQL reverse transpilation should succeed");
        assert!(
            !ssql.contains('(') && !ssql.contains(')'),
            "Generated SSQL must not contain parentheses: {}",
            ssql
        );
        assert!(
            ssql.contains("आह्रियन्ताम्"),
            "3-column projection must use plural आह्रियन्ताम्: {}",
            ssql
        );
        // Round-trip compile in strict mode!
        let roundtrip_sql = compile_dql(&ssql, "sqlite");
        assert!(
            roundtrip_sql.contains("JOIN authors t2 ON t1.author_id = t2.id"),
            "Roundtrip SQL mismatch: {}",
            roundtrip_sql
        );
        assert!(
            roundtrip_sql.contains("LIMIT 10"),
            "Roundtrip SQL missing LIMIT 10: {}",
            roundtrip_sql
        );

        // Also test GROUP BY + HAVING round-trip
        let agg_sql = r#"
            SELECT language, AVG(price) AS avg_price
            FROM books
            GROUP BY language
            HAVING COUNT(*) > 2;
        "#;
        let agg_ssql = crate::sql_importer::import_sql_to_ssql(agg_sql, &schema)
            .expect("GROUP BY + HAVING SQL -> SSQL should succeed");
        let agg_roundtrip = compile_dql(&agg_ssql, "sqlite");
        assert!(
            agg_roundtrip.contains("GROUP BY t1.language") && agg_roundtrip.contains("HAVING COUNT(*) > 2"),
            "GROUP BY + HAVING roundtrip mismatch: {}",
            agg_roundtrip
        );
    }

    #[test]
    fn test_sql_to_ssql_roundtrip_ddl_and_dml() {
        let empty_schema = crate::schema::Schema::default();
        let sql_batch = r#"
            CREATE TABLE books (
                book_id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                price REAL
            );
            INSERT INTO books (book_id, title, price) VALUES (101, 'Meghadutam', 50);
            UPDATE books SET price = 60 WHERE book_id = 101;
            DELETE FROM books WHERE price < 20;
        "#;
        let ssql = crate::sql_importer::import_sql_to_ssql(sql_batch, &empty_schema)
            .expect("DDL + DML SQL batch should reverse-transpile to SSQL");
        assert!(
            ssql.contains("रच्यताम् यत्र"),
            "Expected DDL रच्यताम् यत्र in generated SSQL: {}",
            ssql
        );
        assert!(
            ssql.contains("निवेश्यन्ताम्"),
            "Expected the plural INSERT verb निवेश्यन्ताम् (three columns) in generated SSQL: {}",
            ssql
        );
        assert!(
            ssql.contains("परिवर्त्यताम्"),
            "Expected UPDATE परिवर्त्यताम् in generated SSQL: {}",
            ssql
        );
        assert!(
            ssql.contains("निष्कास्यताम्"),
            "Expected DELETE निष्कास्यताम् in generated SSQL: {}",
            ssql
        );

        // Compile the generated SSQL in strict mode!
        let config = crate::compiler::CompilerConfig {
            dialect: "sqlite".to_string(),
            ..Default::default()
        };
        let mut compiler = crate::compiler::Compiler::new(config).unwrap();
        let compiled_sql = compiler
            .compile_string(&ssql)
            .expect("Reverse-transpiled DDL+DML SSQL must compile cleanly in strict mode");
        assert!(compiled_sql.contains("CREATE TABLE books"));
        assert!(compiled_sql.contains("INSERT INTO books"));
        assert!(compiled_sql.contains("UPDATE books SET price = 60"));
        assert!(compiled_sql.contains("DELETE FROM books"));
    }

    #[test]
    fn test_ssql_formatter_canonical_layout() {
        let messy = r#"पुस्तकानि   इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
शीर्षकम्   इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम्
इति पटलम्।


पुस्तकेभ्यः
मूल्यम्   २० इत्यस्मात् अधिकम् अस्ति इति नियमेन
शीर्षकम् आह्रियताम्।"#;
        let formatted = crate::formatter::format_ssql_source(messy);
        assert!(
            formatted.contains("    शीर्षकम् इति वाक्यम् \"title\" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम्\n  इति पटलम् ।"),
            "DDL indentation mismatch:\n{}",
            formatted
        );
        assert!(
            formatted.contains("पुस्तकेभ्यः\n    मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन\n    शीर्षकम् आह्रियताम् ।"),
            "DQL indentation and danda spacing mismatch:\n{}",
            formatted
        );
    }

    #[test]
    fn test_ssql_linter_diagnostics() {
        let impure = "पुस्तकेभ्यः (मूल्यं 20 इत्यस्मात् अधिकम् अस्ति) इति नियमेन शीर्षकम् आह्रियताम्";
        let diags = crate::linter::lint_ssql_source(impure);
        let codes: Vec<&str> = diags.iter().map(|d| d.code).collect();
        assert!(codes.contains(&"LINT-001"), "Expected LINT-001 (ascii-numerals), got: {:?}", diags);
        assert!(codes.contains(&"LINT-002"), "Expected LINT-002 (legacy-parentheses), got: {:?}", diags);
        assert!(codes.contains(&"LINT-003"), "Expected LINT-003 (anusvara-halanta), got: {:?}", diags);
        assert!(codes.contains(&"LINT-004"), "Expected LINT-004 (missing-purnadanda), got: {:?}", diags);

        let clean = "पुस्तकेभ्यः यत् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति इति नियमेन शीर्षकम् आह्रियताम् ।";
        let clean_diags = crate::linter::lint_ssql_source(clean);
        assert!(clean_diags.is_empty(), "Expected zero lint issues for clean SSQL, got: {:?}", clean_diags);
    }

    #[test]
    fn test_dual_ityayoh_after_bare_select_column_and_vishodhya_subtraction() {
        let ssql = "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च २ च इत्यनयोः गुणनम् द्विगुणमूल्यम् इति आख्या च मूल्यात् ५ विशोध्य अवशिष्टम् न्यूनमूल्यम् इति आख्या च आह्रियन्ताम् ।";
        let sql = compile_dql(ssql, "sqlite");
        assert!(
            sql.contains("t1.title,\n    t1.price * 2 AS द्विगुणमूल्यम्,\n    t1.price - 5 AS न्यूनमूल्यम्"),
            "Expected 3 projected columns (title, price * 2, price - 5), got:\n{}",
            sql
        );
    }

    #[test]
    fn test_cte_singular_ekavachana_ablative_source() {
        let ssql = r#"
            उच्चसमीक्षासारः इति आख्या उच्चसमीक्षासार इति पुंलिङ्गम् एकवचनम् सञ्ज्ञा यत् समीक्षाभ्यः मूल्याङ्कनम् ४ इत्यस्मात् अधिकम् अस्ति इति नियमेन पुस्तकसङ्ख्या च मूल्याङ्कनम् च आह्रियेताम् इति ।
            उच्चसमीक्षासारात् पुस्तकसङ्ख्या च मूल्याङ्कनम् च आह्रियेताम् ।
        "#;
        let sql = compile_dql(ssql, "sqlite");
        assert!(
            sql.contains("WITH उच्चसमीक्षासार AS") && sql.contains("FROM\n    उच्चसमीक्षासार t2"),
            "Expected singular CTE queried in Ablative Singular to compile cleanly, got:\n{}",
            sql
        );
    }

    #[test]
    fn test_bare_select_column_before_multi_arg_pradaya_udf() {
        let ssql = r#"
            करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
                मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
                मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
            इति कार्यम् ।

            पुस्तकेभ्यः
                शीर्षकम् च
                मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च
            आह्रियेताम् ।
        "#;
        let sql = compile_dql(ssql, "sqlite");
        assert!(
            sql.contains("t1.title,\n    ((t1.price) + (5)) AS सकलमूल्यम्"),
            "Expected 2 projected columns (title, price + 5), got:\n{}",
            sql
        );
    }

    #[test]
    fn test_pure_udf_passive_params_and_pradaya_call() {
        let ssql = r#"
            करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
                मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
                मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
            इति कार्यम् ।

            पुस्तकेभ्यः
                शीर्षकम् च
                मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च
            आह्रियेताम् ।
        "#;
        let sql = compile_dql(ssql, "sqlite");
        assert!(
            sql.contains("t1.title,\n    ((t1.price) + (5)) AS सकलमूल्यम्"),
            "Expected the UDF and its प्रदाय call to compile cleanly, got:\n{}",
            sql
        );
    }

    /// A scratch directory holding a copy of the bookstore schema (the
    /// compiler finds `schema.सं` next to the query), removed on drop even
    /// when the test fails - never a file inside the repository.
    struct BookstoreScratch(std::path::PathBuf);

    impl BookstoreScratch {
        fn new(name: &str, id: usize) -> Self {
            let dir = std::env::temp_dir().join(format!("ssql_{}_{}_{}", name, std::process::id(), id));
            std::fs::create_dir_all(&dir).unwrap();
            let schema = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/university_bookstore/SAMSQL/schema.सं");
            std::fs::copy(schema, dir.join("schema.सं")).unwrap();
            BookstoreScratch(dir)
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for BookstoreScratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }


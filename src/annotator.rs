//! # Two-Pass Annotator
//!
//! The annotator is the core innovation of the SSQL compiler.  
//! It converts "dumb" IDENTIFIER tokens into "smart" vibhakti-aware tokens.
//!
//! ## Two-Pass Architecture
//!
//! ### Pass 1: Symbol Table Population (सञ्ज्ञा-स्थापनम्)
//! - Scans for declaration keywords (इति, स्वरूप्यताम्, etc.)
//! - Identifies प्रथमा forms  
//! - Generates all vibhakti forms using morphology engine
//! - Populates symbol table
//!
//! ### Pass 2: Token Enrichment (विशेषण-योजनम्)
//! - Processes each IDENTIFIER token from lexer
//! - Looks up in symbol table
//! - Creates VibhaktiToken with possible interpretations
//! - Outputs enriched token stream for parser

use crate::error::CompilerResult;
use crate::morphology::MorphologyEngine;
use crate::symbol_table::{SymbolEntry, SymbolTable};
use crate::types::{Vibhakti, VibhaktiForm, VibhaktiToken, Vachana};
use std::collections::HashMap;
use anyhow::Result;

/// Raw token from the grapheme-aware lexer (`src/lexer.rs`) before vibhakti annotation.
#[derive(Debug, Clone)]
pub struct LexerToken {
    pub text: String,
    pub token_type: TokenType,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    /// Generic identifier (will be enriched by annotator)
    Identifier,
    
    /// Keywords
    Iti,          // इति
    Ashritya,     // आश्रित्य
    Sutram,       // सूत्रम्
    Niyamah,      // नियमः
    Cha,          // च
    Va,           // वा
    Sadrusham,    // सदृशम् (LIKE)
    Bahya,        // बाह्य (External)
    Sanjna,       // सञ्ज्ञा (Symbol/Variable)
    Aakhya,       // आख्या (Alias)
    
    // --- DDL Tokens ---
    Anivaryah,    // अनिवार्यः
    Vaikalpikah,  // वैकल्पिकः
    Prathamikah,  // प्राथमिकः
    Kramikah,     // क्रमिकः
    Advitiyah,    // अद्वितीयः
    Pumlingam,    // पुंलिङ्गम्
    Strilingam,         // स्त्रीलिङ्गम्
    Napumsakalingam,   // नपुंसकलिङ्गम्
    Ekavachanam,  // एकवचनम्
    Dvivachanam,  // द्विवचनम्
    Bahuvachanam, // बहुवचनम्
    Baddhah,      // बद्धः
    Patalam,      // पटलम्
    Prati,        // प्रति
    Suchi,        // सूची / सूचिका

    /// Verbs
    FetchVerb,    // आह्रियताम्
    InsertVerb,   // निवेश्यताम्
    AddVerb,      // योज्यताम्
    ChangeVerb,   // परिवर्त्यताम्
    AbhidhiyatamVerb, // अभिधीयताम्
    
    /// Other tokens (numbers, strings, punctuation, etc.)
    Number(String),
    String(String),
    Purnadanda,   // ।
    LParen,
    RParen,

    /// Window Function Keywords
    Yatra,        // यत्र
    Vibhajanam,   // विभाजनम्
    Tatha,        // तथा
    Shreni,       // श्रेणी
    PanktiSankhya, // पङ्क्तिसङ्ख्या (ROW_NUMBER)
    SangrahabindumDvitiya, // सङ्ग्रहबिन्दुम् (accusative, ROLLBACK TO: सङ्ग्रहबिन्दुम् प्रति)
    Vina, // विना (in पुनरुक्तिम् विना, SELECT DISTINCT)
    Punaruktim, // पुनरुक्तिम् (in पुनरुक्तिम् विना, SELECT DISTINCT)
    Parinamah, // परिणामः (CAST: <expr-genitive> <type>रूपेण परिणामः)
    Aadhare, // आधारे (an index is built "on" its columns)
    LaghutamasyaGen, // लघुतमस्य (genitive of लघुतमम्, MIN)
    MahattamasyaGen, // महत्तमस्य (genitive of महत्तमम्, MAX)
    MadhyasyaGen, // माध्यस्य (genitive of माध्यम्, AVG)
    GananayahGen, // गणनायाः (genitive of गणना, COUNT)
    YogasyaGen, // योगस्य (genitive of योगः, SUM: a window key)
    Panktinam, // पङ्क्तीनाम् (COUNT(*): पङ्क्तीनाम् गणना, "the count of the rows")
    Sarvatra, // सर्वत्र (UPDATE of every row: "everywhere")
    Bhagankah, // भागाङ्कः (NTILE(n): <n> भागेषु भागाङ्कः)
    Chaturthamshankah, // चतुर्थांशाङ्कः (NTILE(4): the quartile number)
    AnantaraPankteh, // अनन्तरपङ्क्तेः (LEAD: the next row's …)
    PurvaPankteh, // पूर्वपङ्क्तेः (LAG: the previous row's …)
    SaghanaShreni, // सघनश्रेणी (DENSE_RANK)
    Arohah,       // आरोहः
    Avarohah,     // अवरोहः
    Aadharena,    // आधारेण (Based On / Partition By)
    Anusaram,     // अनुसारम् (According To / Order By)
    ArohaKramena, // आरोहक्रमेण (Asc Order)
    AvarohaKramena, // अवरोहक्रमेण (Desc Order)

    /// Explicit Clause Keywords (replacing generic Tritiya)
    Niyamena,     // नियमेन (WHERE)
    Samuhena,     // समूहेन (GROUP BY)
    Samuhaniyamena, // समूहनियमेन (HAVING)
    Kramena,      // क्रमेण (ORDER BY)
    Melanena,     // मेलनेन (JOIN ON)
    VamaMelanena, // वाममेलनेन (LEFT JOIN)
    DakshinaMelanena, // दक्षिणमेलनेन (RIGHT JOIN)
    PurnaMelanena,    // पूर्णमेलनेन (FULL OUTER JOIN)
    SarvaMelanena,    // सर्वमेलनेन (CROSS JOIN)
    Mitya,        // मित्या (LIMIT)
    
    // Union / Set Ops
    Ityayoh,
    Ityesham,
    Chet,
    Sangamah,
    SarvaSangamah, // सर्वसङ्गमः (UNION ALL)
    
    // Aggregates & Utils
    Yogah,
    Madhyam,
    Mahattam,
    Laghutamam,
    Anyat,
    Api,
    Ityasya,
    Ityasmat,
    Ityasyah,
    Ityetasam,
    Tasmat,       // तस्मात्

    // --- Window Frame Specification ---
    Panktayah,
    Vyaptih,
    Samuhah,
    AparimitaPurvavartitah,
    Purvavartitah,
    Vartamanapanktih,
    Anuvartitah,
    AparimitaAnuvartitah,
    Aarabhya,
    Paryantam,
    Varjayitva,
    Samuham,
    Sambaddhan,
    Anyan,
    
    // TCL Tokens
    BeginVerb,
    CommitVerb,
    RollbackVerb,
    Sangrahabinduh,
}

/// The main annotator
pub struct Annotator {
    morphology: MorphologyEngine,
    pub symbol_table: SymbolTable,
}

impl Annotator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            morphology: MorphologyEngine::new()?,
            symbol_table: SymbolTable::new(),
        })
    }
    
    /// Main entry point: annotate a token stream
    /// 
    /// This performs both passes in sequence:
    /// 1. Pass 1: Build symbol table
    /// 2. Pass 2: Enrich tokens
    pub fn annotate(&mut self, tokens: Vec<LexerToken>, schema: &crate::schema::Schema) -> CompilerResult<Vec<VibhaktiToken>> {
        // Pass 1: Build symbol table from Schema (ignore tokens for this phase)
        self.pass1_build_symbol_table(&tokens, schema)?;
        
        // Pass 2: Enrich tokens
        let enriched = self.pass2_enrich_tokens(&tokens, schema)?;

        self.check_table_qualifiers(&tokens, schema)?;
        self.check_column_number(&tokens, schema)?;
        self.check_statement_shape(&tokens, schema)?;
        self.check_declared_column_pronoun(&tokens, schema)?;
        self.check_constraint_agreement(&tokens)?;

        Ok(enriched)
    }

    /// A column-constraint adjective agrees with its column in gender and
    /// number (`अनिवार्या ग्राहकसङ्ख्या`, `प्राथमिकः अङ्कः`; expert review O1).
    /// The column's gender and number are read from its own declaration,
    /// which follows the constraint before the next `च`:
    /// `… <name> इति <type> "<sql>" इति आख्या <stem> इति <gender> <number>`.
    /// `अद्वितीया सूची` (a unique index) agrees with सूची.
    fn check_constraint_agreement(&mut self, tokens: &[LexerToken]) -> CompilerResult<()> {
        // (masculine/neuter stem, feminine stem) of each constraint adjective.
        const ADJECTIVES: &[(TokenType, &str, &str)] = &[
            (TokenType::Anivaryah, "अनिवार्य", "अनिवार्या"),
            (TokenType::Vaikalpikah, "वैकल्पिक", "वैकल्पिकी"),
            (TokenType::Prathamikah, "प्राथमिक", "प्राथमिकी"),
            (TokenType::Kramikah, "क्रमिक", "क्रमिका"),
            (TokenType::Advitiyah, "अद्वितीय", "अद्वितीया"),
            (TokenType::Baddhah, "बद्ध", "बद्धा"),
        ];
        for (index, token) in tokens.iter().enumerate() {
            let Some((_, stem, feminine)) = ADJECTIVES.iter().find(|(t, _, _)| *t == token.token_type) else { continue };
            let (column, gender, number) = if tokens.get(index + 1).map_or(false, |t| t.text == "सूची") {
                ("सूची".to_string(), crate::types::Linga::Feminine, Vachana::Eka)
            } else {
                let mut found = None;
                for j in index + 1..tokens.len() {
                    let t = &tokens[j];
                    if t.text == "च" || t.text == "।" {
                        break;
                    }
                    let gender = match t.text.as_str() {
                        "पुंलिङ्गम्" => crate::types::Linga::Masculine,
                        "स्त्रीलिङ्गम्" => crate::types::Linga::Feminine,
                        "नपुंसकलिङ्गम्" => crate::types::Linga::Neuter,
                        _ => continue,
                    };
                    let number = match tokens.get(j + 1).map(|t| t.text.as_str()) {
                        Some("द्विवचनम्") => Vachana::Dvi,
                        Some("बहुवचनम्") => Vachana::Bahu,
                        _ => Vachana::Eka,
                    };
                    let column = if j >= 2 { tokens[j - 2].text.clone() } else { String::new() };
                    found = Some((column, gender, number));
                    break;
                }
                let Some(found) = found else { continue };
                found
            };
            let adjective = if gender == crate::types::Linga::Feminine { feminine } else { stem };
            let Ok(table) = self.morphology.decline_with_gender(adjective, gender, number) else { continue };
            let Some(expected) = table.forms.get(&Vibhakti::Prathama) else { continue };
            if *expected != token.text {
                return Err(crate::error::SemanticError::ConstraintAgreement {
                    word: token.text.clone(),
                    position: token.position,
                    column,
                    expected: expected.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// A column's table is named in the genitive singular, `पुस्तकस्य शीर्षकम्`
    /// ("the book's title", taken row by row); the plural reads as one
    /// collective title (expert review I1). The plural stays in an index
    /// declaration, which is about the table itself (`पुस्तकानाम् शीर्षकस्य
    /// आधारे … सूची`). The parser rejects the plural too, but its error names
    /// whichever alternative got furthest, so the rule is reported here.
    /// A declared column has a stem, so it is declined like any noun
    /// (`सकलमूल्यस्य योगः`); `सकलमूल्यम् इत्यस्य योगः` treats it as a quoted
    /// name, which is for an alias or a CTE's column (paṇḍit-checklist
    /// review, pcf.md item 28; rule G2).
    fn check_declared_column_pronoun(&mut self, tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<()> {
        let mut genitives: HashMap<String, String> = HashMap::new();
        for (stem, table) in &schema.tables {
            if schema.derived_tables.contains(stem) {
                continue;
            }
            for col in table.fields.values() {
                let Ok(t) = self.morphology.decline_with_gender(&col.pratipadika, col.gender, Vachana::Eka) else { continue };
                if let (Some(nom), Some(gen)) = (t.forms.get(&Vibhakti::Prathama), t.forms.get(&Vibhakti::Shashthi)) {
                    genitives.insert(nom.clone(), gen.clone());
                }
            }
        }
        let aliases: std::collections::HashSet<&str> = tokens
            .windows(3)
            .filter(|w| w[1].text == "इति" && w[2].text == "आख्या")
            .map(|w| w[0].text.as_str())
            .collect();
        // Only where the pronoun is a genitive: before an aggregate, a
        // function, a date part or a window's अनुसारम् / आधारेण. In a
        // comparison (`… सङ्ख्या इत्यस्याः न्यूना`) the feminine form is the
        // ablative of the compared value.
        let genitive_head = |w: &str| {
            crate::reserved_words::BUILTIN_FUNCTION_WORDS.contains(&w)
                || matches!(w, "योगः" | "माध्यम्" | "गणना" | "महत्तमम्" | "लघुतमम्" | "अनुसारम्" | "आधारेण"
                    | "योगस्य" | "माध्यस्य" | "गणनायाः" | "महत्तमस्य" | "लघुतमस्य"
                    | "वर्षम्" | "मासः" | "दिनम्" | "होरा" | "निमेषः" | "क्षणम्" | "वारः")
        };
        for w in tokens.windows(3) {
            if !matches!(w[1].text.as_str(), "इत्यस्य" | "इत्यस्याः")
                || aliases.contains(w[0].text.as_str())
                || !genitive_head(&w[2].text)
            {
                continue;
            }
            if let Some(genitive) = genitives.get(&w[0].text) {
                return Err(crate::error::SemanticError::PronounForDeclaredColumn {
                    word: w[0].text.clone(),
                    pronoun: w[1].text.clone(),
                    position: w[0].position,
                    genitive: genitive.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Two sentence shapes the parser can only report by a far, unrelated
    /// token: a query whose table is in the locative (the case of INSERT and
    /// UPDATE), and a condition (`… अस्ति` / `… नास्ति`) not closed by `इति
    /// नियमेन` or joined by `च` / `वा`. Only `इति`, `च`, `वा`, CASE's
    /// `तस्मिन्` or the sentence's end follow a condition.
    fn check_statement_shape(&mut self, tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<()> {
        let mut locatives: HashMap<String, String> = HashMap::new();
        for table in schema.tables.values() {
            for number in [Vachana::Eka, Vachana::Bahu] {
                if let Ok(t) = self.morphology.decline_with_gender(&table.pratipadika, table.gender, number) {
                    if let (Some(loc), Some(abl)) = (t.forms.get(&Vibhakti::Saptami), t.forms.get(&Vibhakti::Panchami)) {
                        locatives.insert(loc.clone(), abl.clone());
                    }
                }
            }
        }
        for statement in tokens.split(|t| t.text == "।") {
            let (Some(first), Some(last)) = (statement.first(), statement.last()) else { continue };
            if matches!(last.token_type, TokenType::FetchVerb) {
                if let Some(ablative) = locatives.get(&first.text) {
                    return Err(crate::error::SemanticError::QuerySourceCase {
                        word: first.text.clone(),
                        ablative: ablative.clone(),
                    }
                    .into());
                }
            }
            // UPDATE's emptied column: `<column> रिक्त- इति`, the adjective in
            // the column's gender (`भाषा रिक्ता`, `मूल्यम् रिक्तम्`).
            if matches!(last.token_type, TokenType::ChangeVerb) {
                let table = schema.resolve_pratipadika(&first.text).map(str::to_string);
                for w in statement.windows(3) {
                    if !matches!(w[1].text.as_str(), "रिक्तः" | "रिक्ता" | "रिक्तम्") || w[2].text != "इति" {
                        continue;
                    }
                    let Some(gender) = schema.get_column_gender(&w[0].text, table.as_deref()) else { continue };
                    let expected = match gender {
                        crate::types::Linga::Masculine => "रिक्तः",
                        crate::types::Linga::Feminine => "रिक्ता",
                        crate::types::Linga::Neuter => "रिक्तम्",
                    };
                    if w[1].text != expected {
                        return Err(crate::error::SemanticError::EmptyValueAgreement {
                            word: w[1].text.clone(),
                            position: w[1].position,
                            column: w[0].text.clone(),
                            expected: expected.to_string(),
                        }
                        .into());
                    }
                }
            }
            for (i, pair) in statement.windows(2).enumerate() {
                // A condition selected as a value may be named: `… अस्ति
                // <alias> इति आख्या`.
                let alias = statement.get(i + 2).map_or(false, |t| t.text == "इति")
                    && statement.get(i + 3).map_or(false, |t| t.text == "आख्या");
                if matches!(pair[0].text.as_str(), "अस्ति" | "नास्ति")
                    && !matches!(pair[1].text.as_str(), "इति" | "च" | "वा" | "तस्मिन्")
                    && !alias
                {
                    return Err(crate::error::SemanticError::ConditionWithoutMarker {
                        verb: pair[0].text.clone(),
                        after: pair[1].text.clone(),
                    }
                    .into());
                }
            }
        }
        Ok(())
    }

    /// A column is read in its declared, singular number (expert review,
    /// final): `प्रतिक्रियाः` for the column `प्रतिक्रिया` was resolved by stem
    /// and accepted. The genitive plural stays, as an aggregate's argument
    /// (`प्रतिक्रियाणाम् माध्यम्`); a word that is also some declared word's
    /// singular form is left alone.
    fn check_column_number(&mut self, tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<()> {
        let mut singular_forms = std::collections::HashSet::new();
        let mut plurals: HashMap<String, String> = HashMap::new();
        for table in schema.tables.values() {
            if let Ok(t) = self.morphology.decline_with_gender(&table.pratipadika, table.gender, Vachana::Eka) {
                singular_forms.extend(t.forms.into_values());
            }
            for col in table.fields.values() {
                let (Ok(sing), Ok(plur)) = (
                    self.morphology.decline_with_gender(&col.pratipadika, col.gender, Vachana::Eka),
                    self.morphology.decline_with_gender(&col.pratipadika, col.gender, Vachana::Bahu),
                ) else { continue };
                // Nominative first: a form shared by two cases (`-आः` is
                // nominative and accusative) suggests the nominative.
                use Vibhakti::*;
                for case in [Prathama, Dvitiya, Tritiya, Chaturthi, Panchami, Saptami] {
                    if let (Some(form), Some(s)) = (plur.forms.get(&case), sing.forms.get(&case)) {
                        plurals.entry(form.clone()).or_insert_with(|| s.clone());
                    }
                }
                singular_forms.extend(sing.forms.into_values());
            }
        }
        for token in tokens {
            if matches!(token.token_type, TokenType::String(_)) || singular_forms.contains(&token.text) {
                continue;
            }
            if let Some(singular) = plurals.get(&token.text) {
                return Err(crate::error::SemanticError::ColumnInPlural {
                    word: token.text.clone(),
                    singular: singular.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    fn check_table_qualifiers(&mut self, tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<()> {
        let mut plural_genitives: HashMap<String, (&crate::schema::TableInfo, String)> = HashMap::new();
        for table in schema.tables.values() {
            let (Ok(plural), Ok(singular)) = (
                self.morphology.decline_with_gender(&table.pratipadika, table.gender, Vachana::Bahu),
                self.morphology.decline_with_gender(&table.pratipadika, table.gender, Vachana::Eka),
            ) else { continue };
            if let (Some(p), Some(s)) = (plural.forms.get(&Vibhakti::Shashthi), singular.forms.get(&Vibhakti::Shashthi)) {
                if p != s {
                    plural_genitives.insert(p.clone(), (table, s.clone()));
                }
            }
        }
        let mut statement_start = 0;
        for (index, token) in tokens.iter().enumerate() {
            if token.text == "।" {
                statement_start = index + 1;
                continue;
            }
            let Some((table, singular)) = plural_genitives.get(&token.text) else { continue };
            let Some(next) = tokens.get(index + 1) else { continue };
            if !table.column_form_map.contains_key(&next.text) && !table.fields.contains_key(&next.text) {
                continue;
            }
            let statement_end = tokens[index..].iter().position(|t| t.text == "।").map_or(tokens.len(), |p| index + p);
            if tokens[statement_start..statement_end].iter().any(|t| t.text == "सूची") {
                continue;
            }
            return Err(crate::error::SemanticError::PluralTableQualifier {
                word: token.text.clone(),
                singular: singular.clone(),
                column: next.text.clone(),
            }
            .into());
        }
        Ok(())
    }
    
    /// Pass 1: Build symbol table ONLY from Schema (Schema-First)
    ///
    /// We iterate over all types defined in the Schema and populate the SymbolTable.
    /// We do NOT scan the source tokens for declarations anymore.
    fn pass1_build_symbol_table(&mut self, _tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<()> {
        // Iterate over all tables/views/types in the schema
        for (pratipadika, _type_def) in &schema.tables {
             self.process_schema_entry(pratipadika, schema)?;
        }
        // Iterate over all functions in the schema
        for (func_name, _) in &schema.functions {
            let mut stem = func_name.clone();
            let mut gender = crate::types::Linga::Neuter;
            if let Ok(table) = self.morphology.decline(func_name, Vachana::Eka) {
                stem = table.pratipadika.clone();
                gender = table.linga;
            }
            
            let mut entry = SymbolEntry::new(
                stem.clone(),
                Vachana::Eka,
                gender,
                0,
            );
            if let Ok(table) = self.morphology.decline_with_gender(&stem, gender, Vachana::Eka) {
                for (vibhakti, form) in table.forms {
                    entry.add_form(vibhakti, form);
                }
            }
            self.symbol_table.add_symbol(entry);
        }
        Ok(())
    }
    
    /// Process a schema entry and add to Symbol table
    fn process_schema_entry(&mut self, pratipadika: &str, schema: &crate::schema::Schema) -> CompilerResult<()> {
        if let Some(table_info) = schema.tables.get(pratipadika) {
             let gender = table_info.gender;
             // Add singular forms (standard for identifiers)
             // We could add Plural too if needed, but usually Alias references are Singular?
             // Actually, for CTEs/Tables, they might be Plural.
             // Let's add Eka and Bahu.
             
             // Eka
             if let Ok(table) = self.morphology.decline_with_gender(pratipadika, gender, Vachana::Eka) {
                 let mut entry = SymbolEntry::new(
                    pratipadika.to_string(),
                    Vachana::Eka,
                    gender,
                    0, // No source position available from Schema
                 );
                 for (vibhakti, form) in table.forms {
                     entry.add_form(vibhakti, form);
                 }
                 self.symbol_table.add_symbol(entry);
             }
             
             // Dvi (a table named by a dual word, like दम्पती)
             if let Ok(table) = self.morphology.decline_with_gender(pratipadika, gender, Vachana::Dvi) {
                 let mut entry = SymbolEntry::new(
                    pratipadika.to_string(),
                    Vachana::Dvi,
                    gender,
                    0,
                 );
                 for (vibhakti, form) in table.forms {
                     entry.add_form(vibhakti, form);
                 }
                 self.symbol_table.add_symbol(entry);
             }

             // Bahu
             if let Ok(table) = self.morphology.decline_with_gender(pratipadika, gender, Vachana::Bahu) {
                 let mut entry = SymbolEntry::new(
                    pratipadika.to_string(),
                    Vachana::Bahu,
                    gender,
                    0, 
                 );
                 for (vibhakti, form) in table.forms {
                     entry.add_form(vibhakti, form);
                 }
                 self.symbol_table.add_symbol(entry);
             }
        }
        Ok(())
    }

    
    /// Pass 2: Enrich tokens with vibhakti information
    fn pass2_enrich_tokens(&mut self, tokens: &[LexerToken], schema: &crate::schema::Schema) -> CompilerResult<Vec<VibhaktiToken>> {
        let mut enriched = Vec::new();

        for (index, token) in tokens.iter().enumerate() {
            if token.token_type == TokenType::Identifier {
                let lookup_results = self.symbol_table.lookup(&token.text);
                let token_text = token.text.clone();
                // A bare stem is a word only in a declaration's stem slot
                // (`नामन् इति नपुंसकलिङ्गम् …`); anywhere else a name must be
                // one of its declined forms, matched exactly (no anusvāra/म् or ण/न
                // leniency, which let wrong forms through). A name known only
                // as itself (a CTE's output column) matches itself.
                let is_stem_slot = tokens.get(index + 1).map_or(false, |t| t.text == "इति")
                    && tokens.get(index + 2).map_or(false, |t| matches!(t.text.as_str(), "पुंलिङ्गम्" | "स्त्रीलिङ्गम्" | "नपुंसकलिङ्गम्"));

                if !lookup_results.is_empty() {
                     // Found in symbol table (Declared variable)
                     // Extract all possible forms
                     let forms: Vec<crate::types::VibhaktiForm> = lookup_results
                         .iter()
                         .map(|(_, form)| form.clone())
                         .collect();

                     let mut vib_token = VibhaktiToken::new(
                         token_text,
                         forms, // Use forms from lookup
                         token.position,
                         0,
                     );
                     // Set pratipadika from first result
                     if let Some((p, _)) = lookup_results.first() {
                         vib_token.pratipadika = Some(p.clone()); 
                     }
                     enriched.push(vib_token);
                } else {
                    let resolved_pratipadika = schema.resolve_pratipadika(&token.text);
                    
                    if let Some(pratipadika) = resolved_pratipadika {
                        // Found in Schema (Table or Column)
                        if let Some(table_info) = schema.tables.get(pratipadika) {
                        // Is it a Table Form?
                        let mut forms = Vec::new();
                        let gender = table_info.gender;
                        
                        // Try all 3 vachanas
                        for v in [Vachana::Eka, Vachana::Dvi, Vachana::Bahu] {
                            if let Ok(table) = self.morphology.decline_with_gender(pratipadika, gender, v) {
                                for (vibhakti, form_text) in table.forms {
                                    if form_text == token_text { // Compare against canonical text
                                        forms.push(VibhaktiForm {
                                            vibhakti,
                                            vachana: v,
                                            linga: Some(gender),
                                        });
                                    }
                                }
                            }
                        }
                        
                        if forms.is_empty() && !is_stem_slot && pratipadika != token_text {
                            return Err(Self::not_a_form(&token_text, pratipadika, gender, &mut self.morphology));
                        }
                        let mut vib_token = VibhaktiToken::new(
                            token_text,
                            forms,
                            token.position,
                            0,
                        );
                        vib_token.pratipadika = Some(pratipadika.to_string());
                        
                        enriched.push(vib_token);

                     } else {
                         // Logic if resolve_pratipadika worked but not in tables map (e.g. valid column but not a table stem?)
                         let mut forms = Vec::new();
                         let mut resolved_gender = crate::types::Linga::Neuter; // Default
                         // A CTE's field is keyed by its name (`शुल्कम्`), not
                         // its stem; decline the field's own stem. Which table
                         // resolve_pratipadika returned depended on HashMap
                         // order, so a declined column failed one run in two.
                         let mut stem = pratipadika.to_string();
                         for table in schema.tables.values() {
                             if let Some(col_info) = table.fields.get(pratipadika) {
                                 resolved_gender = col_info.gender;
                                 stem = col_info.pratipadika.clone();
                                 break;
                             }
                         }
                         
                         for v in [crate::types::Vachana::Eka, crate::types::Vachana::Bahu] {
                             if let Ok(table) = self.morphology.decline_with_gender(&stem, resolved_gender, v) {
                                 for (vibhakti, form_text) in table.forms {
                                     if form_text == token_text {
                                         forms.push(crate::types::VibhaktiForm {
                                             vibhakti,
                                             vachana: v,
                                             linga: Some(resolved_gender),
                                         });
                                     }
                                 }
                             }
                         }
                         
                         if forms.is_empty() && !is_stem_slot && pratipadika != token_text {
                             return Err(Self::not_a_form(&token_text, pratipadika, resolved_gender, &mut self.morphology));
                         }
                         let mut vib_token = VibhaktiToken::new(
                             token_text,
                             forms,
                             token.position,
                             0,
                         );
                         vib_token.pratipadika = Some(pratipadika.to_string());
                         enriched.push(vib_token);
                     }
                } else {
                    // A declared stem used as a word (`शीर्षक` for शीर्षकम्)
                    // would otherwise reach codegen, which finds columns by
                    // their stem.
                    if !is_stem_slot {
                        if let Some(table) = schema.tables.get(&token_text) {
                            return Err(Self::not_a_form(&token_text, &token_text, table.gender, &mut self.morphology));
                        }
                        // A declared column (it has a number); a CTE's output column
                        // is known only by its name and matches itself.
                        if let Some(col) = schema.tables.values().find_map(|t| t.fields.get(&token_text)).filter(|c| c.vachana.is_some()) {
                            return Err(Self::not_a_form(&token_text, &token_text, col.gender, &mut self.morphology));
                        }
                    }
                    // Unknown identifier (Pratipadika not found)
                    // Keep as generic Identifier
                    let vib_token = VibhaktiToken::new(
                        token.text.clone(),
                        vec![], 
                        token.position,
                        0, 
                    );
                    enriched.push(vib_token);
                }
            }

            } else {
                // Preserve non-identifier tokens
                let vib_token = VibhaktiToken::new(
                    token.text.clone(),
                    vec![], 
                    token.position,
                    0, 
                );
                enriched.push(vib_token);
            }
        }
        
        Ok(enriched)
    }
    
    /// The error for a word that names a declared stem but is none of its
    /// forms (a bare stem, a wrong case ending, a missed ṇatva …).
    fn not_a_form(word: &str, stem: &str, gender: crate::types::Linga, morphology: &mut MorphologyEngine) -> crate::error::CompilerError {
        let mut forms = Vec::new();
        for v in [Vachana::Eka, Vachana::Bahu] {
            if let Ok(table) = morphology.decline_with_gender(stem, gender, v) {
                for vib in [crate::types::Vibhakti::Prathama, crate::types::Vibhakti::Shashthi, crate::types::Vibhakti::Panchami, crate::types::Vibhakti::Saptami, crate::types::Vibhakti::Tritiya] {
                    if let Some(f) = table.forms.get(&vib) {
                        if !forms.contains(f) {
                            forms.push(f.clone());
                        }
                    }
                }
            }
        }
        crate::error::SemanticError::NotAFormOf {
            word: word.to_string(),
            stem: stem.to_string(),
            forms: forms.join(", "),
        }
        .into()
    }

    /// Get the built symbol table (for debugging/inspection)
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_annotator_basic() {
        let mut annotator = Annotator::new().unwrap();
        
        // Simulate tokens: "छात्रः इति पूर्णाङ्कः"
        let tokens = vec![
            LexerToken {
                text: "छात्रः".to_string(),
                token_type: TokenType::Identifier,
                position: 0,
            },
            LexerToken {
                text: "इति".to_string(),
                token_type: TokenType::Iti,
                position: 8,
            },
            LexerToken {
                text: "पूर्णाङ्कः".to_string(),
                token_type: TokenType::Identifier,
                position: 12,
            },
        ];
        
        // Pass 1: should add "छात्रः" to symbol table
        let mut schema = crate::schema::Schema::default();
        schema.tables.insert("छात्र".to_string(), crate::schema::TableInfo {
            sql_name: "students".to_string(),
            gender: crate::types::Linga::Masculine,
            number: Vachana::Bahu,
            pratipadika: "छात्र".to_string(),
            fields: std::collections::HashMap::new(),
            form_map: std::collections::HashMap::new(),
            column_form_map: std::collections::HashMap::new(),
            primary_key_sql_name: None,
        });
        annotator.pass1_build_symbol_table(&tokens, &schema).unwrap();
        assert!(annotator.symbol_table().contains("छात्र"));
        
        // Symbol table should have the declensions
        let entry = annotator.symbol_table().get("छात्र").unwrap();
        assert!(entry.forms.len() > 0);
    }
}

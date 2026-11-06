//! # Type System for SSQL
//!
//! This module defines the core types used in the SSQL compiler, including:
//! - Vibhakti (grammatical case) representation
//! - Vachan (grammatical number) representation  
//! - Token types with multi-vibhakti support
//! - AST node types

use std::fmt;

/// Sanskrit grammatical cases (विभक्ति)
/// 
/// Each case conveys different semantic roles in SSQL:
/// - प्रथमा: Subject, declarations, select columns
/// - द्वितीया: Direct object (rare in SQL)
/// - तृतीया: Instrumental - WHERE, JOIN, GROUP BY, ORDER BY
/// - चतुर्थी: Dative - GRANT TO
/// - पञ्चमी: Ablative - FROM, comparison "than"
/// - षष्ठी: Genitive - property access (table.field)
/// - सप्तमी: Locative - INSERT INTO, UPDATE IN
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Vibhakti {
    /// प्रथमा - Nominative (subject)
    Prathama,
    /// द्वितीया - Accusative (direct object)
    Dvitiya,
    /// तृतीया - Instrumental (by/with)
    Tritiya,
    /// चतुर्थी - Dative (to/for)
    Chaturthi,
    /// पञ्चमी - Ablative (from/than)
    Panchami,
    /// षष्ठी - Genitive ('s/of)
    Shashthi,
    /// सप्तमी - Locative (in/on)
    Saptami,
}

impl fmt::Display for Vibhakti {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Vibhakti::Prathama => "प्रथमा",
            Vibhakti::Dvitiya => "द्वितीया",
            Vibhakti::Tritiya => "तृतीया",
            Vibhakti::Chaturthi => "चतुर्थी",
            Vibhakti::Panchami => "पञ्चमी",
            Vibhakti::Shashthi => "षष्ठी",
            Vibhakti::Saptami => "सप्तमी",
        };
        write!(f, "{}", name)
    }
}

/// Sanskrit grammatical number (वचन)
/// 
/// For SSQL v1.0, we focus on singular and plural only.
/// A column is always singular; a table may be declared in the dual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Vachana {
    /// एकवचन - Singular
    Eka,
    /// द्विवचन - Dual
    Dvi,
    /// बहुवचन - Plural
    Bahu,
}

impl fmt::Display for Vachana {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Vachana::Eka => "एकवचन",
            Vachana::Dvi => "द्विवचन",
            Vachana::Bahu => "बहुवचन",
        };
        write!(f, "{}", name)
    }
}

/// Sanskrit grammatical gender (लिङ्ग)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Linga {
    /// पुंलिङ्ग - Masculine
    Masculine,
    /// स्त्रीलिङ्ग - Feminine
    Feminine,
    /// नपुंसकलिङ्ग - Neuter
    Neuter,
}

/// Combined vibhakti + vachan identifier
/// 
/// This is the "enriched" token type that the annotator produces.
/// Example: "छात्रः" (student, nom. sing.) → VibhaktiForm(Prathama, Eka)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VibhaktiForm {
    pub vibhakti: Vibhakti,
    pub vachana: Vachana,
    pub linga: Option<Linga>,
}

impl VibhaktiForm {
    pub fn new(vibhakti: Vibhakti, vachana: Vachana) -> Self {
        Self { 
            vibhakti, 
            vachana,
            linga: None, 
        }
    }
    
    pub fn with_gender(vibhakti: Vibhakti, vachana: Vachana, linga: Linga) -> Self {
        Self { 
            vibhakti, 
            vachana,
            linga: Some(linga), 
        }
    }
}

impl fmt::Display for VibhaktiForm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.linga {
            Some(l) => write!(f, "{}-{}-{:?}", self.vibhakti, self.vachana, l),
            None => write!(f, "{}-{}", self.vibhakti, self.vachana),
        }
    }
}

/// A token with potentially multiple vibhakti interpretations
///
/// This is the core innovation: unlike typical compilers where each token
/// has exactly one type, SSQL tokens can have multiple possible types
/// (e.g., छात्रः could be प्रथमा-एकवचन OR पञ्चमी-बहुवचन in some contexts).
/// The parser uses grammar context to select the correct interpretation.
#[derive(Debug, Clone)]
pub struct VibhaktiToken {
    /// The actual text from source code (e.g., "छात्रः")
    pub text: String,
    
    /// Possible vibhakti interpretations
    /// If only one possibility, the token is unambiguous
    /// If multiple, parser must use context to disambiguate
    pub possible_forms: Vec<VibhaktiForm>,
    
    /// Source location for error reporting
    pub line: usize,
    pub column: usize,
    
    /// The प्रथमा-एकवचन base form (canonical identifier)
    /// Example: "छात्रेण" → pratipadika = "छात्रः"
    pub pratipadika: Option<String>,
}

impl VibhaktiToken {
    /// Create a new token with multiple possible forms
    pub fn new(text: String, forms: Vec<VibhaktiForm>, line: usize, column: usize) -> Self {
        Self {
            text,
            possible_forms: forms,
            line,
            column,
            pratipadika: None,
        }
    }
    
    /// Create an unambiguous token with a single form
    pub fn unambiguous(
        text: String,
        vibhakti: Vibhakti,
        vachana: Vachana,
        line: usize,
        column: usize,
    ) -> Self {
        Self::new(
            text,
            vec![VibhaktiForm::new(vibhakti, vachana)],
            line,
            column,
        )
    }
    
    /// Check if token has exactly one possible form
    pub fn is_unambiguous(&self) -> bool {
        self.possible_forms.len() == 1
    }
    
    /// Check if token could be a specific vibhakti+vachan
    pub fn could_be(&self, vibhakti: Vibhakti, vachana: Vachana) -> bool {
        self.possible_forms.iter().any(|f| {
            f.vibhakti == vibhakti && f.vachana == vachana
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vibhakti_display() {
        assert_eq!(Vibhakti::Prathama.to_string(), "प्रथमा");
        assert_eq!(Vibhakti::Shashthi.to_string(), "षष्ठी");
    }
    
    #[test]
    fn test_vibhakti_token_ambiguity() {
        let token = VibhaktiToken::new(
            "छात्रः".to_string(),
            vec![
                VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
            ],
            1,
            0,
        );
        
        assert!(!token.is_unambiguous());
        assert!(token.could_be(Vibhakti::Prathama, Vachana::Eka));
        assert!(token.could_be(Vibhakti::Panchami, Vachana::Bahu));
        assert!(!token.could_be(Vibhakti::Shashthi, Vachana::Eka));
    }
}

/// Semantic Classification of an Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SymbolType {
    /// Table or View or CTE
    Table,
    /// Column or Field
    Column,
    /// Variable or Parameter
    Variable,
    /// Function Name
    Function,
    /// Savepoint name (for transactions)
    Savepoint,
    /// Unknown or Ambiguous
    Unknown,
}

/// Sanskrit inflected word form (पदम्)
pub type Padam = String;


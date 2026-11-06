//! Shared list of builtin function names (Sanskrit synonyms) that cannot be
//! used as a table, column, view, or CTE stem, or as a query-level alias.
//!
//! Keep this in sync with `ast_codegen.rs`'s `get_builtin_function` match
//! arms - this is the canonical set of *words*, not the dialect-aware SQL
//! mapping (which stays in ast_codegen.rs since several entries are
//! conditional on the target dialect).
//!
//! Grammar keywords (सह, यत्र, मेलनेन, etc.) are not listed here: the lexer
//! already tokenizes them as their own dedicated `Token` variants rather than
//! generic identifiers, so they can never reach a stem/alias position in the
//! first place. Builtin function names have no such structural protection -
//! they are ordinary identifiers to the lexer - so they need this explicit
//! list; बाह्य and विभाग are two examples where a schema author's ordinary
//! noun collided with a builtin.

pub const BUILTIN_FUNCTION_WORDS: &[&str] = &[
    "निरपेक्षमानम्",
    "सन्निकटनम्",
    "ऊर्ध्वपूर्णाङ्कः",
    "अधःपूर्णाङ्कः",
    "वर्गमूलम्",
    "धनर्णचिह्नम्",
    "ऋणात्मक",
    "ज्या",
    "कोज्या",
    "स्पर्शज्या",
    "कोटिस्पर्शज्या",
    "व्युत्क्रमज्या",
    "व्युत्क्रमकोज्या",
    "चापज्या",
    "चापकोज्या",
    "चापस्पर्शज्या",
    "लघुगणकः",
    "प्राकृतिकलघुगणकः",
    "घाताङ्कः",
    "घातः",
    "कोणांशः",
    "चापमानम्",
    "पूर्णलब्धिः",
    "शेषः",
    "यादृच्छिकः",
    "संयोजनम्",
    "खण्डः",
    "दैर्घ्यम्",
    "महावर्णरूपम्",
    "लघुवर्णरूपम्",
    "विकल्पः",
    "अनावृत्तम्",
    "आद्यमहावर्णरूपम्",
    "मार्जनम्",
    "वाममार्जनम्",
    "दक्षिणमार्जनम्",
    "प्रतिस्थापनम्",
    "स्थानम्",
    "विभाजनखण्डः",
    "पुनरावृत्तिः",
    "विपरीतम्",
    "आदिखण्डः",
    "अन्त्यखण्डः",
    "अद्यतनक्षणः",
    "अद्यतनदिनाङ्कः",
    "अद्यतनसमयः",
    "दिनाङ्कः",
    "समयः",
    "दिनाङ्कसमयः",
    "युगसमयः",
    "प्ररूपितकालः",
    "अन्तरम्",
    "वर्षम्",
    "मासः",
    "दिनम्",
    "योगः",
    "गणना",
    "माध्यम्",
    "महत्तमम्",
    "लघुतमम्",
];

/// Stems a schema may not declare, because they decline into a function name
/// or a keyword (`वर्गमूल` → `वर्गमूलम्`, `वर्ण` → `वर्णानाम्`). A bare stem is
/// not itself a word in a query (expert review P1b).
pub const RESERVED_STEMS: &[&str] = &[
    "ऋणात्मिका",
    "वर्ण",
    "निरपेक्ष",
    "वृत्त",
    "ऊर्ध्व",
    "वर्गमूल",
    "चिह्न",
    "सन्निकटन",
    "ऊर्ध्वपूर्णाङ्क",
    "अधःपूर्णाङ्क",
    "धनर्णचिह्न",
    "लघुगणक",
    "प्राकृतिकलघुगणक",
    "घाताङ्क",
    "घात",
    "कोणांश",
    "चापमान",
    "विभाज्य",
    "पूर्णलब्धि",
    "अवशिष्ट",
    "शेष",
    "यादृच्छिक",
    "संयोजन",
    "संयोग",
    "खण्ड",
    "आयाम",
    "दैर्घ्य",
    "दीर्घरूप",
    "लघुरूप",
    "महावर्णरूप",
    "लघुवर्णरूप",
    "आद्यमहावर्णरूप",
    "विकल्प",
    "अनावृत्त",
    "आद्यक्षरदीर्घ",
    "मार्जन",
    "वाममार्जन",
    "दक्षिणमार्जन",
    "प्रतिस्थापन",
    "स्थान",
    "विभाजनखण्ड",
    "विपरीत",
    "वामखण्ड",
    "दक्षिणखण्ड",
    "अद्यतनदिनाङ्क",
    "अद्यतनसमय",
    "दिनाङ्क",
    "समय",
    "दिनाङ्कसमय",
    "युगसमय",
    "प्ररूपितकाल",
    "अन्तर",
    "वर्ष",
    "मास",
    "दिन",
    "योग",
    "महत्तम",
    "लघुतम",
    "पुनरावृत्ति",
];

/// True if `word` is a reserved builtin function name (or a reserved stem)
/// and therefore cannot be used as a stem/alias.
pub fn is_reserved_builtin(word: &str) -> bool {
    BUILTIN_FUNCTION_WORDS.contains(&word) || RESERVED_STEMS.contains(&word)
}

//! # Lexer Module
//!
//! Grapheme-aware Devanagari lexer for SSQL source code (`unicode-segmentation`).
//! Enforces Devanagari numerals (`०-९`) and tokenizes native SSQL keywords,
//! identifiers, literals, and punctuation (`।`).

use crate::annotator::{LexerToken, TokenType};
use anyhow::Result;
use unicode_segmentation::UnicodeSegmentation;

/// Invisible formatting characters that editors and copy-paste insert into
/// Devanagari text (zero-width space/non-joiner/joiner, word joiner, byte
/// order mark). They change at most how a conjunct is drawn, never which
/// word it is, so outside string literals they are dropped before lexing.
fn is_invisible_format_char(c: char) -> bool {
    matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}')
}

/// The source without invisible formatting characters (outside string
/// literals), plus, for every byte of the result, its offset in the
/// original source so token positions still point into the user's text.
fn strip_invisible_format_chars(source: &str) -> (String, Vec<usize>) {
    let mut cleaned = String::with_capacity(source.len());
    let mut origin = Vec::with_capacity(source.len() + 1);
    let mut in_string = false;
    let mut escaped = false;
    for (offset, c) in source.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if is_invisible_format_char(c) {
            continue;
        }
        cleaned.push(c);
        origin.extend(std::iter::repeat(offset).take(c.len_utf8()));
    }
    origin.push(source.len());
    (cleaned, origin)
}

/// Tokenize SSQL source code
pub fn tokenize(source: &str) -> Result<Vec<LexerToken>> {
    let (cleaned, origin) = strip_invisible_format_chars(source);
    let mut tokens = tokenize_visible(&cleaned)?;
    for token in &mut tokens {
        token.position = origin[token.position];
    }
    Ok(tokens)
}

fn tokenize_visible(source: &str) -> Result<Vec<LexerToken>> {
    let mut tokens = Vec::new();
    let mut current_pos = 0;
    
    let graphemes: Vec<&str> = source.graphemes(true).collect();
    let mut i = 0;
    
    while i < graphemes.len() {
        let ch = graphemes[i];
        
        // Skip whitespace
        if ch.trim().is_empty() {
            i += 1;
            current_pos += ch.len();
            continue;
        }
        
        // Skip block comments /* ... */
        if ch == "/" && i + 1 < graphemes.len() && graphemes[i + 1] == "*" {
            current_pos += graphemes[i].len() + graphemes[i + 1].len();
            i += 2;
            while i < graphemes.len() {
                if graphemes[i] == "*" && i + 1 < graphemes.len() && graphemes[i + 1] == "/" {
                    current_pos += graphemes[i].len() + graphemes[i + 1].len();
                    i += 2;
                    break;
                }
                current_pos += graphemes[i].len();
                i += 1;
            }
            continue;
        }

        // Skip single-line comments: `//` and `--` (Constitution §4)
        let line_comment = (ch == "/" && i + 1 < graphemes.len() && graphemes[i + 1] == "/")
            || (ch == "-" && i + 1 < graphemes.len() && graphemes[i + 1] == "-");
        if line_comment {
            // Skip until end of line
            while i < graphemes.len() && graphemes[i] != "\n" {
                current_pos += graphemes[i].len();
                i += 1;
            }
            continue;
        }
        
        // Parentheses are not Sanskrit punctuation: a group is `यत् … इति`,
        // and SSQL accepts it only where it changes the reading (release
        // policy; expert review Q7).
        if ch == "(" || ch == ")" {
            anyhow::bail!(
                "`{}` at byte offset {} is not SSQL: parentheses are not used; group with `यत् … इति` where the grouping changes the meaning",
                ch, current_pos
            );
        }

        // Punctuation
        if ch == "(" {
            tokens.push(LexerToken {
                token_type: TokenType::LParen,
                text: ch.to_string(),
                position: current_pos,
            });
            i += 1;
            current_pos += ch.len();
            continue;
        }
        
        if ch == ")" {
            tokens.push(LexerToken {
                token_type: TokenType::RParen,
                text: ch.to_string(),
                position: current_pos,
            });
            i += 1;
            current_pos += ch.len();
            continue;
        }
        
        if ch == "।" {
            tokens.push(LexerToken {
                token_type: TokenType::Purnadanda,
                text: ch.to_string(),
                position: current_pos,
            });
            i += 1;
            current_pos += ch.len();
            continue;
        }

        if ch == "?" {
            tokens.push(LexerToken {
                token_type: TokenType::Identifier,
                text: ch.to_string(),
                position: current_pos,
            });
            i += 1;
            current_pos += ch.len();
            continue;
        }
        
        // Strings: `"…"`, where `\"` is a literal quote and `\\` a literal
        // backslash (any other backslash sequence is kept as written).
        if ch == "\"" {
            let start = i;
            let start_pos = current_pos;
            i += 1;
            current_pos += ch.len();

            let mut content = String::new();
            let mut closed = false;
            while i < graphemes.len() {
                let g = graphemes[i];
                if g == "\\" && i + 1 < graphemes.len() && (graphemes[i + 1] == "\"" || graphemes[i + 1] == "\\") {
                    content.push_str(graphemes[i + 1]);
                    current_pos += g.len() + graphemes[i + 1].len();
                    i += 2;
                    continue;
                }
                current_pos += g.len();
                i += 1;
                if g == "\"" {
                    closed = true;
                    break;
                }
                content.push_str(g);
            }
            if !closed {
                anyhow::bail!(
                    "Unterminated string literal starting at byte offset {}: close it with '\"'",
                    start_pos
                );
            }

            let text_with_quotes: String = graphemes[start..i].iter().copied().collect();
            tokens.push(LexerToken {
                token_type: TokenType::String(content),
                text: text_with_quotes,
                position: start_pos,
            });
            continue;
        }
        
        // A negative constant carries its sign against its digits (`-५`); after
        // a word or a digit it would be a subtraction, which is written out.
        let is_digit = |g: &str| g.chars().all(|c| c.is_ascii_digit() || ('०'..='९').contains(&c));
        let signed = ch == "-"
            && graphemes.get(i + 1).map_or(false, |g| is_digit(g))
            && (i == 0 || graphemes[i - 1].chars().all(char::is_whitespace));

        // Numbers (Devanagari numerals ०-९ required; reject unquoted ASCII digits)
        if signed || is_digit(ch) {
            let start = i;
            let start_pos = current_pos;
            if signed {
                current_pos += ch.len();
                i += 1;
            }

            while i < graphemes.len() {
                let g = graphemes[i];
                if g.chars().all(|c| c.is_ascii_digit() || ('०'..='९').contains(&c) || c == '.') {
                    current_pos += g.len();
                    i += 1;
                } else {
                    break;
                }
            }
            
            let text: String = graphemes[start..i].iter().copied().collect();
            if text.chars().any(|c| c.is_ascii_digit()) {
                let suggested: String = text
                    .chars()
                    .map(|c| match c {
                        '0'..='9' => char::from_u32(0x0966 + (c as u32 - '0' as u32)).unwrap_or(c),
                        _ => c,
                    })
                    .collect();
                anyhow::bail!(
                    "ASCII numeral '{}' at byte offset {} is not allowed in SSQL; use Devanagari numerals (०-९) instead (e.g., '{}')",
                    text,
                    start_pos,
                    suggested
                );
            }
            tokens.push(LexerToken {
                token_type: TokenType::Number(text.clone()),
                text,
                position: start_pos,
            });
            continue;
        }
        
        // Devanagari words (identifiers or keywords)
        if is_devanagari(ch) {
            let start = i;
            let start_pos = current_pos;
            
            while i < graphemes.len() {
                let g = graphemes[i];
                // Stop at non-Devanagari/digits OR Danda
                if (!is_devanagari(g) && !g.chars().all(|c| c.is_ascii_digit() || c == '_')) || g == "।" || g == "॥" {
                    break;
                }
                current_pos += g.len();
                i += 1;
            }
            
            let text: String = graphemes[start..i].iter().copied().collect();
            if let Some(retired) = crate::lexicon::retired_keyword_in(&text) {
                anyhow::bail!(
                    "`{}` at byte offset {} is not Sanskrit: `{}` is {}. Use `{}`.",
                    text, start_pos, retired.word, retired.why, retired.replacement
                );
            }
            if let Some((_, why, use_instead)) = REMOVED_FORMS.iter().find(|(w, _, _)| *w == text) {
                anyhow::bail!(
                    "`{}` at byte offset {} is no longer SSQL: {}. Use `{}`.",
                    text, start_pos, why, use_instead
                );
            }
            // The spelling standard (lexicon.rs): `म्` not a final anusvāra,
            // class nasals before stops, no chandrabindu, nukta or `_`, no
            // denied word. String literals are data and are not checked.
            if let Some(finding) = crate::lexicon::check_word(&text).into_iter().next() {
                anyhow::bail!("`{}` at byte offset {}: {}", text, start_pos, finding.message);
            }
            if text.ends_with("पर्यन्तम्") && text != "पर्यन्तम्" {
                let base_len = text.len() - "पर्यन्तम्".len();
                let base = text[..base_len].to_string();
                let base_token_type = classify_token(&base);
                tokens.push(LexerToken {
                    token_type: base_token_type,
                    text: base,
                    position: start_pos,
                });
                tokens.push(LexerToken {
                    token_type: TokenType::Paryantam,
                    text: "पर्यन्तम्".to_string(),
                    position: start_pos + base_len,
                });
            } else {
                let token_type = classify_token(&text);
                tokens.push(LexerToken {
                    token_type,
                    text,
                    position: start_pos,
                });
            }
            continue;
        }
        
        if ch == "-" {
            anyhow::bail!(
                "Unexpected '-' at byte offset {}: `-` marks only a negative constant, against its digits (`-५`); a negative column is `ऋणात्मकम् मूल्यम्`, and a subtraction `मूल्यात् ५ विशोध्य अवशिष्टम्`",
                current_pos
            );
        }

        if ch == "," {
            anyhow::bail!(
                "Unexpected comma ',' at byte offset {}: commas are not valid in SSQL; use distributive 'च' to conjoin items",
                current_pos
            );
        }

        if ch == ";" {
            anyhow::bail!(
                "Unexpected semicolon ';' at byte offset {}: semicolons are not valid in SSQL; terminate statements with pūrṇavirāma '।'",
                current_pos
            );
        }

        anyhow::bail!(
            "Unexpected character '{}' at byte offset {}: only Devanagari script, '।', and quoted string literals are valid in SSQL",
            ch,
            current_pos
        );
    }
    
    Ok(tokens)
}

/// Forms SSQL once accepted alongside the canonical one. Each construct has
/// a single form (a verb in its three numbers), so these are rejected with
/// the form to use: (removed form, why, canonical form).
const REMOVED_FORMS: &[(&str, &str, &str)] = &[
    ("आहर", "it is the active imperative 'bring!', and SSQL verbs are passive", "आह्रियताम्"),
    ("चियताम्", "it was a second SELECT verb (and the passive of चि is चीयताम्)", "आह्रियताम्"),
    ("वर्तमानपङ्क्ति", "it is a bare stem, not a word (it stands only before पर्यन्तम्, as a frame's end)", "वर्तमानपङ्क्तिः"),
    ("चियेताम्", "it was a second SELECT verb (and the passive of चि is चीयेताम्)", "आह्रियेताम्"),
    ("चियन्ताम्", "it was a second SELECT verb (and the passive of चि is चीयन्ताम्)", "आह्रियन्ताम्"),
    ("नवीक्रियताम्", "it was a second UPDATE verb", "परिवर्त्यताम्"),
    // A bare stem is not a word in a sentence (expert review O1, P1b).
    ("पुंलिङ्ग", "a bare stem is not a word in a sentence (1.4.14)", "पुंलिङ्गम्"),
    ("स्त्रीलिङ्ग", "a bare stem is not a word in a sentence (1.4.14)", "स्त्रीलिङ्गम्"),
    ("नपुंसकलिङ्ग", "a bare stem is not a word in a sentence (1.4.14)", "नपुंसकलिङ्गम्"),
    ("एकवचन", "a bare stem is not a word in a sentence (1.4.14)", "एकवचनम्"),
    ("द्विवचन", "a bare stem is not a word in a sentence (1.4.14)", "द्विवचनम्"),
    ("ऋणात्मक", "a bare stem is not a word in a sentence (1.4.14); it agrees with its column", "ऋणात्मकम्"),
    ("बहुवचन", "a bare stem is not a word in a sentence (1.4.14)", "बहुवचनम्"),
    ("पटल", "a bare stem is not a word in a sentence (1.4.14)", "पटलम्"),
    ("तात्कालिक", "a bare stem is not a word in a sentence (1.4.14)", "तात्कालिकम्"),
    ("गुणन", "a bare stem is not a word in a sentence (1.4.14)", "गुणनम्"),
    ("पूर्ववर्तित", "a bare stem is not a word in a sentence (1.4.14)", "पूर्ववर्तितः"),
    ("अनुवर्तित", "a bare stem is not a word in a sentence (1.4.14)", "अनुवर्तितः"),
    ("बाह्य", "the adjective agrees with सञ्ज्ञा, which is feminine", "बाह्या सञ्ज्ञा"),
    ("अनिवार्य", "a constraint adjective agrees with its column", "अनिवार्यः / अनिवार्या / अनिवार्यम्"),
    ("वैकल्पिक", "a constraint adjective agrees with its column", "वैकल्पिकः / वैकल्पिकी / वैकल्पिकम्"),
    ("प्राथमिक", "a constraint adjective agrees with its column", "प्राथमिकः / प्राथमिकी / प्राथमिकम्"),
    ("क्रमिक", "a constraint adjective agrees with its column", "क्रमिकः / क्रमिका / क्रमिकम्"),
    ("अद्वितीय", "a constraint adjective agrees with its column", "अद्वितीयः / अद्वितीया / अद्वितीयम्"),
    ("बद्ध", "a constraint adjective agrees with its column", "बद्धः / बद्धा / बद्धम्"),
    // SELECT DISTINCT is about whole rows, which no adjective on the columns
    // can say (expert review N1); inside an aggregate the genitive forms
    // (`विशिष्टायाः भाषायाः गणना`) stay.
    ("विशिष्टम्", "SELECT DISTINCT removes repeated rows; it is written before the select list", "पुनरुक्तिम् विना"),
    ("विशिष्टः", "SELECT DISTINCT removes repeated rows; it is written before the select list", "पुनरुक्तिम् विना"),
    ("विशिष्टा", "SELECT DISTINCT removes repeated rows; it is written before the select list", "पुनरुक्तिम् विना"),
    ("विशिष्ट", "SELECT DISTINCT removes repeated rows; it is written before the select list", "पुनरुक्तिम् विना"),
    ("क्रिया", "a function is not bound to a SQL name; each built-in is called by its own name", "शीर्षकस्य महावर्णरूपम्"),
    ("भिन्नम्", "≠ is the negated equality, like every other negation", "इत्यनेन समम् नास्ति"),
    ("भिन्नः", "≠ is the negated equality, like every other negation", "इत्यनेन समः नास्ति"),
    ("भिन्ना", "≠ is the negated equality, like every other negation", "इत्यनया समा नास्ति"),
    ("अन्तर्भवति", "it was a second form of IN", "<a> च <b> च … इत्येषु अस्ति"),
    ("पश्चात्", "LAG names the previous row", "पूर्वपङ्क्तेः <column> यत्र …"),
    ("अग्रे", "LEAD names the next row", "अनन्तरपङ्क्तेः <column> यत्र …"),
    ("उत्तमचतुर्थांशः", "NTILE(4) is the quartile number", "चतुर्थांशाङ्कः यत्र …"),
    // The pronouns are all from इदम् (expert review Q2): अयोः is no form
    // at all (7.2.112, 7.3.104 give अनयोः), अया is Vedic, एतयोः/एतासाम् are
    // एतद्, and a final anusvāra is never written in SSQL.
    ("इत्ययोः", "the dual of इदम् is अनयोः", "इत्यनयोः"),
    ("इत्येतयोः", "SSQL's pronouns are the इदम् forms", "इत्यनयोः"),
    ("इत्यया", "अया is Vedic; the classical instrumental is अनया", "इत्यनया"),
    ("इत्येतासाम्", "SSQL's pronouns are the इदम् forms", "इत्यासाम्"),
    ("इत्येषां", "a final anusvāra is written म्", "इत्येषाम्"), // lexicon-allow
    // One verb per statement kind (expert review Q4).
    ("विलोप्यताम्", "it was a second DELETE verb", "निष्कास्यताम्"),
    ("नाश्यताम्", "it was a second DROP verb", "लुप्यताम्"),
    ("नाशयतु", "it is active voice, and SSQL verbs are passive", "लुप्यताम्"),
    ("निर्मीयताम्", "it was a second CREATE verb", "रच्यताम्"),
    ("विधीयताम्", "it was a second CREATE verb", "रच्यताम्"),
    ("सर्जयतु", "it is active voice, and SSQL verbs are passive", "रच्यताम्"),
    ("निरूप्यताम्", "a function is created like any other object", "रच्यताम्"),
    ("दृढीक्रियताम्", "it was a second COMMIT verb", "स्थिरीक्रियताम्"),
    ("प्रत्यावर्त्यताम्", "it was a second ROLLBACK verb", "प्रत्याह्रियताम्"),
    ("प्रतिषधतु", "it is misspelt (प्रतिषेधतु), active, and means DENY", "प्रतिसंह्रियताम्"),
    ("प्रतिददाति", "the return type is stated in the passive", "प्रतिदीयते"),
    ("प्रतिददातु", "the result is returned in the passive", "प्रतिदीयताम्"),
    ("आदाय", "a function's parameters are given in the passive, agreeing in number", "दीयते / दीयेते / दीयन्ते"),
    ("गृहीत्वा", "a function's parameters are given in the passive, agreeing in number", "दीयते / दीयेते / दीयन्ते"),
    ("स्वीकृत्य", "a function's parameters are given in the passive, agreeing in number", "दीयते / दीयेते / दीयन्ते"),
    ("निवार्य", "निवार्य means 'having warded off'; subtraction is शोधन", "विशोध्य"),
    ("शोधयित्वा", "it was a second subtraction form", "विशोध्य"),
    ("अपनीय", "it was a second subtraction form", "विशोध्य"),
    ("अपुनरुक्तस्य", "it was a second DISTINCT form", "विशिष्टस्य"),
    ("अपुनरुक्तायाः", "it was a second DISTINCT form", "विशिष्टायाः"),
    ("अपुनरुक्तम्", "it was a second DISTINCT form", "पुनरुक्तिम् विना"),
    // Keywords are spelt with the class nasal (approved spelling rule).
    ("संज्ञा", "a nasal before a palatal is written ञ्", "सञ्ज्ञा"), // lexicon-allow
    ("संगमः", "a nasal before a velar is written ङ्", "सङ्गमः"), // lexicon-allow
    ("संकुलम्", "SSQL has no modules to import (and a nasal before a velar is written ङ्)", "the statement without an import"), // lexicon-allow
    ("सङ्कुलम्", "SSQL has no modules to import; the import was parsed and ignored", "the statement without an import"),
    ("पङ्कतीः", "it is misspelt", "पङ्क्तयः"),
    ("परिवर्तनेन", "the new value is marked by इति alone", "<column> <value> इति परिवर्त्यताम्"),
    ("सम्मिलनम्", "INTERSECT is the crossing of the results", "प्रतिच्छेदः"),
    ("परिवर्तनम्", "a cast is one thing taking another form", "<expr-genitive> <type>रूपेण परिणामः"),
    ("वाममेलनम्", "the join's kind is its marker", "<condition> इति वाममेलनेन"),
    ("दक्षिणमेलनम्", "the join's kind is its marker", "<condition> इति दक्षिणमेलनेन"),
    ("पूर्णमेलनम्", "the join's kind is its marker", "<condition> इति पूर्णमेलनेन"),
    ("सर्वमेलनम्", "a cross join has no condition", "<table> सह सर्वमेलनेन"),
];

fn is_devanagari(s: &str) -> bool {
    s.chars().all(|c| ('\u{0900}'..='\u{097F}').contains(&c))
}

fn classify_token(text: &str) -> TokenType {
    match text {
        "इति" => TokenType::Iti,
        "चेत्" => TokenType::Chet,
        "आश्रित्य" => TokenType::Ashritya,
        "च" => TokenType::Cha,
        "वा" => TokenType::Va,
        "सङ्गमः" => TokenType::Sangamah,
        "सर्वसङ्गमः" => TokenType::SarvaSangamah,
        "सदृशम्" | "सदृशः" | "सदृशी" => TokenType::Sadrusham,
        "आह्रियताम्" => TokenType::FetchVerb,
        "निवेश्यताम्" | "निवेश्येताम्" | "निवेश्यन्ताम्" => TokenType::InsertVerb,
        "बाह्या" => TokenType::Bahya,
        "सञ्ज्ञा" => TokenType::Sanjna,
        "आख्या" => TokenType::Aakhya,
        
        // DDL Keywords
        "पटलम्" => TokenType::Patalam,
        "प्रति" => TokenType::Prati,
        "योज्यताम्" => TokenType::AddVerb,
        "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्" => TokenType::ChangeVerb,
        "अभिधीयताम्" => TokenType::AbhidhiyatamVerb,
        // Column-constraint adjectives, in every gender and number; which
        // form is right is checked against the column (expert review O1).
        // The common wrong feminines are recognised so they get that
        // message rather than a parse error.
        "अनिवार्यः" | "अनिवार्या" | "अनिवार्यम्" | "अनिवार्याः" | "अनिवार्याणि" => TokenType::Anivaryah,
        "वैकल्पिकः" | "वैकल्पिकी" | "वैकल्पिकम्" | "वैकल्पिकाः" | "वैकल्पिक्यः" | "वैकल्पिकानि" | "वैकल्पिका" => TokenType::Vaikalpikah,
        "प्राथमिकः" | "प्राथमिकी" | "प्राथमिकम्" | "प्राथमिकाः" | "प्राथमिक्यः" | "प्राथमिकानि" | "प्राथमिका" => TokenType::Prathamikah,
        "क्रमिकः" | "क्रमिका" | "क्रमिकम्" | "क्रमिकाः" | "क्रमिकाणि" | "क्रमिकी" => TokenType::Kramikah,
        "अद्वितीयः" | "अद्वितीया" | "अद्वितीयम्" | "अद्वितीयाः" | "अद्वितीयानि" => TokenType::Advitiyah,
        "पुंलिङ्गम्" => TokenType::Pumlingam,
        "स्त्रीलिङ्गम्" => TokenType::Strilingam,
        "नपुंसकलिङ्गम्" => TokenType::Napumsakalingam,
        "एकवचनम्" => TokenType::Ekavachanam,
        "द्विवचनम्" => TokenType::Dvivachanam,
        "बहुवचनम्" => TokenType::Bahuvachanam,
        "बद्धः" | "बद्धा" | "बद्धम्" | "बद्धाः" | "बद्धानि" => TokenType::Baddhah,
        "यत्र" => TokenType::Yatra,
        "विभाजनम्" => TokenType::Vibhajanam,
        "तथा" => TokenType::Tatha,
        "श्रेणी" => TokenType::Shreni,
        "पङ्क्तिसङ्ख्या" => TokenType::PanktiSankhya,
        "सङ्ग्रहबिन्दुम्" => TokenType::SangrahabindumDvitiya,
        "विना" => TokenType::Vina,
        "पुनरुक्तिम्" => TokenType::Punaruktim,
        "परिणामः" => TokenType::Parinamah,
        "आधारे" => TokenType::Aadhare,
        "लघुतमस्य" => TokenType::LaghutamasyaGen,
        "महत्तमस्य" => TokenType::MahattamasyaGen,
        "माध्यस्य" => TokenType::MadhyasyaGen,
        "गणनायाः" => TokenType::GananayahGen,
        "योगस्य" => TokenType::YogasyaGen,
        "पङ्क्तीनाम्" => TokenType::Panktinam,
        "सर्वत्र" => TokenType::Sarvatra,
        "भागाङ्कः" => TokenType::Bhagankah,
        "चतुर्थांशाङ्कः" => TokenType::Chaturthamshankah,
        "अनन्तरपङ्क्तेः" => TokenType::AnantaraPankteh,
        "पूर्वपङ्क्तेः" => TokenType::PurvaPankteh,
        "सघनश्रेणी" => TokenType::SaghanaShreni,
        "आरोहः" => TokenType::Arohah,
        "अवरोहः" => TokenType::Avarohah,
        "आधारेण" => TokenType::Aadharena,
        "अनुसारम्" => TokenType::Anusaram,
        "आरोहक्रमेण" => TokenType::ArohaKramena,
        "अवरोहक्रमेण" => TokenType::AvarohaKramena,
        "सूची" | "सूचीम्" | "सूच्याः" => TokenType::Suchi,

        "नियमेन" => TokenType::Niyamena,
        "समूहेन" => TokenType::Samuhena,
        "समूहनियमेन" => TokenType::Samuhaniyamena,
        "क्रमेण" => TokenType::Kramena,
        "मेलनेन" => TokenType::Melanena,
        "वाममेलनेन" => TokenType::VamaMelanena,
        "दक्षिणमेलनेन" => TokenType::DakshinaMelanena,
        "पूर्णमेलनेन" => TokenType::PurnaMelanena,
        "सर्वमेलनेन" => TokenType::SarvaMelanena,
        "मित्या" => TokenType::Mitya,
    
        // Aggregates & Utils
        "इत्यस्य" => TokenType::Ityasya,
        "इत्यस्याः" => TokenType::Ityasyah,
        "इत्यस्मात्" => TokenType::Ityasmat,
        "तस्मात्" => TokenType::Tasmat,
        "इत्यनयोः" => TokenType::Ityayoh,
        "इत्येषाम्" => TokenType::Ityesham,
        "इत्यासाम्" => TokenType::Ityetasam,
        "योगः" => TokenType::Yogah,
        "माध्यम्" => TokenType::Madhyam,
        "महत्तमम्" => TokenType::Mahattam,
        "लघुतमम्" => TokenType::Laghutamam,
        "अन्यत्" => TokenType::Anyat,
        "अपि" => TokenType::Api,

        // Window Frame Specification
        "पङ्क्तयः" | "पङ्क्तीः" => TokenType::Panktayah,
        "व्याप्तिः" => TokenType::Vyaptih,
        "समूहाः" => TokenType::Samuhah,
        "अपरिमितपूर्ववर्तितः" => TokenType::AparimitaPurvavartitah,
        "पूर्ववर्तितः" => TokenType::Purvavartitah,
        "वर्तमानपङ्क्तिः" | "वर्तमानपङ्क्तिम्" | "वर्तमानपङ्क्ति" => TokenType::Vartamanapanktih,
        "अनुवर्तितः" => TokenType::Anuvartitah,
        "अपरिमितानुवर्तितः" => TokenType::AparimitaAnuvartitah,
        "आरभ्य" => TokenType::Aarabhya,
        "पर्यन्तम्" => TokenType::Paryantam,
        "वर्जयित्वा" => TokenType::Varjayitva,
        "समूहम्" => TokenType::Samuham,
        "सम्बद्धान्" => TokenType::Sambaddhan,
        "अन्यान्" => TokenType::Anyan,
        
        // TCL Keywords
        "प्रारभ्यताम्" => TokenType::BeginVerb,
        "स्थिरीक्रियताम्" => TokenType::CommitVerb,
        "प्रत्याह्रियताम्" => TokenType::RollbackVerb,
        "सङ्ग्रहबिन्दुः" => TokenType::Sangrahabinduh,
        
        _ => TokenType::Identifier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_unquoted_ascii_numerals_with_devanagari_hint() {
        let err = tokenize("पुस्तकेभ्यः 100 आह्रियन्ताम् ।").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ASCII numeral '100'"), "Unexpected error: {}", msg);
        assert!(msg.contains("'१००'"), "Expected Devanagari numeral suggestion '१००' in error: {}", msg);
    }

    #[test]
    fn test_accepts_devanagari_numerals_and_ascii_inside_strings() {
        let tokens = tokenize("पुस्तकेभ्यः १०० \"2023-10-01\" आह्रियन्ताम् ।").unwrap();
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Number("१००".to_string()) && t.text == "१००"));
        assert!(tokens.iter().any(|t| t.token_type == TokenType::String("2023-10-01".to_string()) && t.text == "\"2023-10-01\""));
    }

    #[test]
    fn test_rejects_comma_and_invalid_characters_at_lexer() {
        let err = tokenize("पुस्तकेभ्यः विवरणे \"ERR\" इत्यस्य स्थानम्, विक्रयाधिकारिणः नाम्नि १ स्थानात् आरभ्य ३ वर्णानाम् खण्डः आह्रियेताम् ।").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Unexpected comma ','"),
            "Expected lexer error on comma, got: {}",
            msg
        );

        let err_semi = tokenize("पुस्तकेभ्यः शीर्षकम् आह्रियताम् ;").unwrap_err();
        assert!(err_semi.to_string().contains("Unexpected semicolon ';'"));
    }

    #[test]
    fn invisible_format_chars_are_dropped_outside_strings_only() {
        // BOM before the first word, ZWJ inside a word, ZWNJ after the danda.
        let src = "\u{FEFF}पुस्तके\u{200D}भ्यः \"a\u{200C}b\" ।\u{200C}";
        let tokens = tokenize(src).unwrap();
        assert_eq!(tokens[0].text, "पुस्तकेभ्यः");
        // Positions are offsets into the original source (after the 3-byte BOM).
        assert_eq!(tokens[0].position, 3);
        let string_token = tokens.iter().find(|t| matches!(t.token_type, TokenType::String(_))).unwrap();
        assert!(string_token.text.contains('\u{200C}'), "string literals keep their characters");
        assert_eq!(&src[string_token.position..string_token.position + 1], "\"");
    }

    #[test]
    fn double_dash_starts_a_line_comment() {
        let tokens = tokenize("-- टिप्पणी\nपुस्तकेभ्यः ।").unwrap();
        assert_eq!(tokens[0].text, "पुस्तकेभ्यः");
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn unterminated_string_is_an_error_not_a_panic() {
        let err = tokenize("पुस्तकेभ्यः \"अ").unwrap_err();
        assert!(err.to_string().contains("Unterminated string literal"), "{}", err);
    }

    #[test]
    fn string_escapes_for_quote_and_backslash() {
        let tokens = tokenize(r#"पुस्तकेभ्यः "a\"b\\c\n" ।"#).unwrap();
        let TokenType::String(content) = &tokens[1].token_type else { panic!("expected a string") };
        assert_eq!(content, r#"a"b\c\n"#);
    }
}

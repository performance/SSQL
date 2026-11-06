//! # Morphological Engine
//!
//! This module handles Sanskrit word declension and analysis.
//!
//! ## Strategy
//!
//! 1. **Pattern Matching**: Use Sanskrit nominal stem ending patterns to generate full 21-form declension paradigms.
//! 2. **In-Memory Cache**: Cache resolved declension tables (`MorphologyCache`) keyed by `(stem, gender, vachana)` to avoid recomputation.
//! 3. **Strict Validation**: Any unsupported stem or vachana combination fails explicitly in `generate_declension`.
//!
//! ## Supported Patterns (v1.0)
//!
//! We start with the most common declension patterns:
//!
//! ### Masculine (-अः/-आः)
//! - छात्रः (student), रामः (Rama), etc.
//!
//! ### Feminine (-आ)
//! - सीता (Sita), लता (creeper), etc.
//!
//! ### Neuter (-अम्)
//! - फलम् (fruit), पत्रम् (leaf), etc.
//!
//! ### इन्-stem masculine (-ी nominative, -िनः/-िणः genitive)
//! - स्वामिन् (owner), विक्रयाधिकारिन् (sales rep), etc.
//!
//! ### ऋ-stem masculine (-ता nominative, -ुः genitive)
//! - भोक्तृ (enjoyer), कर्तृ (doer), etc.
//!
//! ### उ-stem, all three genders (-ुः masc./fem. nominative, bare -ु neuter)
//! - भानुः (sun, masc.), धेनुः (cow, fem.), मधु/विक्रयणवस्तु (honey/sale-item, neut.)
//!
//! ### ई-stem feminine (-ी)
//! - नदी (river), वस्तुसूची (inventory), etc. Masculine/neuter ई-stem nouns
//!   (e.g. ग्रामणी) exist but are rare and irregularly formed - not
//!   special-cased here, fall through to the generic feminine-shaped pattern.
//!
//! More patterns will be added as we encounter them in real SSQL code.

use crate::types::{Linga, Vachana, Vibhakti};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete declension table for a word (all 7 vibhaktis for one vachan)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclensionTable {
    pub pratipadika: String,
    pub vachana: Vachana,
    pub linga: Linga,
    pub forms: HashMap<Vibhakti, String>,
}

impl DeclensionTable {
    pub fn new(pratipadika: String, vachana: Vachana, linga: Linga) -> Self {
        Self {
            pratipadika,
            vachana,
            linga,
            forms: HashMap::new(),
        }
    }
    
    pub fn add(&mut self, vibhakti: Vibhakti, form: String) {
        self.forms.insert(vibhakti, form);
    }
}

/// Main morphology engine
pub struct MorphologyEngine {
    /// In-memory cache for previously analyzed words
    cache: MorphologyCache,
}

impl MorphologyEngine {
    pub fn new() -> Result<Self> {
        let cache = MorphologyCache::new()?;
        Ok(Self { cache })
    }
    
    /// Generate all vibhakti forms for a given प्रथमा form
    ///
    /// This is called during Pass 1 when we encounter a declaration.
    /// 
    /// Steps:
    /// 1. Check cache
    /// 2. Try pattern-based generation
    /// 3. Fall back to external API if needed
    /// 4. Update cache
    pub fn decline(&mut self, prathama_form: &str, vachana: Vachana) -> Result<DeclensionTable> {
        // Check cache first
        if let Some(table) = self.cache.get(prathama_form, None, vachana)? {
            return Ok(table);
        }
        
        // Determine linga and pattern from the ending
        let (linga, stem) = self.analyze_prathama_form(prathama_form, vachana)?;
        
        // Generate declension using pattern rules
        let table = self.generate_declension(&stem, linga, vachana)?;
        
        // Cache the result under the Prathama surface form
        self.cache.store(prathama_form, None, &table)?;
        
        Ok(table)
    }

    /// Generate declension forms for an explicit stem, gender, and vachana
    pub fn decline_with_gender(
        &mut self,
        stem: &str,
        gender: Linga,
        vachana: Vachana,
    ) -> Result<DeclensionTable> {
        // Direct generation without analyzing prathama form
        // We still cache it to avoid re-computation
        if let Some(table) = self.cache.get(stem, Some(gender), vachana)? {
            return Ok(table);
        }

        let table = self.generate_declension(stem, gender, vachana)?;
        self.cache.store(stem, Some(gender), &table)?;
        Ok(table)
    }

    /// Analyze a प्रथमा form to determine its linga and stem
    fn analyze_prathama_form(&self, form: &str, vachana: Vachana) -> Result<(Linga, String)> {
        match vachana {
            Vachana::Eka => self.analyze_singular_prathama(form),
            Vachana::Bahu => self.analyze_plural_prathama(form),
            Vachana::Dvi => anyhow::bail!("Dual number (द्विवचन) not yet supported for automatic analysis"),
        }
    }
    
    /// Analyze singular प्रथमा forms
    pub fn analyze_singular_prathama(&self, form: &str) -> Result<(Linga, String)> {
        // Masculine endings: -अः (excluding plural -आः)
        if form.ends_with('ः') && !form.ends_with("ाः") {
            let stem = form.trim_end_matches('ः').to_string();
            return Ok((Linga::Masculine, stem));
        }
        
        // Feminine endings: -आ, -ई
        if form.ends_with('ा') {
            let stem = form.trim_end_matches('ा').to_string();
            return Ok((Linga::Feminine, stem));
        }
        
        if form.ends_with('ी') {
            let stem = form.trim_end_matches('ी').to_string();
            return Ok((Linga::Feminine, stem));
        }
        
        // Neuter endings: -अम्
        if form.ends_with("म्") {
            let stem = form.trim_end_matches("म्").to_string();
            return Ok((Linga::Neuter, stem));
        }
        
        anyhow::bail!("Unable to determine linga for: {}", form)
    }
    
    /// Analyze plural प्रथमा forms
    pub fn analyze_plural_prathama(&self, form: &str) -> Result<(Linga, String)> {
        // Masculine plural: -आः
        if form.ends_with("ाः") {
            let stem = form.trim_end_matches("ाः").to_string();
            return Ok((Linga::Masculine, stem));
        }
        
        // Feminine plural: -आः
        if form.ends_with("ाः") {
            let stem = form.trim_end_matches("ाः").to_string();
            // Surface -आः is shared between masculine and feminine; explicit DDL gender disambiguates.
            return Ok((Linga::Feminine, stem));
        }
        
        // Neuter plural: -आनि
        if form.ends_with("ानि") {
            let stem = form.trim_end_matches("ानि").to_string();
            return Ok((Linga::Neuter, stem));
        }
        
        anyhow::bail!("Unable to determine linga for plural form: {}", form)
    }
    
    /// Generate complete declension table using pattern rules, then apply
    /// ṇatva to every न the declension added (the stem's own spelling is the
    /// declarer's and is left as written).
    fn generate_declension(
        &self,
        stem: &str,
        linga: Linga,
        vachana: Vachana,
    ) -> Result<DeclensionTable> {
        let mut table = self.generate_declension_forms(stem, linga, vachana)?;
        // The inflected part starts after the stem, or at the stem-final न
        // of an इन्/अन् stem, which the declension carries into its forms.
        let from = if stem.ends_with("न्") { stem.len() - "न्".len() } else { stem.len() };
        table.pratipadika = natva(&table.pratipadika, from);
        for form in table.forms.values_mut() {
            *form = natva(form, from);
        }
        Ok(table)
    }

    fn generate_declension_forms(
        &self,
        stem: &str,
        linga: Linga,
        vachana: Vachana,
    ) -> Result<DeclensionTable> {
        // उ/ऊ-ending stems decline the same distinctive way regardless of
        // gender (भानु masc./धेनु fem./मधु neut. all share the same
        // उ→ओ/औ/व् guṇa-vṛddhi pattern, just with different case-endings
        // per gender) - checked before the masculine-only special cases
        // below since it applies to all three genders.
        if stem.ends_with('ु') || stem.ends_with('ू') {
            match (linga, vachana) {
                (Linga::Masculine, Vachana::Eka) => return self.decline_masculine_u_stem_singular(stem),
                (Linga::Masculine, Vachana::Bahu) => return self.decline_masculine_u_stem_plural(stem),
                (Linga::Feminine, Vachana::Eka) => return self.decline_feminine_u_stem_singular(stem),
                (Linga::Feminine, Vachana::Bahu) => return self.decline_feminine_u_stem_plural(stem),
                (Linga::Neuter, Vachana::Eka) => return self.decline_neuter_u_stem_singular(stem),
                (Linga::Neuter, Vachana::Bahu) => return self.decline_neuter_u_stem_plural(stem),
                (Linga::Neuter, Vachana::Dvi) => {
                    let base = stem.trim_end_matches(['ु', 'ू']);
                    return Ok(Self::dual_table(Linga::Neuter, format!("{}ुनी", base), format!("{}ुभ्याम्", base), format!("{}ुनोः", base)));
                }
                (_, Vachana::Dvi) => {
                    let base = stem.trim_end_matches(['ु', 'ू']);
                    return Ok(Self::dual_table(linga, format!("{}ू", base), format!("{}ुभ्याम्", base), format!("{}्वोः", base)));
                }
            }
        }

        // ई-ending stems: feminine only (नदी-model). Masculine/neuter
        // ई-stem nouns are rare and irregularly formed - intentionally not
        // special-cased, falls through to the generic branches below.
        if linga == Linga::Feminine && stem.ends_with('ी') {
            match vachana {
                Vachana::Eka => return self.decline_feminine_ii_stem_singular(stem),
                Vachana::Bahu => return self.decline_feminine_ii_stem_plural(stem),
                Vachana::Dvi => {
                    let base = stem.trim_end_matches('ी');
                    return Ok(Self::dual_table(linga, format!("{}्यौ", base), format!("{}ीभ्याम्", base), format!("{}्योः", base)));
                }
            }
        }

        // इ-stems: masculine (अग्नि-model) and feminine (मति-model).
        if stem.ends_with('ि') {
            return match (linga, vachana) {
                (Linga::Masculine, Vachana::Eka) => Ok(self.decline_masculine_i_stem_singular(stem)),
                (Linga::Masculine, Vachana::Bahu) => Ok(self.decline_masculine_i_stem_plural(stem)),
                (Linga::Feminine, Vachana::Eka) => Ok(self.decline_feminine_i_stem_singular(stem)),
                (Linga::Feminine, Vachana::Bahu) => Ok(self.decline_feminine_i_stem_plural(stem)),
                (Linga::Masculine | Linga::Feminine, Vachana::Dvi) => {
                    let base = stem.trim_end_matches('ि');
                    Ok(Self::dual_table(linga, format!("{}ी", base), format!("{}िभ्याम्", base), format!("{}्योः", base)))
                }
                (Linga::Neuter, _) => anyhow::bail!("Neuter इ-stems (वारि-model) are not yet supported (stem `{}`)", stem),
            };
        }

        // Neuter अन्-stems (नामन्-model; कर्मन्-model after a conjunct in म/व).
        if linga == Linga::Neuter && stem.ends_with("न्") && !stem.ends_with("िन्") {
            return Ok(match vachana {
                Vachana::Eka => self.decline_neuter_an_stem_singular(stem),
                Vachana::Bahu => self.decline_neuter_an_stem_plural(stem),
                // 6.4.136 also allows नामनी; नाम्नी is used, as in the locative.
                Vachana::Dvi => {
                    let (middle, weak) = Self::an_stem_bases(stem);
                    Self::dual_table(Linga::Neuter, format!("{}ी", weak), format!("{}भ्याम्", middle), format!("{}ोः", weak))
                }
            });
        }

        // Any other consonant-final stem, or a vowel this engine has no
        // paradigm for, is an error rather than a guessed form.
        let supported = stem.ends_with("िन्") && linga == Linga::Masculine
            || stem.ends_with('ृ') && linga == Linga::Masculine
            || stem.ends_with('ा') && linga == Linga::Feminine
            || stem.chars().last().map_or(false, |c| ('\u{0915}'..='\u{0939}').contains(&c));
        if !supported {
            anyhow::bail!(
                "The declension of stem `{}` ({:?}) is not supported yet; declare an अ-, आ-, इ-, ई-, उ-, ऋ-, इन्- or अन्-stem",
                stem, linga
            );
        }

        // Check for special stem endings first
        if linga == Linga::Masculine {
            if stem.ends_with("िन्") {
                 match vachana {
                     Vachana::Eka => return self.decline_masculine_in_stem_singular(stem),
                     Vachana::Bahu => return self.decline_masculine_in_stem_plural(stem),
                     Vachana::Dvi => {
                         let base = stem.trim_end_matches("िन्");
                         return Ok(Self::dual_table(linga, format!("{}िनौ", base), format!("{}िभ्याम्", base), format!("{}िनोः", base)));
                     }
                 }
            }
            if stem.ends_with("ृ") { // R-stem
                 match vachana {
                     Vachana::Eka => return self.decline_masculine_r_stem_singular(stem),
                     Vachana::Bahu => return self.decline_masculine_r_stem_plural(stem),
                     Vachana::Dvi => {
                         let base = stem.trim_end_matches("ृ");
                         return Ok(Self::dual_table(linga, format!("{}ारौ", base), format!("{}ृभ्याम्", base), format!("{}्रोः", base)));
                     }
                 }
            }
        }

        match (linga, vachana) {
            (Linga::Masculine, Vachana::Eka) => self.decline_masculine_singular(stem),
            (Linga::Masculine, Vachana::Bahu) => self.decline_masculine_plural(stem),
            (Linga::Feminine, Vachana::Eka) => self.decline_feminine_singular(stem),
            (Linga::Feminine, Vachana::Bahu) => self.decline_feminine_plural(stem),
            (Linga::Neuter, Vachana::Eka) => self.decline_neuter_singular(stem),
            (Linga::Neuter, Vachana::Bahu) => self.decline_neuter_plural(stem),
            (Linga::Masculine, Vachana::Dvi) => Ok(Self::dual_table(linga, format!("{}ौ", stem), format!("{}ाभ्याम्", stem), format!("{}योः", stem))),
            (Linga::Feminine | Linga::Neuter, Vachana::Dvi) => {
                let base = stem.trim_end_matches('ा');
                Ok(Self::dual_table(linga, format!("{}े", base), format!("{}ाभ्याम्", base), format!("{}योः", base)))
            }
        }
    }

    /// A dual has three forms: the nominative and accusative; the
    /// instrumental, dative and ablative; the genitive and locative.
    fn dual_table(linga: Linga, nominative: String, instrumental: String, genitive: String) -> DeclensionTable {
        let mut table = DeclensionTable::new(nominative.clone(), Vachana::Dvi, linga);
        table.add(Vibhakti::Prathama, nominative.clone());
        table.add(Vibhakti::Dvitiya, nominative);
        table.add(Vibhakti::Tritiya, instrumental.clone());
        table.add(Vibhakti::Chaturthi, instrumental.clone());
        table.add(Vibhakti::Panchami, instrumental);
        table.add(Vibhakti::Shashthi, genitive.clone());
        table.add(Vibhakti::Saptami, genitive);
        table
    }

    /// Masculine R-stem singular (e.g., भोक्तृ -> भोक्ता)
    fn decline_masculine_r_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches("ृ");
        
        // Nom Sg: Base + ā (ा)
        // e.g. Bhokt + ā = Bhoktā
        let pratipadika = format!("{}ा", base);
        
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ारम्", base));
        table.add(Vibhakti::Tritiya, format!("{}्रा", base));
        table.add(Vibhakti::Chaturthi, format!("{}्रे", base));
        
        // Panchami/Shashthi: Base + uḥ (ुः)
        table.add(Vibhakti::Panchami, format!("{}ुः", base));
        table.add(Vibhakti::Shashthi, format!("{}ुः", base));
        
        table.add(Vibhakti::Saptami, format!("{}रि", base));
        
        Ok(table)
    }

    /// Masculine R-stem plural
    fn decline_masculine_r_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches("ृ");
        let pratipadika = format!("{}ारः", base); // Bhoktāraḥ
        
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ॄन्", base));
        table.add(Vibhakti::Tritiya, format!("{}ृभिः", base)); // Stem (with r) + bhih

        
        table.add(Vibhakti::Chaturthi, format!("{}ृभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ृभ्यः", base));
        
        table.add(Vibhakti::Shashthi, format!("{}ॄनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ृषु", base));
        
        Ok(table)
    }
    
    /// Masculine In-stem singular (e.g., विक्रयाधिकारिन् -> विक्रयाधिकारी)
    fn decline_masculine_in_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        // Strip the final 'in' (िन्) to get consonent-ending base
        // stem "....in" (inv)
        let base = stem.trim_end_matches("िन्").trim_end(); 
        
        // Nom Sg: Base + ī (ी)
        let pratipadika = format!("{}ी", base); 
        
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, pratipadika.clone());

        let na = "न";

        table.add(Vibhakti::Dvitiya, format!("{}ि{}म्", base, na));
        table.add(Vibhakti::Tritiya, format!("{}ि{}ा", base, na));
        table.add(Vibhakti::Chaturthi, format!("{}ि{}े", base, na));
        table.add(Vibhakti::Panchami, format!("{}ि{}ः", base, na));
        table.add(Vibhakti::Shashthi, format!("{}ि{}ः", base, na));
        table.add(Vibhakti::Saptami, format!("{}ि{}ि", base, na));

        Ok(table)
    }

    /// Masculine In-stem plural
    fn decline_masculine_in_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches("िन्").trim_end();
        
        // Nom Pl: Base + inaḥ / iṇaḥ
        let suffix_nah = "िनः";
        let pratipadika = format!("{}{}", base, suffix_nah);
        
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, pratipadika.clone()); // Assuming Nom Pl is used as Label

        // Dvitiya Plural: same surface form as Prathama for this class.
        table.add(Vibhakti::Dvitiya, pratipadika.clone());

        // Tritiya Plural: Base + ibhiḥ
        table.add(Vibhakti::Tritiya, format!("{}िभिः", base));
        
        // Shashthi: Base + inām / iṇām
        let suffix_nam = "िनाम्";
        table.add(Vibhakti::Shashthi, format!("{}{}", base, suffix_nam));

        // Chaturthi/Panchami: Base + ibhyaḥ
        table.add(Vibhakti::Chaturthi, format!("{}िभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}िभ्यः", base));

        // Saptami: Base + iṣu
        table.add(Vibhakti::Saptami, format!("{}िषु", base));

        Ok(table)
    }

    /// Masculine U-stem singular (e.g., भानु -> भानुः, "sun")
    fn decline_masculine_u_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}ुः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Masculine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ुम्", base));
        table.add(Vibhakti::Tritiya, format!("{}ुना", base));
        table.add(Vibhakti::Chaturthi, format!("{}वे", base));
        table.add(Vibhakti::Panchami, format!("{}ोः", base));
        table.add(Vibhakti::Shashthi, format!("{}ोः", base));
        table.add(Vibhakti::Saptami, format!("{}ौ", base));

        Ok(table)
    }

    /// Masculine U-stem plural
    fn decline_masculine_u_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}वः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Masculine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ून्", base));
        table.add(Vibhakti::Tritiya, format!("{}ुभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ूनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ुषु", base));

        Ok(table)
    }

    /// Feminine U-stem singular (e.g., धेनु -> धेनुः, "cow")
    fn decline_feminine_u_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}ुः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Feminine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ुम्", base));
        table.add(Vibhakti::Tritiya, format!("{}्वा", base));
        // Where 1.4.6 allows both, the non-नदी forms (धेनवे, धेनोः, धेनौ).
        table.add(Vibhakti::Chaturthi, format!("{}वे", base));
        table.add(Vibhakti::Panchami, format!("{}ोः", base));
        table.add(Vibhakti::Shashthi, format!("{}ोः", base));
        table.add(Vibhakti::Saptami, format!("{}ौ", base));

        Ok(table)
    }

    /// Feminine U-stem plural
    fn decline_feminine_u_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}वः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Feminine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ूः", base));
        table.add(Vibhakti::Tritiya, format!("{}ुभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ूनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ुषु", base));

        Ok(table)
    }

    /// Neuter U-stem singular (e.g., मधु -> मधु, "honey")
    fn decline_neuter_u_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}ु", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Neuter);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, pratipadika.clone());
        table.add(Vibhakti::Tritiya, format!("{}ुना", base));
        table.add(Vibhakti::Chaturthi, format!("{}ुने", base));
        table.add(Vibhakti::Panchami, format!("{}ुनः", base));
        table.add(Vibhakti::Shashthi, format!("{}ुनः", base));
        table.add(Vibhakti::Saptami, format!("{}ुनि", base));

        Ok(table)
    }

    /// Neuter U-stem plural (e.g., विक्रयणवस्तु -> विक्रयणवस्तूनि, "sale item")
    fn decline_neuter_u_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches(['ु', 'ू']);
        let pratipadika = format!("{}ूनि", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Neuter);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, pratipadika.clone());
        table.add(Vibhakti::Tritiya, format!("{}ुभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ुभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ूनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ुषु", base));

        Ok(table)
    }

    /// Feminine II-stem (दीर्घ ई) singular (e.g., नदी -> नदी, "river")
    fn decline_feminine_ii_stem_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches('ी');
        let pratipadika = format!("{}ी", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Feminine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ीम्", base));
        table.add(Vibhakti::Tritiya, format!("{}्या", base));
        table.add(Vibhakti::Chaturthi, format!("{}्यै", base));
        table.add(Vibhakti::Panchami, format!("{}्याः", base));
        table.add(Vibhakti::Shashthi, format!("{}्याः", base));
        table.add(Vibhakti::Saptami, format!("{}्याम्", base));

        Ok(table)
    }

    /// Feminine II-stem plural (e.g., वस्तुसूची -> वस्तुसूचीषु, "inventory")
    fn decline_feminine_ii_stem_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = stem.trim_end_matches('ी');
        let pratipadika = format!("{}्यः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Feminine);

        table.add(Vibhakti::Prathama, pratipadika.clone());
        table.add(Vibhakti::Dvitiya, format!("{}ीः", base));
        table.add(Vibhakti::Tritiya, format!("{}ीभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}ीभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ीभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ीनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ीषु", base));

        Ok(table)
    }

    /// Masculine singular declension (-अः pattern like छात्रः, रामः)
    fn decline_masculine_singular(&self, stem: &str) -> Result<DeclensionTable> {
        // If stem already ends with Visarga, assume it is full form
        let base = if stem.ends_with('ः') {
            stem.trim_end_matches('ः')
        } else {
            stem
        };
        
        let pratipadika = format!("{}ः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, format!("{}ः", base));
        table.add(Vibhakti::Dvitiya, format!("{}म्", base));
        table.add(Vibhakti::Tritiya, format!("{}ेन", base));
        table.add(Vibhakti::Chaturthi, format!("{}ाय", base));
        table.add(Vibhakti::Panchami, format!("{}ात्", base));
        table.add(Vibhakti::Shashthi, format!("{}स्य", base));
        table.add(Vibhakti::Saptami, format!("{}े", base));
        
        Ok(table)
    }
    
    /// Masculine plural declension (-आः pattern)
    fn decline_masculine_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let pratipadika = format!("{}ाः", stem);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Masculine);
        
        table.add(Vibhakti::Prathama, format!("{}ाः", stem));
        table.add(Vibhakti::Dvitiya, format!("{}ान्", stem));
        table.add(Vibhakti::Tritiya, format!("{}ैः", stem));
        table.add(Vibhakti::Chaturthi, format!("{}ेभ्यः", stem));
        table.add(Vibhakti::Panchami, format!("{}ेभ्यः", stem));
        table.add(Vibhakti::Shashthi, format!("{}ानाम्", stem));
        table.add(Vibhakti::Saptami, format!("{}ेषु", stem));
        
        Ok(table)
    }
    
    /// Feminine singular declension (-आ pattern like सीता, लता)
    fn decline_feminine_singular(&self, stem: &str) -> Result<DeclensionTable> {
        // If stem already ends with 'aa' (ा), remove it to get base
        let base = if stem.ends_with('ा') {
            stem.trim_end_matches('ा')
        } else {
            stem
        };

        let pratipadika = format!("{}ा", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Eka, Linga::Feminine);
        
        table.add(Vibhakti::Prathama, format!("{}ा", base));
        table.add(Vibhakti::Dvitiya, format!("{}ाम्", base));
        table.add(Vibhakti::Tritiya, format!("{}या", base));
        table.add(Vibhakti::Chaturthi, format!("{}ायै", base));
        table.add(Vibhakti::Panchami, format!("{}ायाः", base));
        table.add(Vibhakti::Shashthi, format!("{}ायाः", base));
        table.add(Vibhakti::Saptami, format!("{}ायाम्", base));
        
        Ok(table)
    }
    
    /// Feminine plural declension  
    fn decline_feminine_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let base = if stem.ends_with('ा') {
            stem.trim_end_matches('ा')
        } else {
            stem
        };
        let pratipadika = format!("{}ाः", base);
        let mut table = DeclensionTable::new(pratipadika.clone(), Vachana::Bahu, Linga::Feminine);
        
        table.add(Vibhakti::Prathama, format!("{}ाः", base));
        table.add(Vibhakti::Dvitiya, format!("{}ाः", base));
        table.add(Vibhakti::Tritiya, format!("{}ाभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}ाभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}ाभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ानाम्", base));
        table.add(Vibhakti::Saptami, format!("{}ासु", base));
        
        Ok(table)
    }
    
    /// Neuter singular declension (-अम् pattern)
    fn decline_neuter_singular(&self, stem: &str) -> Result<DeclensionTable> {
        let mut table = DeclensionTable::new(format!("{}म्", stem), Vachana::Eka, Linga::Neuter);
        table.add(Vibhakti::Prathama, format!("{}म्", stem));
        table.add(Vibhakti::Dvitiya, format!("{}म्", stem));
        table.add(Vibhakti::Tritiya, format!("{}ेन", stem));
        table.add(Vibhakti::Chaturthi, format!("{}ाय", stem));
        table.add(Vibhakti::Panchami, format!("{}ात्", stem));
        table.add(Vibhakti::Shashthi, format!("{}स्य", stem));
        table.add(Vibhakti::Saptami, format!("{}े", stem));
        Ok(table)
    }

    /// Neuter plural declension
    fn decline_neuter_plural(&self, stem: &str) -> Result<DeclensionTable> {
        let mut table = DeclensionTable::new(format!("{}ानि", stem), Vachana::Bahu, Linga::Neuter);
        table.add(Vibhakti::Prathama, format!("{}ानि", stem));
        table.add(Vibhakti::Dvitiya, format!("{}ानि", stem));
        table.add(Vibhakti::Tritiya, format!("{}ैः", stem));
        table.add(Vibhakti::Chaturthi, format!("{}ेभ्यः", stem));
        table.add(Vibhakti::Panchami, format!("{}ेभ्यः", stem));
        table.add(Vibhakti::Shashthi, format!("{}ानाम्", stem));
        table.add(Vibhakti::Saptami, format!("{}ेषु", stem));
        Ok(table)
    }

    /// Masculine इ-stem singular (अग्नि-model: अग्निः, अग्निना, अग्नये, अग्नेः, अग्नौ)
    fn decline_masculine_i_stem_singular(&self, stem: &str) -> DeclensionTable {
        let base = stem.trim_end_matches('ि');
        let mut table = DeclensionTable::new(format!("{}िः", base), Vachana::Eka, Linga::Masculine);
        table.add(Vibhakti::Prathama, format!("{}िः", base));
        table.add(Vibhakti::Dvitiya, format!("{}िम्", base));
        table.add(Vibhakti::Tritiya, format!("{}िना", base));
        table.add(Vibhakti::Chaturthi, format!("{}ये", base));
        table.add(Vibhakti::Panchami, format!("{}ेः", base));
        table.add(Vibhakti::Shashthi, format!("{}ेः", base));
        table.add(Vibhakti::Saptami, format!("{}ौ", base));
        table
    }

    /// Masculine इ-stem plural (अग्नयः, अग्नीन्, अग्निभिः, अग्नीनाम्, अग्निषु)
    fn decline_masculine_i_stem_plural(&self, stem: &str) -> DeclensionTable {
        let base = stem.trim_end_matches('ि');
        let mut table = DeclensionTable::new(format!("{}यः", base), Vachana::Bahu, Linga::Masculine);
        table.add(Vibhakti::Prathama, format!("{}यः", base));
        table.add(Vibhakti::Dvitiya, format!("{}ीन्", base));
        table.add(Vibhakti::Tritiya, format!("{}िभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}िभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}िभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ीनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}िषु", base));
        table
    }

    /// Feminine इ-stem singular (मति-model). Where 1.4.6 allows both, the
    /// non-नदी forms are used (मतये, मतेः, मतौ: the common śāstric forms,
    /// shared with the masculine), as for the feminine उ-stems.
    fn decline_feminine_i_stem_singular(&self, stem: &str) -> DeclensionTable {
        let base = stem.trim_end_matches('ि');
        let mut table = DeclensionTable::new(format!("{}िः", base), Vachana::Eka, Linga::Feminine);
        table.add(Vibhakti::Prathama, format!("{}िः", base));
        table.add(Vibhakti::Dvitiya, format!("{}िम्", base));
        table.add(Vibhakti::Tritiya, format!("{}्या", base));
        table.add(Vibhakti::Chaturthi, format!("{}ये", base));
        table.add(Vibhakti::Panchami, format!("{}ेः", base));
        table.add(Vibhakti::Shashthi, format!("{}ेः", base));
        table.add(Vibhakti::Saptami, format!("{}ौ", base));
        table
    }

    /// Feminine इ-stem plural (मतयः, मतीः, मतिभिः, मतीनाम्, मतिषु)
    fn decline_feminine_i_stem_plural(&self, stem: &str) -> DeclensionTable {
        let base = stem.trim_end_matches('ि');
        let mut table = DeclensionTable::new(format!("{}यः", base), Vachana::Bahu, Linga::Feminine);
        table.add(Vibhakti::Prathama, format!("{}यः", base));
        table.add(Vibhakti::Dvitiya, format!("{}ीः", base));
        table.add(Vibhakti::Tritiya, format!("{}िभिः", base));
        table.add(Vibhakti::Chaturthi, format!("{}िभ्यः", base));
        table.add(Vibhakti::Panchami, format!("{}िभ्यः", base));
        table.add(Vibhakti::Shashthi, format!("{}ीनाम्", base));
        table.add(Vibhakti::Saptami, format!("{}िषु", base));
        table
    }

    /// The two bases of a neuter अन्-stem: the strong/middle base (stem
    /// without the final न्, e.g. नाम) and the weak base before a vowel
    /// ending, where the अ of अन् is elided (6.4.134 अल्लोपोऽनः: नाम्न-)
    /// unless a conjunct ending in म्/व् precedes it (6.4.137: कर्मण-).
    fn an_stem_bases(stem: &str) -> (String, String) {
        let middle = stem.trim_end_matches("न्").to_string(); // नाम, कर्म
        let before: Vec<char> = middle.chars().collect();
        let conjunct_mv = before.len() >= 2
            && matches!(before[before.len() - 1], 'म' | 'व')
            && before[before.len() - 2] == '्';
        let weak = if conjunct_mv { format!("{}न", middle) } else { format!("{}्न", middle) };
        (middle, weak)
    }

    /// Neuter अन्-stem singular (नामन् → नाम, नाम्ना, नाम्ने, नाम्नः, नाम्नि).
    /// 6.4.136 also allows नामनि in the locative; नाम्नि is used.
    fn decline_neuter_an_stem_singular(&self, stem: &str) -> DeclensionTable {
        let (middle, weak) = Self::an_stem_bases(stem);
        let mut table = DeclensionTable::new(middle.clone(), Vachana::Eka, Linga::Neuter);
        table.add(Vibhakti::Prathama, middle.clone());
        table.add(Vibhakti::Dvitiya, middle);
        table.add(Vibhakti::Tritiya, format!("{}ा", weak));
        table.add(Vibhakti::Chaturthi, format!("{}े", weak));
        table.add(Vibhakti::Panchami, format!("{}ः", weak));
        table.add(Vibhakti::Shashthi, format!("{}ः", weak));
        table.add(Vibhakti::Saptami, format!("{}ि", weak));
        table
    }

    /// Neuter अन्-stem plural (नामानि, नामभिः, नामभ्यः, नाम्नाम्, नामसु)
    fn decline_neuter_an_stem_plural(&self, stem: &str) -> DeclensionTable {
        let (middle, weak) = Self::an_stem_bases(stem);
        let mut table = DeclensionTable::new(format!("{}ानि", middle), Vachana::Bahu, Linga::Neuter);
        table.add(Vibhakti::Prathama, format!("{}ानि", middle));
        table.add(Vibhakti::Dvitiya, format!("{}ानि", middle));
        table.add(Vibhakti::Tritiya, format!("{}भिः", middle));
        table.add(Vibhakti::Chaturthi, format!("{}भ्यः", middle));
        table.add(Vibhakti::Panchami, format!("{}भ्यः", middle));
        table.add(Vibhakti::Shashthi, format!("{}ाम्", weak));
        table.add(Vibhakti::Saptami, format!("{}सु", middle));
        table
    }
}

/// ṇatva (Aṣṭādhyāyī 8.4.1-2): a न at or after byte `from` becomes ण when
/// an earlier र, ष, ऋ or ॠ reaches it across only vowels, ह य व र, क- and
/// प-varga consonants and anusvāra (नुम्). A word-final न् is unchanged
/// (8.4.37).
///
/// Only the न of a case ending, of the -आनि augment, or the stem-final न of
/// an इन्/अन् stem is ever at or after `from`, so a trigger in an earlier
/// compound member may reach it. For exactly those न the compound rules
/// make ण correct: obligatory by 8.4.13 when the last member has a क-varga
/// letter (क्रमिकयोगेण), optional by 8.4.11 otherwise (विक्रयव्ययेण, chosen
/// as the one form). Any other न inside a later member is never changed:
/// only 8.4.3 (proper names) would reach it.
pub(crate) fn natva(word: &str, from: usize) -> String {
    const ALLOWED: &[char] = &['ह', 'य', 'व', 'र', 'क', 'ख', 'ग', 'घ', 'ङ', 'प', 'फ', 'ब', 'भ', 'म'];
    let chars: Vec<(usize, char)> = word.char_indices().collect();
    let mut out = String::with_capacity(word.len());
    for (k, &(i, c)) in chars.iter().enumerate() {
        let word_final = k + 2 == chars.len() && chars[k + 1].1 == '्';
        if c != 'न' || i < from || word_final {
            out.push(c);
            continue;
        }
        let mut retroflex = false;
        for &(_, d) in chars[..k].iter().rev() {
            if matches!(d, 'र' | 'ष' | 'ृ' | 'ॄ' | 'ऋ' | 'ॠ') {
                retroflex = true;
                break;
            }
            if ('\u{0915}'..='\u{0939}').contains(&d) && !ALLOWED.contains(&d) {
                break;
            }
        }
        out.push(if retroflex { 'ण' } else { 'न' });
    }
    out
}

/// The adjective `ऋणात्मक` ("negative") in the form that agrees with a noun of
/// this gender, number and case. A feminine stem in `-अक` takes `-इका`
/// (7.3.44), as in `व्यवसायात्मिका`.
pub fn negative_word(linga: Linga, vachana: Vachana, vibhakti: Vibhakti) -> Option<String> {
    let stem = if linga == Linga::Feminine { "ऋणात्मिका" } else { "ऋणात्मक" };
    let table = MorphologyEngine::new().ok()?.decline_with_gender(stem, linga, vachana).ok()?;
    table.forms.get(&vibhakti).cloned()
}

/// Every form of `ऋणात्मक`, generated once from its stems: the words the lexer
/// reads as the adjective, and that no schema may name.
pub fn negative_words() -> &'static [String] {
    static WORDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        use Vibhakti::*;
        let mut words: Vec<String> = [Linga::Masculine, Linga::Feminine, Linga::Neuter]
            .into_iter()
            .flat_map(|l| [Vachana::Eka, Vachana::Dvi, Vachana::Bahu].map(|v| (l, v)))
            .flat_map(|(l, v)| [Prathama, Dvitiya, Tritiya, Chaturthi, Panchami, Shashthi, Saptami].map(|c| (l, v, c)))
            .filter_map(|(l, v, c)| negative_word(l, v, c))
            .collect();
        words.sort_unstable();
        words.dedup();
        words
    })
}

/// In-memory cache for morphological analysis
///
/// Keyed by `(stem_or_prathama, Option<Linga>, Vachana)` so stems with the
/// same spelling declared under different grammatical genders never collide,
/// and concurrent processes/threads never contend on an OS file lock.
struct MorphologyCache {
    entries: HashMap<(String, Option<Linga>, Vachana), DeclensionTable>,
}

impl MorphologyCache {
    fn new() -> Result<Self> {
        Ok(Self {
            entries: HashMap::new(),
        })
    }

    fn get(&self, key_str: &str, gender: Option<Linga>, vachana: Vachana) -> Result<Option<DeclensionTable>> {
        let key = (key_str.to_string(), gender, vachana);
        Ok(self.entries.get(&key).cloned())
    }

    fn store(&mut self, key_str: &str, gender: Option<Linga>, table: &DeclensionTable) -> Result<()> {
        let key = (key_str.to_string(), gender, table.vachana);
        self.entries.insert(key, table.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dual_of_every_paradigm() {
        let mut engine = MorphologyEngine::new().unwrap();
        for (stem, linga, nom, ins, gen) in [
            ("छात्र", Linga::Masculine, "छात्रौ", "छात्राभ्याम्", "छात्रयोः"),
            ("फल", Linga::Neuter, "फले", "फलाभ्याम्", "फलयोः"),
            ("लता", Linga::Feminine, "लते", "लताभ्याम्", "लतयोः"),
            ("अग्नि", Linga::Masculine, "अग्नी", "अग्निभ्याम्", "अग्न्योः"),
            ("मति", Linga::Feminine, "मती", "मतिभ्याम्", "मत्योः"),
            ("नदी", Linga::Feminine, "नद्यौ", "नदीभ्याम्", "नद्योः"),
            ("भानु", Linga::Masculine, "भानू", "भानुभ्याम्", "भान्वोः"),
            ("धेनु", Linga::Feminine, "धेनू", "धेनुभ्याम्", "धेन्वोः"),
            ("मधु", Linga::Neuter, "मधुनी", "मधुभ्याम्", "मधुनोः"),
            ("वस्तु", Linga::Neuter, "वस्तुनी", "वस्तुभ्याम्", "वस्तुनोः"),
            ("योगिन्", Linga::Masculine, "योगिनौ", "योगिभ्याम्", "योगिनोः"),
            ("कर्तृ", Linga::Masculine, "कर्तारौ", "कर्तृभ्याम्", "कर्त्रोः"),
            ("नामन्", Linga::Neuter, "नाम्नी", "नामभ्याम्", "नाम्नोः"),
            ("कर्मन्", Linga::Neuter, "कर्मणी", "कर्मभ्याम्", "कर्मणोः"),
            ("ऋणात्मक", Linga::Masculine, "ऋणात्मकौ", "ऋणात्मकाभ्याम्", "ऋणात्मकयोः"),
            ("ऋणात्मक", Linga::Neuter, "ऋणात्मके", "ऋणात्मकाभ्याम्", "ऋणात्मकयोः"),
            ("ऋणात्मिका", Linga::Feminine, "ऋणात्मिके", "ऋणात्मिकाभ्याम्", "ऋणात्मिकयोः"),
        ] {
            let t = engine.decline_with_gender(stem, linga, Vachana::Dvi).unwrap();
            for (v, want) in [
                (Vibhakti::Prathama, nom), (Vibhakti::Dvitiya, nom),
                (Vibhakti::Tritiya, ins), (Vibhakti::Chaturthi, ins), (Vibhakti::Panchami, ins),
                (Vibhakti::Shashthi, gen), (Vibhakti::Saptami, gen),
            ] {
                assert_eq!(t.forms[&v], want, "{} {:?}", stem, v);
            }
        }
    }

    #[test]
    fn test_masculine_singular_declension() {
        let mut engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_with_gender("छात्र", Linga::Masculine, Vachana::Eka).unwrap();
        
        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "छात्रः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "छात्रम्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "छात्रेण");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "छात्रस्य");
    }
    
    #[test]
    fn test_feminine_singular_declension() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_feminine_singular("सीत").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "सीता");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "सीतया");
    }

    // Regression tests for उ-stem (all three genders) and ई-stem (feminine)
    // declension, plus the इन्-stem vibhakti backfill covering all
    // case-forms, not just Shashthi.

    #[test]
    fn test_masculine_u_stem_declension_singular() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_masculine_u_stem_singular("भानु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "भानुः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "भानुम्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "भानुना");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "भानवे");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "भानोः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "भानोः");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "भानौ");
    }

    #[test]
    fn test_masculine_u_stem_declension_plural() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_masculine_u_stem_plural("भानु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "भानवः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "भानून्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "भानुभिः");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "भानुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "भानुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "भानूनाम्");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "भानुषु");
    }

    #[test]
    fn test_feminine_u_stem_declension_singular() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_feminine_u_stem_singular("धेनु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "धेनुः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "धेनुम्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "धेन्वा");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "धेनवे");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "धेनोः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "धेनोः");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "धेनौ");
    }

    #[test]
    fn test_feminine_u_stem_declension_plural() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_feminine_u_stem_plural("धेनु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "धेनवः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "धेनूः");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "धेनुभिः");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "धेनुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "धेनुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "धेनूनाम्");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "धेनुषु");
    }

    #[test]
    fn test_neuter_u_stem_declension_singular() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_neuter_u_stem_singular("मधु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "मधु");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "मधु");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "मधुना");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "मधुने");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "मधुनः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "मधुनः");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "मधुनि");
    }

    #[test]
    fn test_neuter_u_stem_declension_plural() {
        let engine = MorphologyEngine::new().unwrap();
        // विक्रयणवस्तु (sale_items) -
        // Prathama/Dvitiya must match the schema's own literally-declared
        // nominative plural "विक्रयणवस्तूनि".
        let table = engine.decline_neuter_u_stem_plural("विक्रयणवस्तु").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "विक्रयणवस्तूनि");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "विक्रयणवस्तूनि");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "विक्रयणवस्तुभिः");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "विक्रयणवस्तुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "विक्रयणवस्तुभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "विक्रयणवस्तूनाम्");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "विक्रयणवस्तुषु");
    }

    #[test]
    fn test_feminine_ii_stem_declension_singular() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_feminine_ii_stem_singular("नदी").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "नदी");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "नदीम्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "नद्या");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "नद्यै");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "नद्याः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "नद्याः");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "नद्याम्");
    }

    #[test]
    fn test_feminine_ii_stem_declension_plural() {
        let engine = MorphologyEngine::new().unwrap();
        // वस्तुसूची (inventory).
        let table = engine.decline_feminine_ii_stem_plural("वस्तुसूची").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "वस्तुसूच्यः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "वस्तुसूचीः");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "वस्तुसूचीभिः");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "वस्तुसूचीभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "वस्तुसूचीभ्यः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "वस्तुसूचीनाम्");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "वस्तुसूचीषु");
    }

    #[test]
    fn test_masculine_in_stem_declension_singular_full() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_masculine_in_stem_singular("स्वामिन्").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "स्वामी");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "स्वामिनम्");
        assert_eq!(table.forms.get(&Vibhakti::Tritiya).unwrap(), "स्वामिना");
        assert_eq!(table.forms.get(&Vibhakti::Chaturthi).unwrap(), "स्वामिने");
        assert_eq!(table.forms.get(&Vibhakti::Panchami).unwrap(), "स्वामिनः");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "स्वामिनः");
        assert_eq!(table.forms.get(&Vibhakti::Saptami).unwrap(), "स्वामिनि");
    }

    #[test]
    fn test_masculine_in_stem_declension_singular_retroflex() {
        // विक्रयाधिकारिन् (sales rep) - the "र" in "अधिकार" immediately
        // precedes the इन् suffix, so ṇatva applies (िण, not िन).
        let mut engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_with_gender("विक्रयाधिकारिन्", Linga::Masculine, Vachana::Eka).unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "विक्रयाधिकारिणम्");
        assert_eq!(table.forms.get(&Vibhakti::Shashthi).unwrap(), "विक्रयाधिकारिणः");
    }

    #[test]
    fn test_masculine_in_stem_declension_plural_dvitiya() {
        let engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_masculine_in_stem_plural("स्वामिन्").unwrap();

        assert_eq!(table.forms.get(&Vibhakti::Prathama).unwrap(), "स्वामिनः");
        assert_eq!(table.forms.get(&Vibhakti::Dvitiya).unwrap(), "स्वामिनः");
    }

    /// Full paradigm through the public entry point (ṇatva included), in
    /// Prathama..Saptami order.
    fn paradigm(stem: &str, gender: Linga, vachana: Vachana) -> Vec<String> {
        let mut engine = MorphologyEngine::new().unwrap();
        let table = engine.decline_with_gender(stem, gender, vachana).unwrap();
        [Vibhakti::Prathama, Vibhakti::Dvitiya, Vibhakti::Tritiya, Vibhakti::Chaturthi,
         Vibhakti::Panchami, Vibhakti::Shashthi, Vibhakti::Saptami]
            .iter().map(|v| table.forms[v].clone()).collect()
    }

    #[test]
    fn natva_follows_the_trigger_and_blockers() {
        // Trigger reaches the ending: obligatory ण.
        assert_eq!(paradigm("छात्र", Linga::Masculine, Vachana::Eka)[2], "छात्रेण");
        assert_eq!(paradigm("ग्राहक", Linga::Masculine, Vachana::Bahu)[5], "ग्राहकाणाम्");
        assert_eq!(paradigm("पत्र", Linga::Neuter, Vachana::Bahu)[0], "पत्राणि");
        assert_eq!(paradigm("परीक्ष", Linga::Feminine, Vachana::Bahu)[5], "परीक्षाणाम्");
        // No trigger: न stays.
        assert_eq!(paradigm("लेखक", Linga::Masculine, Vachana::Eka)[2], "लेखकेन");
        assert_eq!(paradigm("लेखक", Linga::Masculine, Vachana::Bahu)[5], "लेखकानाम्");
        // A blocker (श, ण, द …) between trigger and न: न stays.
        assert_eq!(paradigm("प्रकाशक", Linga::Masculine, Vachana::Bahu)[5], "प्रकाशकानाम्");
        assert_eq!(paradigm("श्रेणी", Linga::Feminine, Vachana::Bahu)[5], "श्रेणीनाम्");
        assert_eq!(paradigm("विक्रयदिनाङ्क", Linga::Masculine, Vachana::Eka)[2], "विक्रयदिनाङ्केन");
        // Word-final न् never changes (8.4.37).
        assert_eq!(paradigm("छात्र", Linga::Masculine, Vachana::Bahu)[1], "छात्रान्");
        // u-stem and in-stem endings.
        assert_eq!(paradigm("गुरु", Linga::Masculine, Vachana::Eka)[2], "गुरुणा");
        assert_eq!(paradigm("कर्मचारिन्", Linga::Masculine, Vachana::Eka)[5], "कर्मचारिणः");
        assert_eq!(paradigm("स्वामिन्", Linga::Masculine, Vachana::Eka)[5], "स्वामिनः");
    }

    #[test]
    fn i_stem_paradigms() {
        assert_eq!(paradigm("अग्नि", Linga::Masculine, Vachana::Eka),
                   ["अग्निः", "अग्निम्", "अग्निना", "अग्नये", "अग्नेः", "अग्नेः", "अग्नौ"]);
        assert_eq!(paradigm("अग्नि", Linga::Masculine, Vachana::Bahu),
                   ["अग्नयः", "अग्नीन्", "अग्निभिः", "अग्निभ्यः", "अग्निभ्यः", "अग्नीनाम्", "अग्निषु"]);
        assert_eq!(paradigm("मति", Linga::Feminine, Vachana::Eka),
                   ["मतिः", "मतिम्", "मत्या", "मतये", "मतेः", "मतेः", "मतौ"]);
        assert_eq!(paradigm("मति", Linga::Feminine, Vachana::Bahu),
                   ["मतयः", "मतीः", "मतिभिः", "मतिभ्यः", "मतिभ्यः", "मतीनाम्", "मतिषु"]);
        assert_eq!(paradigm("स्थिति", Linga::Feminine, Vachana::Eka)[0], "स्थितिः");
        assert_eq!(paradigm("वृष्टि", Linga::Feminine, Vachana::Bahu)[5], "वृष्टीनाम्");
    }

    #[test]
    fn an_stem_paradigms() {
        assert_eq!(paradigm("नामन्", Linga::Neuter, Vachana::Eka),
                   ["नाम", "नाम", "नाम्ना", "नाम्ने", "नाम्नः", "नाम्नः", "नाम्नि"]);
        assert_eq!(paradigm("नामन्", Linga::Neuter, Vachana::Bahu),
                   ["नामानि", "नामानि", "नामभिः", "नामभ्यः", "नामभ्यः", "नाम्नाम्", "नामसु"]);
        // Conjunct in म before अन्: no elision, and ṇatva from the र्.
        assert_eq!(paradigm("कर्मन्", Linga::Neuter, Vachana::Eka),
                   ["कर्म", "कर्म", "कर्मणा", "कर्मणे", "कर्मणः", "कर्मणः", "कर्मणि"]);
        assert_eq!(paradigm("कर्मन्", Linga::Neuter, Vachana::Bahu)[5], "कर्मणाम्");
        // A compound: the uttarapada's own न blocks the पूर्वपद's र.
        assert_eq!(paradigm("केन्द्रनामन्", Linga::Neuter, Vachana::Eka)[5], "केन्द्रनाम्नः");
    }

    #[test]
    fn r_stem_plural_has_one_vowel_sign() {
        assert_eq!(paradigm("दातृ", Linga::Masculine, Vachana::Bahu),
                   ["दातारः", "दातॄन्", "दातृभिः", "दातृभ्यः", "दातृभ्यः", "दातॄणाम्", "दातृषु"]);
    }

    #[test]
    fn unsupported_stems_are_errors_not_guesses() {
        let mut engine = MorphologyEngine::new().unwrap();
        assert!(engine.decline_with_gender("मरुत्", Linga::Masculine, Vachana::Eka).is_err());
        assert!(engine.decline_with_gender("मातृ", Linga::Feminine, Vachana::Eka).is_err());
        assert!(engine.decline_with_gender("दाता", Linga::Masculine, Vachana::Eka).is_err());
    }

    #[test]
    fn test_cache_respects_gender_for_same_stem() {
        let mut engine = MorphologyEngine::new().unwrap();
        let masc = engine.decline_with_gender("सङ्ख्य", Linga::Masculine, Vachana::Eka).unwrap();
        let fem = engine.decline_with_gender("सङ्ख्य", Linga::Feminine, Vachana::Eka).unwrap();
        let neut = engine.decline_with_gender("सङ्ख्य", Linga::Neuter, Vachana::Eka).unwrap();

        assert_eq!(masc.linga, Linga::Masculine);
        assert_eq!(masc.forms.get(&Vibhakti::Prathama).unwrap(), "सङ्ख्यः");
        assert_eq!(fem.linga, Linga::Feminine);
        assert_eq!(fem.forms.get(&Vibhakti::Prathama).unwrap(), "सङ्ख्या");
        assert_eq!(neut.linga, Linga::Neuter);
        assert_eq!(neut.forms.get(&Vibhakti::Prathama).unwrap(), "सङ्ख्यम्");
    }
}


//! Parser token type (`Token`) and the LexerToken/VibhaktiToken -> Token
//! conversion/classification logic. No chumsky dependency - this is pure
//! data transformation feeding the actual parser combinators in sibling
//! modules.

use crate::annotator::{LexerToken, TokenType};
use crate::types::{Vachana, Vibhakti, Padam};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    // --- Leaf tokens ---
    Identifier(String),
    Number(String),
    StringLit(String),
    
    // --- Punctuation ---
    LParen,
    RParen,
    Purnadanda,  // ।
    Rinatmaka(String),   // a form of ऋणात्मक, "negative", before a column

    // --- Keywords ---
    Iti,           // इति
    Cha,           // च (and)
    Va,            // वा (or)
    Chet,          // चेत् (if/where)
    Sarvam,        // सर्वम् (ALL / Wildcard)
    Sadrusham(Padam), // सदृशः / सदृशी / सदृशम् (LIKE), agreeing with the column
    Vishishtasya(String),  // विशिष्टस्य / विशिष्टायाः … (DISTINCT inside aggregates)
    Placeholder,   // ?
    Ashritya,      // आश्रित्य (IMPORT)
    Saha,          // सह (WITH for joins)
    Bahya,         // बाह्य (External)
    Sanjna,        // सञ्ज्ञा (Symbol/Variable)
    Aakhya,        // आख्या (Alias)
    Aavartika,     // आवर्तिका (Recursive CTE)
    ChakraNivaranena, // चक्रनिवारणेन (Cycle Prevention)
    
    // --- DDL Tokens ---
    Anivaryah,     // अनिवार्यः
    Vaikalpikah,   // वैकल्पिकः
    Prathamikah,   // प्राथमिकः
    Kramikah,      // क्रमिकः
    Advitiyah,     // अद्वितीयः
    Pumlingam,         // पुंलिङ्गम्
    Strilingam,          // स्त्रीलिङ्गम्
    Napumsakalingam,    // नपुंसकलिङ्गम्
    Ekavachanam,   // एकवचनम्
    Dvivachanam,   // द्विवचनम्
    Bahuvachanam,  // बहुवचनम्
    Baddhah,       // बद्धः
    Tatkalikam,    // तात्कालिकम् / तात्कालिकः
    Patalam,       // पटलम्
    Prati,         // प्रति
    AddVerb,       // योज्यताम्
    AbhidhiyatamVerb, // अभिधीयताम् (RENAME COLUMN/TABLE - "let it be called")
    Vidyate,       // विद्यते
    Suchi,         // सूची / सूचिका

    
    // --- Clause markers (Tritiya forms) ---
    Niyamena,      // नियमेन (WHERE)
    Samuhena,      // समूहेन (GROUP BY)
    Samuhaniyamena, // समूहनियमेन (HAVING)
    Kramena,       // क्रमेण (ORDER BY)
    Melanena,      // मेलनेन (JOIN)
    Mitya,         // मित्या (LIMIT)
    Apasarena,     // अपसारेण (OFFSET)
    
    // --- Verbs ---
    FetchVerb(String),     // आह्रियताम् (SELECT ... FROM)
    InsertVerb(String), // निवेश्यताम् / निवेश्येताम् / निवेश्यन्ताम्
    UpdateVerb(String), // परिवर्त्यताम् / परिवर्त्येताम् / परिवर्त्यन्ताम्
    DeleteVerb,    // निष्कास्यताम्
    CreateVerb,    // सर्जयतु
    DropVerb,      // लुप्यताम्
    GrantVerb,     // अनुमन्यताम्
    RevokeVerb,    // प्रतिषधतु
    BeginVerb,     // प्रारभ्यताम्
    CommitVerb,    // स्थिरीक्रियताम्
    RollbackVerb,  // प्रत्याह्रियताम्
    Sangrahabinduh, // सङ्ग्रहबिन्दुः
    
    // --- Explain ---
    Explain,       // विशदीक्रियताम्
    
    // --- Comparison operators ---
    Adhikam(Padam),       // अधिकम् (greater)
    Nyunam(Padam),        // न्यूनम् (less)
    Samam(Padam),         // समम् (equal)
    Asti,                 // अस्ति (is/exists)
    Nasti,                // नास्ति (is not)
    Riktah(Padam),        // रिक्तः / रिक्ता / रिक्तम्

    
    // --- Case expression ---
    VaraNam,      // वरणम् (CASE)
    Yasmin,        // यस्मिन् (WHEN)
    Tasmin,        // तस्मिन् (THEN)
    Anyasmin,      // अन्यस्मिन् (ELSE)
    
    // --- Date/Time functions ---
    Varsham,       // वर्षम् (YEAR)
    Masah,         // मासः (MONTH)
    Dinam,         // दिनम् (DAY)
    Hora,          // होरा (HOUR)
    Nimesah,       // निमेषः (MINUTE)
    Vipalam,       // विपलम् (SECOND)
    Samprati,      // सम्प्रति (NOW)
    AdyatanaDinankah, // अद्यतनदिनाङ्कः (CURRENT_DATE)
    AdyatanaSamayah,  // अद्यतनसमयः (CURRENT_TIME)
// =================================// --- IN/BETWEEN ---
    Ityeshu(String), // इत्येषु / feminine इत्यासु (IN, 3+ items)
    Ityesham(String),      // इत्येषाम् (Of these plural - Set Op)
    Ityayoh(String),       // इत्यनयोः (IN dual, BETWEEN)
    Madhye,        // मध्ये (BETWEEN)
    Ityasmat(String),      // इत्यस्मात् (comparison source)
    Ityasya(String),       // इत्यस्य (possessive - for date funcs)
    Ityasyah(String),      // इत्यस्याः (feminine genitive singular possessive)
    Ityetasam(String),     // इत्यासाम् (feminine genitive plural possessive)
    Tasmat,        // तस्मात् (RENAME COLUMN/TABLE source-name connector, anaphoric ablative of तद्)
    
    // --- Window Functions ---
    // Duplicates removed
    Yatra,         // यत्र (OVER)
    Vibhajanam,    // विभाजनम् (PARTITION BY)
    Tatha,         // तथा (Connector)
    
    // --- Set Operations ---
    Sangamah,      // सङ्गमः (Union)
    SarvaSangamah, // सर्वसङ्गमः (UNION ALL)
    Sammilanam,    // सम्मिलनम् (Intersect)
    Varjanam,      // वर्जनम् (Except)
    Anyat,         // अन्यत् (Another/Else - for set ops or case)
    Api,           // अपि (Also/And - for set ops or logical)
    
    Yogah,         // योगः (SUM)
    Ganana,        // गणना (COUNT)
    Madhyam,       // माध्यम् (AVG, arithmetic mean)
    Mahattam,      // महत्तमम् (MAX)
    Laghutamam,    // लघुतमम् (MIN)
    
    // --- Order By ---
    Shreni,        // श्रेणी (RANK)
    PanktiSankhya, // पङ्क्तिसङ्ख्या (ROW_NUMBER)
    SangrahabindumDvitiya, // सङ्ग्रहबिन्दुम् (accusative, ROLLBACK TO: सङ्ग्रहबिन्दुम् प्रति)
    Vina, // विना (in पुनरुक्तिम् विना, SELECT DISTINCT)
    Punaruktim, // पुनरुक्तिम् (in पुनरुक्तिम् विना, SELECT DISTINCT)
    Parinamah, // परिणामः (CAST: <expr> इत्यस्य <type> इति परिणामः)
    Aadhare, // आधारे (an index is built "on" its columns)
    LaghutamasyaGen, // लघुतमस्य (genitive of लघुतमम्, MIN)
    MahattamasyaGen, // महत्तमस्य (genitive of महत्तमम्, MAX)
    MadhyasyaGen, // माध्यस्य (genitive of माध्यम्, AVG)
    GananayahGen, // गणनायाः (genitive of गणना, COUNT)
    YogasyaGen, // योगस्य (genitive of योगः, SUM: a window key)
    Panktinam, // पङ्क्तीनाम् (COUNT(*): पङ्क्तीनाम् गणना, "the count of the rows")
    Sarvatra, // सर्वत्र (UPDATE of every row: "everywhere")
    SavepointVerb, // स्थाप्यताम् (SAVEPOINT: a marker is set)
    Karyam, // कार्यम् (a pure user-defined function: शुद्धम् कार्यम्)
    Bhagankah, // भागाङ्कः (NTILE(n): <n> भागेषु भागाङ्कः)
    Chaturthamshankah, // चतुर्थांशाङ्कः (NTILE(4): the quartile number)
    AnantaraPankteh, // अनन्तरपङ्क्तेः (LEAD: the next row's …)
    PurvaPankteh, // पूर्वपङ्क्तेः (LAG: the previous row's …)
    // LAG / LEAD by n rows (pcf.md item 6): the ordinal compounded with the
    // previous / next row, `द्वितीयपूर्वपङ्क्तेः` (two before), `द्वितीयानन्तरपङ्क्तेः`
    // (two after), for n = 2..10.
    PurvaPanktehN(u32),
    AnantaraPanktehN(u32),
    SaghanaShreni, // सघनश्रेणी (DENSE_RANK)
    Arohah,        // आरोहः (ASC)
    Avarohah,      // अवरोहः (DESC)
    Aadharena,     // आधारेण (Base/Partition)
    Anusaram,      // अनुसारम् (Accordance/Order)
    ArohaKramena,  // आरोहक्रमेण
    AvarohaKramena, // अवरोहक्रमेण
    VamaMelanena, // वाममेलनेन / वाममेलनम् (Left Join)
    DakshinaMelanena, // दक्षिणमेलनेन / दक्षिणमेलनम् (Right Join)
    PurnaMelanena,    // पूर्णमेलनेन / पूर्णमेलनम् (Full Outer Join)
    SarvaMelanena,    // सर्वमेलनेन (Cross Join)
    Sutram,        // सूत्रम् (formula declaration marker)
    Gunanam,       // गुणनम् / गुणन (multiplication keyword)
    Pradaya,       // प्रदाय (gerund: "having supplied/given", UDF argument marker)
    Yat,           // यत् (CREATE VIEW ... AS marker)

    // --- Math Keywords (New) ---
    Nivarya,       // विशोध्य
    Avashistam,    // अवशिष्टम्
    Vibhajya,      // विभज्य
    Labdham,       // लब्धम्
    // The result nouns in the case of the slot they fill (expert review,
    // grouped operands): ablative minuend, accusative subtrahend/dividend,
    // instrumental divisor. गुणनम्/अवशिष्टम्/लब्धम् are also the accusative.
    YogatAbl, // योगात्
    YogamAcc, // योगम्
    YogenaIns, // योगेन
    GunanatAbl, // गुणनात्
    GunanenaIns, // गुणनेन
    AvashistatAbl, // अवशिष्टात्
    AvashistenaIns, // अवशिष्टेन
    LabdhatAbl, // लब्धात्
    LabdhenaIns, // लब्धेन
    // The genitive of a result noun, an operand of an aggregate or function
    // (`मूल्यात् १ विशोध्य अवशिष्टस्य योगः`; योगस्य is above).
    GunanasyaGen, // गुणनस्य
    AvashistasyaGen, // अवशिष्टस्य
    LabdhasyaGen, // लब्धस्य
    // An aggregate as an arithmetic operand, in the case of its slot
    // (`मूल्यस्य महत्तमात् मूल्यस्य लघुतमम् विशोध्य अवशिष्टम्`; the ablative of
    // गणना is गणनायाः, above, and माध्यम्/महत्तमम्/लघुतमम् are also accusatives).
    GananamAcc, // गणनाम्
    MadhyatAbl, // माध्यात्
    MahattamatAbl, // महत्तमात्
    LaghutamatAbl, // लघुतमात्
    GananayaIns, // गणनया
    MadhyenaIns, // माध्येन
    MahattamenaIns, // महत्तमेन
    LaghutamenaIns, // लघुतमेन
    // The accusative of इति + इदम्, carrying a group's case where a gerund
    // takes an accusative group (`… यत् … अवशिष्टम् इति इतीदम् विशोध्य`).
    Itidam, // इतीदम्

    // --- Type System ---
    Purnankah,     // पूर्णाङ्कः (Integer)
    Vakyam,        // वाक्यम् (Text)
    Vastavikah,    // वास्तविकः (Real/Decimal)
    Dinankah,      // दिनाङ्कः (Date)
    Dvaitam,       // द्वैतम् (Boolean)
    // CAST's target, "in the form of <type>" (`मूल्यस्य पूर्णाङ्करूपेण परिणामः`).
    PurnankaRupena, // पूर्णाङ्करूपेण
    VakyaRupena, // वाक्यरूपेण
    VastavikaRupena, // वास्तविकरूपेण
    DinankaRupena, // दिनाङ्करूपेण
    DvaitaRupena, // द्वैतरूपेण
    Parivartanam,  // परिवर्तनम् (CAST)

    // --- Scalar Functions ---
    Samyojanam,    // संयोजनम् (Concat)
    Khandah,       // खण्डः (Substr)
    Aayamah,       // आयामः (Length)
    Mahavarnarupam,  // महावर्णरूपम् (Upper)
    Laghuvarnarupam, // लघुवर्णरूपम् (Lower)
    Vikalpah,      // विकल्पः (Coalesce)
    Sthaanaat,     // स्थानात् (position-of, ablative - reserved keyword, chained SUBSTR/INSTR)
    Varnaanaam,    // वर्णानाम् (of-N-characters, genitive plural - reserved keyword, chained SUBSTR/INSTR)
    Vaarah,        // वारः (DayName)
    Anurupena,     // अनुरूपेण (Regexp Like)
    Na,            // न (Negation for EXISTS / BETWEEN)

    // --- Window Frame Specification ---
    Panktayah,              // पङ्क्तयः (ROWS)
    Vyaptih,                // व्याप्तिः (RANGE)
    Samuhah,                // समूहाः (GROUPS)
    AparimitaPurvavartitah, // अपरिमितपूर्ववर्तितः (UNBOUNDED PRECEDING)
    Purvavartitah,          // पूर्ववर्तितः (PRECEDING)
    Vartamanapanktih,       // वर्तमानपङ्क्तिः / वर्तमानपङ्क्तिम् (CURRENT ROW)
    Anuvartitah,            // अनुवर्तितः (FOLLOWING)
    AparimitaAnuvartitah,   // अपरिमितानुवर्तितः (UNBOUNDED FOLLOWING)
    Aarabhya,               // आरभ्य (BETWEEN start)
    Paryantam,              // पर्यन्तम् (AND end)
    Varjayitva,             // वर्जयित्वा (EXCLUDE)
    Samuham,                // समूहम् (GROUP)
    Sambaddhan,             // सम्बद्धान् (TIES)
    Anyan,                  // अन्यान् (OTHERS)

    // --- Enriched identifiers with vibhakti ---
    /// Prathama (nominative) - subjects, columns
    Prathama { text: String, vachana: Vachana, pratipadika: String },
    /// Dvitiya (accusative) - objects
    Dvitiya { text: String, vachana: Vachana, pratipadika: String },
    /// Tritiya (instrumental) - by/with
    Tritiya { text: String, vachana: Vachana, pratipadika: String },
    /// Chaturthi (dative) - to/for
    Chaturthi { text: String, vachana: Vachana, pratipadika: String },
    /// Panchami (ablative) - FROM
    Panchami { text: String, vachana: Vachana, pratipadika: String },
    /// Shashthi (genitive) - of/belonging to
    Shashthi { text: String, vachana: Vachana, pratipadika: String },
    /// Saptami (locative) - in/on
    Saptami { text: String, vachana: Vachana, pratipadika: String },
    
    // --- Special ---
    Eof,
}


impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Number(s) => write!(f, "{}", s),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Purnadanda => write!(f, "।"),
            Token::Rinatmaka(s) => write!(f, "{}", s),
            Token::Iti => write!(f, "इति"),
            Token::Cha => write!(f, "च"),
            Token::Va => write!(f, "वा"),
            Token::Na => write!(f, "न"),
            Token::Chet => write!(f, "चेत्"),
            Token::Sadrusham(s) => write!(f, "{}", s),
            Token::Placeholder => write!(f, "?"),
            Token::FetchVerb(s) => write!(f, "{}", s),
            Token::Explain => write!(f, "विशदीक्रियताम्"),
            Token::Ityayoh(s) | Token::Ityesham(s) | Token::Ityasmat(s) | Token::Ityasya(s) | Token::Ityasyah(s) | Token::Ityetasam(s) => write!(f, "{}", s),
            Token::Tasmat => write!(f, "तस्मात्"),
            Token::Vishishtasya(s) => write!(f, "{}", s),
            Token::CreateVerb => write!(f, "रच्यताम्"),
            Token::DropVerb => write!(f, "लुप्यताम्"),
            Token::GrantVerb => write!(f, "अनुमन्यताम्"),
            Token::RevokeVerb => write!(f, "प्रतिसंह्रियताम्"),
            Token::BeginVerb => write!(f, "प्रारभ्यताम्"),
            Token::CommitVerb => write!(f, "स्थिरीक्रियताम्"),
            Token::RollbackVerb => write!(f, "प्रत्याह्रियताम्"),
            Token::Sangrahabinduh => write!(f, "सङ्ग्रहबिन्दुः"),
            Token::Sangamah => write!(f, "सङ्गमः"),
            Token::SarvaSangamah => write!(f, "सर्वसङ्गमः"),
            Token::PanktiSankhya => write!(f, "पङ्क्तिसङ्ख्या"),
            Token::SangrahabindumDvitiya => write!(f, "सङ्ग्रहबिन्दुम्"),
            Token::Vina => write!(f, "विना"),
            Token::Punaruktim => write!(f, "पुनरुक्तिम्"),
            Token::Parinamah => write!(f, "परिणामः"),
            Token::Aadhare => write!(f, "आधारे"),
            Token::LaghutamasyaGen => write!(f, "लघुतमस्य"),
            Token::MahattamasyaGen => write!(f, "महत्तमस्य"),
            Token::MadhyasyaGen => write!(f, "माध्यस्य"),
            Token::GananayahGen => write!(f, "गणनायाः"),
            Token::YogasyaGen => write!(f, "योगस्य"),
            Token::Panktinam => write!(f, "पङ्क्तीनाम्"),
            Token::Sarvatra => write!(f, "सर्वत्र"),
            Token::SavepointVerb => write!(f, "स्थाप्यताम्"),
            Token::Karyam => write!(f, "कार्यम्"),
            Token::Bhagankah => write!(f, "भागाङ्कः"),
            Token::Chaturthamshankah => write!(f, "चतुर्थांशाङ्कः"),
            Token::AnantaraPankteh => write!(f, "अनन्तरपङ्क्तेः"),
            Token::PurvaPankteh => write!(f, "पूर्वपङ्क्तेः"),
            Token::PurvaPanktehN(n) => write!(f, "{}पूर्वपङ्क्तेः", ordinal_stem(*n)),
            Token::AnantaraPanktehN(n) => write!(f, "{}ानन्तरपङ्क्तेः", ordinal_stem(*n)),
            Token::SaghanaShreni => write!(f, "सघनश्रेणी"),
            Token::Prathama { text, .. } => write!(f, "{}", text),
            Token::Panchami { text, .. } => write!(f, "{}", text),
            Token::Dvitiya { text, .. } | Token::Tritiya { text, .. } | Token::Shashthi { text, .. } | Token::Saptami { text, .. } => write!(f, "{}", text),
            Token::Purnankah => write!(f, "पूर्णाङ्कः"),
            Token::Vakyam => write!(f, "वाक्यम्"),
            Token::Vastavikah => write!(f, "वास्तविकः"),
            Token::Dinankah => write!(f, "दिनाङ्कः"),
            Token::Dvaitam => write!(f, "द्वैतम्"),
            Token::PurnankaRupena => write!(f, "पूर्णाङ्करूपेण"),
            Token::VakyaRupena => write!(f, "वाक्यरूपेण"),
            Token::VastavikaRupena => write!(f, "वास्तविकरूपेण"),
            Token::DinankaRupena => write!(f, "दिनाङ्करूपेण"),
            Token::DvaitaRupena => write!(f, "द्वैतरूपेण"),
            Token::Parivartanam => write!(f, "परिणामः"),
            Token::Bahya => write!(f, "बाह्या"),
            Token::Sanjna => write!(f, "सञ्ज्ञा"),
            Token::Aavartika => write!(f, "आवर्तिका"),
            Token::ChakraNivaranena => write!(f, "चक्रनिवारणेन"),
            Token::Nivarya => write!(f, "विशोध्य"),
            Token::Avashistam => write!(f, "अवशिष्टम्"),
            Token::Vibhajya => write!(f, "विभज्य"),
            Token::Labdham => write!(f, "लब्धम्"),
            Token::YogatAbl => write!(f, "योगात्"),
            Token::YogamAcc => write!(f, "योगम्"),
            Token::YogenaIns => write!(f, "योगेन"),
            Token::GunanatAbl => write!(f, "गुणनात्"),
            Token::GunanenaIns => write!(f, "गुणनेन"),
            Token::AvashistatAbl => write!(f, "अवशिष्टात्"),
            Token::AvashistenaIns => write!(f, "अवशिष्टेन"),
            Token::LabdhatAbl => write!(f, "लब्धात्"),
            Token::LabdhenaIns => write!(f, "लब्धेन"),
            Token::GunanasyaGen => write!(f, "गुणनस्य"),
            Token::AvashistasyaGen => write!(f, "अवशिष्टस्य"),
            Token::LabdhasyaGen => write!(f, "लब्धस्य"),
            Token::GananamAcc => write!(f, "गणनाम्"),
            Token::MadhyatAbl => write!(f, "माध्यात्"),
            Token::MahattamatAbl => write!(f, "महत्तमात्"),
            Token::LaghutamatAbl => write!(f, "लघुतमात्"),
            Token::GananayaIns => write!(f, "गणनया"),
            Token::MadhyenaIns => write!(f, "माध्येन"),
            Token::MahattamenaIns => write!(f, "महत्तमेन"),
            Token::LaghutamenaIns => write!(f, "लघुतमेन"),
            Token::Itidam => write!(f, "इतीदम्"),
            Token::Aadharena => write!(f, "आधारेण"),
            Token::Anusaram => write!(f, "अनुसारम्"),
            Token::ArohaKramena => write!(f, "आरोहक्रमेण"),
            Token::AvarohaKramena => write!(f, "अवरोहक्रमेण"),
            Token::VamaMelanena => write!(f, "वाममेलनेन"),
            Token::DakshinaMelanena => write!(f, "दक्षिणमेलनेन"),
            Token::PurnaMelanena => write!(f, "पूर्णमेलनेन"),
            Token::SarvaMelanena => write!(f, "सर्वमेलनेन"),
            Token::Aakhya => write!(f, "आख्या"),
            Token::Tatkalikam => write!(f, "तात्कालिकम्"),
            Token::Patalam => write!(f, "पटलम्"),
            Token::Prati => write!(f, "प्रति"),
            Token::Pradaya => write!(f, "प्रदाय"),
            Token::Niyamena => write!(f, "नियमेन"),
            Token::AddVerb => write!(f, "योज्यताम्"),
            Token::InsertVerb(s) | Token::UpdateVerb(s) => write!(f, "{}", s),
            Token::AbhidhiyatamVerb => write!(f, "अभिधीयताम्"),
            Token::Suchi => write!(f, "सूची"),
            Token::Nasti => write!(f, "नास्ति"),
            Token::Riktah(s) => write!(f, "{}", s),
            Token::Adhikam(s) | Token::Nyunam(s) | Token::Samam(s) => write!(f, "{}", s),
            Token::Panktayah => write!(f, "पङ्क्तयः"),
            Token::Vyaptih => write!(f, "व्याप्तिः"),
            Token::Samuhah => write!(f, "समूहाः"),
            Token::AparimitaPurvavartitah => write!(f, "अपरिमितपूर्ववर्तितः"),
            Token::Purvavartitah => write!(f, "पूर्ववर्तितः"),
            Token::Vartamanapanktih => write!(f, "वर्तमानपङ्क्तिः"),
            Token::Anuvartitah => write!(f, "अनुवर्तितः"),
            Token::AparimitaAnuvartitah => write!(f, "अपरिमितानुवर्तितः"),
            Token::Aarabhya => write!(f, "आरभ्य"),
            Token::Paryantam => write!(f, "पर्यन्तम्"),
            Token::Varjayitva => write!(f, "वर्जयित्वा"),
            Token::Samuham => write!(f, "समूहम्"),
            Token::Sambaddhan => write!(f, "सम्बद्धान्"),
            Token::Anyan => write!(f, "अन्यान्"),
            Token::Sarvam => write!(f, "सर्वम्"),
            Token::Ashritya => write!(f, "आश्रित्य"),
            Token::Saha => write!(f, "सह"),
            Token::Anivaryah => write!(f, "अनिवार्यः"),
            Token::Vaikalpikah => write!(f, "वैकल्पिकः"),
            Token::Prathamikah => write!(f, "प्राथमिकः"),
            Token::Kramikah => write!(f, "क्रमिकः"),
            Token::Advitiyah => write!(f, "अद्वितीयः"),
            Token::Pumlingam => write!(f, "पुंलिङ्गम्"),
            Token::Strilingam => write!(f, "स्त्रीलिङ्गम्"),
            Token::Napumsakalingam => write!(f, "नपुंसकलिङ्गम्"),
            Token::Ekavachanam => write!(f, "एकवचनम्"),
            Token::Dvivachanam => write!(f, "द्विवचनम्"),
            Token::Bahuvachanam => write!(f, "बहुवचनम्"),
            Token::Baddhah => write!(f, "बद्धः"),
            Token::Vidyate => write!(f, "विद्यते"),
            Token::Samuhena => write!(f, "समूहेन"),
            Token::Samuhaniyamena => write!(f, "समूहनियमेन"),
            Token::Kramena => write!(f, "क्रमेण"),
            Token::Melanena => write!(f, "मेलनेन"),
            Token::Mitya => write!(f, "मित्या"),
            Token::Apasarena => write!(f, "अपसारेण"),
            Token::DeleteVerb => write!(f, "निष्कास्यताम्"),
            Token::Asti => write!(f, "अस्ति"),
            Token::VaraNam => write!(f, "वरणम्"),
            Token::Yasmin => write!(f, "यस्मिन्"),
            Token::Tasmin => write!(f, "तस्मिन्"),
            Token::Anyasmin => write!(f, "अन्यस्मिन्"),
            Token::Varsham => write!(f, "वर्षम्"),
            Token::Masah => write!(f, "मासः"),
            Token::Dinam => write!(f, "दिनम्"),
            Token::Hora => write!(f, "होरा"),
            Token::Nimesah => write!(f, "निमेषः"),
            Token::Vipalam => write!(f, "क्षणः"),
            Token::Samprati => write!(f, "अद्यतनक्षणः"),
            Token::AdyatanaDinankah => write!(f, "अद्यतनदिनाङ्कः"),
            Token::AdyatanaSamayah => write!(f, "अद्यतनसमयः"),
            Token::Madhye => write!(f, "मध्ये"),
            Token::Yatra => write!(f, "यत्र"),
            Token::Vibhajanam => write!(f, "विभाजनम्"),
            Token::Tatha => write!(f, "तथा"),
            Token::Sammilanam => write!(f, "प्रतिच्छेदः"),
            Token::Varjanam => write!(f, "वर्जनम्"),
            Token::Anyat => write!(f, "अन्यत्"),
            Token::Api => write!(f, "अपि"),
            Token::Yogah => write!(f, "योगः"),
            Token::Ganana => write!(f, "गणना"),
            Token::Madhyam => write!(f, "माध्यम्"),
            Token::Mahattam => write!(f, "महत्तमम्"),
            Token::Laghutamam => write!(f, "लघुतमम्"),
            Token::Shreni => write!(f, "श्रेणी"),
            Token::Arohah => write!(f, "आरोहः"),
            Token::Avarohah => write!(f, "अवरोहः"),
            Token::Sutram => write!(f, "सूत्रम्"),
            Token::Gunanam => write!(f, "गुणनम्"),
            Token::Yat => write!(f, "यत्"),
            Token::Samyojanam => write!(f, "संयोजनम्"),
            Token::Khandah => write!(f, "खण्डः"),
            Token::Aayamah => write!(f, "दैर्घ्यम्"),
            Token::Mahavarnarupam => write!(f, "महावर्णरूपम्"),
            Token::Laghuvarnarupam => write!(f, "लघुवर्णरूपम्"),
            Token::Vikalpah => write!(f, "विकल्पः"),
            Token::Sthaanaat => write!(f, "स्थानात्"),
            Token::Varnaanaam => write!(f, "वर्णानाम्"),
            Token::Vaarah => write!(f, "वारः"),
            Token::Anurupena => write!(f, "अनुरूपेण"),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// Token Conversion (LexerToken → Token)
// =============================================================================

/// Convert lexer tokens to parser tokens
/// This is where we map the annotator's enriched tokens to our parser format
/// Convert lexer tokens to parser tokens (Legacy - for tests without annotator)

pub fn convert_tokens(lexer_tokens: Vec<LexerToken>) -> Vec<Token> {
    lexer_tokens.into_iter().map(|lt| match lt.token_type {
        TokenType::Identifier => classify_identifier(&lt.text),
        TokenType::Anivaryah => Token::Anivaryah,
        TokenType::Vaikalpikah => Token::Vaikalpikah,
        TokenType::Prathamikah => Token::Prathamikah,
        TokenType::Kramikah => Token::Kramikah,
        TokenType::Advitiyah => Token::Advitiyah,
        TokenType::Pumlingam => Token::Pumlingam,
        TokenType::Strilingam => Token::Strilingam,
        TokenType::Napumsakalingam => Token::Napumsakalingam,
        TokenType::Ekavachanam => Token::Ekavachanam,
        TokenType::Dvivachanam => Token::Dvivachanam,
        TokenType::Bahuvachanam => Token::Bahuvachanam,
        TokenType::Baddhah => Token::Baddhah,
        TokenType::Number(n) => Token::Number(n),
        TokenType::String(s) => Token::StringLit(s),
        TokenType::LParen => Token::LParen,
        TokenType::RParen => Token::RParen,
        TokenType::Purnadanda => Token::Purnadanda,
        TokenType::Iti => Token::Iti,
        TokenType::Cha => Token::Cha,
        TokenType::Va => Token::Va,
        TokenType::Sangamah => Token::Sangamah,
        TokenType::SarvaSangamah => Token::SarvaSangamah,
        TokenType::Sadrusham => Token::Sadrusham(lt.text),
        TokenType::Bahya => Token::Bahya,
        TokenType::Sanjna => Token::Sanjna,
        TokenType::FetchVerb => Token::FetchVerb(lt.text),
        TokenType::InsertVerb => Token::InsertVerb(lt.text),
        TokenType::Ashritya => Token::Ashritya,
        TokenType::Sutram => Token::Sutram,
        TokenType::Niyamah => Token::Niyamena,
        TokenType::Niyamena => Token::Niyamena,
        TokenType::Samuhena => Token::Samuhena,
        TokenType::Samuhaniyamena => Token::Samuhaniyamena,
        TokenType::Kramena => Token::Kramena,
        TokenType::Melanena => Token::Melanena,
        TokenType::Mitya => Token::Mitya,
        TokenType::Yatra => Token::Yatra,
        TokenType::Vibhajanam => Token::Vibhajanam,
        TokenType::Tatha => Token::Tatha,
        TokenType::Shreni => Token::Shreni,
        TokenType::PanktiSankhya => Token::PanktiSankhya,
        TokenType::SangrahabindumDvitiya => Token::SangrahabindumDvitiya,
        TokenType::Vina => Token::Vina,
        TokenType::Punaruktim => Token::Punaruktim,
        TokenType::Parinamah => Token::Parinamah,
        TokenType::Aadhare => Token::Aadhare,
        TokenType::LaghutamasyaGen => Token::LaghutamasyaGen,
        TokenType::MahattamasyaGen => Token::MahattamasyaGen,
        TokenType::MadhyasyaGen => Token::MadhyasyaGen,
        TokenType::GananayahGen => Token::GananayahGen,
        TokenType::YogasyaGen => Token::YogasyaGen,
        TokenType::Panktinam => Token::Panktinam,
        TokenType::Sarvatra => Token::Sarvatra,
        TokenType::Bhagankah => Token::Bhagankah,
        TokenType::Chaturthamshankah => Token::Chaturthamshankah,
        TokenType::AnantaraPankteh => Token::AnantaraPankteh,
        TokenType::PurvaPankteh => Token::PurvaPankteh,
        TokenType::SaghanaShreni => Token::SaghanaShreni,
        TokenType::Arohah => Token::Arohah,
        TokenType::Avarohah => Token::Avarohah,
        TokenType::Aadharena => Token::Aadharena,
        TokenType::Anusaram => Token::Anusaram,
        TokenType::ArohaKramena => Token::ArohaKramena,
        TokenType::AvarohaKramena => Token::AvarohaKramena,
        TokenType::VamaMelanena => Token::VamaMelanena,
        TokenType::DakshinaMelanena => Token::DakshinaMelanena,
        TokenType::PurnaMelanena => Token::PurnaMelanena,
        TokenType::SarvaMelanena => Token::SarvaMelanena,
        TokenType::Yogah => Token::Yogah,
        TokenType::Madhyam => Token::Madhyam,
        TokenType::Mahattam => Token::Mahattam,
        TokenType::Laghutamam => Token::Laghutamam,
        TokenType::Anyat => Token::Anyat,
        TokenType::Api => Token::Api,
        TokenType::Ityasmat => Token::Ityasmat(lt.text),
        TokenType::Tasmat => Token::Tasmat,
        TokenType::Ityayoh => Token::Ityayoh(lt.text),
        TokenType::Ityesham => Token::Ityesham(lt.text),
        TokenType::Ityasya => Token::Ityasya(lt.text),
        TokenType::Ityasyah => Token::Ityasyah(lt.text),
        TokenType::Ityetasam => Token::Ityetasam(lt.text),
        TokenType::Chet => Token::Chet,
        TokenType::Aakhya => Token::Aakhya,
        TokenType::Patalam => Token::Patalam,
        TokenType::Prati => Token::Prati,
        TokenType::AddVerb => Token::AddVerb,
        TokenType::ChangeVerb => Token::UpdateVerb(lt.text),
        TokenType::AbhidhiyatamVerb => Token::AbhidhiyatamVerb,
        TokenType::Suchi => Token::Suchi,
        TokenType::Panktayah => Token::Panktayah,
        TokenType::Vyaptih => Token::Vyaptih,
        TokenType::Samuhah => Token::Samuhah,
        TokenType::AparimitaPurvavartitah => Token::AparimitaPurvavartitah,
        TokenType::Purvavartitah => Token::Purvavartitah,
        TokenType::Vartamanapanktih => Token::Vartamanapanktih,
        TokenType::Anuvartitah => Token::Anuvartitah,
        TokenType::AparimitaAnuvartitah => Token::AparimitaAnuvartitah,
        TokenType::Aarabhya => Token::Aarabhya,
        TokenType::Paryantam => Token::Paryantam,
        TokenType::Varjayitva => Token::Varjayitva,
        TokenType::Samuham => Token::Samuham,
        TokenType::Sambaddhan => Token::Sambaddhan,
        TokenType::Anyan => Token::Anyan,
        TokenType::BeginVerb => Token::BeginVerb,
        TokenType::CommitVerb => Token::CommitVerb,
        TokenType::RollbackVerb => Token::RollbackVerb,
        TokenType::Sangrahabinduh => Token::Sangrahabinduh,
    }).collect()
}

/// Convert enriched Vibhakti tokens to Parser tokens

pub fn convert_vibhakti_tokens(vibhakti_tokens: Vec<crate::types::VibhaktiToken>) -> Vec<Token> {
    vibhakti_tokens.into_iter().map(|vt| {
        let text = vt.text.as_str();
        
        // 1. Check for Punctuation / Literals (Re-lexing needed because Annotator erased types)
        if text == "(" { return Token::LParen; }
        if text == ")" { return Token::RParen; }
        if text == "।" || text == "॥" { return Token::Purnadanda; }
        if text == "?" { return Token::Placeholder; }
        
        // Strings (start with quote)
        if text.starts_with('"') {
             // Strip quotes
             let content = if text.len() >= 2 {
                 text[1..text.len()-1].to_string()
             } else {
                 text.to_string()
             };
             return Token::StringLit(content);
        }
        
        // Numbers
        if text.trim_start_matches('-').chars().next().map_or(false, |c| c.is_ascii_digit() || ('०'..='९').contains(&c)) {
            return Token::Number(text.to_string());
        }

        // 2. Check for Keywords using classify_identifier logic
        // If it returns anything other than Identifier(text), it's a keyword.
        let parsed = classify_identifier(text);
        if let Token::Identifier(_) = parsed {

            // It's an Identifier (or unknown keyword)
            // 3. Check Vibhakti Forms with Prioritization for Ambiguous cases
            // (e.g. -ebhyah is Chaturthi and Panchami. We prefer Panchami for FROM clause)
            
            let best_form = vt.possible_forms.iter().find(|f| f.vibhakti == Vibhakti::Saptami)
                .or_else(|| vt.possible_forms.iter().find(|f| f.vibhakti == Vibhakti::Shashthi))
                .or_else(|| vt.possible_forms.iter().find(|f| f.vibhakti == Vibhakti::Panchami))
                .or_else(|| vt.possible_forms.iter().find(|f| f.vibhakti == Vibhakti::Prathama))
                .or_else(|| vt.possible_forms.first());

            if let Some(form) = best_form {
                match form.vibhakti {
                    Vibhakti::Prathama => Token::Prathama { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Dvitiya => Token::Dvitiya { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Tritiya => Token::Tritiya { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Chaturthi => Token::Chaturthi { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Panchami => Token::Panchami { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Shashthi => Token::Shashthi { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                    Vibhakti::Saptami => Token::Saptami { 
                        text: text.to_string(), 
                        vachana: form.vachana, 
                        pratipadika: vt.pratipadika.clone().unwrap_or(text.to_string()) 
                    },
                }
            } else {
                // No forms -> Try Heuristic (classify_by_ending)
                let heuristic = classify_by_ending(text, vt.pratipadika.clone());
                // If heuristic also returns Identifier (Prathama default usually), so be it.
                heuristic
            }
        } else {
            // It is a Keyword
            parsed
        }
    }).collect()
}

/// Words the grammar matches by their spelling rather than as tokens (UDF
/// declarations, the view type). A test keeps this in step with the parser.
pub const SPELLED_WORDS: &[&str] = &["उत्तरम्", "दीयते", "दीयेते", "दीयन्ते", "प्रतिदीयते", "प्रतिदीयताम्", "शुद्धम्", "दृश्यम्"];

/// Every keyword string `classify_identifier` recognizes as a dedicated
/// `Token` variant rather than a generic `Identifier`/`Prathama`. Tests use
/// it to prove keyword protection holds for all of them, and
/// `crate::vocabulary` gives it to the editors. Keep in sync with
/// `classify_identifier`'s match arms below.
pub const ALL_KEYWORD_STRINGS: &[&str] = &[
    "अद्यतनक्षणः",
    "अद्यतनदिनाङ्कः",
    "अद्यतनसमयः",
    "अद्वितीयः",
    "अद्वितीयम्",
    "अद्वितीया",
    "अद्वितीयाः",
    "अद्वितीयानि",
    "अधिकः",
    "अधिकम्",
    "अधिका",
    "अनन्तरपङ्क्तेः",
    "अनिवार्यः",
    "अनिवार्यम्",
    "अनिवार्या",
    "अनिवार्याः",
    "अनिवार्याणि",
    "अनुमन्यताम्",
    "अनुरूपेण",
    "अनुवर्तितः",
    "अनुसारम्",
    "अन्यत्",
    "अन्यस्मिन्",
    "अन्यान्",
    "अपरिमितपूर्ववर्तितः",
    "अपरिमितानुवर्तितः",
    "अपसारेण",
    "अपि",
    "अभिधीयताम्",
    "अवरोहः",
    "अवरोहक्रमेण",
    "अवशिष्टम्",
    "अस्ति",
    "आख्या",
    "आधारे",
    "आधारेण",
    "आरभ्य",
    "आरोहः",
    "आरोहक्रमेण",
    "आवर्तिका",
    "आश्रित्य",
    "आह्रियताम्",
    "आह्रियन्ताम्",
    "आह्रियेताम्",
    "इति",
    "इत्यनया",
    "इत्यनयोः",
    "इत्यनेन",
    "इत्यस्मात्",
    "इत्यस्य",
    "इत्यस्याः",
    "इत्याभिः",
    "इत्यासाम्",
    "इत्यासु",
    "इत्येभिः",
    "इत्येषाम्",
    "इत्येषु",
    "एकवचनम्",
    "कार्यम्",
    "क्रमिकः",
    "क्रमिकम्",
    "क्रमिका",
    "क्रमिकाः",
    "क्रमिकाणि",
    "क्रमिकी",
    "क्रमेण",
    "क्षणः",
    "क्षणम्",
    "खण्डः",
    "गणना",
    "गणनायाः",
    "गुणनम्",
    "घात",
    "घातः",
    "च",
    "चक्रनिवारणेन",
    "चतुर्थांशाङ्कः",
    "चियन्ताम्",
    "चियेताम्",
    "चेत्",
    "तथा",
    "तस्मात्",
    "तस्मिन्",
    "तात्कालिकः",
    "तात्कालिकम्",
    "दक्षिणमेलनेन",
    "दिनम्",
    "दिनाङ्कः",
    "महावर्णरूपम्",
    "दैर्घ्यम्",
    "द्विवचनम्",
    "द्वैतम्",
    "न",
    "नपुंसकलिङ्गम्",
    "नास्ति",
    "निमेषः",
    "नियमेन",
    "निवर्त्यताम्",
    "प्रतिसंह्रियताम्",
    "निवेश्यताम्",
    "निवेश्यन्ताम्",
    "निवेश्येताम्",
    "निष्कास्यताम्",
    "न्यूनः",
    "न्यूनम्",
    "न्यूना",
    "पङ्क्तयः",
    "पङ्क्तिसङ्ख्या",
    "पङ्क्तीनाम्",
    "पटलम्",
    "परिणामः",
    "परिवर्त्यताम्",
    "परिवर्त्यन्ताम्",
    "परिवर्त्येताम्",
    "पर्यन्तम्",
    "पुंलिङ्गम्",
    "पुनरुक्तिम्",
    "पूर्णमेलनेन",
    "पूर्णाङ्कः",
    "पूर्वपङ्क्तेः",
    "पूर्ववर्तितः",
    "प्रति",
    "प्रतिच्छेदः",
    "प्रत्याह्रियताम्",
    "प्रदाय",
    "प्राथमिकः",
    "प्राथमिकम्",
    "प्राथमिकाः",
    "प्राथमिकानि",
    "प्राथमिकी",
    "प्राथमिक्यः",
    "प्रारभ्यताम्",
    "बद्धः",
    "बद्धम्",
    "बद्धा",
    "बद्धाः",
    "बद्धानि",
    "बहुवचनम्",
    "बाह्या",
    "भागाङ्कः",
    "मध्ये",
    "महत्तमम्",
    "महत्तमस्य",
    "माध्यम्",
    "माध्यस्य",
    "मासः",
    "मित्या",
    "मेलनेन",
    "यत्",
    "यत्र",
    "यस्मिन्",
    "योगः",
    "योगस्य",
    "योज्यताम्",
    "रच्यताम्",
    "रिक्तः",
    "रिक्तम्",
    "रिक्ता",
    "लघुतमम्",
    "लघुतमस्य",
    "लघुवर्णरूपम्",
    "लब्धम्",
    "योगात्",
    "योगम्",
    "योगेन",
    "गुणनात्",
    "गुणनेन",
    "अवशिष्टात्",
    "अवशिष्टेन",
    "लब्धात्",
    "लब्धेन",
    "गुणनस्य",
    "अवशिष्टस्य",
    "लब्धस्य",
    "गणनाम्",
    "माध्यात्",
    "महत्तमात्",
    "लघुतमात्",
    "गणनया",
    "माध्येन",
    "महत्तमेन",
    "लघुतमेन",
    "द्वितीयपूर्वपङ्क्तेः",
    "तृतीयपूर्वपङ्क्तेः",
    "चतुर्थपूर्वपङ्क्तेः",
    "पञ्चमपूर्वपङ्क्तेः",
    "षष्ठपूर्वपङ्क्तेः",
    "सप्तमपूर्वपङ्क्तेः",
    "अष्टमपूर्वपङ्क्तेः",
    "नवमपूर्वपङ्क्तेः",
    "दशमपूर्वपङ्क्तेः",
    "द्वितीयानन्तरपङ्क्तेः",
    "तृतीयानन्तरपङ्क्तेः",
    "चतुर्थानन्तरपङ्क्तेः",
    "पञ्चमानन्तरपङ्क्तेः",
    "षष्ठानन्तरपङ्क्तेः",
    "सप्तमानन्तरपङ्क्तेः",
    "अष्टमानन्तरपङ्क्तेः",
    "नवमानन्तरपङ्क्तेः",
    "दशमानन्तरपङ्क्तेः",
    "पूर्णाङ्करूपेण",
    "वाक्यरूपेण",
    "वास्तविकरूपेण",
    "दिनाङ्करूपेण",
    "द्वैतरूपेण",
    "इतीदम्",
    "लुप्यताम्",
    "वरणम्",
    "वर्जनम्",
    "वर्जयित्वा",
    "वर्णानाम्",
    "वर्तमानपङ्क्ति",
    "वर्तमानपङ्क्तिः",
    "वर्तमानपङ्क्तिम्",
    "वर्षम्",
    "वा",
    "वाक्यम्",
    "वाममेलनेन",
    "वारः",
    "वास्तविकः",
    "विकल्पः",
    "विद्यते",
    "विना",
    "विभज्य",
    "विभाजनम्",
    "विशदीक्रियताम्",
    "विशिष्टयोः",
    "विशिष्टस्य",
    "विशिष्टानाम्",
    "विशिष्टायाः",
    "विशोध्य",
    "वैकल्पिकः",
    "वैकल्पिकम्",
    "वैकल्पिका",
    "वैकल्पिकाः",
    "वैकल्पिकानि",
    "वैकल्पिकी",
    "वैकल्पिक्यः",
    "व्याप्तिः",
    "श्रेणी",
    "संयोजनम्",
    "सघनश्रेणी",
    "सङ्गमः",
    "सङ्ग्रहबिन्दुः",
    "सङ्ग्रहबिन्दुम्",
    "सञ्ज्ञा",
    "सदृशः",
    "सदृशम्",
    "सदृशी",
    "समः",
    "समम्",
    "समा",
    "समूहनियमेन",
    "समूहम्",
    "समूहाः",
    "समूहेन",
    "सम्बद्धान्",
    "सर्वत्र",
    "सर्वमेलनेन",
    "सर्वम्",
    "सर्वसङ्गमः",
    "सह",
    "सूची",
    "सूचीम्",
    "सूच्याः",
    "सूत्रम्",
    "स्त्रीलिङ्गम्",
    "स्थानात्",
    "स्थाप्यताम्",
    "स्थिरीक्रियताम्",
    "होरा",
];

/// Classify a Devanagari identifier into a more specific token type

/// The ordinal stems of LAG / LEAD offsets 2..10.
const ORDINALS: &[(u32, &str)] = &[
    (2, "द्वितीय"),
    (3, "तृतीय"),
    (4, "चतुर्थ"),
    (5, "पञ्चम"),
    (6, "षष्ठ"),
    (7, "सप्तम"),
    (8, "अष्टम"),
    (9, "नवम"),
    (10, "दशम"),
];

fn ordinal_stem(n: u32) -> &'static str {
    ORDINALS.iter().find(|(k, _)| *k == n).map(|(_, s)| *s).unwrap_or("")
}

pub(crate) fn classify_identifier(text: &str) -> Token {
    if text.starts_with("ऋणात्म") && crate::morphology::negative_words().iter().any(|w| w == text) {
        return Token::Rinatmaka(text.to_string());
    }
    for (n, stem) in ORDINALS {
        if text.strip_prefix(stem) == Some("पूर्वपङ्क्तेः") {
            return Token::PurvaPanktehN(*n);
        }
        if text.strip_prefix(stem).and_then(|r| r.strip_prefix("ा")) == Some("नन्तरपङ्क्तेः") {
            return Token::AnantaraPanktehN(*n);
        }
    }
    match text {
        // Keywords (New additions for text-based classification)
        "इति" => Token::Iti,
        "च" => Token::Cha,
        "वा" => Token::Va,
        "चेत्" => Token::Chet,
        "सङ्गमः" => Token::Sangamah,
        "सर्वसङ्गमः" => Token::SarvaSangamah,
        "आश्रित्य" => Token::Ashritya,
        "सञ्ज्ञा" => Token::Sanjna,
        "आख्या" => Token::Aakhya,
        "प्रारभ्यताम्" => Token::BeginVerb,
        "स्थिरीक्रियताम्" => Token::CommitVerb,
        "प्रत्याह्रियताम्" => Token::RollbackVerb,
        "सङ्ग्रहबिन्दुः" => Token::Sangrahabinduh,
        "आवर्तिका" => Token::Aavartika,
        "चक्रनिवारणेन" => Token::ChakraNivaranena,
        
        // DDL Keywords
        "अनिवार्यः" | "अनिवार्या" | "अनिवार्यम्" | "अनिवार्याः" | "अनिवार्याणि" => Token::Anivaryah,
        "वैकल्पिकः" | "वैकल्पिकी" | "वैकल्पिकम्" | "वैकल्पिकाः" | "वैकल्पिक्यः" | "वैकल्पिकानि" | "वैकल्पिका" => Token::Vaikalpikah,
        "प्राथमिकः" | "प्राथमिकी" | "प्राथमिकम्" | "प्राथमिकाः" | "प्राथमिक्यः" | "प्राथमिकानि" | "प्राथमिका" => Token::Prathamikah,
        "क्रमिकः" | "क्रमिका" | "क्रमिकम्" | "क्रमिकाः" | "क्रमिकाणि" | "क्रमिकी" => Token::Kramikah,
        "अद्वितीयः" | "अद्वितीया" | "अद्वितीयम्" | "अद्वितीयाः" | "अद्वितीयानि" => Token::Advitiyah,
        "पुंलिङ्गम्" => Token::Pumlingam,
        "स्त्रीलिङ्गम्" => Token::Strilingam,
        "नपुंसकलिङ्गम्" => Token::Napumsakalingam,
        "एकवचनम्" => Token::Ekavachanam,
        "द्विवचनम्" => Token::Dvivachanam,
        "बहुवचनम्" => Token::Bahuvachanam,
        "बद्धः" | "बद्धा" | "बद्धम्" | "बद्धाः" | "बद्धानि" => Token::Baddhah,
        "तात्कालिकम्" | "तात्कालिकः" => Token::Tatkalikam,
        "पटलम्" => Token::Patalam,
        "प्रति" => Token::Prati,
        "सूत्रम्" => Token::Sutram,
        "गुणनम्" => Token::Gunanam,
        "कार्यम्" => Token::Karyam,
        "प्रदाय" => Token::Pradaya,
        "यत्" => Token::Yat,
        "सूची" | "सूचीम्" | "सूच्याः" => Token::Suchi,

        "नियमेन" => Token::Niyamena,
        "समूहेन" => Token::Samuhena,
        "समूहनियमेन" => Token::Samuhaniyamena,
        "क्रमेण" => Token::Kramena,
        "मेलनेन" => Token::Melanena,
        "मित्या" => Token::Mitya,
        "अपसारेण" => Token::Apasarena,
        "विशिष्टस्य" | "विशिष्टायाः" | "विशिष्टयोः" | "विशिष्टानाम्"          => Token::Vishishtasya(text.to_string()),
        "सर्वम्" => Token::Sarvam,
        "सह" => Token::Saha,
        "सदृशम्" | "सदृशः" | "सदृशी" => Token::Sadrusham(text.to_string()),
        "?" => Token::Placeholder,
        "बाह्या" => Token::Bahya,
        
        // Verbs
        "आह्रियताम्" | "आह्रियेताम्" | "आह्रियन्ताम्" => Token::FetchVerb(text.to_string()),
        "निवेश्यताम्" | "निवेश्येताम्" | "निवेश्यन्ताम्" => Token::InsertVerb(text.to_string()),
        "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्" => Token::UpdateVerb(text.to_string()),
        "निष्कास्यताम्" => Token::DeleteVerb,
        "रच्यताम्" => Token::CreateVerb,
        "स्थाप्यताम्" => Token::SavepointVerb,
        "योज्यताम्" => Token::AddVerb,
        "अभिधीयताम्" => Token::AbhidhiyatamVerb,
        "लुप्यताम्" => Token::DropVerb,
        "विद्यते" => Token::Vidyate,
        "अनुमन्यताम्" => Token::GrantVerb,
        // REVOKE: `प्रतिसंह्रियताम्` ("let it be withdrawn"), preferred, and
        // `निवर्त्यताम्` (pcf.md item 3).
        "प्रतिसंह्रियताम्" | "निवर्त्यताम्" => Token::RevokeVerb,
        "विशदीक्रियताम्" => Token::Explain,
        
        "पूर्णाङ्कः" => Token::Purnankah,
        "वाक्यम्" => Token::Vakyam,
        "वास्तविकः" => Token::Vastavikah,
        "दिनाङ्कः" => Token::Dinankah,
        "द्वैतम्" => Token::Dvaitam,
        "पूर्णाङ्करूपेण" => Token::PurnankaRupena,
        "वाक्यरूपेण" => Token::VakyaRupena,
        "वास्तविकरूपेण" => Token::VastavikaRupena,
        "दिनाङ्करूपेण" => Token::DinankaRupena,
        "द्वैतरूपेण" => Token::DvaitaRupena,
        
        // Comparison
        "अधिकम्" | "अधिकः" | "अधिका" => Token::Adhikam(text.to_string()),
        "न्यूनम्" | "न्यूनः" | "न्यूना" => Token::Nyunam(text.to_string()),
        "समम्" | "समः" | "समा" => Token::Samam(text.to_string()),
        "अस्ति" => Token::Asti,
        "नास्ति" => Token::Nasti,
        "रिक्तः" | "रिक्ता" | "रिक्तम्" => Token::Riktah(text.to_string()),
        
        // Window
        "यत्र" => Token::Yatra,
        "विभाजनम्" => Token::Vibhajanam,

        "तथा" => Token::Tatha,
        "श्रेणी" => Token::Shreni,
        "पङ्क्तिसङ्ख्या" => Token::PanktiSankhya,
        "सङ्ग्रहबिन्दुम्" => Token::SangrahabindumDvitiya,
        "विना" => Token::Vina,
        "पुनरुक्तिम्" => Token::Punaruktim,
        "परिणामः" => Token::Parinamah,
        "आधारे" => Token::Aadhare,
        "लघुतमस्य" => Token::LaghutamasyaGen,
        "महत्तमस्य" => Token::MahattamasyaGen,
        "माध्यस्य" => Token::MadhyasyaGen,
        "गणनायाः" => Token::GananayahGen,
        "योगस्य" => Token::YogasyaGen,
        "पङ्क्तीनाम्" => Token::Panktinam,
        "सर्वत्र" => Token::Sarvatra,
        "भागाङ्कः" => Token::Bhagankah,
        "चतुर्थांशाङ्कः" => Token::Chaturthamshankah,
        "अनन्तरपङ्क्तेः" => Token::AnantaraPankteh,
        "पूर्वपङ्क्तेः" => Token::PurvaPankteh,
        "सघनश्रेणी" => Token::SaghanaShreni,
        "आरोहः" => Token::Arohah,
        "अवरोहः" => Token::Avarohah,
        "आधारेण" => Token::Aadharena,
        "अनुसारम्" => Token::Anusaram,
        "आरोहक्रमेण" => Token::ArohaKramena,
        "अवरोहक्रमेण" => Token::AvarohaKramena,
        "वाममेलनेन" => Token::VamaMelanena,
        "दक्षिणमेलनेन" => Token::DakshinaMelanena,
        "पूर्णमेलनेन" => Token::PurnaMelanena,
        "सर्वमेलनेन" => Token::SarvaMelanena,
        
        // Dates
        "वर्षम्" => Token::Varsham,
        "मासः" => Token::Masah,
        "दिनम्" => Token::Dinam,
        "होरा" => Token::Hora,
        "निमेषः" => Token::Nimesah,
        "क्षणः" | "क्षणम्" => Token::Vipalam,
        "अद्यतनक्षणः" => Token::Samprati,
        // Dedicated zero-arg keyword tokens mirroring सम्प्रति's own pattern
        // exactly, so CURRENT_DATE/CURRENT_TIME are genuinely reachable - a
        // bare reference left as a generic identifier would fall through as
        // an ordinary, nonexistent column lookup, and the postfix-call
        // grammar requires at least one argument, so neither builtin would
        // have any working call syntax without a dedicated token.
        "अद्यतनदिनाङ्कः" => Token::AdyatanaDinankah,
        "अद्यतनसमयः" => Token::AdyatanaSamayah,
        
        // Aggregates
        "योगः" => Token::Yogah,
        "गणना" => Token::Ganana,
        "माध्यम्" => Token::Madhyam,
        "महत्तमम्" => Token::Mahattam,
        "लघुतमम्" => Token::Laghutamam,
        "अन्यत्" => Token::Anyat,
        "अपि" => Token::Api,

        // Set Ops
        "प्रतिच्छेदः" => Token::Sammilanam,
        "वर्जनम्" => Token::Varjanam,

        // Scalar
        "संयोजनम्" => Token::Samyojanam,
        "खण्डः" => Token::Khandah,
        "दैर्घ्यम्" => Token::Aayamah,
        "महावर्णरूपम्" => Token::Mahavarnarupam,
        "लघुवर्णरूपम्" => Token::Laghuvarnarupam,
        "विकल्पः" => Token::Vikalpah,
        // Reserved keywords for the locative-source chained SUBSTR/INSTR
        // construct ("विवरणे \"ERR\" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः").
        // Deliberately dedicated Token variants (not left as generic
        // vibhakti-tagged vocabulary filtered by pratipadika deep in the
        // parser) so स्थान/वर्ण are genuinely reserved and can never
        // collide with a user-declared column/table stem - see
        // reserved_words.rs's BUILTIN_FUNCTION_WORDS for the matching
        // stem-level reservation.
        "स्थानात्" => Token::Sthaanaat,
        "वर्णानाम्" => Token::Varnaanaam,
        "वारः" => Token::Vaarah,
        // NOTE: deliberately NOT a dedicated keyword token, unlike words such
        // as नाम/तस्मात् that need one to avoid a field-access-vs-marker
        // ambiguity. "घातः"/"घात"/"घातीकरणम्" is the built-in POWER function,
        // called like any other (`मूल्यस्य २ इत्यनेन घातः`); only the surface
        // form is canonicalized here so all its variants resolve to one name.
        "घातः" => Token::Identifier("घातः".to_string()),
        "अनुरूपेण" => Token::Anurupena,

        // IN/BETWEEN / Utils
        "इत्येषु" | "इत्यासु" => Token::Ityeshu(text.to_string()),
        "इत्यनयोः" => Token::Ityayoh(text.to_string()),
        "मध्ये" => Token::Madhye,
        "इत्यस्मात्" => Token::Ityasmat(text.to_string()),
        "तस्मात्" => Token::Tasmat,
        "इत्यस्य" => Token::Ityasya(text.to_string()),
        "इत्यस्याः" => Token::Ityasyah(text.to_string()),
        "इत्येषाम्" => Token::Ityesham(text.to_string()),
        "इत्यासाम्" => Token::Ityetasam(text.to_string()),
        // Window Frame Specification
        "पङ्क्तयः" => Token::Panktayah,
        "व्याप्तिः" => Token::Vyaptih,
        "समूहाः" => Token::Samuhah,
        "अपरिमितपूर्ववर्तितः" => Token::AparimitaPurvavartitah,
        "पूर्ववर्तितः" => Token::Purvavartitah,
        "वर्तमानपङ्क्तिः" | "वर्तमानपङ्क्तिम्" | "वर्तमानपङ्क्ति" => Token::Vartamanapanktih,
        "अनुवर्तितः" => Token::Anuvartitah,
        "अपरिमितानुवर्तितः" => Token::AparimitaAnuvartitah,
        "आरभ्य" => Token::Aarabhya,
        "पर्यन्तम्" => Token::Paryantam,
        "वर्जयित्वा" => Token::Varjayitva,
        "समूहम्" => Token::Samuham,
        "सम्बद्धान्" => Token::Sambaddhan,
        "अन्यान्" => Token::Anyan,
        "न" => Token::Na,
        
        // Math Keywords
        "विशोध्य" => Token::Nivarya,
        "अवशिष्टम्" => Token::Avashistam,
        "विभज्य" => Token::Vibhajya,
        "लब्धम्" => Token::Labdham,
        "योगात्" => Token::YogatAbl,
        "योगम्" => Token::YogamAcc,
        "योगेन" => Token::YogenaIns,
        "गुणनात्" => Token::GunanatAbl,
        "गुणनेन" => Token::GunanenaIns,
        "अवशिष्टात्" => Token::AvashistatAbl,
        "अवशिष्टेन" => Token::AvashistenaIns,
        "लब्धात्" => Token::LabdhatAbl,
        "लब्धेन" => Token::LabdhenaIns,
        "गुणनस्य" => Token::GunanasyaGen,
        "अवशिष्टस्य" => Token::AvashistasyaGen,
        "लब्धस्य" => Token::LabdhasyaGen,
        "गणनाम्" => Token::GananamAcc,
        "माध्यात्" => Token::MadhyatAbl,
        "महत्तमात्" => Token::MahattamatAbl,
        "लघुतमात्" => Token::LaghutamatAbl,
        "गणनया" => Token::GananayaIns,
        "माध्येन" => Token::MadhyenaIns,
        "महत्तमेन" => Token::MahattamenaIns,
        "लघुतमेन" => Token::LaghutamenaIns,
        "इतीदम्" => Token::Itidam,

        // Explicit Tritiya mapping for Iti variants (Instrumental connector)
        "इत्यनेन" | "इत्यनया" => Token::Tritiya {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika: "इति".to_string(),
        },
        "इत्येभिः" | "इत्याभिः" => Token::Tritiya {
            text: text.to_string(),
            vachana: Vachana::Bahu,
            pratipadika: "इति".to_string(),
        },

        // CASE
        "वरणम्" => Token::VaraNam,
        "यस्मिन्" => Token::Yasmin,
        "तस्मिन्" => Token::Tasmin,
        "अन्यस्मिन्" => Token::Anyasmin,

        // Default: Identifier (Wait for Annotator or Heuristic fallback later)
        _ => Token::Identifier(text.to_string()),
    }
}

/// Classify identifier by its grammatical ending

pub(crate) fn classify_by_ending(text: &str, preset_pratipadika: Option<String>) -> Token {
    if text == "घातः" {
        return Token::Identifier("घातः".to_string());
    }

    // Panchami (FROM) endings: -आत्, -एभ्यः, -भ्यः
    if text.ends_with("ात्") {
         return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("ात्").to_string()
        };
    }
    if text.ends_with("एभ्यः") {
         return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("एभ्यः").to_string()
        };
    }
    if text.ends_with("ेभ्यः") {
         return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("ेभ्यः").to_string()
        };
    }
    if text.ends_with("भ्यः") {
         return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("भ्यः").to_string()
        };
    }

    // Tritiya (BY/WITH) endings: -एन, -ऐः, -भिः
    if text.ends_with("एन") {
        return Token::Tritiya {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("एन").to_string()
        };
    }
    if text.ends_with("ेन") {
        return Token::Tritiya {
             text: text.to_string(),
             vachana: Vachana::Eka,
             pratipadika: text.trim_end_matches("ेन").to_string()
        };
    }
    if text.ends_with("एण") {
        return Token::Tritiya {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("एण").to_string()
        };
    }
    if text.ends_with("ेण") {
        return Token::Tritiya {
             text: text.to_string(),
             vachana: Vachana::Eka,
             pratipadika: text.trim_end_matches("ेण").to_string()
        };
    }
    if text.ends_with("ैः") {
        // Singular stem + aiH -> Bahu?
        // e.g. Balakaih -> Balaka
        return Token::Tritiya { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("ैः").to_string()
        };
    }
    if text.ends_with("भिः") {
        return Token::Tritiya { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("भिः").to_string()
        };
    }
    
    // Panchami (FROM) endings: -एभ्यः, -भ्यः
    if text.ends_with("ेभ्यः") {
        return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("ेभ्यः").to_string()
        };
    }
    if text.ends_with("भ्यः") {
        return Token::Panchami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("भ्यः").to_string()
        };
    }

    // Shashthi (OF) endings: -स्य, -योः, -आनाम्, -ः
    if text.ends_with("स्य") {
        return Token::Shashthi { 
            text: text.to_string(), 
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("स्य").to_string()
        };
    }
    if text.ends_with("योः") {
        return Token::Shashthi { 
            text: text.to_string(), 
            vachana: Vachana::Dvi,
            pratipadika: text.trim_end_matches("योः").to_string()
        };
    }
    if text.ends_with("नाम्") || text.ends_with("नां") || text.ends_with("णाम्") || text.ends_with("णां") {
        let pratipadika = if text.ends_with("ानाम्") || text.ends_with("ाणाम्") {
            let s = text.trim_end_matches("ानाम्").trim_end_matches("ाणाम्");
            s.to_string()
        } else if text.ends_with("ानां") || text.ends_with("ाणां") {
            let s = text.trim_end_matches("ानां").trim_end_matches("ाणां");
            s.to_string()
        } else if text.ends_with("िनाम्") || text.ends_with("िणाम्") {
            let s = text.trim_end_matches("िनाम्").trim_end_matches("िणाम्");
            s.to_string() + "िन्"
        } else if text.ends_with("िनां") || text.ends_with("िणां") {
            let s = text.trim_end_matches("िनां").trim_end_matches("िणां");
            s.to_string() + "िन्"
        } else if text.ends_with("ीनाम्") || text.ends_with("ीनां") {
            let s = text.trim_end_matches("ीनाम्").trim_end_matches("ीनां");
            s.to_string() + "ी"
        } else if text.ends_with("ूणाम्") || text.ends_with("ूणां") {
            let s = text.trim_end_matches("ूणाम्").trim_end_matches("ूणां");
            s.to_string() + "ु"
        } else if text.ends_with("ृणाम्") || text.ends_with("ृणां") {
            let s = text.trim_end_matches("ृणाम्").trim_end_matches("ृणां");
            s.to_string() + "ृ"
        } else {
            text.to_string()
        };
        return Token::Shashthi { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika,
        };
    }
    // Specific handling for 'naḥ' (common Shashthi/Panchami for u-stems like 'vastunaḥ')
    if text.ends_with("नः") {
        return Token::Shashthi {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("नः").to_string(), 
        };
    }

    // Specific handling for 'taḥ' (suffix -tah, e.g. Mulyatah) -> Panchami (From...)
    if text.ends_with("तः") && !text.ends_with("ेतः") {
        return Token::Panchami {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("तः").to_string(),
        };
    }

    // Specific handling for 'yāḥ' (common Shashthi/Panchami for Fem stems)
    if text.ends_with("याः") && (text.ends_with("ायाः") || text.ends_with("्याः")) {
        let pratipadika = if text.ends_with("ायाः") {
            // A-stem (e.g. Latā -> Latāyāḥ). Just trim 'yāḥ'.
            text.trim_end_matches("याः").to_string()
        } else {
            // I-stem (e.g. Vastusūcī -> Vastusūcyāḥ). Trim 'yāḥ' and optional virama, add 'ī'.
            let stem = text.trim_end_matches("याः");
            stem.trim_end_matches("्").to_string() + "ी"
        };
        
        return Token::Shashthi {
            text: text.to_string(),
            vachana: Vachana::Eka,
            pratipadika,
        };
    }

    if text.ends_with("ः") {
        // Visarga (ः)
        // - 'aḥ' (e.g. Rāmaḥ, Dināṅkaḥ) -> Prathama Eka (Masculine)
        // - 'iḥ' (e.g. Matiḥ) -> Prathama Eka (Feminine) or Shashthi/Panchami
        // Default to Prathama as it's the safest bet for Identifiers causing parse errors otherwise.
        return Token::Prathama { 
            text: text.to_string(), 
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("ः").to_string()
        };
    }
    
    // Saptami (IN/ON) endings: -ए, -योः, -एषु
    if text.ends_with("े") && !text.ends_with("एन") && !text.ends_with("ेन") { // Prevent overlap with Tritiya -ena if logic permits
        return Token::Saptami { 
            text: text.to_string(), 
            vachana: Vachana::Eka,
            pratipadika: text.trim_end_matches("े").to_string()
        };
    }
    if text.ends_with("एषु") {
        return Token::Saptami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("एषु").to_string()
        };
    }
    if text.ends_with("ेषु") {
        return Token::Saptami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("ेषु").to_string()
        };
    }
    if text.ends_with("ासु") {
        return Token::Saptami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("ासु").to_string() + "ा"
        };
    }
    if text.ends_with("सु") {
        return Token::Saptami { 
            text: text.to_string(), 
            vachana: Vachana::Bahu,
            pratipadika: text.trim_end_matches("सु").to_string()
        };
    }
    
    // Default: Prathama (subject/column)
    let pratipadika = if let Some(p) = preset_pratipadika {
        p
    } else {
        // Try stripping common Prathama suffixes for pratipadika identification
        if text.ends_with("ः") {
            text.trim_end_matches("ः").to_string()
        } else if text.ends_with("म्") {
            text.trim_end_matches("म्").to_string()
        } else if text.ends_with("ा") { // Feminine singular
            text.trim_end_matches("ा").to_string()
        } else {
            text.to_string()
        }
    };
    
    Token::Prathama { 
        text: text.to_string(), 
        vachana: Vachana::Eka, // Assume eka by default for now
        pratipadika,
    }
}

### **SSQL (संस्कृतम् SQL): The v1.0 Design Principles (Constitution)**

#### **१. `मूलदृष्टिः` (The Core Vision): Relational Querying as Formal Instruction**
SSQL is designed as a **formal grammatical instruction** between a `स्थपतिः` (database architect) and a `सहायकः` (relational engine), expressed in valid **Samskrutam** using the passive imperative mood (`कर्मणि लोट् लकारः`, such as `आह्रियताम्`, `रच्यताम्`, `निवेश्यताम्`, `परिवर्त्यताम्`, and `निष्कास्यताम्`).

#### **२. `लौकिकक्षमता` (Real-World Capability)**
SSQL is not a theoretical or toy query notation. Every design decision is guided by the principle that the language must be a **powerful, practical, and robust tool for real-world, production-grade relational databases**.
* **`युक्तिः` (Rationale):** Queries written in SSQL compile deterministically to standard **SQLite** and **PostgreSQL**, supporting multi-table joins, aggregations, window functions (`OVER`, `PARTITION BY`, `ORDER BY`), Common Table Expressions (`WITH` / `WITH RECURSIVE`), set operations (`UNION`, `INTERSECT`, `EXCEPT`), views, indexes, and pure user-defined functions (`शुद्धम् कार्यम्`), alongside native Rust, Python (`ssql-py`), WebAssembly (`ssql-wasm`), and Language Server (`ssql-lsp`) tooling.

#### **३. `अभिगम्यता तथा शिक्षणशास्त्रम्` (Accessibility and Pedagogy)**
The language is intentionally designed to be **accessible both to SQL developers and to a Sanskrit-speaking audience**, lowering the barrier to entry by aligning relational operations with classical Pāṇinian *Kāraka* theory and using **consistent grammatical patterns** (`[Name] इति [Type]`, `[Condition] इति नियमेन`, `[Column] इति समूहेन`) to reduce cognitive load.

#### **४. `वाचिकता तथा शुद्धता` (Speakability and Purity): The Law of Graphemic Purity**
The core syntax of SSQL shall be composed **exclusively of Devanāgarī words, Devanāgarī numerals (`०`–`९`), and the daṇḍa (`।`)** with **zero English loan-words** (`SELECT`, `FROM`, `WHERE`, `INTEGER`, `VARCHAR`, etc. are strictly forbidden in SSQL syntax).
* **Permitted Graphemic Exceptions (v1.0):**
  1. String Delimiters (`" "`) for string literals and physical SQL table/column bindings (`"books" इति आख्या`).
  2. Single-Line Comments (`//` or `--`).
  3. Multi-Line Comments (`/* ... */`).
  4. Decimal Separator (`.`) for `वास्तविकः` numeric literals (`०.९०`).
  5. Escape Sequences inside string literals.
  6. The minus sign of a negative constant, written against its digits (`-५`); a negative column takes the declined adjective instead (`ऋणात्मकम् मूल्यम्`).
* **No parentheses:** SSQL is written without parentheses. Where a group is genuinely needed it is quoted with `यत् … इति`, and a longer value is named first (`इति आख्या`).

#### **५. `स्पष्टतायाः सिद्धान्तः` (The Principle of Clarity)**
Deeply nested, opaque sub-expressions are discouraged. Complex analytical queries should be decomposed into **named Common Table Expressions (`इति आख्या` / `इति सञ्ज्ञा`) or modular views (`इति दृश्यम्`)**, making relational dataflow self-documenting and straightforward to inspect.

#### **६. `शुद्धप्रवाहस्य शैली` (The Pure Flow Style)**
User-defined functions (`शुद्धम् कार्यम्`) in SSQL follow a **pure, side-effect-free relational style** with a clear parameter signature (the passive `दीयते`/`दीयेते`/`दीयन्ते`, agreeing in number with the parameters) and a single return expression (`इति उत्तरम् प्रतिदीयताम्`).
* **`युक्तिः` (Rationale):** Pure scalar functions can be safely inlined into SQLite expressions and compiled into `IMMUTABLE` SQL functions on PostgreSQL without hidden state mutations.

#### **७. `व्याकरणाधारितनिर्माणम्` (Grammar-Based Construction)**
Relational semantics in SSQL are directly encoded by the **grammatical engine of Samskrutam**, using **Vibhakti (nominal case)** and **Kāraka (semantic role)** rather than rigid positional keywords:
* **Vibhakti-to-Relational Mapping:**
  * **`प्रथमा` (1st, Nominative — `उक्तकर्म`):** Projected `SELECT` columns, aliases (`इति`), and the subject of passive verbs (`शीर्षकम् च मूल्यम् च आह्रियेताम्`).
  * **`द्वितीया` (2nd, Accusative — `कर्म`):** Direct object of non-finite/absolutive verbs (`अपकर्षम् विशोध्य अवशिष्टम्`, `मूल्यम् च ५ च प्रदाय`).
  * **`तृतीया` (3rd, Instrumental — `करणम् / सहार्थे`):** Associated table in `JOIN` (`आपणैः सह ... इति मेलनेन`), filtering rule (`इति नियमेन`), grouping (`इति समूहेन`), ordering (`इति क्रमेण`), equality comparison (`इत्यनेन समम्`, the तृतीया with a word of equality rather than a kāraka), and division divisor (`परिमाणेन विभज्य लब्धम्`).
  * **`चतुर्थी` (4th, Dative — `सम्प्रदानम्`):** Beneficiary or target recipient of an operation.
  * **`पञ्चमी` (5th, Ablative — `अपादानम्`):** Source table or CTE in `FROM` (`पुस्तकेभ्यः`, `सारात्`), greater/less-than comparison baseline (`२० इत्यस्मात् अधिकम्`, the ablative of comparison rather than a kāraka), and subtraction minuend (`मूल्यात् ५ विशोध्य अवशिष्टम्`).
  * **`षष्ठी` (6th, Genitive — `सम्बन्धः`):** Qualified table-column ownership (`पुस्तकस्य शीर्षकम्`) and aggregate/function operand (`मूल्यस्य योगः`, `सङ्ख्यायाः गणना`).
  * **`सप्तमी` (7th, Locative — `अधिकरणम्`):** Target table locus for `INSERT INTO` (`पुस्तकेषु ... निवेश्यताम्`) and `UPDATE` (`छात्रेषु ... परिवर्त्यताम्`).

#### **८. `सुरक्षा तथा पूर्वानुमानम्` (Safety and Predictability)**
The SSQL compiler enforces **static grammatical and relational type safety** before any SQL is emitted:
* **`युक्तिः` (Rationale):** This is achieved through:
  1. **Three-Gender Concordance (`लिङ्गसमन्वयः`):** Every column in `schema.सं` declares its grammatical gender (`पुंलिङ्गम्`, `स्त्रीलिङ्गम्`, or `नपुंसकलिङ्गम्`), and the validator verifies that comparison pronouns (`इत्यनेन` vs. `इत्यनया`, `इत्यस्मात्` vs. `इत्यस्याः`), predicate adjectives (`समः`/`समा`/`समम्`, `अधिकः`/`अधिका`/`अधिकम्`, `रिक्तः`/`रिक्ता`/`रिक्तम्`), and DDL constraints agree with their referent column.
  2. **Subject–Verb Number Agreement (`वचनसमन्वयः`):** The terminal *Karmaṇi Loṭ* verb must match the grammatical number of projected columns: singular `आह्रियताम्` (1 column), dual `आह्रियेताम्` (2 columns), or plural `आह्रियन्ताम्` (3+ columns); `सर्वम्`, all columns, is neuter singular and takes `आह्रियताम्`.
  3. **Explicit Nullability & Schema Resolution:** Null checks (`रिक्तः`/`रिक्ता`/`रिक्तम् अस्ति`) and column references are statically verified against `schema.सं`.

#### **९. `विस्तारणीयता तथा पारिस्थितिकी` (Extensibility and Ecosystem)**
SSQL pairs a **small, principled grammatical core** with a full developer ecosystem: bidirectional SQL-to-SSQL transpilation (`--from-sql`), canonical formatting (`--fmt`), Pāṇinian *padapāṭha* linting (`--lint`), the *Adhyetā* word-by-word grammar breakdown in the playground, and the `ssql-lsp` Language Server for modern editors.

#### **१०. `परिवर्तनस्य विधानम्` (The Law of Change)**
SSQL is a living specification that evolves with care and deliberation. All proposed changes to the v1.0 grammar or vocabulary must be introduced via a formal proposal (`सम्भाषणपत्रम्` — Discourse Paper / RFC), ensuring that grammatical soundness, Pāṇinian concordance, and backward compatibility across the example verification corpus are preserved.

This is our constitution.
It serves as both an architectural charter for the SSQL compiler and an invitation to explore relational computing through the precision and beauty of Samskrutam.

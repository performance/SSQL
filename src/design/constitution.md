### **The Samskrtam Programming Language: The v1.0 Design Principles (Constitution)**

#### **१. `मूलदृष्टिः` (The Core Vision): Programming as Formal Instruction**
The language is designed as a **formal conversation** between a `स्थपतिः` (master architect) and a `सहायकः` (capable assistant), using a consistent grammatical voice (`कर्मणि लोट् लकार` and related formal moods).

#### **२. `लौकिकक्षमता` (RealWorld Capability)**
This is not a theoretical or toy language. Every design decision is guided by the principle that the language must be a **powerful, practical, and robust tool for building realworld, productiongrade software**.
*   **`युक्तिः` (Rationale):** This principle is our "Prime Directive," ensuring seamless interoperability with modern ecosystems (like NPM) via a robust **Foreign Function Interface (FFI)**, a firstclass **`async/await` system** for concurrency, and a **Module System** for scalable codebases. Future compilation targets may include WebAssembly or other IRs to further enhance production viability.

#### **३. `अभिगम्यता तथा शिक्षणशास्त्रम्` (Accessibility and Pedagogy)**
The language is intentionally designed to be **accessible to a nonprogramming, Sanskritspeaking audience**, lowering the barrier to entry by aligning with existing linguistic knowledge and using **Consistent Patterns** (`[Name] इति [Type]`, etc.) to reduce cognitive load.

#### **४. `वाचिकता तथा शुद्धता` (Speakability and Purity): The Law of Graphemic Purity**
The core syntax shall be composed **exclusively of Devanagari words and the Purnadanda (`।`)**.
*   **The Seven Permitted Exceptions (v1.0):**
    1.  String Delimiters (`" "`)
    2.  SingleLine Comments (`//`)
    3.  MultiLine Comments (`/* ... */`)
    4.  Decimal Separator (`.`) for `वास्तविकः` literals.
    5.  Negative number literals (`-३.१४`)।
    6.  Escape Sequences and templated strings. (`"/न/"`, `*छात्रनाम*`)।
    7.  **Parentheses (`( )`)**: A pragmatic concession in v1.0 for the unambiguous grouping of expressions. This is a primary target for elimination in future versions.
*   **FutureProofing Clause:** Any future exceptions to this law must be formally added to this list with a clear rationale and must not break existing code.

#### **५. `स्पष्टतायाः सिद्धान्तः` (The Principle of Clarity)**
Complex, nested expressions are forbidden. All nontrivial computations must be broken down into a **sequence of simple, named steps.** This makes the code selfdocumenting and easier to debug.

#### **६. `शुद्धप्रवाहस्य शैली` (The Pure Flow Style)**
The recommended idiomatic style for writing functions is to have a **single entry point and a single exit point.**
*   **`युक्तिः` (Rationale):** This promotes readable and traceable control flow. This style is paired with our error handling system; instead of throwing exceptions, functions that can fail should return a `परिणामः` value, which is then handled by the caller.

#### **७. `व्याकरण आधारित निर्माणम्` (GrammarBased Construction)**
The language's logic is fundamentally tied to the **grammatical engine of Sanskrit**, using **Vibhakti (grammatical case)** to convey the semantic intent of operations.
*   **Vibhakti Usage Table (NonExhaustive):**
    *   **`प्रथमा` (Nominative):** The subject of an action.
    *   **`द्वितीया` (Accusative):** The object of an absolutive/gerund verb - not a main verb, which is passive (`कर्मणि लोट्`) and takes a nominative subject instead (e.g. subtraction's subtrahend: `सकलविक्रयात् अपकर्षम् विशोध्य अवशिष्टम्`, "from gross sales, deductions having been subtracted, the remainder").
    *   **`तृतीया` (Instrumental):** Comparison "with" (`… इत्यनेन समः अस्ति`). Higherorder functions "by means of" (`नियमेन निस्यन्दनम्`).
    *   **`चतुर्थी` (Dative):** Recipient, beneficiary, or purpose of the action (e.g., `सन्देशस्य स्तराय विवरणलेखनम्`)
    *   **`पञ्चमी` (Ablative):** Comparison "than" (`...स्मात् अधिकः अस्ति`).
    *   **`षष्ठी` (Genitive):** Possession or propertylike access (`सरणी इत्यस्य दैर्घ्यम्`).
    *   **`सप्तमी` (Locative):** Inplace operations "in" or "on" (`सरण्याम् उपस्थितिः`).

#### **८. `सुरक्षा तथा पूर्वानुमानम्` (Safety and Predictability)**
The language is designed to eliminate entire classes of common programming errors.
*   **`युक्तिः` (Rationale):** This is achieved through:
    1.  A **`null`free design**.
    2.  The mandatory use of the **`परिणामः` (Result) type** for all fallible operations. **असुरक्षितम् ** blocks make developer intent explicit e.g. for unchecked indexed access or foreign function calls.
    3.  **Compilerenforced exhaustiveness checks** in **`परीक्षा` (Pattern Matching)** blocks, which drastically improve safety at the cost of slightly longer compilation times.

#### **९. `विस्तारणीयता तथा पारिस्थितिकी` (Extensibility and Ecosystem)**
The language is designed with a **small, stable core** and a **rich, extensible library system**, anchored by the **`मानकग्रन्थालयः` (Standard Library)** and a module system (`आश्रित्य`/`निर्यातय`).

#### **१०. `परिवर्तनस्य विधानम्` (The Law of Change)**
The language is intended to be a living system that evolves with care and deliberation. All proposed changes to the v1.0 specification must be introduced via a formal proposal process (`सम्भाषणपत्रम्`  "Discourse Paper" / RFC), ensuring that the community can review and discuss changes before they are adopted. This protects the stability of the language and its ecosystem.
Philosophical Anchors : All proposed changes must align with one or more of the existing principles, and if they contradict a principle, the principle must be amended through a separate RFC.
All proposed changes must reference which existing principle(s) they align with, extend or reinterpret. Any contradiction with an existing principle requires an explicit proposal to amend or supersede that principle.


 This is our constitution. 
 It is intended to be a rigorous, and inspiring document that will serve as a guide for the immense and exciting work ahead.
 We hope this Constitution serves not only as a foundation for implementation, but as an invitation for all who share in this vision to build, refine, and carry it forward with enthusiasm, clarity, discipline, and reverence.
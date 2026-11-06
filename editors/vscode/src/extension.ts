import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import * as cp from 'child_process';
import {
    workspace,
    ExtensionContext,
    window,
    commands,
    Uri,
    languages,
    Hover,
    MarkdownString,
    CompletionItem,
    CompletionItemKind,
    TextDocument,
    Position,
    Range,
} from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let extensionRootPath: string = '';

interface KeywordEntry {
    token: string;
    category: string;
    sql: string;
    grammar: string;
    doc: string;
}

const SSQL_KEYWORD_DOCS: KeywordEntry[] = [
    { token: 'आह्रियताम्', category: 'Verb (क्रियापदम्)', sql: 'SELECT (1 item)', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Fetches exactly 1 projected column.' },
    { token: 'आह्रियेताम्', category: 'Verb (क्रियापदम्)', sql: 'SELECT (2 items)', grammar: 'कर्मणि लोट् द्विवचनम्', doc: 'Fetches exactly 2 projected columns.' },
    { token: 'आह्रियन्ताम्', category: 'Verb (क्रियापदम्)', sql: 'SELECT (3+ items)', grammar: 'कर्मणि लोट् बहुवचनम्', doc: 'Fetches 3 or more projected columns.' },
    { token: 'रच्यताम्', category: 'Verb (क्रियापदम्)', sql: 'CREATE TABLE / VIEW / INDEX / FUNCTION', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Creates a table (`इति पटलम्`), view (`इति दृश्यम्`), index (`इति सूची`) or pure function (`शुद्धम् कार्यम्`).' },
    { token: 'निवेश्यताम्', category: 'Verb (क्रियापदम्)', sql: 'INSERT INTO ...', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Inserts a row into a Locative Plural (`सप्तमी बहुवचनम्`) target table.' },
    { token: 'निवेश्येताम्', category: 'Verb (क्रियापदम्)', sql: 'INSERT INTO ... (2 columns)', grammar: 'कर्मणि लोट् द्विवचनम्', doc: 'The dual form, for exactly 2 columns.' },
    { token: 'निवेश्यन्ताम्', category: 'Verb (क्रियापदम्)', sql: 'INSERT INTO ... (3+ columns)', grammar: 'कर्मणि लोट् बहुवचनम्', doc: 'The plural form, for 3 or more columns.' },
    { token: 'परिवर्त्यताम्', category: 'Verb (क्रियापदम्)', sql: 'UPDATE ... SET ...', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Updates rows on a Locative Plural (`सप्तमी बहुवचनम्`) target table.' },
    { token: 'परिवर्त्येताम्', category: 'Verb (क्रियापदम्)', sql: 'UPDATE ... SET ... (2 columns)', grammar: 'कर्मणि लोट् द्विवचनम्', doc: 'The dual form, for exactly 2 columns.' },
    { token: 'परिवर्त्यन्ताम्', category: 'Verb (क्रियापदम्)', sql: 'UPDATE ... SET ... (3+ columns)', grammar: 'कर्मणि लोट् बहुवचनम्', doc: 'The plural form, for 3 or more columns.' },
    { token: 'निष्कास्यताम्', category: 'Verb (क्रियापदम्)', sql: 'DELETE FROM ...', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Deletes rows matching `नियमेन` from a table.' },
    { token: 'लुप्यताम्', category: 'Verb (क्रियापदम्)', sql: 'DROP ...', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Drops a table, view, index or column.' },
    { token: 'अभिधीयताम्', category: 'Verb (क्रियापदम्)', sql: 'ALTER ... RENAME', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Renames a table or column (`… इति तस्मात् "new" इति अभिधीयताम्`).' },
    { token: 'विशदीक्रियताम्', category: 'Verb (क्रियापदम्)', sql: 'EXPLAIN QUERY PLAN', grammar: 'कर्मणि लोट् एकवचनम्', doc: 'Explains the execution plan of a query.' },
    { token: 'प्रतिदीयते', category: 'Verb (क्रियापदम्)', sql: 'RETURNS <type>', grammar: 'कर्तरि लट् एकवचनम्', doc: 'Declares the return data type of a pure UDF (`शुद्धम् कार्यम्`).' },
    { token: 'प्रतिदीयताम्', category: 'Verb (क्रियापदम्)', sql: 'RETURN <expr>', grammar: 'कर्तरि लोट् एकवचनम्', doc: 'Returns the computed expression (`इति उत्तरम् प्रतिदीयताम्`) from a UDF.' },
    { token: 'प्रदाय', category: 'Absolutive (ल्यबन्तम्)', sql: 'Pass UDF arguments', grammar: 'प्र + दा + ल्यप्', doc: 'Supplies positional or named arguments to a pure UDF (`मूल्यम् च ५ च प्रदाय करसहितमूल्यम्`).' },
    { token: 'इति', category: 'Particle (निपातः)', sql: 'Clause closer / AS', grammar: 'अव्ययम्', doc: 'Closes a clause (`… इति नियमेन`), names an alias (`<expr> <alias> इति आख्या`), or marks a type.' },
    { token: 'यत्र', category: 'Particle (निपातः)', sql: 'WHERE / AS / OVER (', grammar: 'अव्ययम्', doc: 'Opens a table column block, UDF body, or window specification.' },
    { token: 'यत्', category: 'Particle (निपातः)', sql: 'Group / Subquery `(`', grammar: 'अव्ययम्', doc: 'Opens a group or subquery closed by `इति` (`यत् A वा B वा इति च C च`).' },
    { token: 'च', category: 'Conjunction (निपातः)', sql: 'AND / list item', grammar: 'अव्ययम्', doc: 'Follows every item of a list, the last one too (`शीर्षकम् च मूल्यम् च`).' },
    { token: 'वा', category: 'Conjunction (निपातः)', sql: 'OR', grammar: 'अव्ययम्', doc: 'Follows every condition of an OR list, the last one too; group a mixed list with `यत् … इति`.' },
    { token: 'नियमेन', category: 'Clause Marker', sql: 'WHERE', grammar: 'तृतीया एकवचनम्', doc: 'Marks the row filter predicate (`... इति नियमेन`).' },
    { token: 'समूहेन', category: 'Clause Marker', sql: 'GROUP BY', grammar: 'तृतीया एकवचनम्', doc: 'Marks the grouping key (`... इति समूहेन`).' },
    { token: 'समूहनियमेन', category: 'Clause Marker', sql: 'HAVING', grammar: 'तृतीया एकवचनम्', doc: 'Marks the post-aggregation group filter (`... इति समूहनियमेन`).' },
    { token: 'क्रमेण', category: 'Clause Marker', sql: 'ORDER BY', grammar: 'तृतीया एकवचनम्', doc: 'Marks the sort keys, before the select list (`मूल्यम् अवरोहः इति क्रमेण`).' },
    { token: 'सह', category: 'Postposition', sql: 'JOIN ...', grammar: 'अव्ययम् (सहार्थे तृतीया)', doc: 'Follows an Instrumental Plural table name to introduce a JOIN (`आपणैः सह`).' },
    { token: 'मेलनेन', category: 'Clause Marker', sql: 'INNER JOIN ON', grammar: 'तृतीया एकवचनम्', doc: 'Completes an inner join condition (`... इति मेलनेन`).' },
    { token: 'वाममेलनेन', category: 'Clause Marker', sql: 'LEFT JOIN ON', grammar: 'तृतीया एकवचनम्', doc: 'Completes a left outer join condition (`... इति वाममेलनेन`).' },
    { token: 'दक्षिणमेलनेन', category: 'Clause Marker', sql: 'RIGHT JOIN ON', grammar: 'तृतीया एकवचनम्', doc: 'Completes a right outer join condition (`... इति दक्षिणमेलनेन`).' },
    { token: 'पूर्णमेलनेन', category: 'Clause Marker', sql: 'FULL OUTER JOIN ON', grammar: 'तृतीया एकवचनम्', doc: 'Completes a full outer join condition (`... इति पूर्णमेलनेन`).' },
    { token: 'मित्या', category: 'Clause Marker', sql: 'LIMIT', grammar: 'तृतीया एकवचनम्', doc: 'Limits the number of returned rows, before the select list (`५ इति मित्या`).' },
    { token: 'पुनरुक्तिम्', category: 'Modifier', sql: 'SELECT DISTINCT', grammar: 'द्वितीया एकवचनम् (with विना)', doc: '`पुनरुक्तिम् विना` ("without repetition") before the select list removes repeated rows.' },
    { token: 'सर्वम्', category: 'Pronoun', sql: 'SELECT *', grammar: 'सर्वनाम', doc: 'Projects all columns.' },
    { token: 'अधिकः', category: 'Comparison', sql: '> (Masculine)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'Greater than for masculine subjects (`अङ्कः ९० इत्यस्मात् अधिकः`).' },
    { token: 'अधिका', category: 'Comparison', sql: '> (Feminine)', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Greater than for feminine subjects (`सङ्ख्या १० इत्यस्याः अधिका`).' },
    { token: 'अधिकम्', category: 'Comparison', sql: '> (Neuter)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'Greater than for neuter subjects (`मूल्यम् २० इत्यस्मात् अधिकम्`).' },
    { token: 'न्यूनः', category: 'Comparison', sql: '< (Masculine)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'Less than for masculine subjects.' },
    { token: 'न्यूना', category: 'Comparison', sql: '< (Feminine)', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Less than for feminine subjects.' },
    { token: 'न्यूनम्', category: 'Comparison', sql: '< (Neuter)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'Less than for neuter subjects.' },
    { token: 'समः', category: 'Comparison', sql: '= (Masculine)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'Equal to for masculine subjects (`इत्यनेन समः अस्ति`).' },
    { token: 'समा', category: 'Comparison', sql: '= (Feminine)', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Equal to for feminine subjects (`इत्यनया समा अस्ति`).' },
    { token: 'समम्', category: 'Comparison', sql: '= (Neuter)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'Equal to for neuter subjects (`इत्यनेन समम् अस्ति`).' },
    { token: 'रिक्तः', category: 'Comparison', sql: 'IS NULL (Masculine)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'NULL check for masculine columns.' },
    { token: 'रिक्ता', category: 'Comparison', sql: 'IS NULL (Feminine)', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'NULL check for feminine columns (`सङ्ख्या रिक्ता अस्ति`).' },
    { token: 'रिक्तम्', category: 'Comparison', sql: 'IS NULL (Neuter)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'NULL check for neuter columns.' },
    { token: 'इत्यस्मात्', category: 'Pronoun', sql: 'Than (M/N Ablative)', grammar: 'पञ्चमी एकवचनम् (पुंलिङ्गम्/नपुंसकलिङ्गम्)', doc: 'Links a literal or column to `अधिकः/अधिकम्` or `न्यूनः/न्यूनम्`.' },
    { token: 'इत्यस्याः', category: 'Pronoun', sql: 'Than / Of (F Ablative/Genitive)', grammar: 'पञ्चमी/षष्ठी एकवचनम् (स्त्री)', doc: 'Links a feminine literal or column to a comparison or aggregate.' },
    { token: 'इत्यनेन', category: 'Pronoun', sql: 'Equal to / By (M/N Instrumental)', grammar: 'तृतीया एकवचनम् (पुंलिङ्गम्/नपुंसकलिङ्गम्)', doc: 'Links a masculine/neuter operand to `समः/समम्` or multiplication/division.' },
    { token: 'इत्यनया', category: 'Pronoun', sql: 'Equal to / By (F Instrumental)', grammar: 'तृतीया एकवचनम् (स्त्री)', doc: 'Links a feminine operand to `समा अस्ति` (`सङ्ख्या इत्यनया समा अस्ति`).' },
    { token: 'इत्यस्य', category: 'Pronoun', sql: 'Of (Genitive Singular)', grammar: 'षष्ठी एकवचनम्', doc: 'Connects an expression to an aggregate or window function.' },
    { token: 'इत्यनयोः', category: 'Pronoun', sql: 'Of Both (Genitive Dual)', grammar: 'षष्ठी द्विवचनम्', doc: 'Connects two arithmetic operands (`मूल्यम् च करः च इत्यनयोः योगः`).' },
    { token: 'योगः', category: 'Aggregate / Math', sql: 'SUM(x) / (a + b)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'Computes the sum of a column or addition of two values.' },
    { token: 'माध्यम्', category: 'Aggregate', sql: 'AVG(x)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'Computes the arithmetic mean of a column (`मूल्यस्य माध्यम्`).' },
    { token: 'गणना', category: 'Aggregate', sql: 'COUNT(x)', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Counts non-null values (`सङ्ख्यायाः गणना`); COUNT(*) is `पङ्क्तीनाम् गणना`.' },
    { token: 'महत्तमम्', category: 'Aggregate', sql: 'MAX(x)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'The largest value (`मूल्यस्य महत्तमम्`).' },
    { token: 'लघुतमम्', category: 'Aggregate', sql: 'MIN(x)', grammar: 'नपुंसकलिङ्गम् प्रथमा एकवचनम्', doc: 'The smallest value (`मूल्यस्य लघुतमम्`).' },
    { token: 'श्रेणी', category: 'Window Function', sql: 'RANK()', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Rank over a window (`श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण`).' },
    { token: 'सघनश्रेणी', category: 'Window Function', sql: 'DENSE_RANK()', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Window dense rank without gaps.' },
    { token: 'पङ्क्तिसङ्ख्या', category: 'Window Function', sql: 'ROW_NUMBER()', grammar: 'स्त्रीलिङ्गम् प्रथमा एकवचनम्', doc: 'Sequential row number within a window partition.' },
    { token: 'पूर्वपङ्क्तेः', category: 'Window Function', sql: 'LAG(col)', grammar: 'स्त्रीलिङ्गम् षष्ठी एकवचनम्', doc: 'The previous row\'s value (`पूर्वपङ्क्तेः मूल्यम् यत्र …`).' },
    { token: 'अनन्तरपङ्क्तेः', category: 'Window Function', sql: 'LEAD(col)', grammar: 'स्त्रीलिङ्गम् षष्ठी एकवचनम्', doc: 'The next row\'s value (`अनन्तरपङ्क्तेः मूल्यम् यत्र …`).' },
    { token: 'चतुर्थांशाङ्कः', category: 'Window Function', sql: 'NTILE(4)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'The quartile number within the window.' },
    { token: 'भागाङ्कः', category: 'Window Function', sql: 'NTILE(n)', grammar: 'पुंलिङ्गम् प्रथमा एकवचनम्', doc: 'The part number among n parts (`५ भागेषु भागाङ्कः`).' },
    { token: 'पटलम्', category: 'DDL Keyword', sql: 'TABLE', grammar: 'नपुंसकलिङ्गम् एकवचनम्', doc: 'Declares or closes a relational table definition (`इति पटलम्`).' },
    { token: 'शुद्धम्', category: 'DDL Keyword', sql: 'PURE UDF', grammar: 'विशेषणम्', doc: 'Marks a scalar user-defined function (`शुद्धम् कार्यम्`).' },
    { token: 'कार्यम्', category: 'DDL Keyword', sql: 'FUNCTION', grammar: 'नपुंसकलिङ्गम् एकवचनम्', doc: 'Declares or closes a user-defined function (`इति कार्यम्`).' },
    { token: 'पूर्णाङ्कः', category: 'Data Type', sql: 'INTEGER', grammar: 'पुंलिङ्गम् एकवचनम्', doc: '64-bit integer data type.' },
    { token: 'वास्तविकः', category: 'Data Type', sql: 'REAL / NUMERIC', grammar: 'पुंलिङ्गम् एकवचनम्', doc: 'Floating-point / numeric data type.' },
    { token: 'वाक्यम्', category: 'Data Type', sql: 'TEXT / VARCHAR', grammar: 'नपुंसकलिङ्गम् एकवचनम्', doc: 'UTF-8 text string data type.' },
    { token: 'दिनाङ्कः', category: 'Data Type', sql: 'DATE', grammar: 'पुंलिङ्गम् एकवचनम्', doc: 'Calendar date data type.' },
];

const KEYWORD_MAP = new Map<string, KeywordEntry>(
    SSQL_KEYWORD_DOCS.map((k) => [k.token, k])
);

function findBinary(binaryName: string): string {
    const possiblePaths: string[] = [];

    if (extensionRootPath) {
        possiblePaths.push(path.join(extensionRootPath, 'bin', binaryName));
        possiblePaths.push(path.join(extensionRootPath, '..', '..', 'target', 'release', binaryName));
        possiblePaths.push(path.join(extensionRootPath, '..', '..', 'target', 'debug', binaryName));
    }

    if (workspace.workspaceFolders) {
        const root = workspace.workspaceFolders[0].uri.fsPath;
        possiblePaths.push(path.join(root, 'target', 'release', binaryName));
        possiblePaths.push(path.join(root, 'target', 'debug', binaryName));
        possiblePaths.push(path.join(root, binaryName));
    }

    const cargoBin = path.join(os.homedir(), '.cargo', 'bin', binaryName);
    possiblePaths.push(cargoBin);

    for (const p of possiblePaths) {
        if (fs.existsSync(p)) {
            return p;
        }
    }

    return binaryName;
}

function getDevanagariWordAtPosition(document: TextDocument, position: Position): { word: string; range: Range } | undefined {
    const lineText = document.lineAt(position.line).text;
    const isWordChar = (ch: string) => /[\u0900-\u097F\w]/.test(ch);
    let start = position.character;
    let end = position.character;
    while (start > 0 && isWordChar(lineText[start - 1])) {
        start--;
    }
    while (end < lineText.length && isWordChar(lineText[end])) {
        end++;
    }
    if (start === end) {
        return undefined;
    }
    return {
        word: lineText.slice(start, end),
        range: new Range(position.line, start, position.line, end),
    };
}

function discoverSchemaIdentifiers(docPath: string): Array<{ surface: string; sqlName: string; stem: string; kind: string }> {
    const results: Array<{ surface: string; sqlName: string; stem: string; kind: string }> = [];
    let dir = path.dirname(docPath);
    for (let depth = 0; depth < 5; depth++) {
        const schemaCandidate = path.join(dir, 'schema.सं');
        if (fs.existsSync(schemaCandidate)) {
            try {
                const text = fs.readFileSync(schemaCandidate, 'utf-8');
                const re = /([\u0900-\u097F]+)\s+इति\s+(पटलम्|पूर्णाङ्कः|वास्तविकः|वाक्यम्|दिनाङ्कः|द्वैतम्)\s+"([^"]+)"\s+इति\s+आख्या\s+([\u0900-\u097F]+)/g;
                let match: RegExpExecArray | null;
                while ((match = re.exec(text)) !== null) {
                    results.push({
                        surface: match[1],
                        kind: match[2] === 'पटलम्' ? 'Table (पटलम्)' : `Column (${match[2]})`,
                        sqlName: match[3],
                        stem: match[4],
                    });
                }
            } catch {
                // ignore read errors
            }
            break;
        }
        const parent = path.dirname(dir);
        if (parent === dir) {
            break;
        }
        dir = parent;
    }
    return results;
}

export function activate(context: ExtensionContext) {
    extensionRootPath = context.extensionUri.fsPath;
    const config = workspace.getConfiguration("ssql");
    let serverPath = config.get<string>("serverPath");

    if (!serverPath) {
        serverPath = findBinary('ssql-lsp');
    }

    const hasLocalLsp = fs.existsSync(serverPath);

    if (hasLocalLsp) {
        const serverOptions: ServerOptions = {
            run: { command: serverPath },
            debug: { command: serverPath }
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [{ scheme: 'file', language: 'ssql' }],
            synchronize: {
                fileEvents: workspace.createFileSystemWatcher('**/schema.सं')
            }
        };

        client = new LanguageClient(
            'ssql',
            'SSQL Language Server',
            serverOptions,
            clientOptions
        );

        client.start();
    } else {
        // Fallback client-side HoverProvider and CompletionItemProvider when ssql-lsp binary is not yet installed
        context.subscriptions.push(
            languages.registerHoverProvider('ssql', {
                provideHover(document: TextDocument, position: Position) {
                    const found = getDevanagariWordAtPosition(document, position);
                    if (!found) {
                        return undefined;
                    }
                    const kw = KEYWORD_MAP.get(found.word);
                    if (kw) {
                        const md = new MarkdownString();
                        md.appendMarkdown(`**${kw.token}** — *${kw.category}*\n\n`);
                        md.appendMarkdown(`- **SQL Equivalent:** \`${kw.sql}\`\n`);
                        md.appendMarkdown(`- **Grammar:** ${kw.grammar}\n\n`);
                        md.appendMarkdown(kw.doc);
                        return new Hover(md, found.range);
                    }
                    const schemaItems = discoverSchemaIdentifiers(document.uri.fsPath);
                    const hit = schemaItems.find(
                        (s) => s.surface === found.word || s.stem === found.word || found.word.startsWith(s.stem)
                    );
                    if (hit) {
                        const md = new MarkdownString();
                        md.appendMarkdown(`**${found.word}** (stem: \`${hit.stem}\`) — *${hit.kind}*\n\n`);
                        md.appendMarkdown(`- **SQL Mapping:** \`${hit.sqlName}\``);
                        return new Hover(md, found.range);
                    }
                    return undefined;
                },
            }),
            languages.registerCompletionItemProvider(
                'ssql',
                {
                    provideCompletionItems(document: TextDocument) {
                        const items: CompletionItem[] = SSQL_KEYWORD_DOCS.map((kw) => {
                            const item = new CompletionItem(kw.token, CompletionItemKind.Keyword);
                            item.detail = `${kw.sql} — ${kw.category}`;
                            item.documentation = new MarkdownString(`${kw.doc}\n\n*Grammar:* ${kw.grammar}`);
                            return item;
                        });
                        for (const s of discoverSchemaIdentifiers(document.uri.fsPath)) {
                            const item = new CompletionItem(
                                s.surface,
                                s.kind.startsWith('Table') ? CompletionItemKind.Class : CompletionItemKind.Field
                            );
                            item.detail = `${s.kind} → ${s.sqlName} (stem: ${s.stem})`;
                            items.push(item);
                        }
                        return items;
                    },
                },
                ' '
            )
        );
    }

    context.subscriptions.push(
        commands.registerCommand('ssql.generateSqlite', (uri) => runCompilerCommand(uri, ['--dialect', 'sqlite'], true, 'sql')),
        commands.registerCommand('ssql.generatePostgres', (uri) => runCompilerCommand(uri, ['--dialect', 'postgres'], true, 'sql')),
        commands.registerCommand('ssql.formatDocument', (uri) => runCompilerCommand(uri, ['--fmt'])),
        commands.registerCommand('ssql.lintDocument', (uri) => runCompilerCommand(uri, ['--lint'], true, 'plaintext')),
        commands.registerCommand('ssql.importFromSql', (uri) => runCompilerCommand(uri, ['--from-sql'], true, 'ssql')),
        commands.registerCommand('ssql.viewAst', (uri) => runCompilerCommand(uri, ['--dump-ast'], true, 'plaintext')),
        commands.registerCommand('ssql.viewSymTable', (uri) => runCompilerCommand(uri, ['--dump-symtable'], true, 'plaintext')),
        commands.registerCommand('ssql.installCli', installCliCommand)
    );
}

async function runCompilerCommand(
    uri: Uri | undefined,
    args: string[],
    showOutput = false,
    outputLanguage = 'plaintext',
    artifactExt?: string
) {
    let inputPath = uri?.fsPath;
    if (!inputPath && window.activeTextEditor) {
        inputPath = window.activeTextEditor.document.uri.fsPath;
    }

    if (!inputPath) {
        window.showErrorMessage("Open an SSQL (.सं) or SQL (.sql) file first.");
        return;
    }

    const compilerPath = findBinary('ssql_compiler');
    const finalArgs = [...args, inputPath];

    const outputChannel = window.createOutputChannel("SSQL Compiler");
    outputChannel.show(true);
    outputChannel.appendLine(`Running: ${compilerPath} ${finalArgs.join(' ')}`);

    cp.execFile(compilerPath, finalArgs, (err: any, stdout: string, stderr: string) => {
        if (stderr) {
            outputChannel.appendLine(`STDERR:\n${stderr}`);
        }
        if (stdout) {
            outputChannel.appendLine(`STDOUT:\n${stdout}`);
        }

        if (artifactExt) {
            const parsed = path.parse(inputPath!);
            const artifactPath = path.join(parsed.dir, `${parsed.name}${artifactExt}`);
            if (fs.existsSync(artifactPath)) {
                const artifactContent = fs.readFileSync(artifactPath, 'utf-8');
                workspace.openTextDocument({ content: artifactContent, language: outputLanguage })
                    .then(doc => window.showTextDocument(doc));
                return;
            }
        }

        if (showOutput && stdout) {
            workspace.openTextDocument({ content: stdout, language: outputLanguage })
                .then(doc => window.showTextDocument(doc));
        }

        if (err) {
            outputChannel.appendLine(`Error: ${err.message}`);
            window.showErrorMessage("SSQL command reported issues. Check the SSQL Compiler output panel.");
        } else if (!showOutput) {
            window.showInformationMessage("SSQL command completed successfully.");
        }
    });
}

function installCliCommand() {
    const repoRoot = extensionRootPath
        ? path.resolve(extensionRootPath, '..', '..')
        : (workspace.workspaceFolders ? workspace.workspaceFolders[0].uri.fsPath : '.');
    const terminal = window.createTerminal("SSQL CLI Installer");
    terminal.show();
    terminal.sendText(`cd "${repoRoot}" && cargo install --path . && cargo install --path ssql-lsp`);
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

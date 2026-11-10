// Generates syntaxes/qbasic.tmLanguage.json from the keyword tables of
// crates/qb-syntax (language keywords + the built-in registry generated from
// the compiler). Run: npm run grammar
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const crate = join(here, '..', '..', '..', 'crates', 'qb-syntax', 'src');

const builtins = [...readFileSync(join(crate, 'builtins_data.rs'), 'utf8').matchAll(/^\s+\("([^"]+)", (true|false), "([^"]*)"/gm)]
    .map((m) => ({ name: m[1] + m[3], isFunction: m[2] === 'true' }));
const language = [...readFileSync(join(crate, 'keywords.rs'), 'utf8').matchAll(/^\s+\("([A-Z_0-9$]+)", ([SOTFB]),/gm)]
    .map((m) => ({ name: m[1], kind: m[2] }));

const control = new Set(('IF THEN ELSE ELSEIF END ENDIF FOR EACH IN NEXT TO STEP DO LOOP WHILE WEND UNTIL SELECT CASE IS _THROW ' +
    'GOTO GOSUB RETURN EXIT ON RESUME CALL STOP SYSTEM TRY CATCH FINALLY _CONTINUE ERROR').split(' '));
const operators = new Set(language.filter((k) => k.kind === 'O').map((k) => k.name));
const types = new Set(language.filter((k) => k.kind === 'T').map((k) => k.name));
const other = new Set(language.filter((k) => k.kind === 'S' && !control.has(k.name) && k.name !== 'REM').map((k) => k.name));
for (const w of ['DEF', 'DATA', 'LET', 'INPUT', 'PRINT', 'OPEN', 'CLOSE', 'LINE', 'USING', 'SHARED', 'STATIC']) other.add(w);
for (const w of control) other.delete(w);
const functions = new Set([...builtins.filter((b) => b.isFunction).map((b) => b.name), ...language.filter((k) => k.kind === 'F').map((k) => k.name)]);
const subs = new Set([...builtins.filter((b) => !b.isFunction).map((b) => b.name), ...language.filter((k) => k.kind === 'B').map((k) => k.name)].filter((n) => !control.has(n) && !other.has(n)));

const esc = (s) => s.replace(/[$.*+?^()[\]{}|\\]/g, '\\$&');
/** Case-insensitive alternation of whole words (a trailing `$` is part of the word). */
function words(set) {
    const alts = [...set].sort((a, b) => b.length - a.length || a.localeCompare(b)).map(esc).join('|');
    return `(?i)(?<![\\w.$])(?:${alts})(?![\\w.$%&!#~\`])`;
}

const NAME = '[A-Za-z_][\\w.]*[%&!#$~`]*';
const allKeywords = new Set([...control, ...operators, ...types, ...other, ...functions, ...subs, 'REM']);

const grammar = {
    $schema: 'https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json',
    name: 'QBNex BASIC',
    scopeName: 'source.qbasic',
    fileTypes: ['bas', 'bi', 'bm'],
    patterns: [
        { include: '#metacomment' },
        { include: '#comment' },
        { include: '#metacommand' },
        { include: '#string' },
        { include: '#line-start' },
        { include: '#declaration' },
        { include: '#number' },
        { include: '#keywords' },
        { include: '#operator' },
        { include: '#identifier' },
    ],
    repository: {
        metacomment: {
            patterns: [
                {
                    name: 'meta.preprocessor.metacomment.qbasic',
                    match: "(?i)('|(?<![\\w.])REM(?![\\w.]))\\s*(\\$[A-Za-z_]+)(.*)$",
                    captures: {
                        1: { name: 'punctuation.definition.comment.qbasic' },
                        2: { name: 'keyword.control.directive.qbasic' },
                        3: { name: 'string.unquoted.directive.qbasic' },
                    },
                },
            ],
        },
        comment: {
            patterns: [
                { name: 'comment.line.apostrophe.qbasic', match: "'.*$" },
                { name: 'comment.line.rem.qbasic', match: '(?i)(?<![\\w.])REM(?![\\w.]).*$' },
                { name: 'comment.line.number-sign.qbasic', match: '^\\s*#(?=\\s|$|!).*$' },
            ],
        },
        metacommand: {
            patterns: [
                {
                    name: 'meta.preprocessor.qbasic',
                    match: '^\\s*(\\$[A-Za-z_]\\w*)(.*)$',
                    captures: {
                        1: { name: 'keyword.control.directive.qbasic' },
                        2: { name: 'string.unquoted.directive.qbasic' },
                    },
                },
            ],
        },
        string: {
            patterns: [
                { name: 'string.quoted.double.qbasic', begin: '"', end: '"|$', beginCaptures: { 0: { name: 'punctuation.definition.string.begin.qbasic' } }, endCaptures: { 0: { name: 'punctuation.definition.string.end.qbasic' } } },
            ],
        },
        'line-start': {
            patterns: [
                { name: 'entity.name.label.line-number.qbasic', match: '^\\s*\\d+(?=\\s|$|:|\')' },
                {
                    match: `(?i)^\\s*(?!${words(allKeywords).slice(4)}\\s*:)([A-Za-z_][\\w]*)(:)(?!=)`,
                    captures: { 1: { name: 'entity.name.label.qbasic' }, 2: { name: 'punctuation.separator.label.qbasic' } },
                },
            ],
        },
        declaration: {
            patterns: [
                {
                    match: `(?i)(?<![\\w.])(DECLARE\\s+)?(SUB|FUNCTION)\\s+(${NAME})`,
                    captures: { 1: { name: 'storage.modifier.qbasic' }, 2: { name: 'storage.type.function.qbasic' }, 3: { name: 'entity.name.function.qbasic' } },
                },
                {
                    match: `(?i)(?<![\\w.])(DEF)\\s+(FN${NAME})`,
                    captures: { 1: { name: 'storage.type.function.qbasic' }, 2: { name: 'entity.name.function.qbasic' } },
                },
                {
                    match: `(?i)^\\s*(TYPE|CLASS|ENUM)\\s+(${NAME})`,
                    captures: { 1: { name: 'storage.type.qbasic' }, 2: { name: 'entity.name.type.qbasic' } },
                },
                {
                    match: `(?i)(?<![\\w.])(AS)\\s+(?!${words(allKeywords).slice(4)})(${NAME})`,
                    captures: { 1: { name: 'keyword.other.as.qbasic' }, 2: { name: 'entity.name.type.qbasic' } },
                },
            ],
        },
        number: {
            patterns: [
                { name: 'constant.numeric.hex.qbasic', match: '&[Hh][0-9A-Fa-f]+[%&!#~`]*' },
                { name: 'constant.numeric.octal.qbasic', match: '&[Oo][0-7]+[%&!#~`]*' },
                { name: 'constant.numeric.binary.qbasic', match: '&[Bb][01]+[%&!#~`]*' },
                { name: 'constant.numeric.qbasic', match: '(?<![\\w.])(?:\\d+\\.?\\d*|\\.\\d+)(?:[EeDdFf][+-]?\\d+)?[%&!#~`]*' },
            ],
        },
        keywords: {
            patterns: [
                { name: 'keyword.control.qbasic', match: words(control) },
                { name: 'keyword.operator.word.qbasic', match: words(operators) },
                { name: 'storage.type.qbasic', match: words(types) },
                { name: 'support.function.builtin.qbasic', match: words(functions) },
                { name: 'support.function.statement.qbasic', match: words(subs) },
                { name: 'keyword.other.qbasic', match: words(other) },
            ],
        },
        operator: {
            patterns: [
                { name: 'keyword.operator.comparison.qbasic', match: '<>|<=|>=|=<|=>|><|<|>' },
                { name: 'keyword.operator.assignment.qbasic', match: '[-+*/\\\\^]?=' },
                { name: 'keyword.operator.arithmetic.qbasic', match: '[-+*/\\\\^]' },
                { name: 'punctuation.separator.qbasic', match: '[,;:]' },
            ],
        },
        identifier: {
            patterns: [{ name: 'variable.other.qbasic', match: NAME }],
        },
    },
};

writeFileSync(join(here, '..', 'syntaxes', 'qbasic.tmLanguage.json'), JSON.stringify(grammar, null, 2) + '\n');
console.log(`grammar: ${control.size} control, ${operators.size} operators, ${types.size} types, ${functions.size} functions, ${subs.size} statements, ${other.size} other`);

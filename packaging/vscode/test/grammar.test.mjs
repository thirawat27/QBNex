// Tokenizes sample lines with the real TextMate engine (vscode-textmate +
// Oniguruma, as VS Code does) and checks the scopes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const vsctm = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

async function loadGrammar() {
    const wasm = readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm')).buffer;
    await oniguruma.loadWASM(wasm);
    const registry = new vsctm.Registry({
        onigLib: Promise.resolve({
            createOnigScanner: (p) => new oniguruma.OnigScanner(p),
            createOnigString: (s) => new oniguruma.OnigString(s),
        }),
        loadGrammar: async () => vsctm.parseRawGrammar(readFileSync(join(root, 'syntaxes', 'qbasic.tmLanguage.json'), 'utf8'), 'qbasic.tmLanguage.json'),
    });
    return registry.loadGrammar('source.qbasic');
}

/** [text, innermost scope] pairs for each line of `src`. */
function tokenize(grammar, src) {
    let state = vsctm.INITIAL;
    const out = [];
    for (const line of src.split('\n')) {
        const r = grammar.tokenizeLine(line, state);
        for (const t of r.tokens) {
            const text = line.slice(t.startIndex, t.endIndex);
            if (text.trim()) out.push([text.trim(), t.scopes[t.scopes.length - 1]]);
        }
        state = r.ruleStack;
    }
    return out;
}

function scopeOf(tokens, text) {
    const t = tokens.find(([s]) => s === text);
    assert.ok(t, `no token ${JSON.stringify(text)} in ${JSON.stringify(tokens)}`);
    return t[1];
}

test('highlights QBasic/QB64/QBNex constructs', async () => {
    const g = await loadGrammar();
    const tokens = tokenize(g, [
        "'$INCLUDE: 'lib.bi'",
        '$CONSOLE:ONLY',
        'DEFINT A-Z',
        'TYPE Point2',
        '    x AS INTEGER',
        'END TYPE',
        'DECLARE SUB Draw (p AS Point2)',
        'start:',
        '10 PRINT "hi"; LEFT$(a$, 2), _RGB32(255, 0, 0) \' note',
        'IF x& > &HFF THEN GOTO start ELSE y# = 1.5E+3#',
        'rem old style comment',
        'FUNCTION Area! (w!, h!)',
        'Area! = w! * h! MOD 3',
        'END FUNCTION',
        'SUB Draw (p AS Point2)',
        '    LOCATE 1, 1: CLS',
        'END SUB',
    ].join('\n'));
    assert.equal(scopeOf(tokens, "'"), 'punctuation.definition.comment.qbasic');
    assert.equal(scopeOf(tokens, '$INCLUDE'), 'keyword.control.directive.qbasic');
    assert.equal(scopeOf(tokens, '$CONSOLE'), 'keyword.control.directive.qbasic');
    assert.equal(scopeOf(tokens, 'Point2'), 'entity.name.type.qbasic');
    assert.equal(scopeOf(tokens, 'INTEGER'), 'storage.type.qbasic');
    assert.equal(scopeOf(tokens, 'Draw'), 'entity.name.function.qbasic');
    assert.equal(scopeOf(tokens, 'start'), 'entity.name.label.qbasic');
    assert.equal(scopeOf(tokens, '10'), 'entity.name.label.line-number.qbasic');
    assert.equal(scopeOf(tokens, 'PRINT'), 'keyword.other.qbasic');
    assert.equal(scopeOf(tokens, 'LEFT$'), 'support.function.builtin.qbasic');
    assert.equal(scopeOf(tokens, '_RGB32'), 'support.function.builtin.qbasic');
    assert.equal(scopeOf(tokens, "' note"), 'comment.line.apostrophe.qbasic');
    assert.equal(scopeOf(tokens, '&HFF'), 'constant.numeric.hex.qbasic');
    assert.equal(scopeOf(tokens, '1.5E+3#'), 'constant.numeric.qbasic');
    assert.equal(scopeOf(tokens, 'THEN'), 'keyword.control.qbasic');
    assert.equal(scopeOf(tokens, 'rem old style comment'), 'comment.line.rem.qbasic');
    assert.equal(scopeOf(tokens, 'Area!'), 'entity.name.function.qbasic');
    assert.equal(scopeOf(tokens, 'MOD'), 'keyword.operator.word.qbasic');
    assert.equal(scopeOf(tokens, 'LOCATE'), 'support.function.statement.qbasic');
    assert.equal(scopeOf(tokens, 'CLS'), 'support.function.statement.qbasic');
    assert.equal(scopeOf(tokens, 'x&'), 'variable.other.qbasic');
    assert.equal(scopeOf(tokens, 'hi'), 'string.quoted.double.qbasic');
});

test('keywords inside names and strings are not keywords', async () => {
    const g = await loadGrammar();
    const tokens = tokenize(g, 'printer$ = "IF THEN"\nremark = endif.x');
    assert.equal(scopeOf(tokens, 'printer$'), 'variable.other.qbasic');
    assert.equal(scopeOf(tokens, 'remark'), 'variable.other.qbasic');
    assert.equal(scopeOf(tokens, 'endif.x'), 'variable.other.qbasic');
    assert.ok(!tokens.some(([, s]) => s === 'keyword.control.qbasic'));
});

test('every snippet is valid', () => {
    const snippets = JSON.parse(readFileSync(join(root, 'snippets', 'qbasic.json'), 'utf8'));
    assert.ok(Object.keys(snippets).length >= 10);
    for (const [name, s] of Object.entries(snippets)) {
        assert.ok(s.prefix && Array.isArray(s.body) && s.description, name);
    }
});

import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { expandArgs, exeExtension, serverCandidates } = require('../out/util.js');

test('expandArgs replaces known variables only', () => {
    const v = { file: 'C:/p/a.bas', fileBasenameNoExtension: 'a', exeExtension: '.exe' };
    assert.deepEqual(expandArgs(['${file}', '-o', '${fileBasenameNoExtension}${exeExtension}', '${nope}'], v), ['C:/p/a.bas', '-o', 'a.exe', '${nope}']);
});

test('server lookup order', () => {
    assert.equal(exeExtension('win32'), '.exe');
    assert.equal(exeExtension('linux'), '');
    assert.deepEqual(serverCandidates('', '/ext', 'linux'), [join('/ext', 'server', 'qb-lsp'), 'qb-lsp']);
    assert.deepEqual(serverCandidates('/x/qb-lsp', '/ext', 'linux')[0], '/x/qb-lsp');
});

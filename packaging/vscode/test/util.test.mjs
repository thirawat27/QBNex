import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { completeDebugConfig, expandArgs, exeExtension, serverCandidates } = require('../out/util.js');

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

test('debug configuration defaults', () => {
    assert.deepEqual(completeDebugConfig({}, 'C:/p/a.bas', 'qb'), { type: 'qbnex', request: 'launch', name: 'Debug QBNex program', program: 'C:/p/a.bas', compiler: 'qb' });
    assert.equal(completeDebugConfig({}, undefined, 'qb'), undefined);
    const c = completeDebugConfig({ type: 'qbnex', request: 'launch', name: 'x', program: 'b.bas', compiler: 'qbnex' }, 'a.bas', 'qb');
    assert.equal(c.program, 'b.bas');
    assert.equal(c.compiler, 'qbnex');
    assert.equal(serverCandidates('', '/ext', 'linux', 'qb-dap').at(-1), 'qb-dap');
});
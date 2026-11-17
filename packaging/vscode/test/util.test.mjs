import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { posix, win32 } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const {
    completeDebugConfig, expandArgs, expandHome, expandVariables, exeExtension, findOnPath, outputPath, parseDiagnostics,
    resolveCompiler, resolveTool, writtenSince,
} = require('../out/util.js');

/** An `exists` that knows only these files (Windows names ignore case). */
const files = (list, platform = 'linux') => {
    const norm = (p) => (platform === 'win32' ? p.toLowerCase() : p);
    const set = new Set(list.map(norm));
    return (p) => set.has(norm(p));
};

test('expandArgs replaces known variables only', () => {
    const v = { file: 'C:/p/a.bas', fileBasenameNoExtension: 'a', exeExtension: '.exe' };
    assert.deepEqual(expandArgs(['${file}', '-o', '${fileBasenameNoExtension}${exeExtension}', '${nope}'], v), ['C:/p/a.bas', '-o', 'a.exe', '${nope}']);
});

test('expandVariables reads env and names the unknown ones', () => {
    const unknown = [];
    const env = { QB_OUT: '/tmp/out' };
    assert.equal(expandVariables('${env:QB_OUT}/${fileBasename}', { fileBasename: 'a.bas' }, env, unknown, 'linux'), '/tmp/out/a.bas');
    assert.equal(expandVariables('${env:MISSING}x', {}, env, unknown, 'linux'), 'x');
    assert.equal(expandVariables('${config:x}${nope}', {}, env, unknown, 'linux'), '${config:x}${nope}');
    assert.deepEqual(unknown, ['config:x', 'nope']);
    assert.equal(expandVariables('${env:path}', {}, { Path: 'C:\\bin' }, [], 'win32'), 'C:\\bin');
});

test('expandHome', () => {
    assert.equal(expandHome('~/bin/qb-lsp', '/home/u'), '/home/u/bin/qb-lsp');
    assert.equal(expandHome('~', '/home/u'), '/home/u');
    assert.equal(expandHome('~\\qb\\qb-lsp.exe', 'C:\\Users\\u'), 'C:\\Users\\u\\qb\\qb-lsp.exe');
    assert.equal(expandHome('/opt/~x', '/home/u'), '/opt/~x');
});

test('findOnPath: PATH on Linux, PATH and PATHEXT on Windows', () => {
    const env = { PATH: '/usr/bin:/home/u/.local/bin' };
    assert.equal(findOnPath('qb', 'linux', env, files(['/home/u/.local/bin/qb'])), '/home/u/.local/bin/qb');
    assert.equal(findOnPath('qb-lsp', 'linux', env, files(['/home/u/.local/bin/qb'])), undefined);
    assert.equal(findOnPath('./x/qb', 'linux', env, files(['./x/qb'])), './x/qb');
    const wenv = { Path: 'C:\\Windows;"C:\\QBNex"', PATHEXT: '.COM;.EXE;.BAT;.CMD' };
    const wfiles = files(['C:\\QBNex\\qbnex.cmd', 'C:\\QBNex\\qb.exe', 'C:\\QBNex\\qb.cmd'], 'win32');
    assert.equal(findOnPath('qbnex', 'win32', wenv, wfiles), win32.join('C:\\QBNex', 'qbnex.cmd'));
    assert.equal(findOnPath('qb', 'win32', wenv, wfiles), win32.join('C:\\QBNex', 'qb.exe'));
    assert.equal(findOnPath('qb.cmd', 'win32', wenv, wfiles), win32.join('C:\\QBNex', 'qb.cmd'));
    assert.equal(findOnPath('C:\\QBNex\\qb', 'win32', wenv, wfiles), 'C:\\QBNex\\qb.exe');
});

test('resolveCompiler swaps a .cmd front end for qb.exe', () => {
    const wenv = { PATH: 'C:\\QBNex', PATHEXT: '.EXE;.CMD' };
    assert.deepEqual(resolveCompiler('qbnex', 'win32', wenv, files(['C:\\QBNex\\qbnex.cmd', 'C:\\QBNex\\qb.exe'], 'win32')), { path: win32.join('C:\\QBNex', 'qb.exe') });
    const lone = resolveCompiler('D:\\x\\qb.cmd', 'win32', wenv, files(['D:\\x\\qb.cmd'], 'win32'));
    assert.match(lone.error, /batch file.*qb\.exe/);
    assert.match(resolveCompiler('qb', 'linux', { PATH: '/bin' }, files([])).error, /not found/);
    assert.deepEqual(resolveCompiler('qb', 'linux', { PATH: '/bin' }, files(['/bin/qb'])), { path: '/bin/qb' });
});

test('resolveTool lookup order', () => {
    const base = {
        tool: 'qb-lsp', subcommand: 'lsp', configured: '', extensionPath: '/ext', compiler: '/opt/qbnex/qb',
        compilerCommand: 'qb', platform: 'linux', env: { PATH: '/usr/bin' }, home: '/home/u',
    };
    const all = ['/home/u/lsp', '/ext/server/qb-lsp', '/usr/bin/qb-lsp', '/opt/qbnex/qb-lsp'];
    assert.deepEqual(resolveTool({ ...base, configured: '~/lsp', exists: files(all) }), { command: { command: '/home/u/lsp', args: [] } });
    assert.deepEqual(resolveTool({ ...base, exists: files(all) }).command, { command: '/ext/server/qb-lsp', args: [] });
    assert.deepEqual(resolveTool({ ...base, exists: files(all.slice(2)) }).command, { command: '/usr/bin/qb-lsp', args: [] });
    assert.deepEqual(resolveTool({ ...base, exists: files(all.slice(3)) }).command, { command: '/opt/qbnex/qb-lsp', args: [] });
    assert.deepEqual(resolveTool({ ...base, tool: 'qb-dap', subcommand: 'dap', exists: files([]) }).command, { command: '/opt/qbnex/qb', args: ['dap'] });
    assert.deepEqual(resolveTool({ ...base, compiler: undefined, exists: files([]) }).command, { command: 'qb', args: ['lsp'] });
    const missing = resolveTool({ ...base, configured: '/nope/qb-lsp', exists: files(all.slice(3)) });
    assert.equal(missing.missingConfigured, '/nope/qb-lsp');
    assert.deepEqual(missing.command, { command: '/opt/qbnex/qb-lsp', args: [] });
    const w = resolveTool({ ...base, platform: 'win32', extensionPath: 'C:\\ext', compiler: 'C:\\QBNex\\qb.exe', env: { PATH: '' }, exists: files(['C:\\QBNex\\qb-lsp.exe'], 'win32') });
    assert.deepEqual(w.command, { command: 'C:\\QBNex\\qb-lsp.exe', args: [] });
});

test('outputPath follows -o and the platform suffix', () => {
    assert.equal(outputPath(['/p/a.bas'], '/p/a.bas', 'linux'), '/p/a');
    assert.equal(outputPath(['C:\\p\\a.bas'], 'C:\\p\\a.bas', 'win32'), 'C:\\p\\a.exe');
    assert.equal(outputPath(['a.bas', '-o', 'bin/app'], '/p/a.bas', 'linux'), posix.join('/p', 'bin/app'));
    assert.equal(outputPath(['a.bas', '--output', 'bin/app'], 'C:\\p\\a.bas', 'win32'), 'C:\\p\\bin\\app.exe');
    assert.equal(outputPath(['a.bas', '-o', 'C:/out/App.EXE'], 'C:\\p\\a.bas', 'win32'), 'C:\\out\\App.EXE');
    assert.equal(outputPath(['a.bas', '-o', '/tmp/x'], '/p/a.bas', 'linux'), '/tmp/x');
});

test('writtenSince', () => {
    assert.equal(writtenSince(10_500, 10_700), true);
    assert.equal(writtenSince(9_999, 10_700), false);
    assert.equal(writtenSince(12_000, 10_700), true);
});

test('parseDiagnostics reads headers, locations and -w warnings', () => {
    const text = [
        '[!!] QBNex :: Fatal [E1201]  \'x\' is not defined in the current scope',
        '  [@] D:\\work\\show/hello.bas(1,1)',
        '',
        '[x] QBNex :: Error [W2001]  Unused variable can be removed or used',
        '  [@] D:\\work\\show/warn.bas(1)',
        'warn.bas:1: warning: unused variable',
        '[x] QBNex :: Build Halted  1 blocking diagnostic(s)',
    ].join('\r\n');
    assert.deepEqual(parseDiagnostics(text, 'D:\\work\\show', 'win32'), [
        { file: 'D:\\work\\show\\hello.bas', line: 1, column: 1, severity: 'error', message: '\'x\' is not defined in the current scope', code: 'E1201' },
        { file: 'D:\\work\\show\\warn.bas', line: 1, column: undefined, severity: 'error', message: 'Unused variable can be removed or used', code: 'W2001' },
    ]);
    // the warning line for a place a header already reported is not added again
    const lin = parseDiagnostics('[~] QBNex :: Warning [W1]  hm\n  [@] /p/inc\\x.bi(7,3)\nsub/y.bas:4: warning: w\n', '/p', 'linux');
    assert.deepEqual(lin.map((d) => [d.file, d.line, d.severity]), [['/p/inc/x.bi', 7, 'warning'], ['/p/sub/y.bas', 4, 'warning']]);
});

test('debug configuration defaults', () => {
    assert.equal(exeExtension('win32'), '.exe');
    assert.equal(exeExtension('linux'), '');
    assert.deepEqual(completeDebugConfig({}, 'C:/p/a.bas', 'qb'), { type: 'qbnex', request: 'launch', name: 'Debug QBNex program', program: 'C:/p/a.bas', compiler: 'qb' });
    assert.equal(completeDebugConfig({}, undefined, 'qb'), undefined);
    const c = completeDebugConfig({ type: 'qbnex', request: 'launch', name: 'x', program: 'b.bas', compiler: 'qbnex' }, 'a.bas', 'qb');
    assert.equal(c.program, 'b.bas');
    assert.equal(c.compiler, 'qbnex');
});

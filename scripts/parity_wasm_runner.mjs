// Compile a batch of SSQL cases with the playground's WebAssembly bundle.
// Input (argv[2]): JSON [{schema, source, dialect}]; output (stdout): JSON
// [{ok, sql, errors}] in the same order. Used by check_frontend_parity.py.
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { initSync, compile_ssql_wasm } from '../website/pkg/ssql_wasm.js';

const here = path.dirname(fileURLToPath(import.meta.url));
initSync({ module: fs.readFileSync(path.join(here, '../website/pkg/ssql_wasm_bg.wasm')) });

const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const results = cases.map(({ schema, source, dialect }) => {
  const r = JSON.parse(compile_ssql_wasm(source, schema, dialect));
  return { ok: r.ok, sql: r.sql, errors: r.errors };
});
process.stdout.write(JSON.stringify(results));

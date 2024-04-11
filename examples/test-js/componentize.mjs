import { componentize } from '@bytecodealliance/componentize-js';
import { readFile, writeFile } from 'node:fs/promises';

const jsSource = await readFile('test.js', 'utf8');

const { component } = await componentize(jsSource, { witPath: '../../host-wit', worldName: 'test' });

await writeFile('test.wasm', component);
console.log("Wasm component written to `test.wasm`.")

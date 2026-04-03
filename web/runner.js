/**
 * Thin async wrapper around the WebRunner WASM class.
 *
 * Usage:
 *   import { runProgram } from './runner.js';
 *
 *   await runProgram(source, {
 *     onOutput: (line)     => appendToTerminal(line),
 *     onInput:  (variable) => showPrompt(variable),   // must return a Promise<string>
 *     onDone:   ()         => console.log("done"),
 *     onError:  (msg)      => console.error(msg),
 *   });
 *
 * The WASM package is lazy-imported from `./pkg/cambridge_pseudocode_interpreter.js`
 * (the output directory of `wasm-pack build --target web --out-dir web/pkg`).
 */

let wasmModule = null;

async function loadWasm() {
    if (!wasmModule) {
        // Initialise the WASM module once, then cache it.
        const mod = await import('./pkg/cambridge_pseudocode_interpreter.js');
        await mod.default(); // runs the WASM init / memory setup
        wasmModule = mod;
    }
    return wasmModule;
}

/**
 * Run a pseudocode program to completion.
 *
 * @param {string} source  - The pseudocode source text.
 * @param {object} options
 * @param {(line: string) => void}             options.onOutput  - Called for each OUTPUT statement.
 * @param {(variable: string) => Promise<string>} options.onInput - Called when INPUT is needed;
 *                                                                   resolve the promise with the user's value.
 * @param {() => void}                          options.onDone   - Called when the program exits normally.
 * @param {(message: string) => void}           options.onError  - Called when a runtime error occurs.
 */
export async function runProgram(source, { onOutput, onInput, onDone, onError }) {
    const { WebRunner } = await loadWasm();

    let runner;
    try {
        runner = new WebRunner(source);
    } catch (e) {
        onError(String(e));
        return;
    }

    try {
        while (true) {
            const event = runner.step();

            if (event.type === 'Output') {
                onOutput(event.value);
            } else if (event.type === 'NeedsInput') {
                // Caller returns a Promise — we await it so the event loop
                // can update the UI before we resume the interpreter.
                const value = await onInput(event.variable);
                runner.supply_input(value);
            } else if (event.type === 'Done') {
                onDone();
                break;
            } else if (event.type === 'Error') {
                onError(event.message);
                break;
            }
        }
    } finally {
        runner.free(); // release the WASM memory for the runner
    }
}

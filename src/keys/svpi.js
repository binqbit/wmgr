import { execFile } from 'child_process';
import { keypairFromMnemonic } from './index.js';

const SVPI_SCHEMA = 'svpi.response.v1';

function execFileAsync(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { ...opts }, (error, stdout, stderr) => {
      if (error && error.code === 'ENOENT') {
        return reject(new Error(`SVPI CLI not found: ${cmd}`));
      }
      const stdoutText = stdout?.toString() || '';
      const stderrText = stderr?.toString() || '';
      const exitCode = typeof error?.code === 'number' ? error.code : error ? 1 : 0;
      resolve({ stdout: stdoutText, stderr: stderrText, exitCode, error });
    });
  });
}

function parseSvpiResponse(raw) {
  if (!raw || !raw.trim()) {
    throw new Error('SVPI returned an empty response in JSON mode');
  }

  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    throw new Error(`Failed to parse SVPI JSON output: ${err.message}. Raw output: ${raw}`);
  }

  if (parsed.schema !== SVPI_SCHEMA) {
    throw new Error(`Unexpected SVPI response schema: ${parsed.schema || 'unknown'}`);
  }

  return parsed;
}

function ensureSvpiOk(resp) {
  if (resp.ok) return resp;
  const code = resp.error?.code || 'svpi_error';
  const message = resp.error?.message || 'SVPI returned an error';
  const details = resp.error?.details;
  const detailsText = details ? ` Details: ${JSON.stringify(details)}` : '';
  throw new Error(`SVPI error (${code}): ${message}.${detailsText}`);
}

export async function getMnemonicFromSvpi(name, password, filePath) {
  if (!name) throw new Error('SVPI: name is required');
  if (!password) throw new Error('SVPI: password is required');

  const args = ['--mode=json'];
  if (filePath) {
    args.push(`--file=${filePath}`);
  }
  args.push('get', name, `--password=${password}`);

  const { stdout, stderr, exitCode } = await execFileAsync('svpi', args);
  const rawOutput = stdout.trim() || stderr.trim();
  if (!rawOutput) {
    throw new Error(`SVPI returned no JSON output (exit code ${exitCode})`);
  }
  const resp = ensureSvpiOk(parseSvpiResponse(rawOutput));

  const mnemonic = resp.result?.data;
  if (typeof mnemonic !== 'string' || !mnemonic.trim()) {
    throw new Error('SVPI response did not include mnemonic data');
  }

  return mnemonic.trim();
}

export async function keypairFromSvpi({ name, password, filePath, derivationPath = "m/44'/501'/0'/0'", seedPassphrase = '' }) {
  const mnemonic = await getMnemonicFromSvpi(name, password, filePath);
  return keypairFromMnemonic(mnemonic, derivationPath, seedPassphrase);
}

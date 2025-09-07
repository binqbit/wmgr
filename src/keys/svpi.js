import { execFile } from 'child_process';
import { keypairFromMnemonic } from './index.js';

function execFileAsync(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { ...opts }, (error, stdout, stderr) => {
      if (error) {
        // include output in error for debugging
        const msg = `${error.message}\nSTDOUT:\n${stdout}\nSTDERR:\n${stderr}`;
        return reject(new Error(msg));
      }
      resolve({ stdout: stdout?.toString() || '', stderr: stderr?.toString() || '' });
    });
  });
}

function extractDataFromLogs(logText) {
  // Find last occurrence of "Data: ..." and take the rest of the line
  const lines = (logText || '').split(/\r?\n/);
  for (let i = lines.length - 1; i >= 0; i--) {
    const m = /^\s*Data:\s*(.*)\s*$/.exec(lines[i]);
    if (m && m[1] != null) {
      return m[1].trim();
    }
  }
  throw new Error('SVPI output did not contain a Data: line');
}

export async function getMnemonicFromSvpi(name, password, filePath) {
  if (!name) throw new Error('SVPI: name is required');
  if (!password) throw new Error('SVPI: password is required');
  const args = ['get', name, `--password=${password}`];
  if (filePath) {
    args.push(`--file=${filePath}`);
  }
  const { stdout, stderr } = await execFileAsync('svpi', args);
  const combined = `${stdout}\n${stderr}`;
  const data = extractDataFromLogs(combined);
  return data.trim();
}

export async function keypairFromSvpi({ name, password, filePath, derivationPath = "m/44'/501'/0'/0'", seedPassphrase = '' }) {
  const mnemonic = await getMnemonicFromSvpi(name, password, filePath);
  return keypairFromMnemonic(mnemonic, derivationPath, seedPassphrase);
}

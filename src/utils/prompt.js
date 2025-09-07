import readline from 'readline';

// Print prompts to stderr to avoid stdout buffering by wrappers (e.g., Yarn)
export async function prompt(question) {
  const rl = readline.createInterface({ input: process.stdin, output: process.stderr, terminal: true });
  return await new Promise((resolve) => rl.question(`${question} `, (ans) => { rl.close(); resolve(ans.trim()); }));
}

export async function promptHidden(question) {
  const rl = readline.createInterface({ input: process.stdin, output: process.stderr, terminal: true });
  const origWrite = rl._writeToOutput;
  rl._writeToOutput = function writeToOutput(stringToWrite) {
    if (rl.stdoutMuted) {
      return; // suppress typed characters
    }
    return origWrite.call(rl, stringToWrite);
  };
  rl.stdoutMuted = false;
  const result = await new Promise((resolve) => {
    rl.question(`${question} `, (answer) => {
      rl.history = rl.history || [];
      rl.close();
      process.stderr.write('\n');
      resolve(String(answer || '').trim());
    });
    rl.stdoutMuted = true; // mute immediately after printing the question
    rl.stdoutMuted = true;
  });
  return result;
}

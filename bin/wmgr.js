#!/usr/bin/env node
import('../src/cli.js').then(mod => mod.main()).catch(err => {
  console.error(err?.stack || err);
  process.exit(1);
});


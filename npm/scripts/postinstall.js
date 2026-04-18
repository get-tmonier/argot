#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');

const REPO = 'get-tmonier/argot';
const BIN_PATH = path.join(__dirname, '..', 'bin', 'argot');

function detectTarget() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === 'linux' && arch === 'x64') return 'linux-x64';
  if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function get(url, redirects = 5) {
  return new Promise((resolve, reject) => {
    if (redirects === 0) return reject(new Error('Too many redirects'));
    https.get(url, { headers: { 'User-Agent': 'argot-installer' } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return resolve(get(res.headers.location, redirects - 1));
      }
      if (res.statusCode !== 200) return reject(new Error(`HTTP ${res.statusCode}`));
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => resolve(Buffer.concat(chunks)));
      res.on('error', reject);
    }).on('error', reject);
  });
}

async function main() {
  const target = detectTarget();

  // Fetch latest tag
  const meta = JSON.parse(await get(`https://api.github.com/repos/${REPO}/releases/latest`));
  const tag = meta.tag_name;
  const version = tag.replace(/^v/, '');

  const url = `https://github.com/${REPO}/releases/download/${tag}/argot-${target}`;
  console.log(`Downloading argot ${version} for ${target}…`);

  const binary = await get(url);
  fs.mkdirSync(path.dirname(BIN_PATH), { recursive: true });
  fs.writeFileSync(BIN_PATH, binary, { mode: 0o755 });
  console.log(`argot ${version} installed.`);
}

main().catch((e) => {
  console.error('argot postinstall failed:', e.message);
  process.exit(1);
});

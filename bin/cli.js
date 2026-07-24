#!/usr/bin/env node

/**
 * FzComputerAI CLI — Computer Vision via MCP & GUI Rust Launcher
 * Webstorage Tecnologia | Roger Luft <roger@webstorage.com.br>
 * Patrocinadores: www.webstorage.com.br | www.imovelsite.com.br
 */

const { spawn, execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const args = process.argv.slice(2);

console.log('\x1b[36m%s\x1b[0m', '======================================================================');
console.log('\x1b[33m%s\x1b[0m', '   FzComputerAI — Servidor de Visão Computacional & Automação via MCP');
console.log('\x1b[32m%s\x1b[0m', '   Webstorage Tecnologia (www.webstorage.com.br) | Imóvel Site (www.imovelsite.com.br)');
console.log('\x1b[36m%s\x1b[0m', '======================================================================\n');

function findBinary(name) {
  try {
    const whichCmd = process.platform === 'win32' ? `where ${name}` : `which ${name}`;
    const result = execSync(whichCmd, { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim().split('\n')[0];
    if (result && fs.existsSync(result)) return result;
  } catch (e) {
    // Binary not found in PATH
  }

  // Check local release targets
  const rootDir = path.resolve(__dirname, '..');
  const exeName = process.platform === 'win32' ? `${name}.exe` : name;

  const candidatePaths = [
    path.join(rootDir, 'target', 'release', exeName),
    path.join(rootDir, 'fzcomputerai', 'target', 'release', exeName),
    path.join(rootDir, 'cua', 'libs', 'cua-driver', 'rust', 'target', 'release', exeName)
  ];

  for (const p of candidatePaths) {
    if (fs.existsSync(p)) return p;
  }

  return null;
}

const subCommand = args[0] || 'mcp';

if (subCommand === 'gui') {
  const guiBin = findBinary('fzcomputerai');
  if (guiBin) {
    console.log(`\x1b[32m[INFO] Iniciando GUI FzComputerAI: ${guiBin}\x1b[0m`);
    const child = spawn(guiBin, args.slice(1), { stdio: 'inherit' });
    child.on('exit', (code) => process.exit(code || 0));
  } else {
    console.error('\x1b[31m[ERRO] Executável da GUI (fzcomputerai) não encontrado.\x1b[0m');
    console.log('Execute `cargo build --release --manifest-path fzcomputerai/Cargo.toml` para compilar.');
    process.exit(1);
  }
} else {
  const cuaBin = findBinary('cua-driver');
  if (cuaBin) {
    const child = spawn(cuaBin, args, { stdio: 'inherit' });
    child.on('exit', (code) => process.exit(code || 0));
  } else {
    console.warn('\x1b[33m[AVISO] Binário cua-driver não encontrado no PATH ou compilação local.\x1b[0m');
    console.log('Executando script de instalação nativo...');
    
    if (process.platform === 'win32') {
      const psScript = path.resolve(__dirname, '..', 'install.ps1');
      const child = spawn('powershell', ['-ExecutionPolicy', 'Bypass', '-File', psScript], { stdio: 'inherit' });
      child.on('exit', (code) => process.exit(code || 0));
    } else {
      const shScript = path.resolve(__dirname, '..', 'install.sh');
      const child = spawn('bash', [shScript], { stdio: 'inherit' });
      child.on('exit', (code) => process.exit(code || 0));
    }
  }
}

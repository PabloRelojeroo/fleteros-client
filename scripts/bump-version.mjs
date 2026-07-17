#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const raiz = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const version = process.argv[2];

if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('Uso: npm run bump-version -- 1.2.3');
  process.exit(1);
}

function actualizarJson(rutaRelativa) {
  const ruta = path.join(raiz, rutaRelativa);
  const datos = JSON.parse(fs.readFileSync(ruta, 'utf8'));
  datos.version = version;
  fs.writeFileSync(ruta, JSON.stringify(datos, null, 2) + '\n');
  console.log(`OK  ${rutaRelativa}`);
}

function actualizarCargoToml(rutaRelativa) {
  const ruta = path.join(raiz, rutaRelativa);
  const contenido = fs.readFileSync(ruta, 'utf8');
  const actualizado = contenido.replace(
    /(\[package\][^[]*?\nversion\s*=\s*")[^"]*(")/,
    `$1${version}$2`
  );
  if (actualizado === contenido) {
    console.error(`No se encontró "version = ..." dentro de [package] en ${rutaRelativa}`);
    process.exit(1);
  }
  fs.writeFileSync(ruta, actualizado);
  console.log(`OK  ${rutaRelativa}`);
}

actualizarJson('package.json');
actualizarJson('src-tauri/tauri.conf.json');
actualizarCargoToml('src-tauri/Cargo.toml');

console.log('\nSincronizando lockfiles...');

try {
  execSync('npm install --package-lock-only', { cwd: raiz, stdio: 'inherit' });
} catch {
  console.warn('No se pudo sincronizar package-lock.json automáticamente, corré "npm install --package-lock-only" a mano.');
}

try {
  execSync('cargo check', { cwd: path.join(raiz, 'src-tauri'), stdio: 'inherit' });
} catch {
  console.warn('No se pudo sincronizar Cargo.lock automáticamente, corré "cargo check" dentro de src-tauri a mano.');
}

console.log(`\nVersión actualizada a ${version} en package.json, tauri.conf.json y Cargo.toml.`);

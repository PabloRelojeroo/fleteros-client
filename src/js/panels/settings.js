async function iniciarAjustes() {
  await cargarAjustes();
  vincularSliders();
  vincularBotonGuardar();
  vincularDeteccionJava();
}

async function cargarAjustes() {
  const invocar = window.__TAURI__.core.invoke;
  const claves = ['java_path', 'ram_min', 'ram_max', 'res_width', 'res_height',
                 'hide_on_launch', 'concurrent_downloads', 'theme', 'discord_rpc'];

  for (const clave of claves) {
    const valor = await invocar('get_config', { key: clave }).catch(() => null);
    if (valor !== null) aplicarValorConfig(clave, valor);
  }
}

function aplicarValorConfig(clave, valor) {
  const mapa = {
    java_path: 'java-path',
    ram_min: 'ram-min',
    ram_max: 'ram-max',
    res_width: 'res-width',
    res_height: 'res-height',
    concurrent_downloads: 'concurrent-dl',
  };

  if (mapa[clave]) {
    const el = document.getElementById(mapa[clave]);
    if (el) el.value = valor;
  }

  if (clave === 'hide_on_launch') {
    const el = document.getElementById('toggle-hide');
    if (el) el.checked = valor !== 'false';
  }
  if (clave === 'theme') {
    const el = document.getElementById('toggle-theme');
    if (el) el.checked = valor === 'light';
    document.documentElement.setAttribute('data-theme', valor || 'dark');
  }
  if (clave === 'discord_rpc') {
    const el = document.getElementById('toggle-discord');
    if (el) el.checked = valor !== 'false';
  }

  const mapaSufijos = { ram_min: ' MB', ram_max: ' MB', concurrent_downloads: '' };
  const mapaDisplay = { ram_min: 'ram-min-val', ram_max: 'ram-max-val', concurrent_downloads: 'concurrent-dl-val' };
  if (mapaDisplay[clave]) {
    const el = document.getElementById(mapaDisplay[clave]);
    if (el) el.textContent = `${valor}${mapaSufijos[clave]}`;
  }
}

function vincularSliders() {
  [
    { id: 'ram-min', idMuestra: 'ram-min-val', sufijo: ' MB' },
    { id: 'ram-max', idMuestra: 'ram-max-val', sufijo: ' MB' },
    { id: 'concurrent-dl', idMuestra: 'concurrent-dl-val', sufijo: '' },
  ].forEach(({ id, idMuestra, sufijo }) => {
    const control = document.getElementById(id);
    const display = document.getElementById(idMuestra);
    if (control && display) {
      control.addEventListener('input', () => {
        display.textContent = `${control.value}${sufijo}`;
      });
    }
  });

  document.getElementById('toggle-theme')?.addEventListener('change', e => {
    document.documentElement.setAttribute('data-theme', e.target.checked ? 'light' : 'dark');
  });
}

async function vincularBotonGuardar() {
  document.getElementById('btn-save-settings')?.addEventListener('click', async () => {
    const invocar = window.__TAURI__.core.invoke;
    const notificar = window._notify;
    try {
      const ajustes = {
        java_path: document.getElementById('java-path')?.value || '',
        ram_min: document.getElementById('ram-min')?.value || '1024',
        ram_max: document.getElementById('ram-max')?.value || '4096',
        res_width: document.getElementById('res-width')?.value || '1280',
        res_height: document.getElementById('res-height')?.value || '720',
        hide_on_launch: document.getElementById('toggle-hide')?.checked ? 'true' : 'false',
        concurrent_downloads: document.getElementById('concurrent-dl')?.value || '10',
        theme: document.getElementById('toggle-theme')?.checked ? 'light' : 'dark',
        discord_rpc: document.getElementById('toggle-discord')?.checked ? 'true' : 'false',
      };

      for (const [clave, valor] of Object.entries(ajustes)) {
        await invocar('set_config', { key: clave, value: valor });
      }
      notificar('Ajustes guardados', 'success');
    } catch (err) {
      window._notify(`Error guardando: ${err}`, 'error');
    }
  });
}

function vincularDeteccionJava() {
  document.getElementById('btn-detect-java')?.addEventListener('click', async () => {
    const invocar = window.__TAURI__.core.invoke;
    const notificar = window._notify;
    try {
      const instalaciones = await invocar('get_java_paths');
      if (instalaciones.length === 0) {
        notificar('No se encontró Java. Instalá Java 17+', 'warning');
        return;
      }
      const mejor = instalaciones[0];
      const elementoRuta = document.getElementById('java-path');
      if (elementoRuta) elementoRuta.value = mejor.path;
      notificar(`Java detectado: ${mejor.version}`, 'success');
    } catch (err) {
      notificar(`Error: ${err}`, 'error');
    }
  });
}

window._initSettings = iniciarAjustes;

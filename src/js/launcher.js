document.addEventListener('contextmenu', e => e.preventDefault());

function aplicarMarca() {
  const marca = window.BRAND;
  if (!marca) return;
  document.title = marca.appName;
  document.querySelectorAll('.titlebar-title').forEach(el => { el.textContent = marca.appName; });
  const splashTitulo = document.getElementById('splash-title-overlay');
  if (splashTitulo) splashTitulo.textContent = marca.appName;
  const sidebarTitulo = document.getElementById('sidebar-title');
  if (sidebarTitulo) sidebarTitulo.textContent = marca.appName;
  const tagline = document.getElementById('splash-tagline-overlay');
  if (tagline && marca.tagline) tagline.textContent = marca.tagline;
  document.querySelectorAll('img[alt="Fleteros"]').forEach(img => { img.alt = marca.appName; });
}
aplicarMarca();

function urlsSkinPosibles(cuenta, tamano) {
  const uuid = cuenta.uuid?.replace(/-/g, '') || '';
  if (uuid && (cuenta.auth_type === 'microsoft' || cuenta.auth_type === 'azauth')) {
    return [
      `https://mc-heads.net/avatar/${uuid}/${tamano}`,
      './images/default/steve.png',
    ];
  }
  return ['./images/default/steve.png'];
}

function aplicarSkinConFallback(imgEl, cuenta, tamano) {
  const opciones = urlsSkinPosibles(cuenta, tamano);
  let indice = 0;
  const intentarSiguiente = () => {
    if (indice >= opciones.length) return;
    const url = opciones[indice++];
    imgEl.onerror = () => {
      console.warn(`No se pudo cargar la skin desde ${url}`);
      intentarSiguiente();
    };
    imgEl.src = url;
  };
  intentarSiguiente();
}

function establecerSplash(pct, estado) {
  const barra = document.getElementById('splash-bar');
  const estadoEl = document.getElementById('splash-status-overlay');
  if (barra) barra.style.width = `${pct}%`;
  if (estadoEl) estadoEl.textContent = estado;
}

function ocultarSplash() {
  const overlay = document.getElementById('splash-overlay');
  if (!overlay) return;
  overlay.style.opacity = '0';
  setTimeout(() => overlay.remove(), 450);
}

document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.shiftKey && e.key === 'I') {
    try { window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow()?.openDevtools?.(); } catch {}
  }
});

async function iniciar() {
  const invocar = window.__TAURI__.core.invoke;
  const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;
  const notificar = window._notify;
  const ventana = getCurrentWebviewWindow();

  const confirmarCierreConJuegoAbierto = async () => {
    if (!window._juegoEnEjecucion) return true;
    return await window.__TAURI__.dialog.confirm(
      'El juego está abierto. Si cerrás el launcher, Minecraft también se va a cerrar. ¿Cerrar de todos modos?',
      { title: 'Cerrar launcher', kind: 'warning' }
    );
  };

  document.getElementById('btn-close')?.addEventListener('click', () => ventana.close());
  document.getElementById('btn-minimize')?.addEventListener('click', () => ventana.minimize());
  document.getElementById('btn-maximize')?.addEventListener('click', async () => {
    if (await ventana.isMaximized()) ventana.unmaximize();
    else ventana.maximize();
  });

  ventana.onCloseRequested(async (evento) => {
    if (!(await confirmarCierreConJuegoAbierto())) {
      evento.preventDefault();
    }
  });

  document.querySelectorAll('.nav-item').forEach(item => {
    item.addEventListener('click', () => {
      const panel = item.dataset.panel;
      if (panel) mostrarPanel(panel);
    });
  });

  try {
    establecerSplash(20, 'Cargando ajustes...');
    const tema = await invocar('get_config', { key: 'theme' }).catch(() => null);
    if (tema) document.documentElement.setAttribute('data-theme', tema);

    establecerSplash(50, 'Verificando sesión...');
    const cuentas = await invocar('get_accounts');

    establecerSplash(80, 'Iniciando...');

    await window._initLogin({
      onLogin: async (cuenta) => {
        mostrarSidebar();
        await refrescarVistaCuenta();
        mostrarPanel('home');
        await window._initHome();
        await window._initAdminAccess?.();
      }
    });

    await window._initSettings();

    establecerSplash(100, 'Listo');
    await new Promise(r => setTimeout(r, 300));
    ocultarSplash();
    buscarActualizaciones();

    if (cuentas.length > 0) {
      mostrarSidebar();
      await refrescarVistaCuenta();
      mostrarPanel('home');
      await window._initHome();
      await window._initAdminAccess?.();
    } else {
      mostrarPanel('login');
    }

  } catch (err) {
    const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
    console.error('Launcher boot error:', err);
    ocultarSplash();
    notificar?.(`Error al iniciar: ${mensaje}`, 'error');
  }
}

async function buscarActualizaciones() {
  const notificar = window._notify;
  const log = (msg) => { console.log(msg); window._logExterno?.(msg); };

  try {
    const actualizacion = await window.__TAURI__.updater.check();
    log(`[Updater] check() -> available=${actualizacion?.available} actual=${actualizacion?.currentVersion} nueva=${actualizacion?.version}`);

    if (!actualizacion?.available) return;

    if (actualizacion.version === actualizacion.currentVersion) {
      log(`[Updater] La version "disponible" es igual a la actual (${actualizacion.version}) — se aborta para no entrar en loop.`);
      return;
    }

    notificar?.(`Descargando actualización ${actualizacion.version}...`, 'info');
    log(`[Updater] Descargando ${actualizacion.currentVersion} -> ${actualizacion.version}`);

    await actualizacion.downloadAndInstall();
    log('[Updater] Descarga e instalación terminadas, reiniciando...');

    await window.__TAURI__.process.relaunch();
  } catch (err) {
    const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
    console.error('Error buscando actualizaciones:', err);
    log(`[Updater] Error: ${mensaje}`);
  }
}

function mostrarPanel(nombre) {
  document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
  const panel = document.getElementById(`${nombre}-panel`);
  if (panel) panel.classList.add('active');
  const navItem = document.querySelector(`[data-panel="${nombre}"]`);
  if (navItem) navItem.classList.add('active');
}

function mostrarSidebar() {
  const sidebar = document.getElementById('sidebar');
  if (sidebar) {
    sidebar.style.display = 'flex';
    sidebar.style.animation = 'fade-in 0.3s ease both';
  }
}

async function obtenerCuentaActiva() {
  const invocar = window.__TAURI__.core.invoke;
  const cuentas = await invocar('get_accounts');
  if (cuentas.length === 0) return null;
  const idActivo = await invocar('get_config', { key: 'active_account_id' }).catch(() => null);
  return cuentas.find(a => a.id === idActivo) || cuentas[0];
}

async function refrescarVistaCuenta() {
  const cuenta = await obtenerCuentaActiva();
  if (cuenta) actualizarVistaCuenta(cuenta);
}

async function actualizarVistaCuenta(cuenta) {
  const nombreEl = document.getElementById('account-username');
  const tipoEl = document.getElementById('account-type');
  const skinEl = document.getElementById('account-skin-img');
  if (nombreEl) nombreEl.textContent = cuenta.username;
  if (tipoEl) {
    const etiquetas = { microsoft: 'Microsoft', offline: 'Offline', azauth: 'AZauth' };
    tipoEl.textContent = etiquetas[cuenta.auth_type] || cuenta.auth_type;
  }
  if (skinEl) {
    aplicarSkinConFallback(skinEl, cuenta, 36);
  }
}

document.getElementById('account-btn')?.addEventListener('click', () => {
  abrirModalCuentas();
});

async function abrirModalCuentas() {
  const invocar = window.__TAURI__.core.invoke;
  const modal = document.getElementById('accounts-modal');
  const lista = document.getElementById('accounts-list');
  if (!modal || !lista) return;

  document.getElementById('log-panel')?.classList.remove('open');

  const cuentas = await invocar('get_accounts').catch(() => []);
  const idActivo = await invocar('get_config', { key: 'active_account_id' }).catch(() => null);
  const idActual = idActivo || (cuentas[0]?.id);

  lista.innerHTML = '';
  cuentas.forEach(cuenta => {
    const elemento = document.createElement('div');
    elemento.className = 'account-item' + (cuenta.id === idActual ? ' active' : '');

    const etiquetas = { microsoft: 'Microsoft', offline: 'Offline', azauth: 'AZauth' };

    elemento.innerHTML = `
      <div class="account-item-skin">
        <img src="./images/default/steve.png" alt="${cuenta.username}" />
      </div>
      <div class="account-item-info">
        <div class="account-item-name">${cuenta.username}</div>
        <div class="account-item-type">${etiquetas[cuenta.auth_type] || cuenta.auth_type}</div>
      </div>
      <div class="account-item-actions">
        <button class="btn-icon" title="Eliminar" data-id="${cuenta.id}">✕</button>
      </div>
    `;

    elemento.addEventListener('click', async (e) => {
      if (e.target.closest('.btn-icon')) return;
      await invocar('set_config', { key: 'active_account_id', value: cuenta.id });
      await refrescarVistaCuenta();
      modal.classList.remove('open');
      window._notify(`Cuenta activa: ${cuenta.username}`, 'success');
      await window._reloadInstances?.();
    });

    elemento.querySelector('.btn-icon').addEventListener('click', async (e) => {
      e.stopPropagation();
      if (!(await window.__TAURI__.dialog.confirm(`¿Eliminar cuenta ${cuenta.username}?`, { title: 'Eliminar cuenta', kind: 'warning' }))) return;
      await invocar('delete_account_cmd', { accountId: cuenta.id });
      if (cuenta.id === idActual) {
        await invocar('set_config', { key: 'active_account_id', value: '' }).catch(() => {});
      }
      const restantes = await invocar('get_accounts').catch(() => []);
      if (restantes.length === 0) {
        modal.classList.remove('open');
        location.reload();
      } else {
        await refrescarVistaCuenta();
        abrirModalCuentas();
      }
    });

    lista.appendChild(elemento);
    const imgCuenta = elemento.querySelector('.account-item-skin img');
    if (imgCuenta) aplicarSkinConFallback(imgCuenta, cuenta, 32);
  });

  modal.classList.add('open');
}

document.getElementById('btn-accounts-close')?.addEventListener('click', () => {
  document.getElementById('accounts-modal')?.classList.remove('open');
});

document.getElementById('btn-accounts-add')?.addEventListener('click', () => {
  document.getElementById('accounts-modal')?.classList.remove('open');
  mostrarPanel('login');
});

document.getElementById('accounts-modal')?.addEventListener('click', (e) => {
  if (e.target === document.getElementById('accounts-modal')) {
    document.getElementById('accounts-modal').classList.remove('open');
  }
});

window._getActiveAccount = obtenerCuentaActiva;
window._refreshAccountDisplay = refrescarVistaCuenta;

iniciar();

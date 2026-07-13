document.addEventListener('contextmenu', e => e.preventDefault());

async function obtenerSrcCabezaSkin(cuenta, tamano = 36) {
  const uuid = cuenta.uuid?.replace(/-/g, '') || '';
  if (uuid && (cuenta.auth_type === 'microsoft' || cuenta.auth_type === 'azauth')) {
    return `https://crafatar.com/avatars/${uuid}?size=${tamano}&overlay&default=MHF_Steve`;
  }
  return './images/default/steve.png';
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

  document.getElementById('btn-close')?.addEventListener('click', () => ventana.close());
  document.getElementById('btn-minimize')?.addEventListener('click', () => ventana.minimize());
  document.getElementById('btn-maximize')?.addEventListener('click', async () => {
    if (await ventana.isMaximized()) ventana.unmaximize();
    else ventana.maximize();
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
  try {
    const actualizacion = await window.__TAURI__.updater.check();
    if (!actualizacion?.available) return;

    notificar?.(`Descargando actualización ${actualizacion.version}...`, 'info');

    await actualizacion.downloadAndInstall();
    await window.__TAURI__.process.relaunch();
  } catch (err) {
    console.error('Error buscando actualizaciones:', err);
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
    skinEl.onerror = () => { skinEl.src = './images/default/steve.png'; };
    skinEl.src = await obtenerSrcCabezaSkin(cuenta, 36);
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
      if (!confirm(`¿Eliminar cuenta ${cuenta.username}?`)) return;
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
    obtenerSrcCabezaSkin(cuenta, 32).then(src => {
      const img = elemento.querySelector('.account-item-skin img');
      if (img) {
        img.onerror = () => { img.src = './images/default/steve.png'; };
        img.src = src;
      }
    });
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

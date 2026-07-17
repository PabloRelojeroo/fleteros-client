let adminAcceso = { isSuperAdmin: false, editableInstances: [] };
let adminInstanciasCache = {};
let adminEditandoNombre = null;
let adminLogoPathElegido = null;
let adminBackgroundPathElegido = null;
let adminPermisosToken = null;

async function identidadActual() {
  const cuenta = await window._getActiveAccount?.();
  if (!cuenta) return null;
  return { uuid: cuenta.uuid, username: cuenta.username };
}

async function inicializarAccesoAdmin() {
  const invocar = window.__TAURI__.core.invoke;
  const identidad = await identidadActual();
  const navBtn = document.getElementById('nav-admin-btn');
  if (!identidad) {
    if (navBtn) navBtn.style.display = 'none';
    return;
  }

  try {
    const acceso = await invocar('admin_check_access', {
      baseUrl: window.BRAND.urlBase,
      uuid: identidad.uuid,
      username: identidad.username,
    });

    adminAcceso = { isSuperAdmin: acceso.isSuperAdmin, editableInstances: acceso.editableInstances || [] };

    const tienePermiso = adminAcceso.isSuperAdmin || adminAcceso.editableInstances.length > 0;
    if (navBtn) navBtn.style.display = tienePermiso ? '' : 'none';

    const tabPermisos = document.getElementById('admin-tab-permissions');
    if (tabPermisos) tabPermisos.style.display = adminAcceso.isSuperAdmin ? '' : 'none';

    if (tienePermiso) await iniciarAdmin();
  } catch (err) {
    if (navBtn) navBtn.style.display = 'none';
    console.error('Error chequeando acceso admin:', err);
  }
}

let adminYaInicializado = false;

async function iniciarAdmin() {
  if (adminYaInicializado) {
    await cargarInstanciasAdmin();
    return;
  }
  adminYaInicializado = true;

  document.querySelectorAll('.admin-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.admin-tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.admin-view').forEach(v => v.classList.remove('active'));
      tab.classList.add('active');
      const vista = document.getElementById(`admin-view-${tab.dataset.adminTab}`);
      if (vista) vista.classList.add('active');
    });
  });

  document.getElementById('btn-admin-new-instance')?.addEventListener('click', () => abrirFormularioInstancia(null));
  document.getElementById('btn-admin-cancel')?.addEventListener('click', cerrarFormularioInstancia);
  document.getElementById('btn-admin-save')?.addEventListener('click', guardarInstanciaAdmin);
  document.getElementById('btn-admin-delete')?.addEventListener('click', eliminarInstanciaAdmin);

  document.getElementById('btn-admin-pick-logo')?.addEventListener('click', () => elegirImagen('logo'));
  document.getElementById('btn-admin-pick-background')?.addEventListener('click', () => elegirImagen('background'));

  document.getElementById('btn-admin-add-superadmin')?.addEventListener('click', agregarSuperAdmin);
  document.getElementById('btn-admin-add-editor')?.addEventListener('click', agregarEditor);

  await cargarInstanciasAdmin();
}

async function cargarInstanciasAdmin() {
  const invocar = window.__TAURI__.core.invoke;
  const grilla = document.getElementById('admin-instances-grid');
  if (!grilla) return;

  const identidad = await identidadActual();
  if (!identidad) return;

  try {
    const instancias = await invocar('admin_get_instances', {
      baseUrl: window.BRAND.urlBase,
      uuid: identidad.uuid,
      username: identidad.username,
    });

    adminInstanciasCache = instancias || {};
    grilla.innerHTML = '';

    Object.values(adminInstanciasCache).forEach(inst => {
      const card = document.createElement('div');
      card.className = 'admin-instance-card';
      const logo = inst.customization?.logo
        ? `${window.BRAND.urlBase}/files/${inst.customization.logo}`
        : './images/icon.png';
      card.innerHTML = `
        <img src="${logo}" onerror="this.src='./images/icon.png'" />
        <span class="admin-instance-card-name">${inst.customization?.name_display || inst.name}</span>
      `;
      card.addEventListener('click', () => abrirFormularioInstancia(inst));
      grilla.appendChild(card);
    });
  } catch (err) {
    window._notify?.(`Error cargando instancias: ${err}`, 'error');
  }
}

function abrirFormularioInstancia(inst) {
  adminEditandoNombre = inst?.name || null;
  adminLogoPathElegido = null;
  adminBackgroundPathElegido = null;

  document.getElementById('admin-field-name').value = inst?.name || '';
  document.getElementById('admin-field-name').disabled = !!inst;
  document.getElementById('admin-field-displayName').value = inst?.customization?.name_display || '';
  document.getElementById('admin-field-description').value = inst?.customization?.description || '';
  document.getElementById('admin-field-minecraftVersion').value = inst?.loadder?.minecraft_version || '1.20.1';
  document.getElementById('admin-field-loaderType').value = inst?.loadder?.loadder_type || 'none';
  document.getElementById('admin-field-loaderVersion').value = inst?.loadder?.loadder_version || 'latest';
  document.getElementById('admin-logo-path').textContent = inst?.customization?.logo || '—';
  document.getElementById('admin-background-path').textContent = inst?.customization?.background || '—';
  document.getElementById('admin-field-serverName').value = inst?.status?.nameServer || '';
  document.getElementById('admin-field-serverIP').value = inst?.status?.ip || '';
  document.getElementById('admin-field-serverPort').value = inst?.status?.port || 25565;
  document.getElementById('admin-field-whitelistActive').checked = !!inst?.whitelistActive;
  document.getElementById('admin-field-whitelist').value = (inst?.whitelist || []).join('\n');

  document.getElementById('btn-admin-delete').style.display = inst ? '' : 'none';
  document.getElementById('admin-instance-form').style.display = '';
}

function cerrarFormularioInstancia() {
  document.getElementById('admin-instance-form').style.display = 'none';
  adminEditandoNombre = null;
}

async function elegirImagen(kind) {
  const abrir = window.__TAURI__.dialog.open;
  const archivo = await abrir({
    multiple: false,
    filters: [{ name: 'Imagen', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
  });
  if (!archivo) return;

  if (kind === 'logo') {
    adminLogoPathElegido = archivo;
    document.getElementById('admin-logo-path').textContent = archivo;
  } else {
    adminBackgroundPathElegido = archivo;
    document.getElementById('admin-background-path').textContent = archivo;
  }
}

async function guardarInstanciaAdmin() {
  const invocar = window.__TAURI__.core.invoke;
  const notificar = window._notify;
  const identidad = await identidadActual();
  if (!identidad) return;

  const nombre = document.getElementById('admin-field-name').value.trim();
  if (!nombre) { notificar('El nombre interno es obligatorio', 'warning'); return; }

  try {
    let logoPath = document.getElementById('admin-logo-path').textContent;
    let backgroundPath = document.getElementById('admin-background-path').textContent;
    if (logoPath === '—') logoPath = adminInstanciasCache[nombre]?.customization?.logo || '';
    if (backgroundPath === '—') backgroundPath = adminInstanciasCache[nombre]?.customization?.background || '';

    if (adminLogoPathElegido) {
      logoPath = await invocar('admin_upload_image', {
        baseUrl: window.BRAND.urlBase,
        uuid: identidad.uuid,
        username: identidad.username,
        instanceName: nombre,
        kind: 'logo',
        filePath: adminLogoPathElegido,
      });
    }
    if (adminBackgroundPathElegido) {
      backgroundPath = await invocar('admin_upload_image', {
        baseUrl: window.BRAND.urlBase,
        uuid: identidad.uuid,
        username: identidad.username,
        instanceName: nombre,
        kind: 'background',
        filePath: adminBackgroundPathElegido,
      });
    }

    const instancia = {
      name: nombre,
      displayName: document.getElementById('admin-field-displayName').value.trim(),
      description: document.getElementById('admin-field-description').value.trim(),
      minecraftVersion: document.getElementById('admin-field-minecraftVersion').value.trim(),
      loaderType: document.getElementById('admin-field-loaderType').value.trim(),
      loaderVersion: document.getElementById('admin-field-loaderVersion').value.trim(),
      logoPath,
      backgroundPath,
      serverName: document.getElementById('admin-field-serverName').value.trim(),
      serverIP: document.getElementById('admin-field-serverIP').value.trim(),
      serverPort: parseInt(document.getElementById('admin-field-serverPort').value, 10) || 25565,
      whitelistActive: document.getElementById('admin-field-whitelistActive').checked,
      whitelist: document.getElementById('admin-field-whitelist').value
        .split('\n').map(s => s.trim()).filter(Boolean),
    };

    await invocar('admin_save_instance', {
      baseUrl: window.BRAND.urlBase,
      uuid: identidad.uuid,
      username: identidad.username,
      oldName: adminEditandoNombre || '',
      instance: instancia,
    });

    notificar('Instancia guardada', 'success');
    cerrarFormularioInstancia();
    await cargarInstanciasAdmin();
  } catch (err) {
    notificar(`Error guardando instancia: ${err}`, 'error');
  }
}

async function eliminarInstanciaAdmin() {
  if (!adminEditandoNombre) return;
  if (!confirm(`¿Eliminar la instancia "${adminEditandoNombre}"? Esto no borra los archivos del servidor.`)) return;

  const invocar = window.__TAURI__.core.invoke;
  const notificar = window._notify;
  const identidad = await identidadActual();
  if (!identidad) return;

  try {
    await invocar('admin_delete_instance', {
      baseUrl: window.BRAND.urlBase,
      uuid: identidad.uuid,
      username: identidad.username,
      name: adminEditandoNombre,
    });
    notificar('Instancia eliminada', 'success');
    cerrarFormularioInstancia();
    await cargarInstanciasAdmin();
  } catch (err) {
    notificar(`Error eliminando instancia: ${err}`, 'error');
  }
}

// --- Permisos: requiere el login clásico (usuario/contraseña) del panel admin ---
// para que un editor asignado por UUID no pueda otorgarse a sí mismo más acceso.

async function asegurarLoginPermisos() {
  if (adminPermisosToken) return true;

  const usuario = prompt('Usuario admin del backend:');
  if (!usuario) return false;
  const password = prompt('Contraseña admin del backend:');
  if (!password) return false;

  const invocar = window.__TAURI__.core.invoke;
  try {
    adminPermisosToken = await invocar('admin_login', {
      baseUrl: window.BRAND.urlBase,
      username: usuario,
      password,
    });
    return true;
  } catch (err) {
    window._notify?.(`Login admin fallido: ${err}`, 'error');
    return false;
  }
}

async function cargarPermisos() {
  if (!(await asegurarLoginPermisos())) return;
  const invocar = window.__TAURI__.core.invoke;

  try {
    const permisos = await invocar('admin_list_permissions', {
      baseUrl: window.BRAND.urlBase,
      token: adminPermisosToken,
    });
    renderPermisos(permisos || { superAdmins: [], editors: {} });
  } catch (err) {
    window._notify?.(`Error cargando permisos: ${err}`, 'error');
  }
}

function renderPermisos(permisos) {
  const listaSuper = document.getElementById('admin-superadmins-list');
  if (listaSuper) {
    listaSuper.innerHTML = '';
    (permisos.superAdmins || []).forEach(uuid => {
      const item = document.createElement('div');
      item.className = 'admin-uuid-item';
      item.innerHTML = `<span>${uuid}</span><button class="btn-icon" title="Quitar">✕</button>`;
      item.querySelector('.btn-icon').addEventListener('click', () => quitarSuperAdmin(uuid, permisos));
      listaSuper.appendChild(item);
    });
  }

  const listaEditores = document.getElementById('admin-editors-list');
  if (listaEditores) {
    listaEditores.innerHTML = '';
    Object.entries(permisos.editors || {}).forEach(([instancia, uuids]) => {
      const grupo = document.createElement('div');
      grupo.className = 'admin-editor-group';
      grupo.innerHTML = `<div class="admin-editor-group-title">${instancia}</div>`;
      uuids.forEach(uuid => {
        const item = document.createElement('div');
        item.className = 'admin-uuid-item';
        item.innerHTML = `<span>${uuid}</span><button class="btn-icon" title="Quitar">✕</button>`;
        item.querySelector('.btn-icon').addEventListener('click', () => quitarEditor(instancia, uuid, permisos));
        grupo.appendChild(item);
      });
      listaEditores.appendChild(grupo);
    });
  }

  window._admin_permisos_actuales = permisos;
}

async function guardarPermisos(permisos) {
  const invocar = window.__TAURI__.core.invoke;
  try {
    await invocar('admin_set_permissions', {
      baseUrl: window.BRAND.urlBase,
      token: adminPermisosToken,
      permissions: permisos,
    });
    renderPermisos(permisos);
    window._notify?.('Permisos actualizados', 'success');
  } catch (err) {
    window._notify?.(`Error guardando permisos: ${err}`, 'error');
  }
}

async function agregarSuperAdmin() {
  if (!(await asegurarLoginPermisos())) return;
  const input = document.getElementById('admin-new-superadmin-uuid');
  const uuid = input?.value.trim();
  if (!uuid) return;

  const permisos = window._admin_permisos_actuales || { superAdmins: [], editors: {} };
  if (!permisos.superAdmins.includes(uuid)) permisos.superAdmins.push(uuid);
  await guardarPermisos(permisos);
  input.value = '';
}

async function quitarSuperAdmin(uuid, permisos) {
  permisos.superAdmins = (permisos.superAdmins || []).filter(u => u !== uuid);
  await guardarPermisos(permisos);
}

async function agregarEditor() {
  if (!(await asegurarLoginPermisos())) return;
  const instanciaInput = document.getElementById('admin-new-editor-instance');
  const uuidInput = document.getElementById('admin-new-editor-uuid');
  const instancia = instanciaInput?.value.trim();
  const uuid = uuidInput?.value.trim();
  if (!instancia || !uuid) return;

  const permisos = window._admin_permisos_actuales || { superAdmins: [], editors: {} };
  permisos.editors[instancia] = permisos.editors[instancia] || [];
  if (!permisos.editors[instancia].includes(uuid)) permisos.editors[instancia].push(uuid);
  await guardarPermisos(permisos);
  instanciaInput.value = '';
  uuidInput.value = '';
}

async function quitarEditor(instancia, uuid, permisos) {
  permisos.editors[instancia] = (permisos.editors[instancia] || []).filter(u => u !== uuid);
  await guardarPermisos(permisos);
}

document.querySelector('[data-admin-tab="permissions"]')?.addEventListener('click', cargarPermisos);

window._initAdminAccess = inicializarAccesoAdmin;

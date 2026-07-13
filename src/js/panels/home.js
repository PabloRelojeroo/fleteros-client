const URL_BASE = 'https://pablorelojero.online';
const URL_CONFIG = `${URL_BASE}/launcher/config-launcher/config.json`;
const URL_INSTANCIAS = `${URL_BASE}/files/index.php`;

let instanciaSeleccionada = null;
let cancelarEscuchaDescarga = null;
let cancelarEscuchaLog = null;
let cancelarEscuchaSalidaJuego = null;

function alternarPanelLogs() {
  const panel = document.getElementById('log-panel');
  if (!panel) return;
  panel.classList.toggle('open');
}

async function inicializarHome() {
  await cargarInstancias();
  document.getElementById('btn-play')?.addEventListener('click', manejarJugar);
  document.getElementById('expanded-back')?.addEventListener('click', cerrarExpandido);
  document.getElementById('btn-cancel-download')?.addEventListener('click', async () => {
    await window.__TAURI__.core.invoke('cancel_download');
    ocultarOverlayDescarga();
    window._notify('Descarga cancelada', 'warning');
  });

  document.getElementById('nav-logs-btn')?.addEventListener('click', alternarPanelLogs);
  document.getElementById('log-modal-close')?.addEventListener('click', () => {
    document.getElementById('log-panel')?.classList.remove('open');
  });
  document.getElementById('log-panel')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('log-panel')) {
      document.getElementById('log-panel').classList.remove('open');
    }
  });

  const escuchar = window.__TAURI__.event.listen;

  escuchar('game-launching', () => {
    agregarLog('Mc iniciado', null);
    establecerEstadoLog('running', 'Jugando...');
    document.getElementById('log-panel')?.classList.add('open');
    document.getElementById('log-nav-dot')?.classList.add('running');
  });

  escuchar('game-exited', (e) => {
    const codigo = e.payload;
    establecerEstadoLog('', `Salió (código ${codigo})`);
    document.getElementById('log-nav-dot')?.classList.remove('running');
    agregarLog(`[Launcher] Proceso terminó con código: ${codigo}`, codigo !== 0 ? 'error' : null);
    if (codigo !== 0) document.getElementById('log-panel')?.classList.add('open');
  });

  escuchar('game-error', (e) => {
    establecerEstadoLog('', 'Error al iniciar');
    agregarLog(`Error: ${e.payload}`, 'error');
    document.getElementById('log-nav-dot')?.classList.remove('running');
  });

  if (cancelarEscuchaLog) cancelarEscuchaLog();
  cancelarEscuchaLog = await escuchar('game-log', (e) => {
    agregarLog(e.payload);
  });
}

function agregarLog(linea, tipo) {
  const contenido = document.getElementById('log-content');
  if (!contenido) return;
  const div = document.createElement('div');
  div.className = 'log-line' + (tipo ? ` ${tipo}` : '');
  const minuscula = linea.toLowerCase();
  if (!tipo) {
    if (minuscula.includes('error') || minuscula.includes('exception')) div.className += ' error';
    else if (minuscula.includes('warn')) div.className += ' warn';
  }
  div.textContent = linea;
  contenido.appendChild(div);
  while (contenido.children.length > 500) contenido.removeChild(contenido.firstChild);
  contenido.scrollTop = contenido.scrollHeight;
}

function limpiarLog() {
  const contenido = document.getElementById('log-content');
  if (contenido) contenido.innerHTML = '';
}

function establecerEstadoLog(claseDot, texto) {
  const dot = document.getElementById('log-status-dot');
  const elementoTexto = document.getElementById('log-status-text');
  if (dot) { dot.className = 'log-status-dot' + (claseDot ? ` ${claseDot}` : ''); }
  if (elementoTexto) elementoTexto.textContent = texto;
}

async function cargarInstancias() {
  const invocar = window.__TAURI__.core.invoke;
  const grilla = document.getElementById('instances-grid');
  if (!grilla) return;

  cerrarExpandido();
  instanciaSeleccionada = null;

  grilla.innerHTML = `<div style="color:var(--text-muted);padding:20px;grid-column:1/-1;text-align:center;">
    <div class="spinner" style="margin:0 auto 12px;"></div>
    Cargando instancias...
  </div>`;

  try {
    const configuracion = await invocar('get_launcher_config', { configUrl: URL_CONFIG });

    if (configuracion.maintenance === true) {
      grilla.innerHTML = `<div style="color:var(--warning);padding:20px;grid-column:1/-1;">
        ⚠ ${configuracion.maintenance_message || 'El launcher está en mantenimiento.'}
      </div>`;
      return;
    }

    const cuenta = await window._getActiveAccount?.() || null;
    const nombreUsuario = cuenta?.username || '';

    const instancias = await invocar('get_instances', { instancesUrl: URL_INSTANCIAS });

    const visibles = (instancias || []).filter(instancia => {
      if (!instancia.whitelist_active) return true;
      if (instancia.whitelist?.includes(nombreUsuario)) return true;
      return false;
    });

    grilla.innerHTML = '';

    if (visibles.length === 0) {
      grilla.innerHTML = `<p style="color:var(--text-muted);padding:20px;">No hay instancias disponibles.</p>`;
      return;
    }

    const observador = new IntersectionObserver((entradas) => {
      entradas.forEach(entrada => {
        if (entrada.isIntersecting) {
          entrada.target.style.animation = 'reveal-up 350ms cubic-bezier(0.34, 1.56, 0.64, 1) both';
          observador.unobserve(entrada.target);
        }
      });
    }, { threshold: 0.1 });

    visibles.forEach((instancia, i) => {
      const elemento = crearTarjetaInstancia(instancia);
      elemento.style.opacity = '0';
      elemento.style.animationDelay = `${i * 60}ms`;
      grilla.appendChild(elemento);
      observador.observe(elemento);
    });

  } catch (err) {
    const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
    grilla.innerHTML = `<p style="color:var(--error);padding:20px;">Error cargando instancias: ${mensaje}</p>`;
    console.error('cargarInstancias error:', err);
  }
}

function resolverUrlImagen(ruta) {
  if (!ruta) return '';
  if (ruta.startsWith('http')) return ruta;
  return `${URL_BASE}/files/${ruta}`;
}

function crearTarjetaInstancia(instancia) {
  const elemento = document.createElement('div');
  elemento.className = 'library-item';

  const nombreMostrado = instancia.customization?.name_display || instancia.name;
  const urlLogo = resolverUrlImagen(instancia.customization?.logo);
  const urlFondo = resolverUrlImagen(instancia.customization?.background);

  const logoInterno = urlLogo
    ? `<img src="${urlLogo}" alt="${nombreMostrado}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'" /><span class="library-item-logo-placeholder" style="display:none">${nombreMostrado[0]}</span>`
    : `<span class="library-item-logo-placeholder">${nombreMostrado[0]}</span>`;

  elemento.innerHTML = `
    <div class="library-item-logo">${logoInterno}</div>
    <span class="library-item-name">${nombreMostrado}</span>
  `;

  elemento.addEventListener('click', () => abrirExpandido(instancia, urlFondo, urlLogo));
  return elemento;
}

function abrirExpandido(instancia, urlFondo, urlLogo) {
  instanciaSeleccionada = instancia;

  const nombreMostrado = instancia.customization?.name_display || instancia.name;
  const version = instancia.loader?.minecraft_version || '?';
  const tipoLoader = instancia.loader?.loader_type || 'Vanilla';
  const versionLoader = instancia.loader?.loader_version || '';

  const expandido = document.getElementById('instance-expanded');
  const fondo = document.getElementById('expanded-bg');
  const logo = document.getElementById('expanded-logo');
  const nombre = document.getElementById('expanded-name');
  const meta = document.getElementById('expanded-meta');

  if (urlFondo) {
    fondo.style.backgroundImage = `url('${urlFondo}')`;
    fondo.classList.remove('no-bg');
    document.getElementById('background').style.backgroundImage = `url('${urlFondo}')`;
  } else {
    fondo.style.backgroundImage = '';
    fondo.classList.add('no-bg');
  }

  if (urlLogo) {
    logo.src = urlLogo;
    logo.style.display = '';
  } else {
    logo.style.display = 'none';
  }

  nombre.textContent = nombreMostrado;
  meta.innerHTML = `<span>${version}</span><span>${`${tipoLoader} ${versionLoader}`.trim()}</span>`;

  expandido.classList.add('open');
}

function cerrarExpandido() {
  document.getElementById('instance-expanded')?.classList.remove('open');
  instanciaSeleccionada = null;
}

async function manejarJugar() {
  const invocar = window.__TAURI__.core.invoke;
  const escuchar = window.__TAURI__.event.listen;
  const notificar = window._notify;

  if (!instanciaSeleccionada) return;

  const cuenta = await window._getActiveAccount?.() || null;
  if (!cuenta) {
    notificar('Iniciá sesión primero', 'warning');
    return;
  }

  const nombreMostrado = instanciaSeleccionada.customization?.name_display || instanciaSeleccionada.name;
  const directorioJuego = await obtenerDirectorioJuego(instanciaSeleccionada);

  const botonJugar = document.getElementById('btn-play');
  if (botonJugar) {
    botonJugar.classList.add('launching');
    botonJugar.textContent = 'Iniciando';
    botonJugar.disabled = true;
  }

  mostrarOverlayDescarga();

  try {
    const descargasConcurrentes = parseInt((await invocar('get_config', { key: 'concurrent_downloads' })) || '10');
    const versionMc = instanciaSeleccionada.loader?.minecraft_version || '1.20.1';
    const tipoLoader = instanciaSeleccionada.loader?.loader_type || 'none';
    const verLoader = instanciaSeleccionada.loader?.loader_version || 'latest';

    let rutaJava = (await invocar('get_config', { key: 'java_path' }).catch(() => '')) || '';
    if (!rutaJava) {
      rutaJava = await invocar('get_best_java_for_version', { mcVersion: versionMc }).catch(() => '');
    }
    if (!rutaJava) rutaJava = 'java';

    agregarLog(`[Launcher] Instancia: ${instanciaSeleccionada.name}`, null);
    agregarLog(`[Launcher] MC: ${versionMc} | Loader: ${tipoLoader} ${verLoader}`, null);
    agregarLog(`[Launcher] URL archivos: ${instanciaSeleccionada.url || '(ninguna)'}`, null);

    cancelarEscuchaDescarga = await escuchar('download-progress', evento => {
      actualizarProgresoDescarga(evento.payload);
    });

    const resultado = await invocar('download_instance', {
      instanceId: instanciaSeleccionada.name,
      version: versionMc,
      gameDir: directorioJuego,
      maxConcurrent: descargasConcurrentes,
      instanceUrl: instanciaSeleccionada.url || null,
      loaderType: tipoLoader !== 'none' ? tipoLoader : null,
      loaderVersion: verLoader,
      ignored: instanciaSeleccionada.ignored || [],
      javaPath: rutaJava,
    });

    cancelarEscuchaDescarga?.();
    cancelarEscuchaDescarga = null;
    ocultarOverlayDescarga();

    agregarLog(`[Launcher] Java: ${rutaJava}`, null);
    agregarLog(`[Launcher] Version: ${versionMc}`, null);
    agregarLog(`[Launcher] GameDir: ${directorioJuego}`, null);

    notificar(`Lanzando ${nombreMostrado}...`, 'info');

    try { await invocar('update_rpc', { details: `Jugando ${nombreMostrado}`, state: versionMc }); } catch (_) {}

    const obtenerSeguro = async (clave, porDefecto) => { try { return (await invocar('get_config', { key: clave })) || porDefecto; } catch { return porDefecto; } };
    const ramMinima = parseInt(await obtenerSeguro('ram_min', '1024'));
    const ramMaxima = parseInt(await obtenerSeguro('ram_max', '4096'));
    const ancho  = parseInt(await obtenerSeguro('res_width', '1280'));
    const alto = parseInt(await obtenerSeguro('res_height', '720'));
    const ocultarAlJugar = (await obtenerSeguro('hide_on_launch', 'true')) !== 'false';

    if (ocultarAlJugar) {
      const ventana = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
      ventana.minimize().catch(() => {});
    }

    const argumentosJvmExtra = [];

    await invocar('launch_game', {
      config: {
        java_path: rutaJava,
        game_dir: directorioJuego,
        version: versionMc,
        min_ram_mb: ramMinima,
        max_ram_mb: ramMaxima,
        width: ancho,
        height: alto,
        username: cuenta.username,
        uuid: cuenta.uuid,
        access_token: cuenta.access_token,
        main_class: resultado.mainClass || 'net.minecraft.client.main.Main',
        extra_jvm_args: argumentosJvmExtra.length > 0 ? argumentosJvmExtra : null,
      }
    });

    notificar('Minecraft iniciado', 'success');

  } catch (err) {
    cancelarEscuchaDescarga?.();
    cancelarEscuchaDescarga = null;
    ocultarOverlayDescarga();
    const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
    notificar(`Error al lanzar: ${mensaje}`, 'error', 8000);
    agregarLog(`[Error] ${mensaje}`, 'error');
    document.getElementById('log-panel')?.classList.add('open');
    console.error('manejarJugar error:', err);
  } finally {
    if (botonJugar) {
      botonJugar.classList.remove('launching');
      botonJugar.textContent = '▶ JUGAR';
      botonJugar.disabled = false;
    }
  }
}

function mostrarOverlayDescarga() {
  const overlay = document.getElementById('download-overlay');
  if (overlay) overlay.style.display = 'flex';
}

function ocultarOverlayDescarga() {
  const overlay = document.getElementById('download-overlay');
  if (overlay) overlay.style.display = 'none';
}

function actualizarProgresoDescarga(progreso) {
  const asignar = (id, valor) => { const el = document.getElementById(id); if (el) el.textContent = valor; };
  const mapaFases = { fetch: 'Obteniendo datos', download: 'Descargando', verify: 'Verificando', extract: 'Extrayendo' };
  asignar('dl-phase', mapaFases[progreso.phase] || progreso.phase);
  asignar('dl-file', progreso.file);
  asignar('dl-speed', formatearVelocidad(progreso.speed_bps));
  asignar('dl-eta', formatearEta(progreso.eta_seconds));
  asignar('dl-counter', `${progreso.files_done} / ${progreso.files_total} archivos`);
  const porcentaje = progreso.total > 0 ? Math.round((progreso.downloaded / progreso.total) * 100) : 0;
  const relleno = document.getElementById('dl-progress-fill');
  if (relleno) relleno.style.width = `${porcentaje}%`;
}

function formatearVelocidad(velocidadBps) {
  if (!velocidadBps || velocidadBps <= 0) return '—';
  if (velocidadBps > 1e6) return `${(velocidadBps / 1e6).toFixed(1)} MB/s`;
  if (velocidadBps > 1000) return `${(velocidadBps / 1000).toFixed(0)} KB/s`;
  return `${Math.round(velocidadBps)} B/s`;
}

function formatearEta(segundos) {
  if (!segundos || segundos <= 0) return '—';
  if (segundos < 60) return `${Math.round(segundos)}s`;
  return `${Math.round(segundos / 60)}m ${Math.round(segundos % 60)}s`;
}

async function obtenerDirectorioJuego(instancia) {
  let base = await window.__TAURI__.path.appDataDir();
  if (!base.endsWith('/') && !base.endsWith('\\')) base += '\\';
  return `${base}FleterosClient\\${instancia.name}`;
}

window._initHome = inicializarHome;
window._reloadInstances = cargarInstancias;

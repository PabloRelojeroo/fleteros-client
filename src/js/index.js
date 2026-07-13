const relleno = document.getElementById('progress-fill');
const elementoEstado = document.getElementById('splash-status');

function establecerProgreso(pct, estado) {
  if (relleno) relleno.style.width = `${pct}%`;
  if (elementoEstado) elementoEstado.textContent = estado;
}

const esperar = ms => new Promise(r => setTimeout(r, ms));

async function iniciar() {
  try {
    console.log('__TAURI__ available:', !!window.__TAURI__);
    console.log('__TAURI__ keys:', window.__TAURI__ ? Object.keys(window.__TAURI__) : 'none');

    establecerProgreso(30, 'Cargando...');
    await esperar(400);

    establecerProgreso(70, 'Iniciando launcher...');
    await esperar(300);

    establecerProgreso(100, 'Listo');
    await esperar(400);

    window.location.href = 'launcher.html';

  } catch (err) {
    console.error('Boot error:', err);
    if (elementoEstado) {
      const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
      elementoEstado.textContent = `Error: ${mensaje}`;
      elementoEstado.style.color = '#ef4444';
    }
  }
}

iniciar();

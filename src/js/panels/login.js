async function iniciarLogin({ onLogin }) {
  const invocar = window.__TAURI__.core.invoke;
  const notificar = window._notify;

  document.querySelectorAll('.auth-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.auth-tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.auth-form').forEach(f => f.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(`form-${tab.dataset.form}`)?.classList.add('active');
    });
  });

  document.getElementById('btn-microsoft-login')?.addEventListener('click', async () => {
    establecerCargando(true);
    try {
      const cuenta = await invocar('auth_microsoft');
      notificar(`¡Bienvenido, ${cuenta.username}!`, 'success');
      onLogin(cuenta);
    } catch (err) {
      const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
      notificar(`Error Microsoft: ${mensaje}`, 'error');
      console.error(err);
    } finally {
      establecerCargando(false);
    }
  });

  document.getElementById('btn-offline-login')?.addEventListener('click', async () => {
    const nombreUsuario = document.getElementById('offline-username')?.value.trim();
    if (!nombreUsuario) { notificar('Ingresá un nombre de usuario', 'warning'); return; }

    establecerCargando(true);
    try {
      const cuenta = await invocar('auth_offline', { username: nombreUsuario });
      notificar(`Entrando como ${cuenta.username}`, 'success');
      onLogin(cuenta);
    } catch (err) {
      const mensaje = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
      notificar(`Error: ${mensaje}`, 'error');
      console.error(err);
    } finally {
      establecerCargando(false);
    }
  });
}

function establecerCargando(cargando) {
  document.querySelector('.login-card')?.classList.toggle('loading', cargando);
}

window._initLogin = iniciarLogin;

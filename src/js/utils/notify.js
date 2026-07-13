function notificar(mensaje, tipo = 'info', duracion = 4000) {
  const contenedor = document.getElementById('notifications');
  if (!contenedor) return;

  const elemento = document.createElement('div');
  elemento.className = `notification ${tipo}`;
  elemento.textContent = mensaje;
  contenedor.appendChild(elemento);

  elemento.addEventListener('click', () => descartar());

  function descartar() {
    if (!elemento.parentNode) return;
    elemento.classList.add('removing');
    const eliminar = () => { if (elemento.parentNode) elemento.remove(); };
    elemento.addEventListener('animationend', eliminar, { once: true });
    setTimeout(eliminar, 400);
  }

  setTimeout(descartar, duracion);
}

window._notify = notificar;

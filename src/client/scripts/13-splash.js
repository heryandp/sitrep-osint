(function initSplash(){
  const el = $('#splash');
  if (!el) return;
  const bar = $('#spl-bar');
  const st = $('#spl-status');
  const skip = $('#spl-skip');
  const reduced = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  const startedAt = performance.now();
  const minMs = reduced ? 320 : 1280;
  const maxMs = reduced ? 520 : 2300;
  let closed = false;
  let apiDone = false;
  let exitTimer = 0;

  function setProgress(pct, label){
    if (bar) bar.style.transform = 'scaleX(' + Math.max(0.08, Math.min(1, pct)) + ')';
    if (st && label) st.textContent = label;
  }

  function finishSplash(immediate){
    if (closed) return;
    closed = true;
    clearTimeout(exitTimer);
    setProgress(1, 'Konsol siap');
    el.classList.add('is-exit');
    el.setAttribute('aria-hidden', 'true');
    const removeDelay = immediate ? 160 : (reduced ? 200 : 460);
    setTimeout(() => { el.classList.add('is-gone'); }, removeDelay);
  }

  function scheduleFinish(){
    if (closed) return;
    const elapsed = performance.now() - startedAt;
    const wait = Math.max(0, minMs - elapsed);
    clearTimeout(exitTimer);
    exitTimer = setTimeout(() => finishSplash(false), wait);
  }

  window.splashStep = function(pct, label, ready){
    if (closed) return;
    setProgress(pct, label);
    if (ready) {
      apiDone = true;
      scheduleFinish();
    }
  };

  window.replaySplash = function(){
    closed = false;
    apiDone = false;
    el.classList.remove('is-gone', 'is-exit');
    el.removeAttribute('aria-hidden');
    setProgress(0.24, 'Menyiapkan konsol…');
    setTimeout(() => setProgress(0.76, 'Sinkronisasi 67 sumber & peta…'), 380);
    clearTimeout(exitTimer);
    exitTimer = setTimeout(() => finishSplash(false), minMs + 220);
  };

  if (skip) {
    skip.addEventListener('click', e => {
      e.stopPropagation();
      finishSplash(true);
    });
  }
  el.addEventListener('click', e => {
    if (e.target === el) finishSplash(true);
  });
  document.addEventListener('keydown', e => {
    if (!closed && e.key === 'Escape') finishSplash(true);
  });

  setTimeout(() => { if (!closed && !apiDone) setProgress(0.48, 'Menghubungkan konsol…'); }, 320);
  exitTimer = setTimeout(() => finishSplash(false), maxMs);
})();

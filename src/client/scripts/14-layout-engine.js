'use strict';

(() => {
  const STORAGE_KEY = 'sitrep_layout_v1';
  const consoleEl = document.getElementById('main-console');
  if (!consoleEl) return;

  const DEFAULT_LAYOUT = {
    panes: {
      situasi: { visible: true, width: '420px' },
      mkt:     { visible: true, height: 'auto' },
      social:  { visible: true, flex: 0.8 },
      wire:    { visible: true, flex: 1.1 },
      cctv:    { visible: true, flex: 1.1 }
    },
    order: ['social', 'wire', 'cctv'],
    maximized: null
  };

  let state = loadLayout();

  function loadLayout() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        return {
          panes: Object.assign({}, DEFAULT_LAYOUT.panes, parsed.panes || {}),
          order: Array.isArray(parsed.order) && parsed.order.length ? parsed.order : DEFAULT_LAYOUT.order.slice(),
          maximized: parsed.maximized || null
        };
      }
    } catch (_) {}
    return structuredClone(DEFAULT_LAYOUT);
  }

  function saveLayout() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        panes: state.panes,
        order: state.order,
        maximized: state.maximized
      }));
    } catch (_) {}
  }

  function applyLayout() {
    if (!consoleEl) return;

    const allPanes = consoleEl.querySelectorAll('.pane[data-pane]');
    allPanes.forEach(p => {
      const id = p.dataset.pane;
      const isMax = state.maximized === id;
      p.classList.toggle('is-maximized', isMax);

      const maxBtn = p.querySelector('.ph-max');
      if (maxBtn) {
        maxBtn.setAttribute('title', isMax ? 'Kembalikan ukuran normal' : 'Maksimalkan / Fokus panel');
        maxBtn.setAttribute('aria-label', isMax ? 'Kembalikan ukuran' : 'Maksimalkan panel');
        const ic = maxBtn.querySelector('use');
        if (ic) ic.setAttribute('href', isMax ? '#i-contract' : '#i-expand');
      }

      const isVis = state.panes[id] && state.panes[id].visible !== false;
      p.classList.toggle('is-hidden', !isVis && !isMax);
    });

    if (state.maximized) {
      triggerResize();
      updateMenuCheckboxes();
      return;
    }

    const sitVis = state.panes.situasi && state.panes.situasi.visible !== false;
    const mktVis = state.panes.mkt && state.panes.mkt.visible !== false;
    const activeBottom = state.order.filter(id => state.panes[id] && state.panes[id].visible !== false);
    const bottomCols = activeBottom.length;

    let gridAreas = '';
    let gridCols = '';
    let gridRows = '';

    const sitW = (state.panes.situasi && state.panes.situasi.width) || '420px';
    const mktH = (state.panes.mkt && state.panes.mkt.height) || 'auto';

    if (sitVis) {
      if (bottomCols > 0) {
        const colFractions = activeBottom.map(id => {
          const fl = (state.panes[id] && state.panes[id].flex) || 1;
          return `minmax(0, ${fl}fr)`;
        }).join(' ');

        gridCols = `${sitW} ${colFractions}`;

        if (mktVis) {
          const mktRepeat = activeBottom.map(() => 'mkt').join(' ');
          const botRepeat = activeBottom.join(' ');
          gridAreas = `"${'situasi'} ${mktRepeat}" "${'situasi'} ${botRepeat}"`;
          gridRows = `${mktH} minmax(0, 1fr)`;
        } else {
          const botRepeat = activeBottom.join(' ');
          gridAreas = `"${'situasi'} ${botRepeat}"`;
          gridRows = 'minmax(0, 1fr)';
        }
      } else {
        if (mktVis) {
          gridCols = `${sitW} minmax(0, 1fr)`;
          gridAreas = '"situasi mkt"';
          gridRows = 'minmax(0, 1fr)';
        } else {
          gridCols = '1fr';
          gridAreas = '"situasi"';
          gridRows = '1fr';
        }
      }
    } else {
      if (bottomCols > 0) {
        const colFractions = activeBottom.map(id => {
          const fl = (state.panes[id] && state.panes[id].flex) || 1;
          return `minmax(0, ${fl}fr)`;
        }).join(' ');

        gridCols = colFractions;

        if (mktVis) {
          const mktRepeat = activeBottom.map(() => 'mkt').join(' ');
          const botRepeat = activeBottom.join(' ');
          gridAreas = `"${mktRepeat}" "${botRepeat}"`;
          gridRows = `${mktH} minmax(0, 1fr)`;
        } else {
          const botRepeat = activeBottom.join(' ');
          gridAreas = `"${botRepeat}"`;
          gridRows = 'minmax(0, 1fr)';
        }
      } else {
        if (mktVis) {
          gridCols = '1fr';
          gridAreas = '"mkt"';
          gridRows = '1fr';
        } else {
          gridCols = '1fr';
          gridAreas = '"situasi"';
          gridRows = '1fr';
        }
      }
    }

    consoleEl.style.setProperty('--grid-areas', gridAreas);
    consoleEl.style.setProperty('--grid-cols', gridCols);
    consoleEl.style.setProperty('--grid-rows', gridRows);

    updateResizers();
    updateMenuCheckboxes();
    triggerResize();
  }

  function triggerResize() {
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('resize'));
    });
  }

  function updateMenuCheckboxes() {
    const menu = document.getElementById('layout-menu');
    if (!menu) return;

    menu.querySelectorAll('input[data-toggle-pane]').forEach(cb => {
      const id = cb.dataset.togglePane;
      if (state.panes[id]) {
        cb.checked = state.panes[id].visible !== false;
      }
    });

    const sitW = (state.panes.situasi && state.panes.situasi.width) || '420px';
    menu.querySelectorAll('.lm-size-btn').forEach(btn => {
      btn.classList.toggle('is-active', btn.dataset.sitW === sitW);
    });
  }

  function updateResizers() {
    consoleEl.querySelectorAll('.resizer-v, .resizer-col, .resizer-h').forEach(r => r.remove());
    if (state.maximized) return;

    const consoleRect = consoleEl.getBoundingClientRect();
    const sitVis = state.panes.situasi && state.panes.situasi.visible !== false;
    const sitPane = document.getElementById('pane-situasi');

    if (sitVis && sitPane) {
      const resizer = document.createElement('div');
      resizer.className = 'resizer-v';
      resizer.setAttribute('title', 'Tahan dan geser untuk mengubah lebar kolom Situasi');
      consoleEl.appendChild(resizer);

      const updatePos = () => {
        const rect = sitPane.getBoundingClientRect();
        resizer.style.left = `${rect.right - consoleRect.left}px`;
      };
      updatePos();

      resizer.addEventListener('pointerdown', e => {
        e.preventDefault();
        resizer.classList.add('is-active');
        const startX = e.clientX;
        const initialWidth = sitPane.offsetWidth;

        const onPointerMove = moveEv => {
          const deltaX = moveEv.clientX - startX;
          const newWidth = Math.max(260, Math.min(initialWidth + deltaX, window.innerWidth * 0.65));
          state.panes.situasi.width = `${Math.round(newWidth)}px`;
          applyLayout();
        };

        const onPointerUp = () => {
          resizer.classList.remove('is-active');
          window.removeEventListener('pointermove', onPointerMove);
          window.removeEventListener('pointerup', onPointerUp);
          saveLayout();
        };

        window.addEventListener('pointermove', onPointerMove);
        window.addEventListener('pointerup', onPointerUp);
      });
    }

    const mktVis = state.panes.mkt && state.panes.mkt.visible !== false;
    const mktPane = document.getElementById('pane-mkt');
    const activeBottom = state.order.filter(id => state.panes[id] && state.panes[id].visible !== false);

    if (mktVis && mktPane && activeBottom.length > 0) {
      const resizerH = document.createElement('div');
      resizerH.className = 'resizer-h';
      resizerH.setAttribute('title', 'Tahan dan geser untuk mengubah tinggi panel Market');
      consoleEl.appendChild(resizerH);

      const updateHPos = () => {
        const rect = mktPane.getBoundingClientRect();
        resizerH.style.top = `${rect.bottom - consoleRect.top}px`;
        resizerH.style.left = `${rect.left - consoleRect.left}px`;
        resizerH.style.width = `${rect.width}px`;
      };
      updateHPos();

      resizerH.addEventListener('pointerdown', e => {
        e.preventDefault();
        resizerH.classList.add('is-active');
        const startY = e.clientY;
        const initialH = mktPane.offsetHeight;

        const onPointerMove = moveEv => {
          const deltaY = moveEv.clientY - startY;
          const maxH = Math.max(120, consoleEl.offsetHeight * 0.6);
          const newH = Math.max(50, Math.min(initialH + deltaY, maxH));
          if (!state.panes.mkt) state.panes.mkt = { visible: true };
          state.panes.mkt.height = `${Math.round(newH)}px`;
          applyLayout();
        };

        const onPointerUp = () => {
          resizerH.classList.remove('is-active');
          window.removeEventListener('pointermove', onPointerMove);
          window.removeEventListener('pointerup', onPointerUp);
          saveLayout();
        };

        window.addEventListener('pointermove', onPointerMove);
        window.addEventListener('pointerup', onPointerUp);
      });
    }

    if (activeBottom.length > 1) {
      for (let i = 0; i < activeBottom.length - 1; i++) {
        const idA = activeBottom[i];
        const idB = activeBottom[i + 1];
        const paneA = document.getElementById(`pane-${idA}`);
        const paneB = document.getElementById(`pane-${idB}`);
        if (!paneA || !paneB) continue;

        const resizerCol = document.createElement('div');
        resizerCol.className = 'resizer-col';
        resizerCol.setAttribute('title', 'Tahan dan geser untuk mengubah pembagian kolom');
        consoleEl.appendChild(resizerCol);

        const rectA = paneA.getBoundingClientRect();
        resizerCol.style.top = `${rectA.top - consoleRect.top}px`;
        resizerCol.style.height = `${rectA.height}px`;
        resizerCol.style.left = `${rectA.right - consoleRect.left}px`;

        resizerCol.addEventListener('pointerdown', e => {
          e.preventDefault();
          resizerCol.classList.add('is-active');
          const startX = e.clientX;
          const rectStartA = paneA.getBoundingClientRect();
          const rectStartB = paneB.getBoundingClientRect();
          const wA = rectStartA.width;
          const wB = rectStartB.width;
          const totalW = wA + wB;
          const flexA = Number((state.panes[idA] && state.panes[idA].flex) || 1);
          const flexB = Number((state.panes[idB] && state.panes[idB].flex) || 1);
          const totalFlex = flexA + flexB;

          const onPointerMove = moveEv => {
            const deltaX = moveEv.clientX - startX;
            const newWA = Math.max(140, Math.min(wA + deltaX, totalW - 140));
            const ratio = newWA / totalW;
            if (!state.panes[idA]) state.panes[idA] = { visible: true };
            if (!state.panes[idB]) state.panes[idB] = { visible: true };
            state.panes[idA].flex = parseFloat((totalFlex * ratio).toFixed(3));
            state.panes[idB].flex = parseFloat((totalFlex * (1 - ratio)).toFixed(3));
            applyLayout();
          };

          const onPointerUp = () => {
            resizerCol.classList.remove('is-active');
            window.removeEventListener('pointermove', onPointerMove);
            window.removeEventListener('pointerup', onPointerUp);
            saveLayout();
          };

          window.addEventListener('pointermove', onPointerMove);
          window.addEventListener('pointerup', onPointerUp);
        });
      }
    }
  }

  function bindPaneActions() {
    consoleEl.querySelectorAll('.pane[data-pane]').forEach(p => {
      const id = p.dataset.pane;

      const hideBtn = p.querySelector('.ph-hide');
      if (hideBtn) {
        hideBtn.addEventListener('click', e => {
          e.stopPropagation();
          if (state.panes[id]) {
            state.panes[id].visible = false;
            if (state.maximized === id) state.maximized = null;
            applyLayout();
            saveLayout();
          }
        });
      }

      const maxBtn = p.querySelector('.ph-max');
      if (maxBtn) {
        maxBtn.addEventListener('click', e => {
          e.stopPropagation();
          state.maximized = (state.maximized === id) ? null : id;
          applyLayout();
          saveLayout();
        });
      }

      const dragHandle = p.querySelector('.ph-drag');
      if (dragHandle) {
        dragHandle.setAttribute('draggable', 'true');

        dragHandle.addEventListener('dragstart', e => {
          e.dataTransfer.setData('text/plain', id);
          e.dataTransfer.effectAllowed = 'move';
          p.classList.add('is-dragging');
        });

        dragHandle.addEventListener('dragend', () => {
          p.classList.remove('is-dragging');
          consoleEl.querySelectorAll('.pane').forEach(el => el.classList.remove('drop-target'));
        });
      }

      p.addEventListener('dragover', e => {
        const dragging = consoleEl.querySelector('.pane.is-dragging');
        if (dragging && dragging !== p) {
          e.preventDefault();
          e.dataTransfer.dropEffect = 'move';
          p.classList.add('drop-target');
        }
      });

      p.addEventListener('dragleave', () => {
        p.classList.remove('drop-target');
      });

      p.addEventListener('drop', e => {
        e.preventDefault();
        p.classList.remove('drop-target');
        const srcId = e.dataTransfer.getData('text/plain');
        const targetId = id;

        if (srcId && targetId && srcId !== targetId) {
          reorderPanes(srcId, targetId);
        }
      });
    });
  }

  function reorderPanes(srcId, targetId) {
    const srcInBottom = state.order.indexOf(srcId);
    const targetInBottom = state.order.indexOf(targetId);

    if (srcInBottom !== -1 && targetInBottom !== -1) {
      state.order.splice(srcInBottom, 1);
      state.order.splice(targetInBottom, 0, srcId);
      applyLayout();
      saveLayout();
    }
  }

  function bindMenuControls() {
    const layoutWrap = document.getElementById('layout-wrap');
    const layoutBtn = document.getElementById('btn-layout');
    const layoutMenu = document.getElementById('layout-menu');
    if (!layoutBtn || !layoutMenu) return;

    layoutBtn.addEventListener('click', e => {
      e.stopPropagation();
      const open = !layoutMenu.hidden;
      layoutMenu.hidden = open;
      layoutBtn.setAttribute('aria-expanded', String(!open));
    });

    document.addEventListener('click', e => {
      if (!layoutWrap.contains(e.target)) {
        layoutMenu.hidden = true;
        layoutBtn.setAttribute('aria-expanded', 'false');
      }
    });

    layoutMenu.querySelectorAll('input[data-toggle-pane]').forEach(cb => {
      cb.addEventListener('change', () => {
        const id = cb.dataset.togglePane;
        if (state.panes[id]) {
          state.panes[id].visible = cb.checked;
          if (state.maximized === id && !cb.checked) state.maximized = null;
          applyLayout();
          saveLayout();
        }
      });
    });

    layoutMenu.querySelectorAll('.lm-preset-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const preset = btn.dataset.preset;
        if (preset === 'default') {
          state = structuredClone(DEFAULT_LAYOUT);
        } else if (preset === 'intel') {
          state.panes.situasi.visible = true;
          state.panes.situasi.width = '380px';
          state.panes.mkt.visible = true;
          state.panes.social.visible = true;
          state.panes.social.flex = 1.2;
          state.panes.wire.visible = true;
          state.panes.wire.flex = 1.6;
          state.panes.cctv.visible = false;
          state.order = ['social', 'wire', 'cctv'];
          state.maximized = null;
        } else if (preset === 'cctv') {
          state.panes.situasi.visible = false;
          state.panes.mkt.visible = false;
          state.panes.social.visible = false;
          state.panes.wire.visible = false;
          state.panes.cctv.visible = true;
          state.panes.cctv.flex = 1;
          state.maximized = 'cctv';
        } else if (preset === 'geo') {
          state.panes.situasi.visible = true;
          state.panes.situasi.width = '540px';
          state.panes.mkt.visible = true;
          state.panes.social.visible = false;
          state.panes.wire.visible = true;
          state.panes.cctv.visible = true;
          state.maximized = null;
        }
        applyLayout();
        saveLayout();
      });
    });

    layoutMenu.querySelectorAll('.lm-size-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const w = btn.dataset.sitW;
        if (w) {
          state.panes.situasi.width = w;
          applyLayout();
          saveLayout();
        }
      });
    });

    const resetBtn = document.getElementById('lm-reset');
    if (resetBtn) {
      resetBtn.addEventListener('click', () => {
        state = structuredClone(DEFAULT_LAYOUT);
        localStorage.removeItem(STORAGE_KEY);
        applyLayout();
      });
    }

    const perfTog = document.getElementById('perf-lite-tog');
    if (perfTog) {
      perfTog.addEventListener('change', () => {
        const isLite = perfTog.checked;
        document.documentElement.classList.toggle('is-lite', isLite);
        try {
          localStorage.setItem('sitrep_perf_mode', isLite ? 'lite' : 'full');
        } catch (_) {}
        if (typeof renderCams === 'function') renderCams();
      });
    }
  }

  function isDeviceLowSpec() {
    try {
      if (typeof navigator.hardwareConcurrency === 'number' && navigator.hardwareConcurrency <= 4) return true;
      if (typeof navigator.deviceMemory === 'number' && navigator.deviceMemory <= 4) return true;
      if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return true;
    } catch (_) {}
    return false;
  }

  function initPerfMode() {
    let active = false;
    try {
      const saved = localStorage.getItem('sitrep_perf_mode');
      if (saved === 'lite') active = true;
      else if (saved === 'full') active = false;
      else active = isDeviceLowSpec();
    } catch (_) {
      active = isDeviceLowSpec();
    }
    document.documentElement.classList.toggle('is-lite', active);
    const tog = document.getElementById('perf-lite-tog');
    if (tog) tog.checked = active;
  }

  window.addEventListener('DOMContentLoaded', () => {
    initPerfMode();
    bindPaneActions();
    bindMenuControls();
    applyLayout();
  });

  if (document.readyState !== 'loading') {
    initPerfMode();
    bindPaneActions();
    bindMenuControls();
    applyLayout();
  }

  window.addEventListener('resize', () => {
    updateResizers();
  });
})();

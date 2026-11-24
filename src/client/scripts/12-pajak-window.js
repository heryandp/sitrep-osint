let pjOpen = false;
let pjData = null;
let pjWin = '24h';
let pjScope = 'nasional';
let pjTheme = '';
let pjSrc = 'all';
let pjSort = 'priority';
let pjQuery = '';
let pjExpanded = new Set();
let pjBusy = false;
const pjExaCache = new Map();
const pjExaOpen = new Set();
const pjExaLoading = new Set();
let pjBriefOpen = false;
let pjBriefBusy = false;
let pjBriefData = null;
let pjBriefKey = '';

const PJ_DEFAULT_THEMES = [
  { id: 'ideologi', label: 'Ideologi', short: 'Ideologi', desc: 'Keadilan fiskal, nasionalisme pajak, gotong royong, dan narasi boikot/anti-pajak', color: '#fb7185', lightColor: '#be123c' },
  { id: 'politik', label: 'Politik', short: 'Politik', desc: 'Kebijakan DPR/Komisi XI, Menkeu, RUU/UU HPP, dan protes kebijakan tarif', color: '#60a5fa', lightColor: '#1d4ed8' },
  { id: 'sosbud', label: 'Sosial dan Budaya', short: 'Sosial & Budaya', desc: 'Kepatuhan sukarela, literasi/Tax Center, kepercayaan publik, UMKM, layanan KPP', color: '#fbbf24', lightColor: '#9a4208' },
  { id: 'ekonomi', label: 'Ekonomi', short: 'Ekonomi', desc: 'Penerimaan pajak, tax ratio, APBN, insentif fiskal, PPN/PPh, dan dampak industri', color: '#34d399', lightColor: '#047857' },
  { id: 'hukum', label: 'Hukum dan HAM', short: 'Hukum & HAM', desc: 'Pidana pajak, PPATK, sita aset, gijzeling, Pengadilan Pajak, dan hak wajib pajak', color: '#c084fc', lightColor: '#6d28d9' },
  { id: 'teknologi', label: 'Teknologi', short: 'Teknologi', desc: 'Coretax DJP, gangguan sistem, e-Faktur, e-Bupot, NIK–NPWP, dan keamanan siber', color: '#22d3ee', lightColor: '#0b637c' },
];
const pjThemeMeta = id => ((pjData && pjData.themes) || PJ_DEFAULT_THEMES).find(t => t.id === id) || PJ_DEFAULT_THEMES[0];

function renderPajakThemes(){
  const el = $('#pj-themes');
  if (!el) return;
  const list = (pjData && Array.isArray(pjData.themes) && pjData.themes.length) ? pjData.themes : PJ_DEFAULT_THEMES;
  el.innerHTML = list.map((t, i) => {
    const active = pjTheme === t.id;
    const total = Number(t.totalItems) || 0;
    const clN = Number(t.clusterCount) || 0;
    const nN = Number(t.newsCount) || 0;
    const sN = Number(t.socialCount) || 0;
    const urg = Number(t.urgentCount) || 0;
    const pct = Math.min(100, Math.max(total > 0 ? 8 : 0, Number(t.sharePct) || 0));
    const idx = String(i + 1).padStart(2, '0');
    const emptyCls = clN === 0 && !active ? ' is-empty' : '';
    return `<button class="pj-dim${emptyCls}" type="button" data-pj-theme="${esc(t.id)}" aria-pressed="${active ? 'true' : 'false'}" style="--dc:${esc(t.color)};--dc-l:${esc(t.lightColor || t.color)}" title="${esc(t.desc || t.label)}">
      <div class="pj-dim-top">
        <span class="pj-dim-k">${idx} · ${esc(t.label)}</span>
        ${urg ? `<span class="pj-dim-urg">${urg} atensi</span>` : ''}
      </div>
      <div class="pj-dim-mid">
        <b class="pj-dim-n">${clN}</b>
        <span class="pj-dim-sub">${total} item · ${nN} berita · ${sN} sosial</span>
      </div>
      <div class="pj-dim-bar" aria-hidden="true"><i style="width:${pct}%"></i></div>
      <div class="pj-dim-desc">${esc(t.desc || '')}</div>
    </button>`;
  }).join('');
}

function pjUrgLabel(u){
  return u === 'kritis' ? 'Kritis' : u === 'waspada' ? 'Waspada' : 'Pantau';
}

function renderPajakList(){
  const el = $('#pj-list');
  if (!el) return;
  const bar = $('#pj-active-bar');
  const lbl = $('#pj-active-lbl');
  if (bar && lbl) {
    if (pjTheme) {
      const m = pjThemeMeta(pjTheme);
      lbl.textContent = 'Filter Dimensi: ' + m.label + ' — ' + m.desc;
      bar.hidden = false;
    } else {
      bar.hidden = true;
    }
  }

  if (!pjData) {
    el.innerHTML = '<div class="empty wait">Mengelompokkan isu perpajakan ke 6 dimensi…</div>';
    return;
  }

  let list = (pjData.clusters || []).slice();
  if (pjTheme) {
    list = list.filter(c => c.theme === pjTheme || (c.secondaryThemes || []).includes(pjTheme));
  }
  if (pjSrc === 'news') list = list.filter(c => c.newsCount > 0);
  else if (pjSrc === 'social') list = list.filter(c => c.socialCount > 0);
  else if (pjSrc === 'cross') list = list.filter(c => c.crossVerified);

  const q = pjQuery.trim().toLowerCase();
  if (q) {
    list = list.filter(c => {
      const hay = [
        c.title, c.desc, c.place, c.prov, c.themeLabel,
        (c.keywords || []).join(' '),
        (c.newsItems || []).map(x => x.title + ' ' + x.src).join(' '),
        (c.socialItems || []).map(x => x.title + ' ' + x.src).join(' ')
      ].join(' ').toLowerCase();
      return hay.includes(q);
    });
  }

  if (pjSort === 'latest') list.sort((a, b) => (b.latestTs || 0) - (a.latestTs || 0));
  else if (pjSort === 'volume') list.sort((a, b) => (b.totalItems - a.totalItems) || (b.latestTs - a.latestTs));
  else list.sort((a, b) => (b.score - a.score) || (b.latestTs - a.latestTs));

  const totalRaw = (pjData.clusters || []).length;
  const matchEl = $('#pj-match-count');
  if (matchEl) {
    matchEl.textContent = list.length === totalRaw
      ? `${list.length} klaster`
      : `${list.length} dari ${totalRaw} klaster`;
  }

  if (!list.length) {
    const reasons = [];
    if (pjTheme) reasons.push('dimensi ' + pjThemeMeta(pjTheme).label);
    if (pjScope === 'nasional') reasons.push('cakupan Pajak Nasional (DJP)');
    else if (pjScope === 'daerah') reasons.push('cakupan Pajak Daerah');
    if (pjSrc === 'news') reasons.push('sumber Berita');
    else if (pjSrc === 'social') reasons.push('sumber Sosial');
    else if (pjSrc === 'cross') reasons.push('status Terkonfirmasi (Media + Sosial)');
    if (q) reasons.push('kata kunci "' + pjQuery.trim() + '"');
    const hasFilter = reasons.length > 0;
    const winLabel = pjWin === '2h' ? '2 jam terakhir' : pjWin === 'today' ? 'hari ini' : '24 jam terakhir';
    el.innerHTML = `<div class="pj-empty-box" role="status">
      <div class="pj-empty-t">${hasFilter ? 'Tidak ada klaster yang cocok dengan filter aktif' : 'Belum ada isu perpajakan pada rentang ' + esc(winLabel)}</div>
      <div class="pj-empty-sub">${hasFilter ? 'Filter aktif: ' + esc(reasons.join(' · ')) + ' (' + esc(winLabel) + ').' : 'Data diperbarui otomatis setiap menit dari 67 sumber berita dan media sosial.'}</div>
      <div class="pj-empty-acts">
        ${hasFilter ? '<button class="pj-empty-btn" type="button" data-pj-reset="1">Reset Semua Filter</button>' : '<button class="pj-empty-btn" type="button" data-pj-retry="1">Tarik Data Terbaru</button>'}
      </div>
    </div>`;
    return;
  }

  el.innerHTML = list.map(c => {
    const m = pjThemeMeta(c.theme);
    const open = pjExpanded.has(c.id);
    const secHtml = (c.secondaryThemes || []).map(sid => {
      const sm = pjThemeMeta(sid);
      return `<span class="pj-tag sec" style="--dc:${esc(sm.color)};--dc-l:${esc(sm.lightColor || sm.color)}">+ ${esc(sm.short || sm.label)}</span>`;
    }).join('');
    const kwHtml = (c.keywords || []).length
      ? `<div class="pj-kws">${c.keywords.map(k => `<span class="pj-kw">#${esc(k)}</span>`).join('')}</div>` : '';
    const hasGeo = typeof c.la === 'number' && typeof c.lo === 'number';
    const detailHtml = open ? `<div class="pj-detail">
      ${c.newsItems && c.newsItems.length ? `<div class="pj-det-sec">
        <div class="pj-det-h">Berita Media (${c.newsCount})</div>
        ${c.newsItems.map(n => `<div class="pj-det-row">
          <span class="pj-det-tm">${n.ts ? esc(wibTime(n.ts)) : '--:--'}</span>
          <button class="pj-det-a" type="button" data-pj-read="${escUrl(n.url)}" data-pj-kind="news">${esc(n.title)}</button>
          <span class="pj-det-src">${esc(n.src)}</span>
        </div>`).join('')}
      </div>` : ''}
      ${c.socialItems && c.socialItems.length ? `<div class="pj-det-sec">
        <div class="pj-det-h">Percakapan Sosial (${c.socialCount})</div>
        ${c.socialItems.map(s => `<div class="pj-det-row">
          <span class="pj-det-tm">${s.ts ? esc(wibTime(s.ts)) : '--:--'}</span>
          <button class="pj-det-a" type="button" data-pj-read="${escUrl(s.url)}" data-pj-kind="social" data-pj-platform="${esc(s.platform || 'Threads')}">${esc(s.title)}</button>
          <span class="pj-det-src">${esc(s.src)} · ${esc(s.platform || 'Sosial')}</span>
        </div>`).join('')}
      </div>` : ''}
    </div>` : '';

    const exaOpen = pjExaOpen.has(c.id);
    const exaLoading = pjExaLoading.has(c.id);
    const exaHit = pjExaCache.get(c.id);
    const exaHtml = exaOpen
      ? (exaLoading ? '<div class="exa-loading">Merangkum kejadian dengan Exa AI…</div>' : (exaHit ? exaCardHtml(exaHit) : ''))
      : '';
    return `<article class="pj-card${open ? ' is-open' : ''}" data-pj-id="${esc(c.id)}">
      <div class="pj-card-hd">
        <span class="pj-urg ${esc(c.urgency)}">${esc(pjUrgLabel(c.urgency))}</span>
        <span class="pj-scope ${esc(c.taxScope || 'nasional')}">${c.taxScope === 'daerah' ? 'Pajak Daerah' : 'Nasional · DJP'}</span>
        <span class="pj-tag" style="--dc:${esc(m.color)};--dc-l:${esc(m.lightColor || m.color)}">${esc(m.label)}</span>
        ${secHtml}
        ${c.crossVerified ? '<span class="pj-cross">Media + Sosial</span>' : ''}
        ${c.place ? `<span class="pj-loc">${esc(c.place)}</span>` : ''}
        <time class="pj-time">${c.latestTs ? esc(wibTime(c.latestTs)) + ' WIB' : ''}</time>
      </div>
      <button class="pj-title" type="button" data-pj-read="${escUrl(c.url)}" data-pj-kind="${esc(c.repKind || 'news')}">${esc(c.title)}</button>
      ${c.desc ? `<p class="pj-desc">${esc(c.desc)}</p>` : ''}
      ${kwHtml}
      <div class="pj-foot">
        ${c.newsCount ? `<span class="pj-stat news">Berita: ${c.newsCount}</span>` : ''}
        ${c.socialCount ? `<span class="pj-stat soc">Sosial: ${c.socialCount}</span>` : ''}
        <span class="pj-stat">${c.sourceCount} sumber</span>
        ${hasGeo ? `<button class="pj-btn pj-map-go" type="button" data-pj-lat="${c.la}" data-pj-lon="${c.lo}">Lihat di Peta</button>` : ''}
        <button class="pj-btn pj-exa-go" type="button" data-pj-exa="${esc(c.id)}" aria-expanded="${exaOpen ? 'true' : 'false'}">${exaLoading ? 'Merangkum…' : (exaOpen ? 'Tutup Exa' : 'Ringkas Exa')}</button>
        <button class="pj-btn pj-more" type="button" data-pj-toggle="${esc(c.id)}" aria-expanded="${open ? 'true' : 'false'}">${open ? 'Tutup Detail' : 'Detail (' + c.totalItems + ')'}</button>
      </div>
      ${exaHtml}
      ${detailHtml}
    </article>`;
  }).join('');
}

function renderPajakWindow(){
  renderPajakThemes();
  renderPajakList();
  const meta = $('#pj-meta');
  const pod = $('#pj-pod-n');
  const fs = $('#pj-foot-sync');
  const bNas = $('#pj-sc-nas');
  const bDae = $('#pj-sc-dae');
  if (pjData) {
    const nCl = Number(pjData.nasionalClusters) || 0;
    const dCl = Number(pjData.daerahClusters) || 0;
    if (bNas) bNas.textContent = 'Nasional (DJP) · ' + nCl;
    if (bDae) bDae.textContent = 'Daerah · ' + dCl;
    if (meta) meta.textContent = `${pjData.totalClusters} klaster · Nasional ${nCl} · Daerah ${dCl}`;
    if (pod) pod.textContent = nCl > 0 ? String(nCl) : (pjData.totalClusters > 0 ? String(pjData.totalClusters) : '6D');
    if (fs) fs.textContent = 'Sync ' + wibTime(pjData.at || Date.now()) + ' WIB';
  }
}

async function pullPajakClusters(fresh){
  if (API == null || pjBusy) return;
  pjBusy = true;
  const rBtn = $('#pj-refresh');
  if (rBtn) rBtn.dataset.busy = '1';
  try {
    const sc = (pjScope === 'nasional' || pjScope === 'daerah') ? '&scope=' + encodeURIComponent(pjScope) : '';
    const q = '?win=' + encodeURIComponent(pjWin) + sc + (fresh && SITREP_CAN_FRESH ? '&fresh=1' : '');
    const d = await getJson(API + '/api/pajak/clusters' + q, 25000);
    if (d && d.ok) {
      pjData = d;
      renderPajakWindow();
    }
  } catch (e) {
    if (!pjData) {
      const el = $('#pj-list');
      if (el) el.innerHTML = `<div class="pj-empty-box" role="alert">
        <div class="pj-empty-t">Gagal memuat klaster perpajakan</div>
        <div class="pj-empty-sub">Koneksi ke endpoint klastering sedang terganggu atau waktu permintaan habis.</div>
        <div class="pj-empty-acts"><button class="pj-empty-btn" type="button" data-pj-retry="1">Coba Muat Ulang</button></div>
      </div>`;
    }
  } finally {
    pjBusy = false;
    if (rBtn) delete rBtn.dataset.busy;
  }
}

function openPajakWindow(){
  if (typeof mapOpen !== 'undefined' && mapOpen && typeof closeMap === 'function') closeMap();
  const wrap = $('#pajakwrap');
  if (!wrap) return;
  wrap.classList.add('on');
  wrap.inert = false;
  wrap.setAttribute('aria-hidden', 'false');
  const btn = $('#btn-pajak');
  if (btn) btn.setAttribute('aria-expanded', 'true');
  pjOpen = true;
  renderPajakWindow();
  pullPajakClusters(false);
  setTimeout(() => { const cl = $('#pj-close'); if (cl) cl.focus(); }, 60);
}

function closePajakWindow(){
  const wrap = $('#pajakwrap');
  if (!wrap) return;
  wrap.classList.remove('on');
  wrap.inert = true;
  wrap.setAttribute('aria-hidden', 'true');
  const btn = $('#btn-pajak');
  if (btn) {
    btn.setAttribute('aria-expanded', 'false');
    btn.focus();
  }
  pjOpen = false;
}

function togglePajakWindow(){
  if (pjOpen) closePajakWindow();
  else openPajakWindow();
}

(function initPajakWindow(){
  const btn = $('#btn-pajak');
  if (btn) btn.addEventListener('click', togglePajakWindow);
  const cls = $('#pj-close');
  if (cls) cls.addEventListener('click', closePajakWindow);
  const ref = $('#pj-refresh');
  if (ref) ref.addEventListener('click', () => pullPajakClusters(true));

  const winEl = $('#pj-win');
  if (winEl) {
    winEl.addEventListener('click', e => {
      const b = e.target.closest('.pj-wb[data-win]');
      if (!b || b.dataset.win === pjWin) return;
      pjWin = b.dataset.win;
      winEl.querySelectorAll('.pj-wb').forEach(x => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
      pullPajakClusters(false);
    });
  }

  const scEl = $('#pj-scope');
  if (scEl) {
    scEl.addEventListener('click', e => {
      const b = e.target.closest('.pj-sb[data-scope]');
      if (!b || b.dataset.scope === pjScope) return;
      pjScope = b.dataset.scope;
      scEl.querySelectorAll('.pj-sb').forEach(x => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
      pullPajakClusters(false);
    });
  }

  const thEl = $('#pj-themes');
  if (thEl) {
    thEl.addEventListener('click', e => {
      const b = e.target.closest('.pj-dim[data-pj-theme]');
      if (!b) return;
      const id = b.dataset.pjTheme;
      pjTheme = pjTheme === id ? '' : id;
      renderPajakWindow();
    });
  }

  const clrBtn = $('#pj-clear-theme');
  if (clrBtn) {
    clrBtn.addEventListener('click', () => {
      pjTheme = '';
      renderPajakWindow();
    });
  }

  const qEl = $('#pj-q');
  if (qEl) {
    qEl.addEventListener('input', debounce(() => {
      pjQuery = qEl.value || '';
      renderPajakList();
    }, 140));
  }

  const srcEl = $('#pj-src-filter');
  if (srcEl) {
    srcEl.addEventListener('click', e => {
      const b = e.target.closest('.pj-sb[data-src]');
      if (!b) return;
      pjSrc = b.dataset.src;
      srcEl.querySelectorAll('.pj-sb').forEach(x => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
      renderPajakList();
    });
  }

  const sortEl = $('#pj-sort');
  if (sortEl) {
    sortEl.addEventListener('click', e => {
      const b = e.target.closest('.pj-sb[data-sort]');
      if (!b) return;
      pjSort = b.dataset.sort;
      sortEl.querySelectorAll('.pj-sb').forEach(x => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
      renderPajakList();
    });
  }

  const listEl = $('#pj-list');
  if (listEl) {
    listEl.addEventListener('click', e => {
      const rst = e.target.closest('[data-pj-reset]');
      if (rst) {
        const needPull = pjScope !== 'all';
        pjTheme = '';
        pjSrc = 'all';
        pjScope = 'all';
        pjQuery = '';
        const qIn = $('#pj-q');
        if (qIn) qIn.value = '';
        const sc = $('#pj-scope');
        if (sc) sc.querySelectorAll('.pj-sb').forEach(x => x.setAttribute('aria-pressed', x.dataset.scope === 'all' ? 'true' : 'false'));
        const sf = $('#pj-src-filter');
        if (sf) sf.querySelectorAll('.pj-sb').forEach(x => x.setAttribute('aria-pressed', x.dataset.src === 'all' ? 'true' : 'false'));
        if (needPull) pullPajakClusters(false);
        else renderPajakWindow();
        return;
      }
      const rty = e.target.closest('[data-pj-retry]');
      if (rty) {
        pullPajakClusters(true);
        return;
      }
      const exaBtn = e.target.closest('[data-pj-exa]');
      if (exaBtn) {
        const cid = exaBtn.dataset.pjExa;
        if (pjExaOpen.has(cid) && !pjExaLoading.has(cid)) {
          pjExaOpen.delete(cid);
          renderPajakList();
          return;
        }
        pjExaOpen.add(cid);
        if (pjExaCache.has(cid)) {
          renderPajakList();
          return;
        }
        const cl = pjData && (pjData.clusters || []).find(x => x.id === cid);
        if (!cl || API == null || pjExaLoading.has(cid)) return;
        pjExaLoading.add(cid);
        renderPajakList();
        const urls = [cl.url, ...((cl.newsItems || []).map(n => n && n.url))].filter(Boolean).slice(0, 4);
        postJson(API + '/api/pajak/summarize', {
          clusterId: cl.id,
          urls,
          title: cl.title || '',
          desc: cl.desc || '',
          themeLabel: cl.themeLabel || ''
        }, 25000).then(res => {
          if (res && res.ok) pjExaCache.set(cid, res);
          else pjExaCache.set(cid, { ok: true, mode: 'fallback', ringkasan: cl.desc || cl.title || 'Ringkasan tidak tersedia.', dampak_djp: '', entitas_kunci: [], atensi: '' });
        }).catch(() => {
          pjExaCache.set(cid, { ok: true, mode: 'fallback', ringkasan: cl.desc || cl.title || 'Gagal menghubungi Exa AI.', dampak_djp: '', entitas_kunci: [], atensi: '' });
        }).finally(() => {
          pjExaLoading.delete(cid);
          renderPajakList();
        });
        return;
      }
      const tog = e.target.closest('[data-pj-toggle]');
      if (tog) {
        const id = tog.dataset.pjToggle;
        if (pjExpanded.has(id)) pjExpanded.delete(id);
        else pjExpanded.add(id);
        renderPajakList();
        return;
      }
      const goMap = e.target.closest('[data-pj-lat][data-pj-lon]');
      if (goMap) {
        const la = Number(goMap.dataset.pjLat);
        const lo = Number(goMap.dataset.pjLon);
        closePajakWindow();
        if (typeof layerOn !== 'undefined') {
          layerOn.pajak = true;
          const pb = document.querySelector('[data-layer="pajak"]');
          if (pb) pb.setAttribute('aria-pressed', 'true');
        }
        if (typeof openMap === 'function') openMap();
        setTimeout(() => {
          if (typeof map !== 'undefined' && map && isFinite(la) && isFinite(lo)) {
            map.setView([la, lo], 10, { animate: true });
          }
        }, 450);
        return;
      }
      const rd = e.target.closest('[data-pj-read]');
      if (rd) {
        const url = rd.dataset.pjRead;
        if (!url || url === '#') return;
        const kind = rd.dataset.pjKind;
        if (kind === 'social' && typeof rdOpenPost === 'function') {
          const card = rd.closest('.pj-card');
          const cl = pjData && (pjData.clusters || []).find(x => x.id === (card && card.dataset.pjId));
          const hit = cl && (cl.socialItems || []).find(s => s.url === url);
          if (hit) {
            rdOpenPost({
              url: hit.url,
              text: hit.text || hit.title,
              name: hit.author || hit.src,
              username: hit.handle || '',
              handle: hit.src || '',
              ts: hit.ts
            }, (hit.platform || '').toLowerCase() === 'x' ? 'x' : 'threads');
            return;
          }
        }
        if (typeof rdOpen === 'function') {
          rdOpen(url, null, true, '').catch(() => window.open(url, '_blank', 'noopener'));
        } else {
          window.open(url, '_blank', 'noopener');
        }
      }
    });
  }

  const briefBtn = $('#pj-brief-btn');
  const briefBox = $('#pj-exa-brief');
  function renderPajakBrief(){
    if (!briefBox || !briefBtn) return;
    briefBtn.setAttribute('aria-expanded', pjBriefOpen ? 'true' : 'false');
    if (!pjBriefOpen) { briefBox.hidden = true; return; }
    briefBox.hidden = false;
    if (pjBriefBusy) {
      briefBox.innerHTML = '<div class="exa-loading">Menyusun briefing eksekutif Exa AI dari berita pajak teratas…</div>';
      return;
    }
    if (!pjBriefData) {
      briefBox.innerHTML = '<div class="exa-loading">Belum ada briefing. Klik Briefing Exa AI untuk memulai.</div>';
      return;
    }
    const d = pjBriefData;
    const isExa = d.mode === 'exa';
    const badge = isExa ? (d.cached ? 'Exa AI · Cache' : 'Exa AI · Live') : 'Ringkasan Lokal';
    const rows = (d.highlights || []).map(h => `<div class="pj-br-item">
      <div class="pj-br-t">${h.url ? `<button class="pj-det-a" type="button" data-pj-brief-read="${escUrl(h.url)}">${esc(h.title || 'Berita Pajak')}</button>` : esc(h.title || '')}</div>
      <p class="pj-br-p">${esc(h.ringkasan || '')}</p>
      ${h.dampak_djp ? `<div class="pj-br-imp"><b>Dampak DJP:</b> ${esc(h.dampak_djp)}</div>` : ''}
    </div>`).join('');
    const ents = Array.isArray(d.entitas_kunci) && d.entitas_kunci.length
      ? `<div class="exa-ents">${d.entitas_kunci.map(e => `<span class="exa-ent">${esc(e)}</span>`).join('')}</div>` : '';
    briefBox.innerHTML = `<div class="pj-br-hd">
      <span class="exa-k">${esc(d.headline || 'Briefing Eksekutif DJP')}</span>
      <span class="exa-mode${isExa ? ' live' : ''}">${esc(badge)}</span>
      <button class="pj-clear-btn" type="button" id="pj-brief-close">Tutup</button>
    </div>
    <div class="pj-br-list">${rows}</div>
    ${ents}`;
  }
  if (briefBtn) {
    briefBtn.addEventListener('click', async () => {
      const k = pjWin + '|' + pjScope + '|' + pjTheme;
      if (pjBriefOpen && pjBriefKey === k && !pjBriefBusy) {
        pjBriefOpen = false;
        renderPajakBrief();
        return;
      }
      pjBriefOpen = true;
      if (pjBriefData && pjBriefKey === k) {
        renderPajakBrief();
        return;
      }
      if (API == null || pjBriefBusy) return;
      pjBriefBusy = true;
      pjBriefKey = k;
      renderPajakBrief();
      try {
        const res = await postJson(API + '/api/pajak/brief', { win: pjWin, scope: pjScope, theme: pjTheme }, 28000);
        if (res && res.ok) pjBriefData = res;
      } catch (e) {
        pjBriefData = { ok: true, mode: 'fallback', headline: 'Gagal menyusun briefing', highlights: [{ title: 'Gangguan sementara', ringkasan: 'Coba klik Briefing Exa AI kembali sebentar lagi.' }], entitas_kunci: [] };
      } finally {
        pjBriefBusy = false;
        renderPajakBrief();
      }
    });
  }
  if (briefBox) {
    briefBox.addEventListener('click', e => {
      if (e.target.closest('#pj-brief-close')) {
        pjBriefOpen = false;
        renderPajakBrief();
        return;
      }
      const rd = e.target.closest('[data-pj-brief-read]');
      if (rd && typeof rdOpen === 'function') {
        const u = rd.dataset.pjBriefRead;
        if (u && u !== '#') rdOpen(u, null, true, '').catch(() => window.open(u, '_blank', 'noopener'));
      }
    });
  }

  schedulePoll(() => {
    if (pjOpen || !pjData) pullPajakClusters(false);
  }, 60000, { dedupGlobalMs: 15000 });
})();

let djpUptimeData = null;
let djpUptimeBusy = false;

function djpUpChipHtml(it){
  const st = it.state || (it.status === 'up' ? 'up' : 'down');
  const lbl = st === 'down' ? 'DOWN' : (st === 'slow' ? 'LAMBAT' : 'UP');
  const msText = it.responseMs != null ? (it.responseMs >= 1000 ? (it.responseMs / 1000).toFixed(1) + 's' : it.responseMs + 'ms') : (it.status === 'up' ? 'OK' : 'ERR');
  const tipParts = [
    it.domain,
    'Status: ' + lbl,
    it.httpCode ? 'HTTP ' + it.httpCode : null,
    it.responseMs != null ? 'Latency ' + it.responseMs + 'ms' : null,
    it.ip ? 'IP ' + it.ip : null,
    it.error ? 'Error: ' + it.error : null,
    it.checkedAt ? 'Dicek ' + wibTime(it.checkedAt) + ' WIB' : null
  ].filter(Boolean);
  const hb = Array.isArray(it.history) && it.history.length
    ? `<span class="up-hb" aria-hidden="true">${it.history.slice(-6).map(h => `<i class="${esc(h)}"></i>`).join('')}</span>` : '';
  return `<a class="up-chip is-${esc(st)}" href="${escUrl(it.url)}" target="_blank" rel="noopener noreferrer" title="${esc(tipParts.join(' · '))}">
    <i class="up-dot" aria-hidden="true"></i>
    <span class="up-name">${esc(it.name)}</span>
    <span class="up-st">${esc(lbl)}</span>
    <span class="up-ms">${esc(msText)}</span>
    ${hb}
  </a>`;
}

function renderDjpUptime(){
  const mainList = $('#djp-up-list');
  const pjBar = $('#pj-uptime');
  if (!djpUptimeData || !Array.isArray(djpUptimeData.items)) return;
  const chips = djpUptimeData.items.map(djpUpChipHtml).join('');
  if (mainList) mainList.innerHTML = chips;
  if (pjBar) {
    const downN = Number(djpUptimeData.downCount) || 0;
    const sumLbl = downN > 0 ? `${downN} GANGGUAN` : `${djpUptimeData.upCount}/${djpUptimeData.total} UP`;
    pjBar.innerHTML = `<span class="up-hd${downN > 0 ? ' is-alert' : ''}">Portal · ${esc(sumLbl)}</span><div class="up-list">${chips}</div><button class="up-ref" type="button" data-pj-up-ref="1" title="Cek ulang status portal via isdownapi.com" aria-label="Cek ulang status portal"><svg class="ic" aria-hidden="true"><use href="#i-sync"></use></svg><span>Cek</span></button>`;
  }
}

async function pullDjpUptime(fresh){
  if (API == null || djpUptimeBusy) return;
  djpUptimeBusy = true;
  const b1 = $('#djp-up-ref');
  const b2 = document.querySelector('[data-pj-up-ref]');
  if (b1) b1.dataset.busy = '1';
  if (b2) b2.dataset.busy = '1';
  try {
    const q = fresh && SITREP_CAN_FRESH ? '?fresh=1' : '';
    const d = await getJson(API + '/api/pajak/uptime' + q, 25000);
    if (d && d.ok && Array.isArray(d.items)) {
      djpUptimeData = d;
      renderDjpUptime();
    }
  } catch (e) {
    const mainList = $('#djp-up-list');
    if (mainList && !djpUptimeData) mainList.innerHTML = '<span class="up-wait">Gagal mengecek status portal. Klik Cek untuk mengulang.</span>';
  } finally {
    djpUptimeBusy = false;
    if (b1) delete b1.dataset.busy;
    if (b2) delete b2.dataset.busy;
  }
}

(function initDjpUptime(){
  const b1 = $('#djp-up-ref');
  if (b1) b1.addEventListener('click', () => pullDjpUptime(true));
  const pjBar = $('#pj-uptime');
  if (pjBar) {
    pjBar.addEventListener('click', e => {
      if (e.target.closest('[data-pj-up-ref]')) pullDjpUptime(true);
    });
  }
  schedulePoll(() => pullDjpUptime(false), 95000, { dedupGlobalMs: 30000 });
})();

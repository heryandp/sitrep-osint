'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const SOC = require('../src/social');
const ASSESS = require('../src/assessment');
const CLUSTER = require('../src/social-cluster');
const GEO = require('../src/geocode');

const ROOT = path.resolve(__dirname, '..');

test('Perpajakan / DJP / Coretax topic registry, classification, watchlist, POI, and UI priority', async (t) => {
  await t.test('coretax and pajak topics sit at priority front of SOC_TOPICS and map to pajak category', () => {
    assert.equal(SOC.SOC_TOPICS[0].id, 'coretax');
    assert.equal(SOC.SOC_TOPICS[1].id, 'pajak');
    assert.equal(ASSESS.CATEGORY.coretax, 'pajak');
    assert.equal(ASSESS.CATEGORY.pajak, 'pajak');
    assert.equal(ASSESS.FEED_ONLY.has('coretax'), false, 'coretax must be a full signal topic');
    assert.equal(ASSESS.FEED_ONLY.has('pajak'), false, 'pajak must be a full signal topic');
  });

  await t.test('Coretax outages and DJP tax news classify as signalEligible pajak events', () => {
    const coretaxRes = SOC.socAssess('Coretax error gagal login dan e-Faktur tidak bisa terbitkan faktur pajak hari ini di Jakarta');
    assert.ok(coretaxRes.topics.includes('coretax'), 'must tag coretax');
    assert.equal(coretaxRes.signalEligible, true, 'Coretax outage must be signalEligible');
    assert.ok(coretaxRes.events.some(e => e.category === 'pajak' && e.type === 'coretax_outage'), 'must emit coretax_outage event');

    const djpRes = SOC.socAssess('DJP Raup Tambahan Penerimaan Pajak Rp898,99 Miliar dari Program PSA di Jakarta');
    assert.ok(djpRes.topics.includes('pajak'), 'must tag pajak');
    assert.equal(djpRes.signalEligible, true, 'DJP revenue report must be signalEligible');
    assert.ok(djpRes.events.some(e => e.category === 'pajak'), 'must emit pajak category event');

    const spamRes = SOC.socAssess('Pajak jadian makan bareng temen habis nonton film drakor');
    assert.equal(spamRes.topics.includes('pajak'), false, 'colloquial joke pajak jadian must be rejected');
  });

  await t.test('Official DJP / Kemenkeu handles receive official_agency provenance', () => {
    assert.ok(SOC.SOC_WATCH_X.includes('ditjenpajakri'));
    assert.ok(SOC.SOC_WATCH_X.includes('kring_pajak'));
    assert.ok(SOC.SOC_WATCH_X.includes('kemenkeuri'));
    for (const handle of ['DitjenPajakRI', 'kring_pajak', 'KemenkeuRI']) {
      const prov = CLUSTER.classifySourceProvenance({ handle, text: 'Info pemeliharaan Coretax DJP' });
      assert.equal(prov.sourceType, 'official_agency', `${handle} must be official_agency`);
    }
  });

  await t.test('DJP, Kemenkeu, and Pengadilan Pajak POIs resolve cleanly via GEO.locate', () => {
    const djp = GEO.locate('Antrean wajib pajak di Kantor Pusat DJP hari ini');
    assert.ok(djp && djp.name === 'Kantor Pusat DJP', 'Kantor Pusat DJP must resolve');
    const kmk = GEO.locate('Rapat penerimaan pajak di Kementerian Keuangan');
    assert.ok(kmk && kmk.name === 'Kementerian Keuangan', 'Kementerian Keuangan must resolve');
    const pp = GEO.locate('Sidang sengketa faktur pajak di Pengadilan Pajak');
    assert.ok(pp && pp.name === 'Pengadilan Pajak', 'Pengadilan Pajak must resolve');
  });

  await t.test('Frontend compiled index.html places pajak at priority position 0 with icon and Night/Light colors', () => {
    const html = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
    assert.ok(html.includes("const CAT_ORDER = ['pajak',"), 'CAT_ORDER must place pajak first');
    assert.ok(html.includes("const GEO_SHINGLE_ORDER = ['pajak',"), 'GEO_SHINGLE_ORDER must place pajak first');
    assert.ok(html.includes('id="i-tk-pajak"'), 'SVG sprite must include #i-tk-pajak');
    assert.ok(html.includes("pajak:'#i-tk-pajak'"), 'TK_SIG must map pajak to #i-tk-pajak');
    assert.ok(html.includes('html[data-theme="light"] .tk[data-cat="pajak"]'), 'Light mode CSS must style pajak category');
    assert.ok(html.includes('data-layer="pajak"'), 'Peta Insiden must expose dedicated Pajak layer button');
    assert.ok(html.includes('pajakLayer = L.layerGroup()') && html.includes('class="pin-pajak"'), 'Leaflet map must render dedicated pajakLayer with .pin-pajak markers');
    assert.ok(html.includes('function initHScroll(') && html.includes('function initAllHScrolls('), 'Client must wire horizontal scroll/slide controls for clipped tab bars');
  });
});
